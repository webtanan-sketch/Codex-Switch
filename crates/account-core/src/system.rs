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
fn clean_env(cmd: &mut Command) {
    // Authentication overrides would defeat profile selection in the child process.
    for key in [
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "CODEX_ACCESS_TOKEN",
        "OPENAI_ACCESS_TOKEN",
        "OPENAI_IDENTITY_TOKEN_FILE",
        "OPENAI_FEDERATION_ISSUER",
    ] {
        cmd.env_remove(key);
    }
}
pub fn launch_vscode(exe: &Path, home: &Path, data: &Path) -> Result<()> {
    validate_exe(exe)?;
    profile_vault::private_dir(data)?;
    let mut cmd = Command::new(exe);
    clean_env(&mut cmd);
    cmd.env("CODEX_HOME", home)
        .args(["--new-window", "--user-data-dir"])
        .arg(data)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
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
