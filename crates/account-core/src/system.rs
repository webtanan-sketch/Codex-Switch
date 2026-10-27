//! Windows process integration kept behind a small adapter.
use crate::*;
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn validate_exe(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || !path.is_file()
        || !path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("exe"))
    {
        return Err(Error::Executable);
    }
    Ok(())
}
pub fn discover_vscode() -> Option<PathBuf> {
    for key in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(key) {
            for suffix in [
                "Programs/Microsoft VS Code/Code.exe",
                "Microsoft VS Code/Code.exe",
            ] {
                let path = PathBuf::from(&base).join(suffix);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    #[cfg(windows)]
    if let Ok(out) = Command::new("where.exe").arg("code").output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let cli = PathBuf::from(line.trim());
            if let Some(root) = cli.parent().and_then(Path::parent) {
                let exe = root.join("Code.exe");
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
    }
    None
}
pub fn discover_codex() -> Option<PathBuf> {
    // npm's codex.cmd is not run through a shell. Locate its native binary instead.
    fn find(root: &Path, depth: usize) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        for entry in std::fs::read_dir(root).ok()?.flatten() {
            let path = entry.path();
            if entry.file_type().ok()?.is_symlink() {
                continue;
            }
            if path
                .file_name()
                .is_some_and(|s| s.eq_ignore_ascii_case("codex.exe"))
            {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(p) = find(&path, depth - 1) {
                    return Some(p);
                }
            }
        }
        None
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join("codex.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    std::env::var_os("APPDATA")
        .and_then(|base| find(&PathBuf::from(base).join("npm/node_modules/@openai"), 10))
}
pub(crate) fn clean_env(cmd: &mut Command) {
    // Authentication overrides would defeat profile selection in the child process.
    for key in [
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "CODEX_ACCESS_TOKEN",
        "OPENAI_ACCESS_TOKEN",
        "OPENAI_IDENTITY_TOKEN_FILE",
        "OPENAI_FEDERATION_ISSUER",
        // Code.exe must start Electron's GUI, not inherit the caller's Node host.
        "ELECTRON_RUN_AS_NODE",
        "ELECTRON_NO_ASAR",
        "NODE_OPTIONS",
        "NODE_PATH",
        "VSCODE_IPC_HOOK_CLI",
        "VSCODE_PID",
        "VSCODE_CWD",
    ] {
        cmd.env_remove(key);
    }
}
const CODEX_EXTENSION: &str = "openai.chatgpt";
fn editor_command(exe: &Path, home: &Path, data: &Path, extensions: &Path) -> Command {
    let mut cmd = Command::new(exe);
    clean_env(&mut cmd);
    // A terminal opened by VS Code can inherit routing to the original instance.
    cmd.env_remove("VSCODE_DEV")
        .env_remove("VSCODE_IPC_HOOK_CLI")
        .env_remove("ELECTRON_RUN_AS_NODE")
        .env("CODEX_HOME", home)
        .arg("--user-data-dir")
        .arg(data)
        .arg("--extensions-dir")
        .arg(extensions);
    cmd
}
fn run_editor_cli(
    exe: &Path,
    home: &Path,
    data: &Path,
    extensions: &Path,
    args: &[&str],
) -> Result<()> {
    let cli = exe
        .parent()
        .ok_or(Error::Executable)?
        .join("resources/app/out/cli.js");
    if !cli.is_file() {
        return Err(Error::EditorSetup);
    }
    let mut cmd = Command::new(exe);
    clean_env(&mut cmd);
    cmd.env_remove("VSCODE_DEV")
        .env_remove("VSCODE_IPC_HOOK_CLI")
        .env("ELECTRON_RUN_AS_NODE", "1")
        .env("CODEX_HOME", home)
        .arg(cli)
        .arg("--user-data-dir")
        .arg(data)
        .arg("--extensions-dir")
        .arg(extensions)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|_| Error::EditorSetup)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        if let Some(status) = child.try_wait().map_err(|_| Error::EditorSetup)? {
            return if status.success() {
                Ok(())
            } else {
                Err(Error::EditorSetup)
            };
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::EditorSetup);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
fn codex_installed(extensions: &Path) -> bool {
    std::fs::read_dir(extensions)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let manifest = entry.path().join("package.json");
            let Ok(bytes) = std::fs::read(manifest) else {
                return false;
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                return false;
            };
            value["publisher"]
                .as_str()
                .is_some_and(|p| p.eq_ignore_ascii_case("openai"))
                && value["name"].as_str() == Some("chatgpt")
        })
}
pub fn launch_vscode(exe: &Path, home: &Path, data: &Path) -> Result<()> {
    validate_exe(exe)?;
    profile_vault::private_dir(data)?;
    let extensions = data.join("extensions");
    profile_vault::private_dir(&extensions)?;
    // Register Codex in this exact isolated environment, not the user's default one.
    if !codex_installed(&extensions) {
        run_editor_cli(
            exe,
            home,
            data,
            &extensions,
            &["--install-extension", CODEX_EXTENSION],
        )?;
        if !codex_installed(&extensions) {
            return Err(Error::EditorSetup);
        }
    }
    let mut child = editor_command(exe, home, data, &extensions)
        .arg("--new-window")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Spawn success alone used to report a window even when Electron exited
    // immediately (for example, after inheriting ELECTRON_RUN_AS_NODE).
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(800);
    while std::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(Error::Launch)
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Ok(())
}
pub fn launch_login(exe: &Path, home: &Path) -> Result<()> {
    validate_exe(exe)?;
    let mut cmd = Command::new(exe);
    clean_env(&mut cmd);
    cmd.env("CODEX_HOME", home)
        .args(["-c", "cli_auth_credentials_store=\"file\"", "login"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE);
    }
    cmd.spawn()?;
    Ok(())
}
pub fn guard_auth(path: &Path) -> Result<Option<File>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE);
    }
    Ok(Some(
        options.open(path).map_err(|_| Error::ConcurrentWrite)?,
    ))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub windows: bool,
    pub home: String,
    pub auth_present: bool,
    pub config_present: bool,
    pub storage: codex_auth::CredentialStore,
    pub vscode_path: Option<String>,
    pub codex_path: Option<String>,
    pub legacy_maintenance_present: bool,
    pub legacy_profiles_present: bool,
    pub recovery_pending: bool,
}
pub fn diagnostics(manager: &Manager, user: &Path) -> Result<Diagnostics> {
    let settings = manager.settings()?;
    Ok(Diagnostics {
        windows: cfg!(windows),
        home: manager.home.display().to_string(),
        auth_present: manager.home.join("auth.json").is_file(),
        config_present: manager.home.join("config.toml").is_file(),
        storage: manager.storage()?,
        vscode_path: settings
            .vscode_path
            .or_else(|| discover_vscode().map(|p| p.display().to_string())),
        codex_path: settings
            .codex_path
            .or_else(|| discover_codex().map(|p| p.display().to_string())),
        legacy_maintenance_present: user.join(".codex-switch/maintenance/v1").exists(),
        legacy_profiles_present: user.join(".codex-switch/profiles.json").is_file()
            && user.join(".codex-switch/profiles").is_dir(),
        recovery_pending: manager.root.join("transaction.json").exists(),
    })
}

