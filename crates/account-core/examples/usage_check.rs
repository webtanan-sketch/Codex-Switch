//! Optional live check: reads only the current registered account's quota.
//! Never prints authentication payloads, emails, account IDs, or raw RPC errors.
use account_core::{system, Manager};
use std::{path::PathBuf, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = PathBuf::from(std::env::var_os("USERPROFILE").ok_or("Windows user required")?);
    let root = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA required")?)
        .join("CodexAccountManager");
    let home = codex_auth::resolve_home(&user, std::env::var("CODEX_HOME").ok().as_deref())?;
    let config = std::fs::read(home.join("config.toml")).ok();
    let manager = Manager::new(
        root,
        home.clone(),
        user.join(".codex-hub"),
        Arc::new(profile_vault::Dpapi),
    )?;
    let before = manager.usage_profile_id()?;
    let exe = manager
        .settings()?
        .codex_path
        .map(PathBuf::from)
        .or_else(system::discover_codex)
        .ok_or("Native Codex CLI not found")?;
    let usage = manager.read_usage(&exe)?;
    println!(
        "{}",
        serde_json::json!({
            "windows":usage.windows,
            "ordinaryUsageAllowed":usage.ordinary_usage_allowed,
            "identityUnchanged":before == manager.usage_profile_id()?,
            "configurationUnchanged":config == std::fs::read(home.join("config.toml")).ok()
        })
    );
    Ok(())
}
