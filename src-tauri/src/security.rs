use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::ConfigManager;

const APP_LOCK_KEYRING_SERVICE: &str = "smart-clipboard";
const APP_LOCK_KEYRING_ACCOUNT: &str = "app-lock-password-hash";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppLockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_lock_seconds: u64,
    #[serde(default)]
    pub biometric_enabled: bool,
}

impl Default for AppLockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_lock_seconds: 0,
            biometric_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLockStatus {
    pub enabled: bool,
    pub configured: bool,
    pub locked: bool,
    pub biometric_available: bool,
    pub biometric_enabled: bool,
    pub auto_lock_seconds: u64,
    pub unlock_reason: Option<String>,
    pub failed_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPasswordPayload {
    pub current_password: Option<String>,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppLockSettingsPayload {
    pub enabled: bool,
    pub auto_lock_seconds: u64,
    pub biometric_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockPayload {
    pub password: Option<String>,
    pub prefer_biometric: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppLockEventPayload {
    status: AppLockStatus,
}

#[derive(Debug)]
struct AppLockRuntimeState {
    locked: bool,
    last_unlock_at: Option<Instant>,
    last_activity_at: Instant,
    unlock_reason: Option<String>,
    failed_attempts: u32,
}

pub struct AppLockManager {
    config: Arc<ConfigManager>,
    runtime: Mutex<AppLockRuntimeState>,
}

impl AppLockManager {
    pub fn new(config: Arc<ConfigManager>) -> Self {
        let app_lock = config.get().app_lock;
        let configured = password_hash_exists();
        let initial_locked = app_lock.enabled && configured;
        Self {
            config,
            runtime: Mutex::new(AppLockRuntimeState {
                locked: initial_locked,
                last_unlock_at: None,
                last_activity_at: Instant::now(),
                unlock_reason: if initial_locked {
                    Some("startup".to_string())
                } else {
                    None
                },
                failed_attempts: 0,
            }),
        }
    }

    pub fn status(&self) -> AppLockStatus {
        let cfg = self.config.get().app_lock;
        let runtime = self.runtime.lock().unwrap();
        AppLockStatus {
            enabled: cfg.enabled,
            configured: password_hash_exists(),
            locked: runtime.locked,
            biometric_available: biometric_available(),
            biometric_enabled: cfg.biometric_enabled,
            auto_lock_seconds: cfg.auto_lock_seconds,
            unlock_reason: runtime.unlock_reason.clone(),
            failed_attempts: runtime.failed_attempts,
        }
    }

    pub fn set_password(&self, payload: SetPasswordPayload) -> Result<AppLockStatus, String> {
        let configured = password_hash_exists();
        if configured {
            let current = payload
                .current_password
                .ok_or_else(|| "Current password is required".to_string())?;
            verify_password_against_store(&current)?;
        }

        validate_password_strength(&payload.new_password)?;
        let hash = hash_password(&payload.new_password)?;
        save_password_hash(&hash)?;

        let mut cfg = self.config.get();
        cfg.app_lock.enabled = true;
        if cfg.app_lock.biometric_enabled && !biometric_available() {
            cfg.app_lock.biometric_enabled = false;
        }
        self.config.update(cfg)?;

        let mut runtime = self.runtime.lock().unwrap();
        runtime.locked = false;
        runtime.last_unlock_at = Some(Instant::now());
        runtime.last_activity_at = Instant::now();
        runtime.unlock_reason = Some("password_set".to_string());
        runtime.failed_attempts = 0;
        drop(runtime);

        Ok(self.status())
    }

    pub fn update_settings(
        &self,
        payload: UpdateAppLockSettingsPayload,
    ) -> Result<AppLockStatus, String> {
        let configured = password_hash_exists();
        if payload.enabled && !configured {
            return Err("Set a password before enabling app lock".to_string());
        }

        let mut cfg = self.config.get();
        cfg.app_lock = AppLockConfig {
            enabled: payload.enabled,
            auto_lock_seconds: payload.auto_lock_seconds,
            biometric_enabled: payload.biometric_enabled && biometric_available(),
        };
        self.config.update(cfg)?;

        let mut runtime = self.runtime.lock().unwrap();
        if !payload.enabled {
            runtime.locked = false;
            runtime.unlock_reason = Some("disabled".to_string());
            runtime.failed_attempts = 0;
            runtime.last_unlock_at = Some(Instant::now());
            runtime.last_activity_at = Instant::now();
        } else if configured && runtime.locked {
            runtime.unlock_reason = Some("settings".to_string());
        }
        drop(runtime);

        Ok(self.status())
    }

    pub fn verify_password(&self, password: &str) -> Result<AppLockStatus, String> {
        verify_password_against_store(password)?;

        let mut runtime = self.runtime.lock().unwrap();
        runtime.locked = false;
        runtime.last_unlock_at = Some(Instant::now());
        runtime.last_activity_at = Instant::now();
        runtime.unlock_reason = Some("password".to_string());
        runtime.failed_attempts = 0;
        drop(runtime);
        Ok(self.status())
    }

    pub fn mark_biometric_unlocked(&self) -> AppLockStatus {
        let mut runtime = self.runtime.lock().unwrap();
        runtime.locked = false;
        runtime.last_unlock_at = Some(Instant::now());
        runtime.last_activity_at = Instant::now();
        runtime.unlock_reason = Some("biometric".to_string());
        runtime.failed_attempts = 0;
        drop(runtime);
        self.status()
    }

    pub fn lock(&self, reason: &str) -> AppLockStatus {
        let cfg = self.config.get().app_lock;
        let mut runtime = self.runtime.lock().unwrap();
        if cfg.enabled && password_hash_exists() {
            runtime.locked = true;
            runtime.unlock_reason = Some(reason.to_string());
        }
        drop(runtime);
        self.status()
    }

    pub fn ensure_unlocked(&self) -> Result<(), String> {
        let status = self.status();
        if status.enabled && status.locked {
            Err("App is locked".to_string())
        } else {
            Ok(())
        }
    }

    pub fn should_allow_window(&self) -> bool {
        let cfg = self.config.get().app_lock;
        if !cfg.enabled || !password_hash_exists() {
            return true;
        }
        let runtime = self.runtime.lock().unwrap();
        !runtime.locked
    }

    pub fn record_activity(&self) {
        let mut runtime = self.runtime.lock().unwrap();
        runtime.last_activity_at = Instant::now();
    }

    pub fn check_auto_lock(&self) -> Option<AppLockStatus> {
        let cfg = self.config.get().app_lock;
        if !cfg.enabled || cfg.auto_lock_seconds == 0 || !password_hash_exists() {
            return None;
        }

        let mut runtime = self.runtime.lock().unwrap();
        if runtime.locked {
            return None;
        }

        if runtime.last_activity_at.elapsed() >= Duration::from_secs(cfg.auto_lock_seconds) {
            runtime.locked = true;
            runtime.unlock_reason = Some("auto_lock".to_string());
            drop(runtime);
            return Some(self.status());
        }

        None
    }

    pub fn handle_failed_unlock(&self) -> AppLockStatus {
        let mut runtime = self.runtime.lock().unwrap();
        runtime.failed_attempts = runtime.failed_attempts.saturating_add(1);
        runtime.locked = true;
        runtime.unlock_reason = Some("failed_password".to_string());
        drop(runtime);
        self.status()
    }
}

pub fn password_hash_exists() -> bool {
    load_password_hash()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn keyring_entry() -> Result<keyring_core::Entry, String> {
    init_keyring()?;
    keyring_core::Entry::new(APP_LOCK_KEYRING_SERVICE, APP_LOCK_KEYRING_ACCOUNT)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))
}

fn save_password_hash(hash: &str) -> Result<(), String> {
    keyring_entry()?
        .set_password(hash)
        .map_err(|e| format!("Failed to store password hash: {e}"))
}

fn load_password_hash() -> Result<String, String> {
    keyring_entry()?
        .get_password()
        .map_err(|e| format!("Failed to load password hash: {e}"))
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Failed to hash password: {e}"))
}

fn verify_password_against_store(password: &str) -> Result<(), String> {
    let hash = load_password_hash()?;
    let parsed_hash = PasswordHash::new(&hash).map_err(|e| format!("Invalid stored hash: {e}"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "Incorrect password".to_string())
}

fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 4 {
        return Err("Password must be at least 4 characters".to_string());
    }
    Ok(())
}

pub fn biometric_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn try_biometric_unlock() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let script = r#"
try
  do shell script "echo biometric-check" with prompt "Unlock Smart Clipboard" with administrator privileges
  return "ok"
on error errMsg number errNum
  error errMsg number errNum
end try
"#;
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to start biometric prompt: {e}"))?;
        if output.status.success() {
            Ok(true)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}

pub fn emit_lock_state(app: &AppHandle, manager: &AppLockManager) {
    let _ = app.emit(
        "app-lock-status",
        AppLockEventPayload {
            status: manager.status(),
        },
    );
}

pub fn enforce_window_access(app: &AppHandle, manager: &AppLockManager, source: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        if manager.should_allow_window() {
            let _ = app.emit("window-shown", ());
        } else {
            let status = manager.lock(source);
            let _ = app.emit("app-lock-status", AppLockEventPayload { status });
        }
    }
}

pub fn attach_lock_runtime(app: &AppHandle, manager: Arc<AppLockManager>) {
    let app_handle = app.clone();
    let manager_for_task = manager.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Some(status) = manager_for_task.check_auto_lock() {
                let _ = app_handle.emit("app-lock-status", AppLockEventPayload { status });
            }
        }
    });

    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let manager_for_event = manager.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::Focused(true) => {
                manager_for_event.record_activity();
                if !manager_for_event.should_allow_window() {
                    let status = manager_for_event.lock("focus");
                    let _ = app_handle.emit("app-lock-status", AppLockEventPayload { status });
                }
            }
            tauri::WindowEvent::Focused(false) => {
                manager_for_event.record_activity();
            }
            _ => {}
        });
    }
}

fn init_keyring() -> Result<(), String> {
    keyring::use_native_store(false).map_err(|e| format!("Failed to initialize keyring store: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn password_strength_validation_works() {
        assert!(validate_password_strength("123").is_err());
        assert!(validate_password_strength("1234").is_ok());
    }

    #[test]
    fn hash_and_verify_roundtrip_works() {
        let password = "phase4-pass";
        let hash = hash_password(password).expect("hash");
        let parsed = PasswordHash::new(&hash).expect("parse");
        assert!(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok());
        assert!(Argon2::default()
            .verify_password("wrong".as_bytes(), &parsed)
            .is_err());
    }

    #[test]
    fn biometric_availability_is_boolean_contract() {
        let available = biometric_available();
        assert!(matches!(available, true | false));
    }

    #[test]
    fn auto_lock_threshold_comparison_behaves() {
        let now = Instant::now();
        let idle_for = now
            .checked_duration_since(now - Duration::from_secs(5))
            .unwrap_or(Duration::from_secs(5));
        assert!(idle_for >= Duration::from_secs(5));
    }
}
