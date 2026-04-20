use std::sync::{Arc, Mutex};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, DiscoveredDevice, PairedDevice, SyncStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub device_name: String,
    pub port: u16,
    pub auto_sync: bool,
    pub sync_images: bool,
    pub sync_sensitive: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        let default_name = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "Smart Clipboard Device".to_string());

        Self {
            enabled: false,
            device_name: default_name,
            port: 23456,
            auto_sync: true,
            sync_images: false,
            sync_sensitive: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRuntimeStatus {
    pub enabled: bool,
    pub state: String,
    pub paired_count: usize,
    pub online_count: usize,
    pub last_sync_at: Option<String>,
    pub message: String,
}

pub struct SyncManager {
    db: Arc<Database>,
    config: Arc<ConfigManager>,
    discovered_devices: Mutex<Vec<DiscoveredDevice>>,
    runtime_status: Mutex<SyncRuntimeStatus>,
}

impl SyncManager {
    pub fn new(db: Arc<Database>, config: Arc<ConfigManager>) -> Self {
        let discovered_devices = vec![
            DiscoveredDevice {
                id: "demo-macbook".to_string(),
                name: "MacBook Pro (Demo)".to_string(),
                host: "192.168.1.23".to_string(),
                port: 23456,
                version: "mvp".to_string(),
                last_seen_at: Local::now().naive_local(),
                is_paired: false,
            },
            DiscoveredDevice {
                id: "demo-ipad".to_string(),
                name: "iPad Mini (Demo)".to_string(),
                host: "192.168.1.45".to_string(),
                port: 23456,
                version: "mvp".to_string(),
                last_seen_at: Local::now().naive_local(),
                is_paired: false,
            },
        ];

        let sync_config = config.get().sync;
        let runtime_status = SyncRuntimeStatus {
            enabled: sync_config.enabled,
            state: if sync_config.enabled { "idle" } else { "disabled" }.to_string(),
            paired_count: 0,
            online_count: 0,
            last_sync_at: None,
            message: "LAN Sync MVP is ready. Discovery and pairing are local-only scaffolding in this release.".to_string(),
        };

        Self {
            db,
            config,
            discovered_devices: Mutex::new(discovered_devices),
            runtime_status: Mutex::new(runtime_status),
        }
    }

    pub fn get_config(&self) -> SyncConfig {
        self.config.get().sync
    }

    pub fn update_config(&self, sync_config: SyncConfig) -> Result<(), String> {
        let mut app_config: AppConfig = self.config.get();
        app_config.sync = sync_config.clone();
        self.config.update(app_config)?;

        let mut status = self.runtime_status.lock().unwrap();
        status.enabled = sync_config.enabled;
        status.state = if sync_config.enabled { "idle" } else { "disabled" }.to_string();
        Ok(())
    }

    pub fn get_discovered_devices(&self) -> Result<Vec<DiscoveredDevice>, String> {
        let paired_ids: std::collections::HashSet<String> = self
            .db
            .get_paired_devices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|d| d.id)
            .collect();

        let mut devices = self.discovered_devices.lock().unwrap().clone();
        for device in &mut devices {
            device.is_paired = paired_ids.contains(&device.id);
            device.last_seen_at = Local::now().naive_local();
        }
        Ok(devices)
    }

    pub fn get_paired_devices(&self) -> Result<Vec<PairedDevice>, String> {
        let devices = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&devices);
        Ok(devices)
    }

    pub fn get_status(&self) -> Result<SyncStatus, String> {
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired);
        let runtime = self.runtime_status.lock().unwrap().clone();
        Ok(SyncStatus {
            enabled: runtime.enabled,
            state: runtime.state,
            paired_count: runtime.paired_count as i64,
            online_count: runtime.online_count as i64,
            last_sync_at: runtime.last_sync_at,
            message: runtime.message,
        })
    }

    pub fn pair_device(&self, device_id: &str) -> Result<(), String> {
        let device = self
            .discovered_devices
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == device_id)
            .cloned()
            .ok_or_else(|| format!("Device not found: {}", device_id))?;

        let device_name = device.name.clone();
        self.db
            .upsert_paired_device(&PairedDevice {
                id: device.id,
                name: device.name,
                host: device.host,
                port: device.port as i64,
                public_key: None,
                shared_secret: None,
                last_seen_at: Some(Local::now().naive_local()),
                is_active: true,
                paired_at: Local::now().naive_local(),
            })
            .map_err(|e| e.to_string())?;

        let mut status = self.runtime_status.lock().unwrap();
        status.last_sync_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        status.message = format!("Paired with {}. Network transport is not enabled yet in MVP.", device_name);
        drop(status);
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired);
        Ok(())
    }

    pub fn unpair_device(&self, device_id: &str) -> Result<(), String> {
        self.db.unpair_device(device_id).map_err(|e| e.to_string())?;
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired);
        Ok(())
    }

    pub fn toggle_device_sync(&self, device_id: &str, enabled: bool) -> Result<(), String> {
        self.db
            .set_paired_device_active(device_id, enabled)
            .map_err(|e| e.to_string())?;
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired);
        Ok(())
    }

    fn refresh_status(&self, paired: &[PairedDevice]) {
        let sync_enabled = self.config.get().sync.enabled;
        let mut status = self.runtime_status.lock().unwrap();
        status.enabled = sync_enabled;
        status.paired_count = paired.len();
        status.online_count = paired.iter().filter(|d| d.is_active).count();
        status.state = if !sync_enabled {
            "disabled".to_string()
        } else if paired.is_empty() {
            "idle".to_string()
        } else {
            "ready".to_string()
        };
        if paired.is_empty() {
            status.message = "No paired devices yet. Pair a discovered device to prepare LAN sync.".to_string();
        }
    }
}
