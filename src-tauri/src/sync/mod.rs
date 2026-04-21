use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Local, NaiveDateTime};
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, DiscoveredDevice, PairedDevice, SyncStatus};
use serde_json::json;

pub mod client;
pub mod crypto;
pub mod mdns;
pub mod protocol;
pub mod server;
pub mod webdav;

const DISCOVERY_STALE_AFTER_SECS: i64 = 20;
const DEFAULT_SERVICE_TYPE: &str = "_smartclip._tcp.local.";
const HEARTBEAT_INTERVAL_SECS: u64 = 30;
const HEARTBEAT_TIMEOUT_SECS: i64 = 65;
const RECONNECT_BACKOFF_SECS: [u64; 6] = [1, 2, 4, 8, 16, 30];
const MAX_SYNC_PAYLOAD_BYTES: usize = 1_048_576; // 1 MB

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConnectionState {
    pub status: String,
    pub last_event_at: String,
    pub last_error: Option<String>,
    pub connection_role: Option<String>,
    pub connection_attempts: u32,
    pub reconnect_scheduled_in_secs: Option<u64>,
    pub last_ping_at: Option<String>,
    pub last_pong_at: Option<String>,
}

impl Default for DeviceConnectionState {
    fn default() -> Self {
        Self {
            status: "offline".to_string(),
            last_event_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            last_error: None,
            connection_role: None,
            connection_attempts: 0,
            reconnect_scheduled_in_secs: None,
            last_ping_at: None,
            last_pong_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalDeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub port: u16,
    pub public_key: Vec<u8>,
}

pub struct SyncManager {
    db: Arc<Database>,
    config: Arc<ConfigManager>,
    discovery: Arc<mdns::MdnsDiscoveryService>,
    runtime_status: RwLock<SyncRuntimeStatus>,
    connection_states: RwLock<HashMap<String, DeviceConnectionState>>,
    local_device: LocalDeviceInfo,
    local_private_key: Vec<u8>,
    outgoing_tx: broadcast::Sender<protocol::SyncEntryPayload>,
    app_handle: RwLock<Option<tauri::AppHandle>>,
}

impl SyncManager {
    pub fn new(db: Arc<Database>, config: Arc<ConfigManager>) -> Arc<Self> {
        let sync_config = config.get().sync;
        let device_id = mdns::load_or_create_device_id(config.as_ref());
        let discovery = mdns::MdnsDiscoveryService::start(mdns::MdnsConfig {
            service_type: DEFAULT_SERVICE_TYPE.to_string(),
            device_name: sync_config.device_name.clone(),
            device_id: device_id.clone(),
            port: sync_config.port,
            enabled: sync_config.enabled,
        });

        let keypair = Self::load_or_create_local_keypair(config.as_ref());

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

        let (outgoing_tx, _) = broadcast::channel::<protocol::SyncEntryPayload>(64);

        let manager = Arc::new(Self {
            db,
            config,
            discovery,
            runtime_status: RwLock::new(runtime_status),
            connection_states: RwLock::new(HashMap::new()),
            local_device: LocalDeviceInfo {
                device_id,
                device_name: sync_config.device_name.clone(),
                port: sync_config.port,
                public_key: keypair.public_key.clone(),
            },
            local_private_key: keypair.private_key,
            outgoing_tx,
            app_handle: RwLock::new(None),
        });

        manager.start_transport_tasks();
        manager
    }

    pub fn get_config(&self) -> SyncConfig {
        self.config.get().sync
    }

    pub fn local_device_info(&self) -> LocalDeviceInfo {
        let cfg = self.config.get().sync;
        LocalDeviceInfo {
            device_id: self.local_device.device_id.clone(),
            device_name: cfg.device_name,
            port: cfg.port,
            public_key: self.local_device.public_key.clone(),
        }
    }

    pub fn is_sync_enabled(&self) -> bool {
        self.config.get().sync.enabled
    }

    /// Check if an entry should be synced based on current config filters.
    pub fn should_sync_entry(&self, entry: &crate::storage::ClipboardEntry) -> bool {
        let config = self.get_config();
        if !config.enabled || !config.auto_sync {
            return false;
        }
        if entry.source_device.is_some() {
            return false;
        }
        if entry.content_type == "image" && !config.sync_images {
            return false;
        }
        if entry.is_sensitive && !config.sync_sensitive {
            return false;
        }
        if entry.content.len() > MAX_SYNC_PAYLOAD_BYTES {
            return false;
        }
        true
    }

    /// Convert a local ClipboardEntry into a SyncEntryPayload for transmission.
    pub fn entry_to_sync_payload(
        &self,
        entry: &crate::storage::ClipboardEntry,
    ) -> protocol::SyncEntryPayload {
        protocol::SyncEntryPayload {
            content: entry.content.clone(),
            content_type: entry.content_type.clone(),
            category: entry.category.clone(),
            hash: entry.hash.clone(),
            source_app: entry.source_app.clone(),
            is_sensitive: entry.is_sensitive,
            source_device: self.local_device.device_id.clone(),
            created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// Set the Tauri AppHandle for emitting frontend events.
    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.blocking_write() = Some(handle);
    }

    /// Get a clone of the AppHandle if available.
    pub fn app_handle(&self) -> Option<tauri::AppHandle> {
        self.app_handle.blocking_read().clone()
    }

    /// Get a reference to the database.
    pub fn db_ref(&self) -> &Database {
        &self.db
    }

    /// Subscribe to outgoing entry broadcasts (used by server/client connections).
    pub fn subscribe_outgoing(&self) -> broadcast::Receiver<protocol::SyncEntryPayload> {
        self.outgoing_tx.subscribe()
    }

    /// Broadcast a clipboard entry to all connected paired devices.
    pub fn broadcast_entry(&self, entry: &crate::storage::ClipboardEntry) {
        if !self.should_sync_entry(entry) {
            return;
        }
        let payload = self.entry_to_sync_payload(entry);
        log::info!(
            "Broadcasting clipboard entry {} to paired devices",
            payload.hash
        );
        let _ = self.outgoing_tx.send(payload);
    }

    /// Handle an incoming ClipboardSync message from a remote device.
    pub fn handle_incoming_sync(
        &self,
        sender_device_id: &str,
        payload: &protocol::SyncEntryPayload,
    ) -> Result<Option<crate::storage::ClipboardEntry>, String> {
        let hash = &payload.hash;

        // 1. Check if we already have this entry (by hash in DB)
        match self.db.find_by_hash(hash) {
            Ok(Some(_)) => {
                log::info!("Skipping duplicate sync entry: {}", hash);
                let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
                return Ok(None);
            }
            Ok(None) => {}
            Err(e) => return Err(format!("DB error during sync dedup: {}", e)),
        }

        // 2. Check sync_log for already-received entries
        if self.db.has_received_entry(hash).unwrap_or(false) {
            log::info!("Already received sync entry via sync_log: {}", hash);
            return Ok(None);
        }

        // 3. Parse created_at
        let created_at =
            chrono::NaiveDateTime::parse_from_str(&payload.created_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| chrono::Local::now().naive_local());
        let now = chrono::Local::now().naive_local();

        // 4. Build ClipboardEntry
        let entry = crate::storage::ClipboardEntry {
            id: None,
            content: payload.content.clone(),
            content_type: payload.content_type.clone(),
            category: payload.category.clone(),
            hash: hash.clone(),
            source_app: payload.source_app.clone(),
            is_favorite: false,
            is_sensitive: payload.is_sensitive,
            use_count: 1,
            created_at,
            updated_at: now,
            expires_at: None,
            source_device: Some(sender_device_id.to_string()),
        };

        // 5. Insert into DB
        match self.db.insert_entry(&entry) {
            Ok(id) => {
                let mut stored = entry;
                stored.id = Some(id);
                let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
                self.touch_last_sync(format!(
                    "Received clipboard entry from {}",
                    sender_device_id
                ));
                log::info!(
                    "Stored synced entry {} from device {}",
                    hash,
                    sender_device_id
                );
                Ok(Some(stored))
            }
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    log::info!("Sync entry {} already exists (race dedup)", hash);
                    let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
                    Ok(None)
                } else {
                    Err(format!("Failed to insert synced entry: {}", e))
                }
            }
        }
    }

    pub fn update_config(&self, sync_config: SyncConfig) -> Result<(), String> {
        let mut app_config: AppConfig = self.config.get();
        app_config.sync = sync_config.clone();
        self.config.update(app_config)?;

        self.discovery.update_config(mdns::MdnsConfig {
            service_type: DEFAULT_SERVICE_TYPE.to_string(),
            device_name: sync_config.device_name.clone(),
            device_id: self.local_device.device_id.clone(),
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
            "LAN Sync transport is active. Paired devices will connect automatically when reachable.".to_string()
        } else {
            "LAN Sync is disabled. Enable it to advertise and discover nearby devices.".to_string()
        };
        drop(status);

        if !sync_config.enabled {
            let mut states = self.connection_states.blocking_write();
            for state in states.values_mut() {
                state.status = "disabled".to_string();
                state.reconnect_scheduled_in_secs = None;
                state.connection_role = None;
                state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }
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
        let mut devices = self.db.get_paired_devices().map_err(|e| e.to_string())?;
        self.apply_runtime_device_states(&mut devices);
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
        let paired = self.get_paired_devices()?;
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
                id: device.id.clone(),
                name: device.name.clone(),
                device_name: device.device_name,
                host: device.host.clone(),
                address: device.address,
                ip: device.ip,
                port: device.port,
                status: "connecting".to_string(),
                public_key: None,
                local_public_key: Some(self.local_device.public_key.clone()),
                shared_secret: None,
                last_seen_at: Some(device.last_seen_at),
                is_active: true,
                enabled: true,
                sync_enabled: true,
                paired_at: Local::now().naive_local(),
                fingerprint: None,
            })
            .map_err(|e| e.to_string())?;

        self.mark_connecting(device_id, Some("client".to_string()));

        let mut status = self.runtime_status.blocking_write();
        status.last_sync_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        status.message = format!(
            "Paired with {}. Transport handshake will be attempted automatically.",
            device_name
        );
        drop(status);
        let paired = self.get_paired_devices()?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    pub fn unpair_device(&self, device_id: &str) -> Result<(), String> {
        self.db
            .unpair_device(device_id)
            .map_err(|e| e.to_string())?;
        self.connection_states.blocking_write().remove(device_id);
        let paired = self.get_paired_devices()?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    pub fn toggle_device_sync(&self, device_id: &str, enabled: bool) -> Result<(), String> {
        self.db
            .set_paired_device_active(device_id, enabled)
            .map_err(|e| e.to_string())?;
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = if enabled { "offline" } else { "disabled" }.to_string();
        state.reconnect_scheduled_in_secs = None;
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        drop(states);
        let paired = self.get_paired_devices()?;
        self.refresh_status(&paired, None);
        Ok(())
    }

    pub fn accept_incoming_connection(&self, device_id: &str) -> bool {
        self.db
            .get_paired_devices()
            .map(|devices| {
                devices
                    .into_iter()
                    .any(|d| d.id == device_id && d.is_active)
            })
            .unwrap_or(false)
    }

    pub fn handle_hello(&self, device_id: &str, device_name: Option<String>, port: Option<u16>) {
        if let Ok(Some(mut device)) = self.find_paired_device(device_id) {
            if let Some(name) = device_name {
                device.name = name.clone();
                device.device_name = name;
            }
            if let Some(port) = port {
                device.port = i64::from(port);
            }
            device.last_seen_at = Some(Local::now().naive_local());
            let _ = self.db.upsert_paired_device(&device);
        }
    }

    pub fn mark_connecting(&self, device_id: &str, role: Option<String>) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "connecting".to_string();
        state.connection_role = role;
        state.connection_attempts = state.connection_attempts.saturating_add(1);
        state.reconnect_scheduled_in_secs = None;
        state.last_error = None;
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn mark_connected(&self, device_id: &str, role: Option<String>) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "connected".to_string();
        state.connection_role = role;
        state.reconnect_scheduled_in_secs = None;
        state.last_error = None;
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if state.last_pong_at.is_none() {
            state.last_pong_at = Some(state.last_event_at.clone());
        }
    }

    pub fn mark_ping(&self, device_id: &str) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.last_ping_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    }

    pub fn mark_pong(&self, device_id: &str) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "connected".to_string();
        state.last_pong_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        state.last_event_at = state.last_pong_at.clone().unwrap_or_default();
        state.reconnect_scheduled_in_secs = None;
        state.last_error = None;
    }

