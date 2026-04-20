pub mod client;
pub mod index;
pub mod poller;
pub mod rate_limiter;

use std::sync::Arc;

use chrono::Utc;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use client::WebDavClient;
use index::{IndexEntry, IndexManager};
use poller::SyncPoller;
use rate_limiter::TokenBucketLimiter;

use crate::storage::{ClipboardEntry, Database};
use crate::sync::crypto;
use crate::sync::protocol::SyncEntryPayload;

const MAX_SYNC_PAYLOAD_BYTES: usize = 1_048_576; // 1 MB

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    pub enabled: bool,
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub sync_password: String,
    pub poll_interval_secs: u64,
    pub sync_images: bool,
    pub sync_sensitive: bool,
    pub rate_limit_capacity: u32,
    pub rate_limit_refill_minutes: u32,
    pub remote_path: String,
    pub max_cloud_entries: u32,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: String::new(),
            username: String::new(),
            password: String::new(),
            sync_password: String::new(),
            poll_interval_secs: 30,
            sync_images: false,
            sync_sensitive: false,
            rate_limit_capacity: 150,
            rate_limit_refill_minutes: 30,
            remote_path: "/SmartClipboard".to_string(),
            max_cloud_entries: 2000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavSyncStatus {
    pub status: String,
    pub last_sync_at: Option<String>,
    pub cloud_entry_count: u32,
    pub registered_devices: Vec<index::RegisteredDevice>,
    pub rate_limit_available: u32,
    pub rate_limit_capacity: u32,
    pub error: Option<String>,
}

