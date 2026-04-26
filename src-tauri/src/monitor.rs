use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Duration, Local};
use image::ImageBuffer;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::analyzer::{classify, detect_sensitive};
use crate::clipboard::ClipboardMonitor;
use crate::config::ConfigManager;
use crate::encryption::EncryptionManager;
use crate::platform;
use crate::storage::{ClipboardEntry, Database};
use crate::sync::webdav::WebDavSyncManager;
use crate::sync::SyncManager;

/// Shared context for the clipboard monitor processing loop.
pub(crate) struct MonitorContext {
    pub app_handle: AppHandle,
    pub db: Arc<Database>,
    pub config_manager: Arc<ConfigManager>,
    pub encryption_manager: Arc<EncryptionManager>,
    pub sync_manager: Arc<SyncManager>,
    pub webdav_manager: Arc<WebDavSyncManager>,
    pub images_dir: PathBuf,
}

/// Start the clipboard monitor and spawn an async task that processes
/// incoming clipboard changes (dedup, classify, encrypt, persist, broadcast).
pub(crate) fn start_clipboard_monitor(ctx: MonitorContext, monitor_interval_ms: u64) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let monitor = ClipboardMonitor::new(monitor_interval_ms);
    monitor.start(tx);

    tauri::async_runtime::spawn(async move {
        while let Some(change) = rx.recv().await {
            process_clipboard_change(&change, &ctx).await;
        }
    });
}

async fn process_clipboard_change(
    change: &crate::clipboard::ClipboardChange,
    ctx: &MonitorContext,
) {
    // Capture frontmost application as source_app
    let source_app = platform::get_frontmost_app();

    // Check if the source app is in the excluded list
    let excluded_apps = ctx.config_manager.get().excluded_apps;
    if let Some(ref app) = source_app {
        if excluded_apps.iter().any(|excluded| {
            app.to_lowercase().contains(&excluded.to_lowercase())
                || excluded.to_lowercase().contains(&app.to_lowercase())
        }) {
            log::debug!("Skipping clipboard from excluded app: {}", app);
            return;
        }
    }

    let is_image = change.content_type == "image";

    // For images, hash the raw RGBA bytes; for text, hash the content string
    let hash = if is_image {
        if let Some(ref img_data) = change.image_data {
            format!("{:x}", Sha256::digest(&img_data.bytes))
        } else {
            return; // image change without data - skip
        }
    } else {
        format!("{:x}", Sha256::digest(change.content.as_bytes()))
    };

    // Check deduplication
    match ctx.db.find_by_hash(&hash) {
        Ok(Some(_)) => {
            let _ = ctx.db.update_use_count(&hash);
            return;
        }
        Ok(None) => {}
        Err(e) => {
            log::error!("DB error during dedup check: {}", e);
            return;
        }
    }

    let now = Local::now().naive_local();

    // Handle image vs text differently
    let (content, category, is_sensitive, expires_at) = if is_image {
        match save_image_entry(change, &ctx.images_dir, &hash) {
            Some(content) => (content, "image".to_string(), false, None),
            None => return,
        }
    } else {
        build_text_entry(&change.content, &ctx.config_manager, now)
    };

    // Encrypt content if encryption is enabled (skip images)
    let (stored_content, is_encrypted) = if !is_image && ctx.encryption_manager.is_enabled() {
        match ctx.encryption_manager.encrypt_content(&content) {
            Ok(encrypted) => (encrypted, true),
            Err(e) => {
                log::error!("Failed to encrypt clipboard content: {}", e);
                (content.clone(), false)
            }
        }
    } else {
        (content.clone(), false)
    };

    let entry = ClipboardEntry {
        id: None,
        content: stored_content,
        content_type: change.content_type.clone(),
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

    match ctx
        .db
        .insert_entry_with_encrypted_flag(&entry, is_encrypted)
    {
        Ok(id) => {
            // Emit the decrypted version to the frontend
            let mut stored_entry = entry.clone();
            stored_entry.id = Some(id);
            if is_encrypted {
                stored_entry.content = content;
            }
            let _ = ctx.app_handle.emit("clipboard-changed", &stored_entry);
            // Broadcast to paired devices via sync pipeline
            ctx.sync_manager.broadcast_entry(&stored_entry);
            // Push to WebDAV cloud sync
            let webdav = ctx.webdav_manager.clone();
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

/// Save an image clipboard change to disk and return the file path as content.
/// Returns `None` if the image cannot be saved.
fn save_image_entry(
    change: &crate::clipboard::ClipboardChange,
    images_dir: &Path,
    hash: &str,
) -> Option<String> {
    let img_data = change.image_data.as_ref().unwrap();
    let png_path = images_dir.join(format!("{}.png", hash));

    match ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        img_data.width as u32,
        img_data.height as u32,
        img_data.bytes.clone(),
    ) {
        Some(img) => {
            if let Err(e) = img.save(&png_path) {
                log::error!("Failed to save image: {}", e);
                return None;
            }
        }
        None => {
            log::error!("Failed to create image buffer from RGBA data");
            return None;
        }
    }

    Some(png_path.to_string_lossy().to_string())
}

/// Classify text content, detect sensitivity, and compute optional expiry.
fn build_text_entry(
    content: &str,
    config_manager: &Arc<ConfigManager>,
    now: chrono::NaiveDateTime,
) -> (String, String, bool, Option<chrono::NaiveDateTime>) {
    let category = classify(content);
    let is_sensitive = detect_sensitive(content);
    let expires_at = if is_sensitive {
        let expiry_minutes = config_manager.get().sensitive_expiry_minutes;
        if expiry_minutes > 0 {
            Some(now + Duration::minutes(expiry_minutes as i64))
        } else {
            None
        }
    } else {
        None
    };
    (
        content.to_string(),
        category.as_str().to_string(),
        is_sensitive,
        expires_at,
    )
}
