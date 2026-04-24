use std::borrow::Cow;
use std::sync::Arc;

use tauri::State;

use crate::config::{AppConfig, ConfigManager};
use crate::encryption::{EncryptionManager, EncryptionStatus};
use crate::security::{
    self, AppLockManager, AppLockStatus, SetPasswordPayload, UnlockPayload,
    UpdateAppLockSettingsPayload,
};
use crate::storage::{Database, SearchQuery, SearchResult, Statistics, Tag};
use crate::sync::webdav::{WebDavConfig, WebDavSyncManager, WebDavSyncStatus};
use crate::sync::{SyncConfig, SyncManager};
use crate::updater::UpdaterManager;
use crate::AppDataDir;

fn require_unlocked(lock: &State<'_, Arc<AppLockManager>>) -> Result<(), String> {
    lock.ensure_unlocked()
}

fn decrypt_entries(encryption: &EncryptionManager, entries: &mut [crate::storage::ClipboardEntry]) {
    for entry in entries.iter_mut() {
        if let Ok(decrypted) = encryption.decrypt_content(&entry.content) {
            entry.content = decrypted;
        }
    }
}

fn decrypt_search_result(encryption: &EncryptionManager, result: &mut SearchResult) {
    decrypt_entries(encryption, &mut result.entries);
}

#[tauri::command]
pub async fn get_app_lock_status(
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    Ok(lock.status())
}

