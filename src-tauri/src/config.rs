use crate::sync::SyncConfig;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

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
                Ok(contents) => {
                    serde_json::from_str(&contents).unwrap_or_default()
                }
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
        let json =
            serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
        std::fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        *self.config.lock().unwrap() = new_config;
        Ok(())
    }
}