pub struct WebDavSyncManager {
    db: Arc<Database>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    salt: RwLock<Option<Vec<u8>>>,
    client: RwLock<Option<Arc<WebDavClient>>>,
    index_manager: RwLock<Option<Arc<IndexManager>>>,
    poller: RwLock<Option<Arc<SyncPoller>>>,
    rate_limiter: RwLock<Option<Arc<TokenBucketLimiter>>>,
    config: RwLock<WebDavConfig>,
    status: RwLock<String>,
    last_sync_at: RwLock<Option<String>>,
    last_error: RwLock<Option<String>>,
    device_id: String,
    device_name: String,
    public_key: Vec<u8>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl WebDavSyncManager {
    pub fn new(
        db: Arc<Database>,
        config: WebDavConfig,
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            master_key: Arc::new(RwLock::new(None)),
            salt: RwLock::new(None),
            client: RwLock::new(None),
            index_manager: RwLock::new(None),
            poller: RwLock::new(None),
            rate_limiter: RwLock::new(None),
            config: RwLock::new(config),
            status: RwLock::new("disconnected".to_string()),
            last_sync_at: RwLock::new(None),
            last_error: RwLock::new(None),
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            public_key: public_key.to_vec(),
            app_handle: Arc::new(RwLock::new(None)),
        })
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        let app_handle = self.app_handle.clone();
        tauri::async_runtime::spawn(async move {
            *app_handle.write().await = Some(handle);
        });
    }

    pub async fn connect(
        &self,
        server_url: &str,
        username: &str,
        password: &str,
        sync_password: &str,
    ) -> Result<(), String> {
        *self.status.write().await = "connecting".to_string();
        *self.last_error.write().await = None;

        let config = self.config.read().await;
        let rate_limiter = Arc::new(TokenBucketLimiter::new(
            config.rate_limit_capacity,
            config.rate_limit_refill_minutes,
        ));
        let poll_interval = config.poll_interval_secs;
        let remote_path = config.remote_path.clone();
        drop(config);

        let client = Arc::new(WebDavClient::new(
            server_url,
            username,
            password,
            rate_limiter.clone(),
        )?);

        if let Err(e) = client.test_connection().await {
            *self.status.write().await = "error".to_string();
            *self.last_error.write().await = Some(e.clone());
            return Err(e);
        }

        client.ensure_directory_structure(&remote_path).await?;

        let index_manager = Arc::new(IndexManager::new(
            client.clone(),
            &remote_path,
            &self.device_id,
        ));

        let trimmed_path = remote_path.trim_matches('/');
        let devices_exist = client
            .exists(&format!("{}/meta/devices.enc", trimmed_path))
            .await?;

        let (master_key, salt) = if devices_exist {
            let (encrypted, _) = client
                .get(&format!("{}/meta/devices.enc", trimmed_path))
                .await?;

            if encrypted.len() < 36 {
                return Err("Device registry file is corrupted".to_string());
            }
            let salt = encrypted[4..20].to_vec();
            let master_key = crypto::derive_key_from_password(sync_password, &salt)?;

            crypto::decrypt_file(&encrypted, &master_key).map_err(|_| {
                "Incorrect sync password — cannot decrypt device registry".to_string()
            })?;

            index_manager
                .register_device(
                    &master_key,
                    &salt,
                    &self.device_id,
                    &self.device_name,
                    &self.public_key,
                )
                .await?;

            (master_key, salt)
        } else {
            let salt = crypto::generate_salt();
            let master_key = crypto::derive_key_from_password(sync_password, &salt)?;

            index_manager
                .initialize_registry(
                    &master_key,
                    &salt,
                    &self.device_id,
                    &self.device_name,
                    &self.public_key,
                )
                .await?;

            let empty_index = index::SyncIndex {
                version: 1,
                updated_at: Utc::now().to_rfc3339(),
                updated_by: self.device_id.clone(),
                entries: vec![],
            };
            index_manager
                .save_index(&empty_index, &master_key, None)
                .await?;

            (master_key, salt)
        };

        *self.master_key.write().await = Some(master_key);
        *self.salt.write().await = Some(salt);
        *self.client.write().await = Some(client);
        *self.index_manager.write().await = Some(index_manager.clone());
        *self.rate_limiter.write().await = Some(rate_limiter);

        let poller = Arc::new(SyncPoller::new(
            self.db.clone(),
            index_manager,
            self.master_key.clone(),
            &self.device_id,
            self.app_handle.clone(),
        ));
        poller.start(poll_interval).await;
        *self.poller.write().await = Some(poller);

        *self.status.write().await = "connected".to_string();
        *self.last_sync_at.write().await = Some(Utc::now().to_rfc3339());
        info!("WebDAV sync connected to {}", server_url);
        Ok(())
    }

    pub async fn disconnect(&self) {
        if let Some(poller) = self.poller.read().await.as_ref() {
            poller.stop().await;
        }
        *self.poller.write().await = None;
        *self.client.write().await = None;
        *self.index_manager.write().await = None;
        *self.rate_limiter.write().await = None;
        *self.master_key.write().await = None;
        *self.salt.write().await = None;
        *self.status.write().await = "disconnected".to_string();
        *self.last_error.write().await = None;
        info!("WebDAV sync disconnected");
    }

    pub async fn push_entry(&self, entry: &ClipboardEntry) -> Result<(), String> {
        if !self.should_sync_entry(entry).await {
            return Ok(());
        }

        let master_key = {
            let guard = self.master_key.read().await;
            match guard.as_ref() {
                Some(k) => k.clone(),
                None => return Ok(()),
            }
        };

        let index_manager = {
            let guard = self.index_manager.read().await;
            match guard.as_ref() {
                Some(im) => im.clone(),
                None => return Ok(()),
            }
        };

        let config = self.config.read().await;
        let max_cloud_entries = config.max_cloud_entries;
        drop(config);

        let payload = SyncEntryPayload {
            content: entry.content.clone(),
            content_type: entry.content_type.clone(),
            category: entry.category.clone(),
            hash: entry.hash.clone(),
            source_app: entry.source_app.clone(),
            is_sensitive: entry.is_sensitive,
            source_device: self.device_id.clone(),
            created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        let json = serde_json::to_vec(&payload)
            .map_err(|e| format!("Failed to serialize entry: {e}"))?;

        index_manager
            .upload_entry(&entry.hash, &json, &master_key)
            .await?;

        let index_entry = IndexEntry {
            hash: entry.hash.clone(),
            content_type: entry.content_type.clone(),
            category: entry.category.clone(),
            source_device: self.device_id.clone(),
            created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            size_bytes: json.len() as u64,
        };
        index_manager
            .append_entry(index_entry, &master_key)
            .await?;

        if let Err(e) = index_manager
            .enforce_entry_limit(max_cloud_entries, &master_key)
            .await
        {
            warn!("Failed to enforce entry limit: {}", e);
        }

        *self.last_sync_at.write().await = Some(Utc::now().to_rfc3339());
        info!("Pushed entry {} to WebDAV", entry.hash);
        Ok(())
    }

    async fn should_sync_entry(&self, entry: &ClipboardEntry) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }
        if entry.is_sensitive && !config.sync_sensitive {
            return false;
        }
        if entry.content_type == "image" && !config.sync_images {
            return false;
        }
        if entry.content.len() > MAX_SYNC_PAYLOAD_BYTES {
            return false;
        }
        true
    }

    pub async fn trigger_sync(&self) -> Result<u32, String> {
        let poller = self.poller.read().await;
        match poller.as_ref() {
            Some(p) => p.poll_now().await,
            None => Err("Not connected".to_string()),
        }
    }

    pub async fn get_status(&self) -> WebDavSyncStatus {
        let status = self.status.read().await.clone();
        let last_sync_at = self.last_sync_at.read().await.clone();
        let last_error = self.last_error.read().await.clone();

        let (cloud_entry_count, registered_devices) =
            if let Some(ref im) = *self.index_manager.read().await {
                let master_key_guard = self.master_key.read().await;
                if let Some(ref key) = *master_key_guard {
                    let count = im
                        .load_index(key)
                        .await
                        .map(|(idx, _)| idx.entries.len() as u32)
                        .unwrap_or(0);
                    let devices = im
                        .load_device_registry(key)
                        .await
                        .map(|reg| reg.devices)
                        .unwrap_or_default();
                    (count, devices)
                } else {
                    (0, vec![])
                }
            } else {
                (0, vec![])
            };

        let (rate_available, rate_capacity) =
            if let Some(ref rl) = *self.rate_limiter.read().await {
                (rl.available(), rl.capacity())
            } else {
                (0, 0)
            };

        WebDavSyncStatus {
            status,
            last_sync_at,
            cloud_entry_count,
            registered_devices,
            rate_limit_available: rate_available,
            rate_limit_capacity: rate_capacity,
            error: last_error,
        }
    }

    pub async fn update_config(&self, new_config: WebDavConfig) {
        let was_connected = *self.status.read().await == "connected";
        *self.config.write().await = new_config.clone();
        if was_connected {
            if let Some(ref poller) = *self.poller.read().await {
                poller.set_interval(new_config.poll_interval_secs).await;
            }
        }
    }

    pub async fn remove_device(&self, target_device_id: &str) -> Result<(), String> {
        let master_key = self
            .master_key
            .read()
            .await
            .clone()
            .ok_or("Not connected")?;
        let salt = self
            .salt
            .read()
            .await
            .clone()
            .ok_or("Not connected")?;
        let index_manager = self
            .index_manager
            .read()
            .await
            .clone()
            .ok_or("Not connected")?;

        let mut registry = index_manager.load_device_registry(&master_key).await?;
        registry
            .devices
            .retain(|d| d.device_id != target_device_id);
        index_manager
            .save_device_registry(&registry, &master_key, &salt)
            .await?;
        info!("Removed device {} from cloud registry", target_device_id);
        Ok(())
    }
}