#[cfg(test)]
mod editor_tests {
    use super::*;
    #[test]
    fn installation_is_checked_by_manifest_not_folder_name() {
        let dir = tempfile::tempdir().unwrap();
        let extension = dir.path().join("openai.chatgpt-fake");
        std::fs::create_dir(&extension).unwrap();
        assert!(!codex_installed(dir.path()));
        std::fs::write(
            extension.join("package.json"),
            br#"{"publisher":"other","name":"chatgpt"}"#,
        )
        .unwrap();
        assert!(!codex_installed(dir.path()));
        std::fs::write(
            extension.join("package.json"),
            br#"{"publisher":"OpenAI","name":"chatgpt"}"#,
        )
        .unwrap();
        assert!(codex_installed(dir.path()));
    }
    #[test]
    fn setup_failure_does_not_touch_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Code.exe");
        std::fs::write(&exe, b"TEST_ONLY_NOT_EXECUTED").unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let auth = home.join("auth.json");
        std::fs::write(&auth, b"TEST_ONLY_UNCHANGED").unwrap();
        assert!(matches!(
            launch_vscode(&exe, &home, &dir.path().join("data")),
            Err(Error::EditorSetup)
        ));
        assert_eq!(std::fs::read(auth).unwrap(), b"TEST_ONLY_UNCHANGED");
    }
    #[test]
    fn launch_keeps_account_and_extension_directories_together() {
        let cmd = editor_command(
            Path::new("Code.exe"),
            Path::new("account-home"),
            Path::new("account-data"),
            Path::new("account-data/extensions"),
        );
        let args: Vec<_> = cmd
            .get_args()
            .map(|v| v.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            args,
            [
                "--user-data-dir",
                "account-data",
                "--extensions-dir",
                "account-data/extensions"
            ]
        );
        let env: Vec<_> = cmd.get_envs().collect();
        assert!(env
            .iter()
            .any(|(key, value)| *key == "VSCODE_IPC_HOOK_CLI" && value.is_none()));
        assert!(env.iter().any(|(key, value)| *key == "CODEX_HOME"
            && *value == Some(std::ffi::OsStr::new("account-home"))));
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn child_launch_drops_electron_node_mode() {
        let shell = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
        let mut command = Command::new(shell);
        command.env("ELECTRON_RUN_AS_NODE", "1").args([
            "/d",
            "/c",
            "if defined ELECTRON_RUN_AS_NODE (exit /b 17) else (exit /b 0)",
        ]);

        clean_env(&mut command);
        let status = command.status().expect("run child environment probe");

        assert_eq!(
            status.code(),
            Some(0),
            "Code.exe must not inherit ELECTRON_RUN_AS_NODE, which prevents an editor window from opening"
        );
    }

    #[test]
    fn child_launch_drops_node_options() {
        let shell = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
        let mut command = Command::new(shell);
        command.env("NODE_OPTIONS", "--tls-max-v1.2").args([
            "/d",
            "/c",
            "if defined NODE_OPTIONS (exit /b 18) else (exit /b 0)",
        ]);

        clean_env(&mut command);
        let status = command.status().expect("run child environment probe");

        assert_eq!(
            status.code(),
            Some(0),
            "Code.exe must not inherit NODE_OPTIONS from the account manager's parent process"
        );
    }
}
