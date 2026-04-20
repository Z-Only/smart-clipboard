use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, DiscoveredDevice, PairedDevice, SyncStatus};

pub mod mdns;

const DISCOVERY_STALE_AFTER_SECS: i64 = 20;
const DEFAULT_SERVICE_TYPE: &str = "_smartclip._tcp.local.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    discovery: Arc<mdns::MdnsDiscoveryService>,
    runtime_status: RwLock<SyncRuntimeStatus>,
}

impl SyncManager {
    pub fn new(db: Arc<Database>, config: Arc<ConfigManager>) -> Self {
        let sync_config = config.get().sync;
        let discovery = mdns::MdnsDiscoveryService::start(mdns::MdnsConfig {
            service_type: DEFAULT_SERVICE_TYPE.to_string(),
            device_name: sync_config.device_name.clone(),
            device_id: mdns::load_or_create_device_id(config.as_ref()),
            port: sync_config.port,
            enabled: sync_config.enabled,
        });

        let runtime_status = SyncRuntimeStatus {
            enabled: sync_config.enabled,
            state: if sync_config.enabled {
                "idle"
            } else {
                "disabled"
            }
            .to_string(),
            paired_count: 0,
            online_count: 0,
            last_sync_at: None,
            message: if sync_config.enabled {
                "LAN Sync discovery is active. Nearby devices will appear automatically when advertised via mDNS.".to_string()
            } else {
                "LAN Sync is disabled. Enable it to advertise and discover nearby devices."
                    .to_string()
            },
        };

        Self {
            db,
            config,
            discovery,
            runtime_status: RwLock::new(runtime_status),
        }
    }

    pub fn get_config(&self) -> SyncConfig {
        self.config.get().sync
    }

    pub fn update_config(&self, sync_config: SyncConfig) -> Result<(), String> {
        let mut app_config: AppConfig = self.config.get();
        app_config.sync = sync_config.clone();
        self.config.update(app_config)?;

        self.discovery.update_config(mdns::MdnsConfig {
            service_type: DEFAULT_SERVICE_TYPE.to_string(),
            device_name: sync_config.device_name.clone(),
            device_id: mdns::load_or_create_device_id(self.config.as_ref()),
            port: sync_config.port,
            enabled: sync_config.enabled,
        });

        let mut status = self.runtime_status.blocking_write();
        status.enabled = sync_config.enabled;
        status.state = if sync_config.enabled {
            "idle"
        } else {
            "disabled"
        }
        .to_string();
        status.message = if sync_config.enabled {
            "LAN Sync discovery is active. Nearby devices will appear automatically when advertised via mDNS.".to_string()
        } else {
            "LAN Sync is disabled. Enable it to advertise and discover nearby devices.".to_string()
        };
        Ok(())
    }

    pub fn get_discovered_devices(&self) -> Result<Vec<DiscoveredDevice>, String> {
        let paired_ids: HashSet<String> = self
            .db
            .get_paired_devices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|d| d.id)
            .collect();

        let now = Local::now().naive_local();
        let mut devices = self.discovery.current_devices();
        for device in &mut devices {
            device.is_paired = paired_ids.contains(&device.id);
        }
        devices.sort_by(|a, b| {
            b.last_seen_at
                .cmp(&a.last_seen_at)
                .then_with(|| a.name.cmp(&b.name))
        });

        let online_count = devices
            .iter()
            .filter(|device| {
                now.signed_duration_since(device.last_seen_at).num_seconds()
                    <= DISCOVERY_STALE_AFTER_SECS
            })
            .count();

        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired, Some(online_count));
        Ok(devices)
    }

    pub fn get_paired_devices(&self) -> Result<Vec<PairedDevice>, String> {
        let devices = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        let discovered = self.discovery.current_devices();
        let now = Local::now().naive_local();
        let online_count = discovered
            .iter()
            .filter(|device| {
                now.signed_duration_since(device.last_seen_at).num_seconds()
                    <= DISCOVERY_STALE_AFTER_SECS
            })
            .count();
        self.refresh_status(&devices, Some(online_count));
        Ok(devices)
    }

    pub fn get_status(&self) -> Result<SyncStatus, String> {
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        let discovered = self.discovery.current_devices();
        let now = Local::now().naive_local();
        let online_count = discovered
            .iter()
            .filter(|device| {
                now.signed_duration_since(device.last_seen_at).num_seconds()
                    <= DISCOVERY_STALE_AFTER_SECS
            })
            .count();
        self.refresh_status(&paired, Some(online_count));
        let runtime = self.runtime_status.blocking_read().clone();
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
            .discovery
            .current_devices()
            .into_iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("Device not found: {}", device_id))?;

        let device_name = device.name.clone();
        self.db
            .upsert_paired_device(&PairedDevice {
                id: device.id,
                name: device.name.clone(),
                device_name: device.device_name,
                host: device.host.clone(),
                address: device.address,
                ip: device.ip,
                port: device.port,
                status: sync_device_status(Some(device.last_seen_at)).to_string(),
                public_key: None,
                shared_secret: None,
                last_seen_at: Some(device.last_seen_at),
                is_active: true,
                enabled: true,
                sync_enabled: true,
                paired_at: Local::now().naive_local(),
                fingerprint: None,
            })
            .map_err(|e| e.to_string())?;

        let mut status = self.runtime_status.blocking_write();
        status.last_sync_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        status.message = format!(
            "Paired with {}. Phase 3 discovery is live; transport and encryption layers are still pending.",
            device_name
        );
        drop(status);
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    pub fn unpair_device(&self, device_id: &str) -> Result<(), String> {
        self.db
            .unpair_device(device_id)
            .map_err(|e| e.to_string())?;
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    pub fn toggle_device_sync(&self, device_id: &str, enabled: bool) -> Result<(), String> {
        self.db
            .set_paired_device_active(device_id, enabled)
            .map_err(|e| e.to_string())?;
        let paired = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    fn refresh_status(&self, paired: &[PairedDevice], online_count: Option<usize>) {
        let sync_enabled = self.config.get().sync.enabled;
        let discovered = self.discovery.current_devices();
        let now = Local::now().naive_local();
        let online_count = online_count.unwrap_or_else(|| {
            discovered
                .iter()
                .filter(|device| {
                    now.signed_duration_since(device.last_seen_at).num_seconds()
                        <= DISCOVERY_STALE_AFTER_SECS
                })
                .count()
        });

        let mut status = self.runtime_status.blocking_write();
        status.enabled = sync_enabled;
        status.paired_count = paired.len();
        status.online_count = online_count;
        status.state = if !sync_enabled {
            "disabled".to_string()
        } else if online_count > 0 {
            "ready".to_string()
        } else {
            "idle".to_string()
        };
        status.message = if !sync_enabled {
            "LAN Sync is disabled. Enable it to advertise and discover nearby devices.".to_string()
        } else if online_count > 0 {
            format!("Discovered {} nearby device(s) via mDNS.", online_count)
        } else if paired.is_empty() {
            "Scanning for nearby Smart Clipboard devices via mDNS…".to_string()
        } else {
            "Waiting for paired devices to appear online on the local network.".to_string()
        };
    }
}

pub fn sync_device_status(last_seen_at: Option<NaiveDateTime>) -> &'static str {
    match last_seen_at {
        Some(last_seen) => {
            if Local::now()
                .naive_local()
                .signed_duration_since(last_seen)
                .num_seconds()
                <= DISCOVERY_STALE_AFTER_SECS
            {
                "online"
            } else {
                "offline"
            }
        }
        None => "unknown",
    }
}
