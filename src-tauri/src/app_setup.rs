use std::sync::Arc;

use tauri::Manager;

use crate::config::ConfigManager;
use crate::encryption::EncryptionManager;
use crate::monitor::MonitorContext;
use crate::security::AppLockManager;
use crate::storage::Database;
use crate::sync::webdav::WebDavSyncManager;
use crate::sync::SyncManager;
use crate::updater::UpdaterManager;
use crate::{
    emit_initial_lock_state, handle_main_window_close, handle_main_window_focus, hotkey, monitor,
    security, tray, AppDataDir,
};

/// Main setup logic extracted from the Tauri `.setup()` closure.
///
/// Initialises logging, configuration, database, encryption, sync managers,
/// security runtime, hotkey, tray, window events, clipboard monitor and
/// performs the initial cleanup.
pub(crate) fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    setup_logging(app)?;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");

    app.manage(Arc::new(AppDataDir(app_data_dir.clone())));

    // --- Configuration & managers ---

    let config_manager = Arc::new(ConfigManager::new(app_data_dir.clone()));
    let config = config_manager.get();
    app.manage(config_manager.clone());

    let app_lock_manager = Arc::new(AppLockManager::new(config_manager.clone()));
    app.manage(app_lock_manager.clone());

    let updater_manager = Arc::new(UpdaterManager::new(env!("CARGO_PKG_VERSION").to_string()));
    updater_manager
        .restore_from_disk(&app_data_dir, env!("CARGO_PKG_VERSION"))
        .ok();
    app.manage(updater_manager.clone());

    // --- Database ---

    let db_path = app_data_dir.join("clipboard.db");
    let db =
        Arc::new(Database::new(&db_path.to_string_lossy()).expect("Failed to initialize database"));
    app.manage(db.clone());

    // --- Encryption ---

    let encryption_manager = Arc::new(EncryptionManager::new(config_manager.clone()));
    app.manage(encryption_manager.clone());

    // --- Sync managers ---

    let sync_manager = SyncManager::new(db.clone(), config_manager.clone());
    sync_manager.set_app_handle(app.handle().clone());
    app.manage(sync_manager.clone());

    let webdav_config = config.webdav.clone();
    let local_info = sync_manager.local_device_info();
    let webdav_manager = WebDavSyncManager::new(
        db.clone(),
        webdav_config,
        &local_info.device_id,
        &local_info.device_name,
        &local_info.public_key,
    );
    webdav_manager.set_app_handle(app.handle().clone());
    app.manage(webdav_manager.clone());

    // --- Security runtime ---

    security::attach_lock_runtime(app.handle(), app_lock_manager.clone());

    // --- Desktop integration ---

    if let Err(e) = hotkey::setup_hotkey(app.handle()) {
        log::error!("Failed to setup hotkey: {}", e);
    }

    if let Err(e) = tray::setup_tray(app.handle()) {
        log::error!("Failed to setup tray: {}", e);
    }

    setup_window_events(app, &app_lock_manager);

    // --- Image storage directory ---

    let images_dir = app_data_dir.join("images");
    std::fs::create_dir_all(&images_dir).ok();

    // --- Initial state broadcasts ---

    emit_initial_lock_state(app.handle(), &app_lock_manager);
    updater_manager.emit_status(app.handle());

    // --- Clipboard monitor ---

    let monitor_ctx = MonitorContext {
        app_handle: app.handle().clone(),
        db: db.clone(),
        config_manager: config_manager.clone(),
        encryption_manager,
        sync_manager,
        webdav_manager,
        images_dir,
    };
    monitor::start_clipboard_monitor(monitor_ctx, config.monitor_interval_ms);

    // --- Initial cleanup ---

    let db_cleanup = db.clone();
    let max_entries = config.max_entries;
    tauri::async_runtime::spawn(async move {
        let _ = db_cleanup.delete_expired();
        let _ = db_cleanup.delete_oldest_beyond_limit(max_entries);
    });

    Ok(())
}

/// Build and register the log plugin with filters that suppress noisy
/// mdns_sd errors on macOS awdl0 interface.
fn setup_logging(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let log_builder = tauri_plugin_log::Builder::default()
        .level(log::LevelFilter::Info)
        .level_for("mdns_sd::service_daemon", log::LevelFilter::Warn)
        .filter(|metadata| {
            if metadata.target().starts_with("mdns_sd") && metadata.level() == log::Level::Error {
                return false;
            }
            true
        });

    app.handle().plugin(log_builder.build())?;
    Ok(())
}

/// Bind window close (hide-to-tray) and focus (lock-state emit) events.
fn setup_window_events(app: &mut tauri::App, app_lock_manager: &Arc<AppLockManager>) {
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        let app_handle = app.handle().clone();
        let lock_manager = app_lock_manager.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                handle_main_window_close(&window_clone, lock_manager.as_ref());
            }
            if let tauri::WindowEvent::Focused(focused) = event {
                handle_main_window_focus(&app_handle, lock_manager.as_ref(), *focused);
            }
        });
    }
}
