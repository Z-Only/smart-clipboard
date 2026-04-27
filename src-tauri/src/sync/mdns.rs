use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use log::{debug, error, warn};
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{json, Value};

use crate::config::{AppConfig, ConfigManager};
use crate::storage::DiscoveredDevice;
use crate::sync::sync_device_status;

const DEVICE_ID_KEY: &str = "device_id";
const VERSION_KEY: &str = "version";
const VERSION_VALUE: &str = "1";
const HOST_LABEL: &str = "smart-clipboard.local.";
const DISCOVERY_CLEANUP_SECS: i64 = 60;

#[derive(Debug, Clone)]
pub struct MdnsConfig {
    pub service_type: String,
    pub device_name: String,
    pub device_id: String,
    pub port: u16,
    pub enabled: bool,
}

pub struct MdnsDiscoveryService {
    daemon: Option<ServiceDaemon>,
    config: Mutex<MdnsConfig>,
    discovered: Arc<Mutex<HashMap<String, DiscoveredDevice>>>,
    registered_fullname: Arc<Mutex<Option<String>>>,
}

impl MdnsDiscoveryService {
    pub fn start(config: MdnsConfig) -> Arc<Self> {
        let discovered = Arc::new(Mutex::new(HashMap::new()));
        let registered_fullname = Arc::new(Mutex::new(None));
        let daemon = match ServiceDaemon::new() {
            Ok(daemon) => Some(daemon),
            Err(err) => {
                error!("Failed to start mDNS daemon: {err}");
                None
            }
        };

        let service = Arc::new(Self {
            daemon,
            config: Mutex::new(config),
            discovered,
            registered_fullname,
        });

        service.spawn_browser();
        service.apply_registration();
        service
    }

    pub fn update_config(&self, new_config: MdnsConfig) {
        *self.config.lock().unwrap() = new_config;
        self.apply_registration();
    }

    pub fn current_devices(&self) -> Vec<DiscoveredDevice> {
        let mut devices = self.discovered.lock().unwrap();
        prune_stale_devices(&mut devices);
        for device in devices.values_mut() {
            device.status = sync_device_status(Some(device.last_seen_at)).to_string();
        }
        devices.values().cloned().collect()
    }

    fn spawn_browser(self: &Arc<Self>) {
        let Some(daemon) = &self.daemon else {
            return;
        };

        let service_type = self.config.lock().unwrap().service_type.clone();
        let Ok(receiver) = daemon.browse(&service_type) else {
            error!("Failed to browse mDNS service type: {service_type}");
            return;
        };

        let discovered = Arc::clone(&self.discovered);
        let self_device_id = self.config.lock().unwrap().device_id.clone();
        let service_type_for_remove = service_type.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match receiver.recv_async().await {
                    Ok(event) => match event {
                        ServiceEvent::ServiceResolved(info) => {
                            if let Some(device) = resolved_service_to_device(&info, &self_device_id)
                            {
                                debug!(
                                    "Resolved mDNS device {} @ {}:{}",
                                    device.id, device.host, device.port
                                );
                                discovered.lock().unwrap().insert(device.id.clone(), device);
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            let removed_id = {
                                let mut map = discovered.lock().unwrap();
                                let to_remove = map
                                    .iter()
                                    .find(|(_, device)| {
                                        service_fullname(device, &service_type_for_remove)
                                            == fullname
                                    })
                                    .map(|(id, _)| id.clone());
                                if let Some(id) = &to_remove {
                                    map.remove(id);
                                }
                                to_remove
                            };
                            if let Some(id) = removed_id {
                                debug!("Removed mDNS device {id} due to service removal event");
                            }
                        }
                        ServiceEvent::SearchStopped(_) => break,
                        _ => {}
                    },
                    Err(err) => {
                        warn!("mDNS browser loop stopped: {err}");
                        break;
                    }
                }
            }
        });
    }

    fn apply_registration(&self) {
        let Some(daemon) = &self.daemon else {
            return;
        };

        if let Some(previous) = self.registered_fullname.lock().unwrap().take() {
            let _ = daemon.unregister(&previous);
        }

        let config = self.config.lock().unwrap().clone();
        if !config.enabled {
            return;
        }

        let instance_name = sanitize_instance_name(&config.device_name);
        let mut txt_records = HashMap::new();
        txt_records.insert(DEVICE_ID_KEY.to_string(), config.device_id.clone());
        txt_records.insert(VERSION_KEY.to_string(), VERSION_VALUE.to_string());

        let service = match ServiceInfo::new(
            &config.service_type,
            &instance_name,
            HOST_LABEL,
            "",
            config.port,
            txt_records,
        ) {
            Ok(service) => service.enable_addr_auto(),
            Err(err) => {
                error!("Failed to create mDNS service info: {err}");
                return;
            }
        };

        let fullname = service.get_fullname().to_string();
        if let Err(err) = daemon.register(service) {
            error!("Failed to register mDNS service: {err}");
            return;
        }
        *self.registered_fullname.lock().unwrap() = Some(fullname);
    }
}