#[tauri::command]
pub async fn set_app_lock_password<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: SetPasswordPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.set_password(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn update_app_lock_settings<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UpdateAppLockSettingsPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.update_settings(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn lock_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    let status = lock.lock("manual");
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn unlock_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UnlockPayload,
) -> Result<AppLockStatus, String> {
    let prefer_biometric = payload.prefer_biometric.unwrap_or(false);
    if prefer_biometric && lock.status().biometric_enabled {
        match security::try_biometric_unlock() {
            Ok(true) => {
                let status = lock.mark_biometric_unlocked();
                security::emit_lock_state(&app, &lock);
                return Ok(status);
            }
            Ok(false) | Err(_) => {
                // Fall through to password unlock.
            }
        }
    }

    let password = payload
        .password
        .ok_or_else(|| "Password is required".to_string())?;
    match lock.verify_password(&password) {
        Ok(status) => {
            security::emit_lock_state(&app, &lock);
            Ok(status)
        }
        Err(err) => {
            let _ = lock.handle_failed_unlock();
            security::emit_lock_state(&app, &lock);
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn get_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
    category: Option<String>,
    is_favorite: Option<bool>,
) -> Result<SearchResult, String> {
    require_unlocked(&lock)?;
    let mut result = db
        .get_entries(limit, offset, category.as_deref(), is_favorite)
        .map_err(|e| e.to_string())?;
    decrypt_search_result(&encryption, &mut result);
    Ok(result)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    keyword: String,
    category: Option<String>,
    is_favorite: Option<bool>,
    limit: i64,
    offset: i64,
) -> Result<SearchResult, String> {
    require_unlocked(&lock)?;
    let query = SearchQuery {
        keyword: Some(keyword),
        category,
        is_favorite,
        limit,
        offset,
    };
    let mut result = db.search(&query).map_err(|e| e.to_string())?;
    decrypt_search_result(&encryption, &mut result);
    Ok(result)
}

#[tauri::command]
pub async fn delete_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.delete_entry_with_cleanup(id, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    ids: Vec<i64>,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.delete_entries_with_cleanup(&ids, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
) -> Result<String, String> {
    require_unlocked(&lock)?;
    let mut entries = db.get_entries_by_ids(&ids).map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut entries);

    let merged = entries
        .into_iter()
        .filter(|entry| entry.content_type != "image")
        .map(|entry| entry.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if merged.trim().is_empty() {
        return Ok(String::new());
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&merged)
        .map_err(|e| format!("Clipboard error: {}", e))?;

    Ok(merged)
}

#[tauri::command]
pub async fn set_favorite_state_for_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    favorite: bool,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.set_favorite_state_for_entries(&ids, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_favorite(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<bool, String> {
    require_unlocked(&lock)?;
    db.toggle_favorite(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_count(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.get_entry_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_statistics(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<Statistics, String> {
    require_unlocked(&lock)?;
    let mut stats = db.get_statistics().map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut stats.most_used);

    let db_path = app_data_dir.0.join("clipboard.db");
    stats.storage_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(stats)
}

#[tauri::command]
pub async fn paste_entry(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut entry = db
        .get_entry_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Entry not found".to_string())?;

    // Decrypt content if encrypted
    if let Ok(decrypted) = encryption.decrypt_content(&entry.content) {
        entry.content = decrypted;
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;

    if entry.content_type == "image" {
        let img = image::open(&entry.content)
            .map_err(|e| format!("Failed to open image: {}", e))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        let arboard_img = arboard::ImageData {
            bytes: Cow::from(img.into_raw()),
            width: w as usize,
            height: h as usize,
        };
        clipboard
            .set_image(arboard_img)
            .map_err(|e| format!("Clipboard error: {}", e))?;
    } else {
        clipboard
            .set_text(&entry.content)
            .map_err(|e| format!("Clipboard error: {}", e))?;
    }

    db.update_use_count(&entry.hash)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_updater_status(
    updater: State<'_, Arc<UpdaterManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    Ok(updater.get_status())
}

#[tauri::command]
pub async fn check_for_updates_now<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let updater_config = config.get().updater;
    let target = crate::updater::current_target();
    let status =
        updater.check_now_with_fallible_fetcher(&updater_config, false, target, |url: &str| {
            let handle = tokio::runtime::Handle::current();
            match handle.block_on(crate::updater::http::fetch_text(url)) {
                Ok(body) => Ok(Some(body)),
                Err(err) => Err(err),
            }
        })?;
    updater.emit_status(&app);
    Ok(status)
}

#[tauri::command]
pub async fn download_available_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    config: State<'_, Arc<ConfigManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let updater_config = config.get().updater;
    let target = crate::updater::current_target();
    let status =
        updater.check_now_with_fallible_fetcher(&updater_config, false, target, |url: &str| {
            let handle = tokio::runtime::Handle::current();
            match handle.block_on(crate::updater::http::fetch_text(url)) {
                Ok(body) => Ok(Some(body)),
                Err(err) => Err(err),
            }
        })?;

    if status.phase != crate::updater::UpdaterPhase::UpdateAvailable {
        updater.emit_status(&app);
        return Ok(status);
    }

    let manifest_body = crate::updater::http::fetch_text(crate::updater::CANONICAL_MANIFEST_URL)
        .await
        .map_err(|e| format!("Failed to refetch manifest: {e}"))?;
    let app_handle = app.clone();
    let download_status = updater.download_update_with_handlers_and_progress(
        &app_data_dir.0,
        &updater_config,
        &manifest_body,
        target,
        crate::updater::CANONICAL_MANIFEST_URL,
        |asset_url: &str| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(crate::updater::http::fetch_bytes_with_progress(
                asset_url,
                |_| {},
            ))
        },
        crate::updater::verify::verify_downloaded_artifact,
        |progress: f64| {
            let mut status = updater.get_status();
            status.phase = if progress >= 1.0 {
                crate::updater::UpdaterPhase::ReadyToInstall
            } else {
                crate::updater::UpdaterPhase::Downloading
            };
            status.download_progress = Some(progress);
            updater.set_status(status);
            updater.emit_status(&app_handle);
        },
    )?;
    updater.emit_status(&app);
    Ok(download_status)
}

#[tauri::command]
pub async fn install_pending_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let status = updater.install_pending()?;
    updater.emit_status(&app);
    Ok(status)
}

#[tauri::command]
pub async fn discard_pending_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let status = updater.discard_pending(&app_data_dir.0)?;
    updater.emit_status(&app);
    Ok(status)
}

#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle<impl tauri::Runtime>) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_config(config: State<'_, Arc<ConfigManager>>) -> Result<AppConfig, String> {
    Ok(config.get())
}

#[tauri::command]
pub async fn update_config(
    config: State<'_, Arc<ConfigManager>>,
    new_config: AppConfig,
) -> Result<(), String> {
    config.update(new_config)
}

#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn create_tag(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    name: String,
) -> Result<Tag, String> {
    require_unlocked(&lock)?;
    db.create_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.delete_tag(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_tags(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<Tag>, String> {
    require_unlocked(&lock)?;
    db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag_to_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.add_tag_to_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.remove_tag_from_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_tags_for_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    tag_ids: Vec<i64>,
    mode: Option<String>,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.set_tags_for_entries(&ids, &tag_ids, mode.as_deref().unwrap_or("replace"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_tags(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<Vec<Tag>, String> {
    require_unlocked(&lock)?;
    db.get_entry_tags(entry_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entries_by_tag(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    tag_id: i64,
) -> Result<Vec<crate::storage::ClipboardEntry>, String> {
    require_unlocked(&lock)?;
    let mut entries = db.get_entries_by_tag(tag_id).map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut entries);
    Ok(entries)
}

#[tauri::command]
pub async fn transform_content(content: String, transform_type: String) -> Result<String, String> {
    transform::apply_transform(&content, &transform_type)
}

pub mod transform {
    use base64::{engine::general_purpose::STANDARD, Engine};

    pub fn apply_transform(content: &str, transform_type: &str) -> Result<String, String> {
        match transform_type {
            "uppercase" => Ok(content.to_uppercase()),
            "lowercase" => Ok(content.to_lowercase()),
            "title_case" => Ok(to_title_case(content)),
            "url_encode" => Ok(url_encode(content)),
            "url_decode" => url_decode(content),
            "json_format" => json_format(content),
            "json_compact" => json_compact(content),
            "base64_encode" => Ok(STANDARD.encode(content.as_bytes())),
            "base64_decode" => base64_decode(content),
            "trim" => Ok(trim_whitespace(content)),
            "html_escape" => Ok(html_escape(content)),
            "html_unescape" => Ok(html_unescape(content)),
            _ => Err(format!("Unknown transform type: {}", transform_type)),
        }
    }

    fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn url_encode(s: &str) -> String {
        urlencoding::encode(s).into_owned()
    }
    fn url_decode(s: &str) -> Result<String, String> {
        match urlencoding::decode(s) {
            Ok(v) => Ok(v.into_owned()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn json_format(s: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(s)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .map_err(|e| e.to_string())
    }
    fn json_compact(s: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(s)
            .and_then(|v| serde_json::to_string(&v))
            .map_err(|e| e.to_string())
    }
    fn base64_decode(s: &str) -> Result<String, String> {
        match STANDARD.decode(s) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn trim_whitespace(s: &str) -> String {
        s.trim().to_string()
    }
    fn html_escape(s: &str) -> String {
        html_escape::encode_safe(s).to_string()
    }
    fn html_unescape(s: &str) -> String {
        html_escape::decode_html_entities(s).to_string()
    }
}

#[tauri::command]
pub async fn get_sync_status(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<crate::storage::SyncStatus, String> {
    require_unlocked(&lock)?;
    sync_manager.get_status()
}

#[tauri::command]
pub async fn get_sync_config(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<SyncConfig, String> {
    require_unlocked(&lock)?;
    Ok(config.get().sync)
}

#[tauri::command]
pub async fn update_sync_config(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    config: State<'_, Arc<ConfigManager>>,
    new_config: SyncConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut app_config = config.get();
    app_config.sync = new_config.clone();
    config.update(app_config)?;
    sync_manager.update_config(new_config)
}

#[tauri::command]
pub async fn get_discovered_devices(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::DiscoveredDevice>, String> {
    require_unlocked(&lock)?;
    sync_manager.get_discovered_devices()
}

#[tauri::command]
pub async fn get_paired_devices(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::PairedDevice>, String> {
    require_unlocked(&lock)?;
    sync_manager.get_paired_devices()
}

#[tauri::command]
pub async fn pair_device(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.pair_device(&device_id)
}

#[tauri::command]
pub async fn unpair_device(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.unpair_device(&device_id)
}

#[tauri::command]
pub async fn toggle_device_sync(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
    enabled: bool,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.toggle_device_sync(&device_id, enabled)
}

#[tauri::command]
pub async fn webdav_connect(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    config: WebDavConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager
        .connect(
            &config.server_url,
            &config.username,
            &config.password,
            &config.sync_password,
        )
        .await?;
    manager.update_config(config).await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_disconnect(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager.disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_get_status(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<WebDavSyncStatus, String> {
    require_unlocked(&lock)?;
    Ok(manager.get_status().await)
}

#[tauri::command]
pub async fn webdav_get_config(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<WebDavConfig, String> {
    require_unlocked(&lock)?;
    Ok(config.get().webdav)
}

#[tauri::command]
pub async fn webdav_update_config(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    config: State<'_, Arc<ConfigManager>>,
    new_config: WebDavConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut app_config = config.get();
    app_config.webdav = new_config.clone();
    config.update(app_config)?;
    manager.update_config(new_config).await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_trigger_sync(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<u32, String> {
    require_unlocked(&lock)?;
    manager.trigger_sync().await
}

#[tauri::command]
pub async fn webdav_remove_device(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager.remove_device(&device_id).await
}

// --- Database encryption commands ---

#[tauri::command]
pub async fn get_encryption_status(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<EncryptionStatus, String> {
    require_unlocked(&lock)?;
    Ok(encryption.status(&db))
}

#[tauri::command]
pub async fn enable_encryption(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<EncryptionStatus, String> {
    require_unlocked(&lock)?;
    encryption.enable(&db)
}

#[tauri::command]
pub async fn disable_encryption(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<EncryptionStatus, String> {
    require_unlocked(&lock)?;
    encryption.disable(&db)
}

#[cfg(test)]
mod command_guard_tests {
    use super::{get_app_lock_status, get_entries, lock_app, set_app_lock_password, unlock_app};
    use crate::config::ConfigManager;
    use crate::security::{self, AppLockStatus};
    use crate::storage::{ClipboardEntry, Database, SearchResult, Template};
    use chrono::Local;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{self, MockRuntime};
    use tauri::webview::InvokeRequest;
    use tauri::{App, WebviewWindow, WebviewWindowBuilder};

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
                "smart-clipboard-app-lock-command-tests-{}-{}",
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

    struct TestHarness {
        _app: App<MockRuntime>,
        webview: WebviewWindow<MockRuntime>,
        config: Arc<ConfigManager>,
    }

    fn create_harness(base_dir: &Path) -> TestHarness {
        let config = Arc::new(ConfigManager::new(base_dir.to_path_buf()));
        let lock = Arc::new(crate::security::AppLockManager::new(config.clone()));
        let encryption = Arc::new(crate::encryption::EncryptionManager::new(config.clone()));
        let db = Arc::new(
            Database::new(&base_dir.join("clipboard.db").to_string_lossy())
                .expect("failed to initialize database"),
        );

        let now = Local::now().naive_local();
        db.insert_entry(&ClipboardEntry {
            id: None,
            content: "alpha entry".to_string(),
            content_type: "text".to_string(),
            category: "text".to_string(),
            hash: "hash-alpha-entry".to_string(),
            source_app: Some("test-suite".to_string()),
            is_favorite: false,
            is_sensitive: false,
            use_count: 0,
            created_at: now,
            updated_at: now,
            expires_at: None,
            source_device: None,
        })
        .expect("failed to seed clipboard entry");

        let app = test::mock_builder()
            .manage(config.clone())
            .manage(lock)
            .manage(encryption)
            .manage(db)
            .invoke_handler(tauri::generate_handler![
                get_app_lock_status,
                set_app_lock_password,
                lock_app,
                unlock_app,
                get_entries,
                crate::templates::commands::create_template,
                crate::templates::commands::get_templates
            ])
            .build(test::mock_context(test::noop_assets()))
            .expect("failed to build mock app");

        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("failed to build mock webview");

        TestHarness {
            _app: app,
            webview,
            config,
        }
    }

    fn invoke_request(cmd: &str, body: Value) -> InvokeRequest {
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().expect("valid tauri url"),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: test::INVOKE_KEY.to_string(),
        }
    }

    fn invoke<T: DeserializeOwned>(
        harness: &TestHarness,
        cmd: &str,
        body: Value,
    ) -> Result<T, Value> {
        test::get_ipc_response(&harness.webview, invoke_request(cmd, body)).map(
            |response: tauri::ipc::InvokeResponseBody| {
                response
                    .deserialize::<T>()
                    .expect("command response should deserialize")
            },
        )
    }

    fn enable_biometric_for_test(config: &ConfigManager) {
        let mut app_config = config.get();
        app_config.app_lock.enabled = true;
        app_config.app_lock.biometric_enabled = true;
        config
            .update(app_config)
            .expect("failed to enable biometric for test");
    }

    #[test]
    fn locked_sensitive_command_is_rejected_via_invoke() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");

        let lock_status: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");
        assert!(lock_status.locked);

        let error = invoke::<SearchResult>(
            &harness,
            "get_entries",
            json!({
                "limit": 20,
                "offset": 0,
                "category": null,
                "is_favorite": null
            }),
        )
        .expect_err("locked app should reject sensitive commands");

        assert_eq!(error, json!("App is locked"));
    }

    #[test]
    fn unlock_restores_sensitive_command_access() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");
        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let unlock_status: AppLockStatus = invoke(
            &harness,
            "unlock_app",
            json!({
                "payload": {
                    "password": "phase4-pass",
                    "prefer_biometric": false
                }
            }),
        )
        .expect("unlock should succeed");

        assert!(!unlock_status.locked);
        assert_eq!(unlock_status.unlock_reason.as_deref(), Some("password"));

        let result: SearchResult = invoke(
            &harness,
            "get_entries",
            json!({
                "limit": 20,
                "offset": 0,
                "category": null,
                "is_favorite": null
            }),
        )
        .expect("unlocked app should allow sensitive commands");

        assert_eq!(result.total_count, 1);
        assert_eq!(result.entries[0].content, "alpha entry");
    }

    #[test]
    fn wrong_password_keeps_app_locked_and_tracks_failed_attempts() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");
        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let error = invoke::<AppLockStatus>(
            &harness,
            "unlock_app",
            json!({
                "payload": {
                    "password": "wrong-pass",
                    "prefer_biometric": false
                }
            }),
        )
        .expect_err("wrong password should fail");

        assert_eq!(error, json!("Incorrect password"));

        let status: AppLockStatus = invoke(&harness, "get_app_lock_status", json!({}))
            .expect("status command should succeed");
        assert!(status.locked);
        assert_eq!(status.failed_attempts, 1);
        assert_eq!(status.unlock_reason.as_deref(), Some("failed_password"));
    }

    #[test]
    fn biometric_failure_falls_back_to_password_unlock() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");

        enable_biometric_for_test(&harness.config);
        security::set_test_biometric_result(Some(Err("biometric failed".to_string())));

        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let unlock_status: AppLockStatus = invoke(
            &harness,
            "unlock_app",
            json!({
                "payload": {
                    "password": "phase4-pass",
                    "prefer_biometric": true
                }
            }),
        )
        .expect("password fallback should succeed");

        assert!(!unlock_status.locked);
        assert_eq!(unlock_status.unlock_reason.as_deref(), Some("password"));
        assert_eq!(unlock_status.failed_attempts, 0);

        let result: SearchResult = invoke(
            &harness,
            "get_entries",
            json!({
                "limit": 20,
                "offset": 0,
                "category": null,
                "is_favorite": null
            }),
        )
        .expect("fallback unlock should restore access");

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn locked_template_command_is_rejected_via_invoke() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");
        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let error = invoke::<Template>(
            &harness,
            "create_template",
            json!({
                "name": "Greeting",
                "content": "Hello {{name}}",
                "category": "general"
            }),
        )
        .expect_err("locked app should reject template commands");

        assert_eq!(error, json!("App is locked"));
    }

    #[test]
    fn unlock_restores_template_command_access() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = create_harness(&temp_dir.path);

        let _: AppLockStatus = invoke(
            &harness,
            "set_app_lock_password",
            json!({
                "payload": {
                    "current_password": null,
                    "new_password": "phase4-pass"
                }
            }),
        )
        .expect("setting password should succeed");
        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let _: AppLockStatus = invoke(
            &harness,
            "unlock_app",
            json!({
                "payload": {
                    "password": "phase4-pass",
                    "prefer_biometric": false
                }
            }),
        )
        .expect("unlock should succeed");

        let created: Template = invoke(
            &harness,
            "create_template",
            json!({
                "name": "Greeting",
                "content": "Hello {{name}}",
                "category": "general"
            }),
        )
        .expect("unlocked app should allow creating templates");

        assert_eq!(created.name, "Greeting");
        assert_eq!(created.content, "Hello {{name}}");
        assert_eq!(created.category, "general");

        let templates: Vec<Template> = invoke(
            &harness,
            "get_templates",
            json!({
                "category": null
            }),
        )
        .expect("unlocked app should allow listing templates");

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Greeting");
        assert_eq!(templates[0].content, "Hello {{name}}");
    }
}
