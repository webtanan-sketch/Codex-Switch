mod service;
pub mod system;
use serde::{Deserialize, Serialize};
pub use service::*;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Auth(#[from] codex_auth::AuthError),
    #[error(transparent)]
    Vault(#[from] profile_vault::VaultError),
    #[error("A local file operation failed. No credential details are logged.")]
    Io,
    #[error("Another operation is running. Try again shortly.")]
    Busy,
    #[error("Import the current account before switching; its identity is not saved.")]
    UnknownActive,
    #[error("Profile was not found or its encrypted credentials are damaged.")]
    Profile,
    #[error("Codex changed authentication during the operation. Retry after Codex is idle.")]
    ConcurrentWrite,
    #[error("Credential storage must explicitly be configured as file. Review System Check.")]
    StorageMode,
    #[error("Authentication verification failed. Previous credentials have been restored.")]
    Verification,
    #[error("Recovery needs attention. Preserved encrypted credentials are available; no further changes were made.")]
    Recovery,
    #[error("Choose a valid executable installed on this computer.")]
    Executable,
    #[error("The login is not complete. Finish Codex login in the opened terminal.")]
    LoginPending,
    #[error("Profile name must have 1–80 characters and no control characters.")]
    Name,
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub identity: codex_auth::Identity,
    pub created_at: u64,
    pub last_used_at: Option<u64>,
    pub last_synced_at: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub profiles: Vec<Profile>,
    pub active_profile_id: Option<String>,
    pub previous_profile_id: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub vscode_path: Option<String>,
    pub codex_path: Option<String>,
}
fn json_read<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match fs::read(path) {
        Ok(b) => serde_json::from_slice(&b).map_err(|_| Error::Recovery),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(_) => Err(Error::Io),
    }
}
fn json_write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    profile_vault::atomic_write(
        path,
        &serde_json::to_vec_pretty(value).map_err(|_| Error::Io)?,
    )?;
    Ok(())
}
/// Only structured, allowlisted events are written; no arbitrary error strings.
pub fn audit(root: &Path, operation: &str, ok: bool) {
    use std::io::Write;
    let allowed = [
        "switch", "sync", "import", "recovery", "watcher", "launch", "login",
    ];
    let op = if allowed.contains(&operation) {
        operation
    } else {
        "operation"
    };
    let path = root.join("app.log");
    if fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024) {
        let _ = fs::rename(&path, root.join("app.previous.log"));
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} operation={} success={}", now(), op, ok);
    }
}
/// Defense in depth for future diagnostics; never log payloads even after redaction.
pub fn redact(input: &str) -> String {
    if input.contains('{')
        || [
            "access_token",
            "refresh_token",
            "id_token",
            "authorization",
            "bearer ",
        ]
        .iter()
        .any(|k| input.to_ascii_lowercase().contains(k))
    {
        return "[REDACTED sensitive message]".into();
    }
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)(?:[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}|sk-[a-z0-9_-]+|(?:refresh|oauth)[_-][a-z0-9_-]+)").unwrap()).replace_all(input, "[REDACTED]").into_owned()
}