    pub fn mark_disconnected(&self, device_id: &str, reason: Option<String>) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "offline".to_string();
        state.last_error = reason;
        state.reconnect_scheduled_in_secs = None;
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn mark_reconnect_scheduled(
        &self,
        device_id: &str,
        delay_secs: u64,
        reason: Option<String>,
    ) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "reconnecting".to_string();
        state.reconnect_scheduled_in_secs = Some(delay_secs);
        state.last_error = reason;
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn mark_error(&self, device_id: &str, error: String) {
        let mut states = self.connection_states.blocking_write();
        let state = states.entry(device_id.to_string()).or_default();
        state.status = "error".to_string();
        state.last_error = Some(error);
        state.last_event_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn touch_last_sync(&self, message: impl Into<String>) {
        let mut status = self.runtime_status.blocking_write();
        status.last_sync_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        status.message = message.into();
    }

    fn start_transport_tasks(self: &Arc<Self>) {
        server::spawn(self.clone());
        client::spawn(self.clone());
        self.spawn_connection_watchdog();
    }

    fn spawn_connection_watchdog(self: &Arc<Self>) {
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(10));
            loop {
                ticker.tick().await;
                manager.reconcile_connection_states().await;
            }
        });
    }

    async fn reconcile_connection_states(&self) {
        let discovered = self.discovery.current_devices();
        let discovered_map: HashMap<String, DiscoveredDevice> = discovered
            .into_iter()
            .map(|device| (device.id.clone(), device))
            .collect();

        let paired = match self.db.get_paired_devices() {
            Ok(devices) => devices,
            Err(err) => {
                warn!("Failed to load paired devices during reconcile: {err}");
                return;
            }
        };

        let now = Local::now().naive_local();
        let mut states = self.connection_states.write().await;
        for device in &paired {
            let state = states.entry(device.id.clone()).or_default();
            if !self.is_sync_enabled() || !device.is_active {
                state.status = "disabled".to_string();
                state.reconnect_scheduled_in_secs = None;
                continue;
            }

            let discovered_online = discovered_map
                .get(&device.id)
                .map(|d| {
                    now.signed_duration_since(d.last_seen_at).num_seconds()
                        <= DISCOVERY_STALE_AFTER_SECS
                })
                .unwrap_or(false);

            let last_pong_recent = state
                .last_pong_at
                .as_ref()
                .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S").ok())
                .map(|dt| now.signed_duration_since(dt).num_seconds() <= HEARTBEAT_TIMEOUT_SECS)
                .unwrap_or(false);

            if state.status == "connected" && !last_pong_recent {
                state.status = if discovered_online {
                    "online".to_string()
                } else {
                    "offline".to_string()
                };
            } else if state.status == "offline" && discovered_online {
                state.status = "online".to_string();
            } else if state.status == "disabled"
                && discovered_online
                && self.is_sync_enabled()
                && device.is_active
            {
                state.status = "online".to_string();
            }
        }
    }

    fn apply_runtime_device_states(&self, devices: &mut [PairedDevice]) {
        let discovered = self.discovery.current_devices();
        let discovered_map: HashMap<String, DiscoveredDevice> = discovered
            .into_iter()
            .map(|device| (device.id.clone(), device))
            .collect();
        let states = self.connection_states.blocking_read().clone();
        let sync_enabled = self.is_sync_enabled();
        let now = Local::now().naive_local();

        for device in devices.iter_mut() {
            if let Some(discovered) = discovered_map.get(&device.id) {
                device.host = discovered.host.clone();
                device.address = discovered.address.clone();
                device.ip = discovered.ip.clone();
                device.port = discovered.port;
                device.last_seen_at = Some(discovered.last_seen_at);
            }

            let discovered_online = device
                .last_seen_at
                .map(|last_seen| {
                    now.signed_duration_since(last_seen).num_seconds() <= DISCOVERY_STALE_AFTER_SECS
                })
                .unwrap_or(false);

            device.status = if !sync_enabled || !device.is_active {
                "disabled".to_string()
            } else if let Some(state) = states.get(&device.id) {
                let mut status = state.status.clone();
                if status == "offline" && discovered_online {
                    status = "online".to_string();
                }
                status
            } else if discovered_online {
                "online".to_string()
            } else {
                "offline".to_string()
            };

            if let Some(state) = states.get(&device.id) {
                let mut extras = Vec::new();
                if let Some(role) = &state.connection_role {
                    extras.push(format!("role={role}"));
                }
                if state.connection_attempts > 0 {
                    extras.push(format!("attempts={}", state.connection_attempts));
                }
                if let Some(delay) = state.reconnect_scheduled_in_secs {
                    extras.push(format!("retry={}s", delay));
                }
                if let Some(last_error) = &state.last_error {
                    extras.push(format!("error={last_error}"));
                }
                device.fingerprint = if extras.is_empty() {
                    None
                } else {
                    Some(extras.join(" | "))
                };
            }
        }
    }

    fn refresh_status(&self, paired: &[PairedDevice], online_count: Option<usize>) {
        let sync_enabled = self.config.get().sync.enabled;
        let discovered = self.discovery.current_devices();
        let now = Local::now().naive_local();
        let mdns_online_count = online_count.unwrap_or_else(|| {
            discovered
                .iter()
                .filter(|device| {
                    now.signed_duration_since(device.last_seen_at).num_seconds()
                        <= DISCOVERY_STALE_AFTER_SECS
                })
                .count()
        });
        let connected_count = paired
            .iter()
            .filter(|device| matches!(device.status.as_str(), "connected" | "online"))
            .count();

        let mut status = self.runtime_status.blocking_write();
        status.enabled = sync_enabled;
        status.paired_count = paired.len();
        status.online_count = connected_count;
        status.state = if !sync_enabled {
            "disabled".to_string()
        } else if paired.iter().any(|device| device.status == "connecting") {
            "connecting".to_string()
        } else if connected_count > 0 {
            "connected".to_string()
        } else if mdns_online_count > 0 {
            "ready".to_string()
        } else {
            "idle".to_string()
        };
        status.message = if !sync_enabled {
            "LAN Sync is disabled. Enable it to advertise and discover nearby devices.".to_string()
        } else if connected_count > 0 {
            format!(
                "{} paired device(s) currently connected over WebSocket.",
                connected_count
            )
        } else if paired.iter().any(|device| device.status == "reconnecting") {
            "Waiting for transport reconnection to complete…".to_string()
        } else if paired.iter().any(|device| device.status == "connecting") {
            "Attempting WebSocket transport handshake with paired devices…".to_string()
        } else if paired.is_empty() {
            "Scanning for nearby Smart Clipboard devices via mDNS…".to_string()
        } else {
            "Waiting for paired devices to become reachable on the local network.".to_string()
        };
    }

    fn find_paired_device(&self, device_id: &str) -> Result<Option<PairedDevice>, String> {
        self.db
            .find_paired_device(device_id)
            .map_err(|e| e.to_string())
    }

    pub fn local_public_key_base64(&self) -> String {
        crypto::encode_key_material(&self.local_device.public_key)
    }

    pub fn ensure_pairing_secret(
        &self,
        remote_device_id: &str,
        remote_device_name: Option<String>,
        remote_host: Option<String>,
        remote_port: Option<u16>,
        remote_public_key_b64: &str,
    ) -> Result<PairedDevice, String> {
        let remote_public_key = crypto::decode_key_material(remote_public_key_b64)?;
        let shared_secret =
            crypto::derive_shared_secret(&self.local_private_key, &remote_public_key)?;
        let mut device = self
            .find_paired_device(remote_device_id)?
            .unwrap_or(PairedDevice {
                id: remote_device_id.to_string(),
                name: remote_device_name
                    .clone()
                    .unwrap_or_else(|| remote_device_id.to_string()),
                device_name: remote_device_name
                    .clone()
                    .unwrap_or_else(|| remote_device_id.to_string()),
                host: remote_host
                    .clone()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                address: remote_host
                    .clone()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                ip: remote_host
                    .clone()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                port: i64::from(remote_port.unwrap_or(23456)),
                status: "connecting".to_string(),
                public_key: None,
                local_public_key: None,
                shared_secret: None,
                last_seen_at: Some(Local::now().naive_local()),
                is_active: true,
                enabled: true,
                sync_enabled: true,
                paired_at: Local::now().naive_local(),
                fingerprint: None,
            });
        if let Some(name) = remote_device_name {
            device.name = name.clone();
            device.device_name = name;
        }
        if let Some(host) = remote_host.clone() {
            device.host = host.clone();
            device.address = host.clone();
            device.ip = host;
        }
        if let Some(port) = remote_port {
            device.port = i64::from(port);
        }
        device.public_key = Some(remote_public_key);
        device.local_public_key = Some(self.local_device.public_key.clone());
        device.shared_secret = Some(shared_secret);
        device.fingerprint = Some(crypto::compute_fingerprint(
            device.shared_secret.as_ref().unwrap(),
        ));
        device.last_seen_at = Some(Local::now().naive_local());
        self.db
            .upsert_paired_device(&device)
            .map_err(|e| e.to_string())?;
        Ok(device)
    }

    pub fn encrypt_protocol_message(
        &self,
        device_id: &str,
        message: &protocol::SyncMessage,
    ) -> Result<protocol::SyncMessage, String> {
        let device = self
            .find_paired_device(device_id)?
            .ok_or_else(|| format!("Paired device not found: {device_id}"))?;
        let secret = device
            .shared_secret
            .ok_or_else(|| format!("Shared secret is not established for device: {device_id}"))?;
        let plaintext = message.to_text()?;
        let encrypted = crypto::encrypt(plaintext.as_bytes(), &secret)?;
        Ok(protocol::SyncMessage::EncryptedPayload {
            message: encrypted.into(),
        })
    }

    pub fn decrypt_protocol_message(
        &self,
        device_id: &str,
        message: protocol::SyncMessage,
    ) -> Result<protocol::SyncMessage, String> {
        match message {
            protocol::SyncMessage::EncryptedPayload { message } => {
                let device = self
                    .find_paired_device(device_id)?
                    .ok_or_else(|| format!("Paired device not found: {device_id}"))?;
                let secret = device.shared_secret.ok_or_else(|| {
                    format!("Shared secret is not established for device: {device_id}")
                })?;
                let plaintext = crypto::decrypt(&message.into(), &secret)?;
                let text = String::from_utf8(plaintext)
                    .map_err(|e| format!("Decrypted payload was not valid UTF-8: {e}"))?;
                protocol::SyncMessage::from_text(&text)
            }
            other => Ok(other),
        }
    }

    pub fn build_encrypted_placeholder(
        &self,
        device_id: &str,
    ) -> Result<protocol::SyncMessage, String> {
        let payload = protocol::SyncMessage::ClipboardSyncPlaceholder {
            entry_hash: format!("phase3-placeholder-{}", now_string()),
            timestamp: Local::now().timestamp_millis(),
        };
        self.encrypt_protocol_message(device_id, &payload)
    }

    fn load_or_create_local_keypair(config: &ConfigManager) -> crypto::DeviceKeyPair {
        let mut app_config = config.get();
        let mut metadata = app_config
            .sync_metadata
            .clone()
            .unwrap_or_else(|| json!({}));

        let private_existing = metadata.get("sync_private_key").and_then(|v| v.as_str());
        let public_existing = metadata.get("sync_public_key").and_then(|v| v.as_str());
        if let (Some(private_key), Some(public_key)) = (private_existing, public_existing) {
            if let (Ok(private_key), Ok(public_key)) = (
                crypto::decode_key_material(private_key),
                crypto::decode_key_material(public_key),
            ) {
                return crypto::DeviceKeyPair {
                    private_key,
                    public_key,
                };
            }
        }

        let keypair = crypto::generate_keypair();
        metadata["sync_private_key"] = json!(crypto::encode_key_material(&keypair.private_key));
        metadata["sync_public_key"] = json!(crypto::encode_key_material(&keypair.public_key));
        app_config.sync_metadata = Some(metadata);
        let _ = config.update(app_config);
        keypair
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

pub fn heartbeat_interval() -> Duration {
    Duration::from_secs(HEARTBEAT_INTERVAL_SECS)
}

pub fn reconnect_backoff(attempt: usize) -> Duration {
    Duration::from_secs(RECONNECT_BACKOFF_SECS[attempt.min(RECONNECT_BACKOFF_SECS.len() - 1)])
}

pub fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
