pub mod analyzer;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod storage;
pub mod tray;

use std::sync::Arc;

use chrono::{Duration, Local};
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

use analyzer::{classify, detect_sensitive};
use clipboard::ClipboardMonitor;
use config::ConfigManager;
use storage::{ClipboardEntry, Database};

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
            commands::toggle_favorite,
            commands::get_entry_count,
            commands::paste_entry,
            commands::get_config,
            commands::update_config,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::transform_content,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Determine app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Initialize config
            let config_manager = Arc::new(ConfigManager::new(app_data_dir.clone()));
            let config = config_manager.get();
            app.manage(config_manager.clone());

            // Initialize database
            let db_path = app_data_dir.join("clipboard.db");
            let db = Arc::new(
                Database::new(&db_path.to_string_lossy()).expect("Failed to initialize database"),
            );
            app.manage(db.clone());

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
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            // Start clipboard monitor
            let (tx, mut rx) = mpsc::unbounded_channel();
            let monitor = ClipboardMonitor::new(config.monitor_interval_ms);
            monitor.start(tx);

            // Process incoming clipboard changes
            let db_for_rx = db.clone();
            let app_handle = app.handle().clone();
            let config_for_rx = config_manager.clone();

            tauri::async_runtime::spawn(async move {
                while let Some(change) = rx.recv().await {
                    let hash = format!("{:x}", Sha256::digest(change.content.as_bytes()));

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

                    // Classify content
                    let category = classify(&change.content);
                    let now = Local::now().naive_local();

                    // Detect sensitive content
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

                    let entry = ClipboardEntry {
                        id: None,
                        content: change.content,
                        content_type: change.content_type,
                        category: category.as_str().to_string(),
                        hash,
                        source_app: change.source_app,
                        is_favorite: false,
                        is_sensitive,
                        use_count: 1,
                        created_at: now,
                        updated_at: now,
                        expires_at,
                    };

                    match db_for_rx.insert_entry(&entry) {
                        Ok(id) => {
                            let mut stored_entry = entry;
                            stored_entry.id = Some(id);
                            let _ = app_handle.emit("clipboard-changed", &stored_entry);
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
