use crate::encryption::EncryptionConfig;
use crate::security::AppLockConfig;
use crate::sync::webdav::WebDavConfig;
use crate::sync::SyncConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterConfig {
    pub auto_check_enabled: bool,
    pub check_interval_hours: u64,
    pub auto_download_enabled: bool,
    pub wifi_only: bool,
    pub mirrors: Vec<String>,
    pub last_check_at: Option<String>,
}

impl Default for UpdaterConfig {
    fn default() -> Self {
        Self {
            auto_check_enabled: true,
            check_interval_hours: 24,
            auto_download_enabled: false,
            wifi_only: true,
            mirrors: vec![],
            last_check_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub max_entries: i64,
    pub retention_days: i64,
    pub excluded_apps: Vec<String>,
    pub monitor_interval_ms: u64,
    pub autostart_enabled: bool,
    pub sensitive_expiry_minutes: u64,
    #[serde(default)]
    pub sync: SyncConfig,
    pub sync_metadata: Option<Value>,
    #[serde(default)]
    pub webdav: WebDavConfig,
    #[serde(default)]
    pub app_lock: AppLockConfig,
    #[serde(default)]
    pub encryption: EncryptionConfig,
    #[serde(default)]
    pub updater: UpdaterConfig,
    #[serde(default)]
    pub plugin_enabled: HashMap<String, bool>,
    #[serde(default = "default_quick_paste_shortcut")]
    pub quick_paste_shortcut: String,
    #[serde(default = "default_quick_paste_entry_count")]
    pub quick_paste_entry_count: u8,
}

fn default_quick_paste_shortcut() -> String {
    "CommandOrControl+Shift+C".to_string()
}

fn default_quick_paste_entry_count() -> u8 {
    9
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_entries: 5000,
            retention_days: 30,
            excluded_apps: vec![],
            monitor_interval_ms: 500,
            autostart_enabled: false,
            sensitive_expiry_minutes: 5,
            sync: SyncConfig::default(),
            sync_metadata: None,
            webdav: WebDavConfig::default(),
            app_lock: AppLockConfig::default(),
            encryption: EncryptionConfig::default(),
            updater: UpdaterConfig::default(),
            plugin_enabled: HashMap::new(),
            quick_paste_shortcut: default_quick_paste_shortcut(),
            quick_paste_entry_count: default_quick_paste_entry_count(),
        }
    }
}

pub struct ConfigManager {
    config: Mutex<AppConfig>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("config.json");
        let config = Self::load_from_file(&config_path);
        Self {
            config: Mutex::new(config),
            config_path,
        }
    }

    fn load_from_file(path: &PathBuf) -> AppConfig {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            }
        } else {
            AppConfig::default()
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update(&self, new_config: AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
        std::fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        *self.config.lock().unwrap() = new_config;
        Ok(())
    }
}
