pub mod analyzer;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod platform;
pub mod storage;
pub mod sync;
pub mod templates;
pub mod tray;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Local};
use image::ImageBuffer;
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

use analyzer::{classify, detect_sensitive};
use clipboard::ClipboardMonitor;
use config::ConfigManager;
use storage::{ClipboardEntry, Database};
use sync::webdav::WebDavSyncManager;
use sync::SyncManager;

/// Managed state holding the app data directory path.
pub struct AppDataDir(pub PathBuf);

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
            commands::get_statistics,
            commands::paste_entry,
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

            // Create images directory for image clipboard support
            let images_dir = app_data_dir.join("images");
            std::fs::create_dir_all(&images_dir).ok();

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
