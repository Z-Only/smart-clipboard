pub mod analyzer;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod platform;
pub mod security;
pub mod storage;
pub mod sync;
pub mod templates;
pub mod tray;
pub mod updater;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Local};
use image::ImageBuffer;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tokio::sync::mpsc;

use analyzer::{classify, detect_sensitive};
use clipboard::ClipboardMonitor;
use config::ConfigManager;
use security::AppLockManager;
use storage::{ClipboardEntry, Database};
use sync::webdav::WebDavSyncManager;
use sync::SyncManager;
use updater::UpdaterManager;

/// Managed state holding the app data directory path.
pub struct AppDataDir(pub PathBuf);

pub(crate) fn emit_initial_lock_state<R: Runtime>(
    app_handle: &AppHandle<R>,
    lock_manager: &AppLockManager,
) {
    security::emit_lock_state(app_handle, lock_manager);
}

pub(crate) fn handle_main_window_focus<R: Runtime>(
    app_handle: &AppHandle<R>,
    lock_manager: &AppLockManager,
    focused: bool,
) {
    if focused {
        security::emit_lock_state(app_handle, lock_manager);
    }
}

pub(crate) fn handle_main_window_close<R: Runtime>(
    window: &WebviewWindow<R>,
    lock_manager: &AppLockManager,
) {
    let _ = window.hide();
    lock_manager.record_activity();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_entries,
            commands::search_entries,
            commands::delete_entry,
            commands::delete_entries,
            commands::copy_entries,
            commands::set_favorite_state_for_entries,
            commands::toggle_favorite,
            commands::get_entry_count,
            commands::get_statistics,
            commands::paste_entry,
            commands::get_updater_status,
            commands::check_for_updates_now,
            commands::download_available_update,
            commands::install_pending_update,
            commands::discard_pending_update,
            commands::quit_app,
            commands::get_config,
            commands::update_config,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::transform_content,
            commands::create_tag,
            commands::delete_tag,
            commands::get_all_tags,
            commands::add_tag_to_entry,
            commands::remove_tag_from_entry,
            commands::get_entry_tags,
            commands::set_tags_for_entries,
            commands::get_entries_by_tag,
            commands::get_sync_status,
            commands::get_sync_config,
            commands::update_sync_config,
            commands::get_discovered_devices,
            commands::get_paired_devices,
            commands::pair_device,
            commands::unpair_device,
            commands::toggle_device_sync,
            templates::commands::create_template,
            templates::commands::update_template,
            templates::commands::delete_template,
            templates::commands::get_templates,
            templates::commands::get_template,
            templates::commands::use_template,
            templates::commands::get_template_categories,
            templates::commands::get_template_placeholders,
            commands::webdav_connect,
            commands::webdav_disconnect,
            commands::webdav_get_status,
            commands::webdav_get_config,
            commands::webdav_update_config,
            commands::webdav_trigger_sync,
            commands::webdav_remove_device,
            commands::get_app_lock_status,
            commands::set_app_lock_password,
            commands::update_app_lock_settings,
            commands::lock_app,
            commands::unlock_app,
        ])
        .setup(|app| {
            {
                // Build the log plugin with a custom filter that suppresses
                // noisy mdns_sd errors on macOS awdl0 interface.
                // "Network is down (os error 50)" is expected and harmless.
                let log_builder = tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .level_for("mdns_sd::service_daemon", log::LevelFilter::Warn)
                    .filter(|metadata| {
                        // Also filter via the callback for additional safety
                        if metadata.target().starts_with("mdns_sd")
                            && metadata.level() == log::Level::Error
                        {
                            return false;
                        }
                        true
                    });

                app.handle().plugin(log_builder.build())?;
            }

            // Determine app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Store app_data_dir as managed state for commands
            app.manage(Arc::new(AppDataDir(app_data_dir.clone())));

            // Initialize config
            let config_manager = Arc::new(ConfigManager::new(app_data_dir.clone()));
            let config = config_manager.get();
            app.manage(config_manager.clone());

            let app_lock_manager = Arc::new(AppLockManager::new(config_manager.clone()));
            app.manage(app_lock_manager.clone());

            let updater_manager =
                Arc::new(UpdaterManager::new(env!("CARGO_PKG_VERSION").to_string()));
            updater_manager
                .restore_from_disk(&app_data_dir, env!("CARGO_PKG_VERSION"))
                .ok();
            app.manage(updater_manager.clone());

            // Initialize database
            let db_path = app_data_dir.join("clipboard.db");
            let db = Arc::new(
                Database::new(&db_path.to_string_lossy()).expect("Failed to initialize database"),
            );
            app.manage(db.clone());

            let sync_manager = SyncManager::new(db.clone(), config_manager.clone());
            sync_manager.set_app_handle(app.handle().clone());
            app.manage(sync_manager.clone());

            // Initialize WebDAV sync manager
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

            security::attach_lock_runtime(app.handle(), app_lock_manager.clone());

            // Setup hotkey
            if let Err(e) = hotkey::setup_hotkey(app.handle()) {
                log::error!("Failed to setup hotkey: {}", e);
            }

            // Setup system tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                log::error!("Failed to setup tray: {}", e);
            }

            // Handle window close: hide to tray instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                let app_handle = app.handle().clone();
                let lock_manager = app_lock_manager.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        handle_main_window_close(&w, lock_manager.as_ref());
                    }
                    if let tauri::WindowEvent::Focused(focused) = event {
                        handle_main_window_focus(&app_handle, lock_manager.as_ref(), *focused);
                    }
                });
            }

            // Create images directory for image clipboard support
            let images_dir = app_data_dir.join("images");
            std::fs::create_dir_all(&images_dir).ok();

            emit_initial_lock_state(app.handle(), &app_lock_manager);
            updater_manager.emit_status(app.handle());

            // Start clipboard monitor
            let (tx, mut rx) = mpsc::unbounded_channel();
            let monitor = ClipboardMonitor::new(config.monitor_interval_ms);
            monitor.start(tx);

            // Process incoming clipboard changes
            let db_for_rx = db.clone();
            let app_handle = app.handle().clone();
            let config_for_rx = config_manager.clone();
            let images_dir_for_rx = images_dir.clone();
            let sync_for_rx = sync_manager.clone();
            let webdav_for_rx = webdav_manager.clone();

            tauri::async_runtime::spawn(async move {
                while let Some(change) = rx.recv().await {
                    // Capture frontmost application as source_app
                    let source_app = platform::get_frontmost_app();

                    // Check if the source app is in the excluded list
                    let excluded_apps = config_for_rx.get().excluded_apps;
                    if let Some(ref app) = source_app {
                        if excluded_apps.iter().any(|excluded| {
                            app.to_lowercase().contains(&excluded.to_lowercase())
                                || excluded.to_lowercase().contains(&app.to_lowercase())
                        }) {
                            log::debug!("Skipping clipboard from excluded app: {}", app);
                            continue;
                        }
                    }

                    let is_image = change.content_type == "image";

                    // For images, hash the raw RGBA bytes; for text, hash the content string
                    let hash = if is_image {
                        if let Some(ref img_data) = change.image_data {
                            format!("{:x}", Sha256::digest(&img_data.bytes))
                        } else {
                            continue; // image change without data - skip
                        }
                    } else {
                        format!("{:x}", Sha256::digest(change.content.as_bytes()))
                    };

                    // Check deduplication
                    match db_for_rx.find_by_hash(&hash) {
                        Ok(Some(_)) => {
                            let _ = db_for_rx.update_use_count(&hash);
                            continue;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::error!("DB error during dedup check: {}", e);
                            continue;
                        }
                    }

                    let now = Local::now().naive_local();

                    // Handle image vs text differently
                    let (content, category, is_sensitive, expires_at) = if is_image {
                        // Save image as PNG file
                        let img_data = change.image_data.as_ref().unwrap();
                        let png_path = images_dir_for_rx.join(format!("{}.png", &hash));

                        match ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                            img_data.width as u32,
                            img_data.height as u32,
                            img_data.bytes.clone(),
                        ) {
                            Some(img) => {
                                if let Err(e) = img.save(&png_path) {
                                    log::error!("Failed to save image: {}", e);
                                    continue;
                                }
                            }
                            None => {
                                log::error!("Failed to create image buffer from RGBA data");
                                continue;
                            }
                        }

                        let content = png_path.to_string_lossy().to_string();
                        // Images: skip classification and sensitive detection
                        (content, "image".to_string(), false, None)
                    } else {
                        // Text content: classify and detect sensitive
                        let category = classify(&change.content);
                        let is_sensitive = detect_sensitive(&change.content);
                        let expires_at = if is_sensitive {
                            let expiry_minutes = config_for_rx.get().sensitive_expiry_minutes;
                            if expiry_minutes > 0 {
                                Some(now + Duration::minutes(expiry_minutes as i64))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        (
                            change.content.clone(),
                            category.as_str().to_string(),
                            is_sensitive,
                            expires_at,
                        )
                    };

                    let entry = ClipboardEntry {
                        id: None,
                        content,
                        content_type: change.content_type,
                        category,
                        hash,
                        source_app,
                        is_favorite: false,
                        is_sensitive,
                        use_count: 1,
                        created_at: now,
                        updated_at: now,
                        expires_at,
                        source_device: None,
                    };

                    match db_for_rx.insert_entry(&entry) {
                        Ok(id) => {
                            let mut stored_entry = entry;
                            stored_entry.id = Some(id);
                            let _ = app_handle.emit("clipboard-changed", &stored_entry);
                            // Broadcast to paired devices via sync pipeline
                            sync_for_rx.broadcast_entry(&stored_entry);
                            // Push to WebDAV cloud sync
                            let webdav = webdav_for_rx.clone();
                            let entry_for_webdav = stored_entry.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = webdav.push_entry(&entry_for_webdav).await {
                                    log::error!("WebDAV push error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            log::error!("Failed to insert clipboard entry: {}", e);
                        }
                    }
                }
            });

            // Run initial cleanup
            let db_cleanup = db.clone();
            let max_entries = config.max_entries;
            tauri::async_runtime::spawn(async move {
                let _ = db_cleanup.delete_expired();
                let _ = db_cleanup.delete_oldest_beyond_limit(max_entries);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod runtime_tests {
    use super::{emit_initial_lock_state, handle_main_window_close, handle_main_window_focus};
    use crate::config::ConfigManager;
    use crate::security::{
        self, emit_auto_lock_if_needed, AppLockManager, AppLockStatus, SetPasswordPayload,
        UpdateAppLockSettingsPayload,
    };
    use serde::Deserialize;
    use std::path::PathBuf;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tauri::test::{self, MockRuntime};
    use tauri::{App, Listener, Manager, WebviewWindow, WebviewWindowBuilder};

    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should move forward")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "smart-clipboard-lib-runtime-tests-{}-{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("failed to create temp test dir");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct TestKeyringGuard;

    impl TestKeyringGuard {
        fn new() -> Self {
            security::install_test_keyring_store();
            security::set_test_biometric_result(None);
            Self
        }
    }

    impl Drop for TestKeyringGuard {
        fn drop(&mut self) {
            security::set_test_biometric_result(None);
            security::reset_test_keyring_store();
        }
    }

    #[derive(Debug, Deserialize)]
    struct LockStatusEventPayload {
        status: AppLockStatus,
    }

    struct TestHarness {
        _app: App<MockRuntime>,
        window: WebviewWindow<MockRuntime>,
        lock: Arc<AppLockManager>,
        app_lock_status_rx: Receiver<AppLockStatus>,
    }

    impl TestHarness {
        fn new(base_dir: PathBuf) -> Self {
            let config = Arc::new(ConfigManager::new(base_dir));
            let lock = Arc::new(AppLockManager::new(config));
            Self::with_lock(lock)
        }

        fn new_startup_locked(base_dir: PathBuf) -> Self {
            let config = Arc::new(ConfigManager::new(base_dir));
            let bootstrap = AppLockManager::new(config.clone());
            bootstrap
                .set_password(SetPasswordPayload {
                    current_password: None,
                    new_password: "phase4-pass".to_string(),
                })
                .expect("setting password should succeed");
            bootstrap
                .update_settings(UpdateAppLockSettingsPayload {
                    enabled: true,
                    auto_lock_seconds: 0,
                    biometric_enabled: false,
                })
                .expect("enabling app lock should succeed");

            let lock = Arc::new(AppLockManager::new(config));
            Self::with_lock(lock)
        }

        fn with_lock(lock: Arc<AppLockManager>) -> Self {
            let app = test::mock_builder()
                .manage(lock.clone())
                .build(test::mock_context(test::noop_assets()))
                .expect("failed to build mock app");

            let window = WebviewWindowBuilder::new(&app, "main", Default::default())
                .build()
                .expect("failed to build mock webview");

            let (app_lock_tx, app_lock_status_rx) = std::sync::mpsc::sync_channel(8);
            app.listen_any("app-lock-status", move |event| {
                let payload: LockStatusEventPayload =
                    serde_json::from_str(event.payload()).expect("lock payload should be json");
                let _ = app_lock_tx.send(payload.status);
            });

            Self {
                _app: app,
                window,
                lock,
                app_lock_status_rx,
            }
        }

        fn configure_password(&self) {
            self.lock
                .set_password(SetPasswordPayload {
                    current_password: None,
                    new_password: "phase4-pass".to_string(),
                })
                .expect("setting password should succeed");
        }

        fn configure_auto_lock(&self, enabled: bool, auto_lock_seconds: u64) {
            self.lock
                .update_settings(UpdateAppLockSettingsPayload {
                    enabled,
                    auto_lock_seconds,
                    biometric_enabled: false,
                })
                .expect("updating app lock settings should succeed");
        }

        fn app_handle(&self) -> tauri::AppHandle<MockRuntime> {
            self.window.app_handle().clone()
        }
    }

    fn recv_lock_status(rx: &Receiver<AppLockStatus>) -> AppLockStatus {
        rx.recv_timeout(Duration::from_millis(200))
            .expect("expected lock status event")
    }

    fn assert_no_lock_status(rx: &Receiver<AppLockStatus>) {
        assert!(
            rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "did not expect lock status event"
        );
    }

    #[test]
    fn startup_locked_app_emits_current_lock_status() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new_startup_locked(temp_dir.path.clone());

        emit_initial_lock_state(&harness.app_handle(), &harness.lock);

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.enabled);
        assert!(status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("startup"));
    }

    #[test]
    fn window_focus_emits_current_lock_status_when_unlocked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        handle_main_window_focus(&harness.app_handle(), &harness.lock, true);

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.enabled);
        assert!(!status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("password_set"));
    }

    #[test]
    fn window_close_refreshes_activity_before_auto_lock_check() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.configure_auto_lock(true, 1);
        harness
            .lock
            .rewind_last_activity_for_test(Duration::from_secs(2));

        handle_main_window_close(&harness.window, &harness.lock);
        emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

        assert_no_lock_status(&harness.app_lock_status_rx);
        assert!(!harness.lock.status().locked);
    }
}
