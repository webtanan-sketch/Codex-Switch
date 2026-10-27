//! Optional GUI smoke check. Uses an empty Codex home and dedicated VS Code
//! user data, so no saved account or global auth is selected by this probe.
use account_core::system;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?.join("target/launch-smoke");
    let home = root.join("home");
    profile_vault::private_dir(&home)?;
    profile_vault::atomic_write(
        &home.join("config.toml"),
        b"cli_auth_credentials_store = \"file\"\n",
    )?;
    let exe = system::discover_vscode().ok_or("VS Code executable not found")?;
    system::launch_vscode(&exe, &home, &root.join("vscode-data"))?;
    println!(
        "Launched VS Code with a dedicated empty test home: {}",
        root.display()
    );
    Ok(())
}
