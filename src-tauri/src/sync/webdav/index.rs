use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use log::{info, warn};
use serde::{Deserialize, Serialize};

use super::client::{PutResult, WebDavClient};
use crate::sync::crypto;

const MAX_ETAG_RETRIES: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistry {
    pub version: u32,
    pub salt: String,
    pub devices: Vec<RegisteredDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredDevice {
    pub device_id: String,
    pub device_name: String,
    pub public_key: String,
    pub registered_at: String,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncIndex {
    pub version: u32,
    pub updated_at: String,
    pub updated_by: String,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub hash: String,
    pub content_type: String,
    pub category: String,
    pub source_device: String,
    pub created_at: String,
    pub size_bytes: u64,
}

pub struct IndexManager {
    client: Arc<WebDavClient>,
    remote_path: String,
    device_id: String,
}

impl IndexManager {
    pub fn new(client: Arc<WebDavClient>, remote_path: &str, device_id: &str) -> Self {
        Self {
            client,
            remote_path: remote_path.trim_matches('/').to_string(),
            device_id: device_id.to_string(),
        }
    }

    fn devices_path(&self) -> String {
        format!("{}/meta/devices.enc", self.remote_path)
    }

    fn index_path(&self) -> String {
        format!("{}/meta/index.enc", self.remote_path)
    }

    fn entry_path(&self, hash: &str) -> String {
        let prefix = if hash.len() >= 12 { &hash[..12] } else { hash };
        format!("{}/entries/{}.enc", self.remote_path, prefix)
    }

    // --- Device Registry ---

    pub async fn load_device_registry(&self, master_key: &[u8]) -> Result<DeviceRegistry, String> {
        let (encrypted, _etag) = self.client.get(&self.devices_path()).await?;
        let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to parse device registry: {e}"))
    }

    pub async fn save_device_registry(
        &self,
        registry: &DeviceRegistry,
        master_key: &[u8],
        salt: &[u8],
    ) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(registry)
            .map_err(|e| format!("Failed to serialize device registry: {e}"))?;
        let encrypted = crypto::encrypt_file(&json, master_key, salt)?;
        self.client.put(&self.devices_path(), &encrypted).await
    }

    pub async fn initialize_registry(
        &self,
        master_key: &[u8],
        salt: &[u8],
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Result<DeviceRegistry, String> {
        let registry = DeviceRegistry {
            version: 1,
            salt: crypto::encode_key_material(salt),
            devices: vec![RegisteredDevice {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                public_key: crypto::encode_key_material(public_key),
                registered_at: Utc::now().to_rfc3339(),
                last_sync_at: None,
            }],
        };
        self.save_device_registry(&registry, master_key, salt)
            .await?;
        info!("Initialized device registry with device {}", device_id);
        Ok(registry)
    }

    pub async fn register_device(
        &self,
        master_key: &[u8],
        salt: &[u8],
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Result<(), String> {
        let mut registry = self.load_device_registry(master_key).await?;

        if let Some(existing) = registry
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
        {
            existing.device_name = device_name.to_string();
            existing.public_key = crypto::encode_key_material(public_key);
            existing.last_sync_at = Some(Utc::now().to_rfc3339());
        } else {
            registry.devices.push(RegisteredDevice {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                public_key: crypto::encode_key_material(public_key),
                registered_at: Utc::now().to_rfc3339(),
                last_sync_at: None,
            });
        }

        self.save_device_registry(&registry, master_key, salt)
            .await?;
        info!("Registered device {} in cloud registry", device_id);
        Ok(())
    }

    // --- Index File ---

    pub async fn load_index(
        &self,
        master_key: &[u8],
    ) -> Result<(SyncIndex, Option<String>), String> {
        let result = self.client.get(&self.index_path()).await;
        match result {
            Ok((encrypted, etag)) => {
                let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
                let index: SyncIndex = serde_json::from_slice(&plaintext)
                    .map_err(|e| format!("Failed to parse index: {e}"))?;
                Ok((index, etag))
            }
            Err(e) if e == "NotFound" => {
                let empty = SyncIndex {
                    version: 1,
                    updated_at: Utc::now().to_rfc3339(),
                    updated_by: self.device_id.clone(),
                    entries: vec![],
                };
                Ok((empty, None))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn save_index(
        &self,
        index: &SyncIndex,
        master_key: &[u8],
        etag: Option<&str>,
    ) -> Result<bool, String> {
        let json = serde_json::to_vec_pretty(index)
            .map_err(|e| format!("Failed to serialize index: {e}"))?;
        let zero_salt = vec![0u8; 16];
        let encrypted = crypto::encrypt_file(&json, master_key, &zero_salt)?;

        if let Some(etag) = etag {
            match self
                .client
                .put_with_etag(&self.index_path(), &encrypted, etag)
                .await?
            {
                PutResult::Ok => Ok(true),
                PutResult::EtagConflict => Ok(false),
            }
        } else {
            self.client.put(&self.index_path(), &encrypted).await?;
            Ok(true)
        }
    }

    pub async fn append_entry(&self, entry: IndexEntry, master_key: &[u8]) -> Result<(), String> {
        for attempt in 0..MAX_ETAG_RETRIES {
            let (mut index, etag) = self.load_index(master_key).await?;

            if index.entries.iter().any(|e| e.hash == entry.hash) {
                return Ok(());
            }

            index.entries.push(entry.clone());
            index.updated_at = Utc::now().to_rfc3339();
            index.updated_by = self.device_id.clone();

            let saved = self.save_index(&index, master_key, etag.as_deref()).await?;
            if saved {
                return Ok(());
            }

            warn!(
                "Index ETag conflict on attempt {}, retrying...",
                attempt + 1
            );
        }
        Err("Failed to update index after max retries (ETag conflict)".to_string())
    }

    pub async fn enforce_entry_limit(
        &self,
        max_entries: u32,
        master_key: &[u8],
    ) -> Result<u32, String> {
        let (mut index, etag) = self.load_index(master_key).await?;
        let count = index.entries.len() as u32;
        if count <= max_entries {
            return Ok(0);
        }

        let remove_count = count - max_entries;
        let removed: Vec<IndexEntry> = index.entries.drain(..remove_count as usize).collect();

        for entry in &removed {
            if let Err(e) = self.client.delete(&self.entry_path(&entry.hash)).await {
                warn!("Failed to delete old entry file {}: {}", entry.hash, e);
            }
        }

        index.updated_at = Utc::now().to_rfc3339();
        index.updated_by = self.device_id.clone();
        self.save_index(&index, master_key, etag.as_deref()).await?;

        info!("Cleaned up {} old entries from cloud", remove_count);
        Ok(remove_count)
    }

    pub fn find_new_entries(
        &self,
        index: &SyncIndex,
        known_hashes: &HashSet<String>,
        local_device_id: &str,
    ) -> Vec<IndexEntry> {
        index
            .entries
            .iter()
            .filter(|e| !known_hashes.contains(&e.hash) && e.source_device != local_device_id)
            .cloned()
            .collect()
    }

    // --- Entry Files ---

    pub async fn upload_entry(
        &self,
        hash: &str,
        plaintext_json: &[u8],
        master_key: &[u8],
    ) -> Result<(), String> {
        let zero_salt = vec![0u8; 16];
        let encrypted = crypto::encrypt_file(plaintext_json, master_key, &zero_salt)?;
        self.client.put(&self.entry_path(hash), &encrypted).await
    }

    pub async fn download_entry(&self, hash: &str, master_key: &[u8]) -> Result<Vec<u8>, String> {
        let (encrypted, _etag) = self.client.get(&self.entry_path(hash)).await?;
        let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
        Ok(plaintext)
    }
}
