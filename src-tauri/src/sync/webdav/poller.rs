use std::collections::HashSet;
use std::sync::Arc;

use chrono::Local;
use log::{error, info, warn};
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::index::IndexManager;
use crate::storage::{ClipboardEntry, Database};
use crate::sync::protocol::SyncEntryPayload;

pub struct SyncPoller {
    db: Arc<Database>,
    index_manager: Arc<IndexManager>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    device_id: String,
    poll_handle: RwLock<Option<JoinHandle<()>>>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl SyncPoller {
    pub fn new(
        db: Arc<Database>,
        index_manager: Arc<IndexManager>,
        master_key: Arc<RwLock<Option<Vec<u8>>>>,
        device_id: &str,
        app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> Self {
        Self {
            db,
            index_manager,
            master_key,
            device_id: device_id.to_string(),
            poll_handle: RwLock::new(None),
            app_handle,
        }
    }

    pub async fn start(&self, interval_secs: u64) {
        self.stop().await;

        let db = self.db.clone();
        let index_manager = self.index_manager.clone();
        let master_key = self.master_key.clone();
        let device_id = self.device_id.clone();
        let app_handle = self.app_handle.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.tick().await; // Skip first immediate tick

            loop {
                interval.tick().await;

                let key = {
                    let guard = master_key.read().await;
                    match guard.as_ref() {
                        Some(k) => k.clone(),
                        None => {
                            warn!("WebDAV poller: no master key available, skipping poll");
                            continue;
                        }
                    }
                };

                if let Err(e) =
                    Self::do_poll(&db, &index_manager, &key, &device_id, &app_handle).await
                {
                    error!("WebDAV poll error: {}", e);
                }
            }
        });

        *self.poll_handle.write().await = Some(handle);
        info!("WebDAV poller started with {}s interval", interval_secs);
    }

    pub async fn stop(&self) {
        if let Some(handle) = self.poll_handle.write().await.take() {
            handle.abort();
            info!("WebDAV poller stopped");
        }
    }

    pub async fn set_interval(&self, interval_secs: u64) {
        let is_running = self.poll_handle.read().await.is_some();
        if is_running {
            self.start(interval_secs).await;
        }
    }

    pub async fn poll_now(&self) -> Result<u32, String> {
        let key = {
            let guard = self.master_key.read().await;
            guard
                .as_ref()
                .ok_or_else(|| "No master key available".to_string())?
                .clone()
        };
        Self::do_poll(
            &self.db,
            &self.index_manager,
            &key,
            &self.device_id,
            &self.app_handle,
        )
        .await
    }

    async fn do_poll(
        db: &Arc<Database>,
        index_manager: &Arc<IndexManager>,
        master_key: &[u8],
        device_id: &str,
        app_handle: &Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> Result<u32, String> {
        let (index, _etag) = index_manager.load_index(master_key).await?;

        let known_hashes = Self::build_known_hashes(db)?;

        let new_entries = index_manager.find_new_entries(&index, &known_hashes, device_id);
        if new_entries.is_empty() {
            return Ok(0);
        }

        info!(
            "WebDAV poll: found {} new entries to download",
            new_entries.len()
        );

        let mut downloaded = 0u32;
        for index_entry in &new_entries {
            match index_manager
                .download_entry(&index_entry.hash, master_key)
                .await
            {
                Ok(plaintext) => match serde_json::from_slice::<SyncEntryPayload>(&plaintext) {
                    Ok(payload) => {
                        if Self::insert_synced_entry(db, &payload, app_handle).await {
                            downloaded += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse entry {}: {}", index_entry.hash, e);
                    }
                },
                Err(e) => {
                    warn!("Failed to download entry {}: {}", index_entry.hash, e);
                }
            }
        }

        info!("WebDAV poll: downloaded {} new entries", downloaded);
        Ok(downloaded)
    }

    fn build_known_hashes(db: &Database) -> Result<HashSet<String>, String> {
        db.get_all_hashes()
            .map_err(|e| format!("Failed to get known hashes: {e}"))
    }

    async fn insert_synced_entry(
        db: &Arc<Database>,
        payload: &SyncEntryPayload,
        app_handle: &Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> bool {
        match db.find_by_hash(&payload.hash) {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(e) => {
                error!("DB error during WebDAV sync dedup: {}", e);
                return false;
            }
        }

        let created_at =
            chrono::NaiveDateTime::parse_from_str(&payload.created_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Local::now().naive_local());
        let now = Local::now().naive_local();

        let entry = ClipboardEntry {
            id: None,
            content: payload.content.clone(),
            content_type: payload.content_type.clone(),
            category: payload.category.clone(),
            hash: payload.hash.clone(),
            source_app: payload.source_app.clone(),
            is_favorite: false,
            is_sensitive: payload.is_sensitive,
            use_count: 1,
            created_at,
            updated_at: now,
            expires_at: None,
            source_device: Some(payload.source_device.clone()),
        };

        match db.insert_entry(&entry) {
            Ok(id) => {
                let mut stored = entry;
                stored.id = Some(id);
                if let Some(handle) = app_handle.read().await.as_ref() {
                    let _ = handle.emit("clipboard-changed", &stored);
                }
                true
            }
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    false
                } else {
                    error!("Failed to insert WebDAV synced entry: {}", e);
                    false
                }
            }
        }
    }
}
