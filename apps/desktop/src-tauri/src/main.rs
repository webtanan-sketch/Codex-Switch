#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use account_core::{system, Manager, Profile, Settings};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tauri::{Emitter, Manager as _};
use vscode_ipc::{Broadcaster, ProfileEvent};
struct App {
    manager: Manager,
    bus: Broadcaster,
    user: PathBuf,
    ipc_ready: bool,
}
fn err(e: account_core::Error) -> String {
    e.to_string()
}
fn broadcast(app: &App, p: Option<&Profile>, kind: &str) {
    app.bus.publish(ProfileEvent {
        version: 1,
        kind: kind.into(),
        home_key: vscode_ipc::home_key(&app.manager.home),
        profile_id: p.map(|p| p.id.clone()),
        profile_name: p.map(|p| p.name.clone()),
        timestamp: account_core::now(),
    });
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    state: account_core::State,
    diagnostics: system::Diagnostics,
    settings: Settings,
    ipc_ready: bool,
}
#[tauri::command]
fn snapshot(app: tauri::State<App>) -> Result<Snapshot, String> {
    let mut state = app.manager.state().map_err(err)?;
    // Fingerprints stay in Rust; the frontend receives no raw account identifiers.
    for p in &mut state.profiles {
        p.identity.fingerprint.clear();
    }
    Ok(Snapshot {
        state,
        diagnostics: system::diagnostics(&app.manager, &app.user).map_err(err)?,
        settings: app.manager.settings().map_err(err)?,
        ipc_ready: app.ipc_ready,
    })
}
fn refresh_tray(handle: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    let app = handle.state::<App>();
    let menu = Menu::new(handle)?;
    for p in app.manager.state().map(|s| s.profiles).unwrap_or_default() {
        let active = app
            .manager
            .state()
            .ok()
            .and_then(|s| s.active_profile_id)
            .as_deref()
            == Some(&p.id);
        menu.append(&MenuItem::with_id(
            handle,
            format!("switch:{}", p.id),
            format!("{} {}", if active { "●" } else { "○" }, p.name),
            true,
            None::<&str>,
        )?)?;
        menu.append(&MenuItem::with_id(
            handle,
            format!("open:{}", p.id),
            format!("Open {} in VS Code", p.name),
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&MenuItem::with_id(
        handle,
        "manager",
        "Open Manager / تنظیمات",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        handle,
        "exit",
        "Exit / خروج",
        true,
        None::<&str>,
    )?)?;
    if let Some(tray) = handle.tray_by_id("manager") {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}
#[tauri::command]
async fn switch_profile(handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let copy = handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app = copy.state::<App>();
        let p = app.manager.switch(&id).map_err(err)?;
        broadcast(&app, Some(&p), "PROFILE_CHANGED");
        let _ = refresh_tray(&copy);
        let _ = copy.emit("profiles-updated", ());
        Ok(())
    })
    .await
    .map_err(|_| "Background operation failed".to_string())?
}
#[tauri::command]
fn import_current(
    handle: tauri::AppHandle,
    app: tauri::State<App>,
    name: String,
) -> Result<(), String> {
    let p = app.manager.import_current(&name).map_err(err)?;
    broadcast(&app, Some(&p), "STATUS");
    let _ = refresh_tray(&handle);
    Ok(())
}
#[tauri::command]
fn configure_file_storage(app: tauri::State<App>) -> Result<(), String> {
    app.manager.configure_file_storage().map_err(err)
}
#[tauri::command]
fn save_settings(app: tauri::State<App>, settings: Settings) -> Result<(), String> {
    app.manager.save_settings(settings).map_err(err)
}
fn open_code(app: &App, id: &str) -> Result<(), String> {
    let home = app.manager.prepare_isolated(id).map_err(err)?;
    let exe = app
        .manager
        .settings()
        .map_err(err)?
        .vscode_path
        .map(PathBuf::from)
        .or_else(system::discover_vscode)
        .ok_or("VS Code executable was not found")?;
    system::launch_vscode(&exe, &home, &app.manager.hub.join("vscode-data").join(id)).map_err(err)
}
#[tauri::command]
async fn open_vscode(handle: tauri::AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_code(&handle.state::<App>(), &id))
        .await
        .map_err(|_| "VS Code setup task failed".to_string())?
}
#[tauri::command]
fn begin_login(app: tauri::State<App>) -> Result<String, String> {
    let exe = app
        .manager
        .settings()
        .map_err(err)?
        .codex_path
        .map(PathBuf::from)
        .or_else(system::discover_codex)
        .ok_or("Codex native executable was not found; select codex.exe in Settings")?;
    let (id, home) = app.manager.prepare_login().map_err(err)?;
    system::launch_login(&exe, &home).map_err(err)?;
    Ok(id)
}
#[tauri::command]
fn finish_login(
    handle: tauri::AppHandle,
    app: tauri::State<App>,
    id: String,
    name: String,
) -> Result<(), String> {
    app.manager.finish_login(&id, &name).map_err(err)?;
    let _ = refresh_tray(&handle);
    Ok(())
}
#[tauri::command]
async fn restore_previous(handle: tauri::AppHandle) -> Result<(), String> {
    let id = handle
        .state::<App>()
        .manager
        .state()
        .map_err(err)?
        .previous_profile_id
        .ok_or("No previous profile is saved")?;
    switch_profile(handle, id).await
}
#[tauri::command]
fn recover(app: tauri::State<App>) -> Result<(), String> {
    app.manager.recover_startup().map_err(err)
}
#[tauri::command]
fn repair(app: tauri::State<App>) -> Result<(), String> {
    app.manager.repair_active().map_err(err)
}
#[tauri::command]
fn reset_state(app: tauri::State<App>) -> Result<(), String> {
    app.manager.reset_state().map_err(err)
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|handle, _, _| {
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|handle| {
            let user = PathBuf::from(
                std::env::var_os("USERPROFILE").ok_or("USERPROFILE is required (Windows only)")?,
            );
            let root = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA is required")?)
                .join("CodexAccountManager");
            let home =
                codex_auth::resolve_home(&user, std::env::var("CODEX_HOME").ok().as_deref())?;
            let manager = Manager::new(
                root,
                home,
                user.join(".codex-hub"),
                Arc::new(profile_vault::Dpapi),
            )?;
            let recovery = manager.recover_startup();
            if recovery.is_err() {
                account_core::audit(&manager.root, "recovery", false);
            }
            let bus = Broadcaster::default();
            let ipc_ready = bus.start().is_ok();
            let app = App {
                manager: manager.clone(),
                bus,
                user,
                ipc_ready,
            };
            let state = manager.state()?;
            let p = state
                .profiles
                .iter()
                .find(|p| Some(&p.id) == state.active_profile_id.as_ref());
            broadcast(&app, p, "STATUS");
            handle.manage(app);
            let mut tray =
                tauri::tray::TrayIconBuilder::with_id("manager").tooltip("Codex Account Manager");
            if let Some(icon) = handle.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|handle, event| {
                let id = event.id.as_ref();
                if id == "exit" {
                    handle.exit(0);
                } else if id == "manager" {
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                } else if let Some(profile) = id.strip_prefix("switch:") {
                    let profile = profile.to_owned();
                    let copy = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(message) = switch_profile(copy.clone(), profile).await {
                            let _ = copy.emit("operation-error", message);
                            if let Some(w) = copy.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    });
                } else if let Some(profile) = id.strip_prefix("open:") {
                    let profile = profile.to_owned();
                    let copy = handle.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        if let Err(message) = open_code(&copy.state::<App>(), &profile) {
                            let _ = copy.emit("operation-error", message);
                            if let Some(w) = copy.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    });
                }
            })
            .build(handle)?;
            refresh_tray(handle.handle())?;
            let handle = handle.handle().clone();
            std::thread::spawn(move || {
                let mut observed = HashMap::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    match manager.sync_files(&mut observed) {
                        Ok(true) => {
                            let _ = handle.emit("profiles-updated", ());
                        }
                        Err(account_core::Error::Busy) => (),
                        Err(_) => account_core::audit(&manager.root, "watcher", false),
                        _ => (),
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            switch_profile,
            import_current,
            configure_file_storage,
            save_settings,
            open_vscode,
            begin_login,
            finish_login,
            restore_previous,
            recover,
            repair,
            reset_state
        ])
        .run(tauri::generate_context!())
        .expect("Codex Account Manager could not start");
}