impl Drop for MdnsDiscoveryService {
    fn drop(&mut self) {
        if let Some(daemon) = &self.daemon {
            if let Some(previous) = self.registered_fullname.lock().unwrap().take() {
                let _ = daemon.unregister(&previous);
            }
            let _ = daemon.shutdown();
        }
    }
}

fn resolved_service_to_device(
    info: &ResolvedService,
    self_device_id: &str,
) -> Option<DiscoveredDevice> {
    let properties = info.get_properties();
    let device_id = properties
        .get_property_val_str(DEVICE_ID_KEY)
        .map(str::to_string)
        .unwrap_or_else(|| info.get_fullname().to_string());

    if device_id == self_device_id {
        return None;
    }

    let address = info
        .get_addresses()
        .iter()
        .next()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());

    let version = properties
        .get_property_val_str(VERSION_KEY)
        .unwrap_or(VERSION_VALUE)
        .to_string();

    let name = info
        .get_fullname()
        .split('.')
        .next()
        .unwrap_or("Smart Clipboard Device")
        .to_string();
    let last_seen_at = Local::now().naive_local();

    Some(DiscoveredDevice {
        id: device_id,
        name: name.clone(),
        device_name: name,
        host: address.clone(),
        address: address.clone(),
        ip: address,
        port: i64::from(info.get_port()),
        version,
        status: sync_device_status(Some(last_seen_at)).to_string(),
        last_seen_at,
        is_paired: false,
        enabled: true,
        sync_enabled: true,
        paired_at: None,
        fingerprint: None,
    })
}

fn prune_stale_devices(devices: &mut HashMap<String, DiscoveredDevice>) {
    let now = Local::now().naive_local();
    devices.retain(|_, device| {
        now.signed_duration_since(device.last_seen_at).num_seconds() <= DISCOVERY_CLEANUP_SECS
    });
}

fn sanitize_instance_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Smart Clipboard Device".to_string()
    } else {
        trimmed.replace('.', "-")
    }
}

fn service_fullname(device: &DiscoveredDevice, service_type: &str) -> String {
    format!("{}.{}", sanitize_instance_name(&device.name), service_type)
}

pub fn load_or_create_device_id(config: &ConfigManager) -> String {
    let mut app_config: AppConfig = config.get();
    if let Some(existing) = read_sync_metadata(&app_config)
        .get("deviceId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
    {
        return existing;
    }

    let generated = format!("smartclip-{}", uuid_like_seed());
    let mut metadata = read_sync_metadata(&app_config);
    metadata["deviceId"] = Value::String(generated.clone());
    app_config.sync_metadata = Some(metadata);
    if let Err(err) = config.update(app_config) {
        warn!("Failed to persist sync device id: {err}");
    }
    generated
}

fn read_sync_metadata(config: &AppConfig) -> Value {
    config.sync_metadata.clone().unwrap_or_else(|| json!({}))
}

fn uuid_like_seed() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    format!("{:032x}", nanos)
}
