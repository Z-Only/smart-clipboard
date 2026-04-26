use std::sync::Arc;

use tauri::State;

use crate::config::ConfigManager;
use crate::encryption::{EncryptionManager, EncryptionStatus};
use crate::security::AppLockManager;
use crate::storage::Database;
use crate::sync::webdav::{WebDavConfig, WebDavSyncManager, WebDavSyncStatus};
use crate::sync::{SyncConfig, SyncManager};

use super::require_unlocked;

// --- P2P sync commands ---

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

// --- WebDAV sync commands ---

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
