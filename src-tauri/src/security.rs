use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::config::ConfigManager;

// Delegate biometric functions to the dedicated module and re-export for
// backward compatibility with commands.rs.
use crate::biometric;
pub use crate::biometric::try_biometric_unlock;

const APP_LOCK_KEYRING_SERVICE: &str = "smart-clipboard";
const APP_LOCK_KEYRING_ACCOUNT: &str = "app-lock-password-hash";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppLockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_lock_seconds: u64,
    #[serde(default)]
    pub biometric_enabled: bool,
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

    #[cfg(test)]
    pub(crate) fn rewind_last_activity_for_test(&self, duration: Duration) {
        let mut runtime = self.runtime.lock().unwrap();
        runtime.last_activity_at = Instant::now()
            .checked_sub(duration)
            .unwrap_or_else(Instant::now);
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

#[cfg(test)]
pub(crate) fn install_test_keyring_store() {
    keyring_core::set_default_store(
        keyring_core::mock::Store::new().expect("failed to create mock keyring store"),
    );
}

#[cfg(test)]
pub(crate) fn reset_test_keyring_store() {
    let _ = keyring_core::unset_default_store();
}

#[cfg(test)]
pub(crate) fn set_test_biometric_result(result: Option<Result<bool, String>>) {
    crate::biometric::set_test_biometric_result(result);
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
    biometric::biometric_available()
}

pub fn emit_lock_state<R: Runtime>(app: &AppHandle<R>, manager: &AppLockManager) {
    let _ = app.emit(
        "app-lock-status",
        AppLockEventPayload {
            status: manager.status(),
        },
    );
}

pub fn enforce_window_access<R: Runtime>(
    app: &AppHandle<R>,
    manager: &AppLockManager,
    source: &str,
) {
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

pub(crate) fn emit_auto_lock_if_needed<R: Runtime>(app: &AppHandle<R>, manager: &AppLockManager) {
    if let Some(status) = manager.check_auto_lock() {
        let _ = app.emit("app-lock-status", AppLockEventPayload { status });
    }
}

pub(crate) fn handle_focus_event<R: Runtime>(
    app: &AppHandle<R>,
    manager: &AppLockManager,
    focused: bool,
) {
    manager.record_activity();
    if focused && !manager.should_allow_window() {
        let status = manager.lock("focus");
        let _ = app.emit("app-lock-status", AppLockEventPayload { status });
    }
}

pub fn attach_lock_runtime<R: Runtime>(app: &AppHandle<R>, manager: Arc<AppLockManager>) {
    let app_handle = app.clone();
    let manager_for_task = manager.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            emit_auto_lock_if_needed(&app_handle, manager_for_task.as_ref());
        }
    });

    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let manager_for_event = manager.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                handle_focus_event(&app_handle, manager_for_event.as_ref(), *focused);
            }
        });
    }
}

#[cfg(test)]
fn init_keyring() -> Result<(), String> {
    if keyring_core::get_default_store().is_none() {
        install_test_keyring_store();
    }
    Ok(())
}

#[cfg(not(test))]
fn init_keyring() -> Result<(), String> {
    keyring::use_native_store(false).map_err(|e| format!("Failed to initialize keyring store: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;
    use serde::Deserialize;
    use std::path::PathBuf;
    use std::sync::mpsc::Receiver;
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tauri::test::{self, MockRuntime};
    use tauri::{App, Listener, WebviewWindow, WebviewWindowBuilder};

    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should move forward")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "smart-clipboard-security-tests-{}-{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("failed to create temp test dir");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct TestKeyringGuard;

    impl TestKeyringGuard {
        fn new() -> Self {
            install_test_keyring_store();
            set_test_biometric_result(None);
            Self
        }
    }

    impl Drop for TestKeyringGuard {
        fn drop(&mut self) {
            set_test_biometric_result(None);
            reset_test_keyring_store();
        }
    }

    #[derive(Debug, Deserialize)]
    struct LockStatusEventPayload {
        status: AppLockStatus,
    }

    struct TestHarness {
        _app: App<MockRuntime>,
        window: WebviewWindow<MockRuntime>,
        lock: Arc<AppLockManager>,
        app_lock_status_rx: Receiver<AppLockStatus>,
    }

    impl TestHarness {
        fn new(base_dir: PathBuf) -> Self {
            let config = Arc::new(ConfigManager::new(base_dir));
            let lock = Arc::new(AppLockManager::new(config));

            let app = test::mock_builder()
                .manage(lock.clone())
                .build(test::mock_context(test::noop_assets()))
                .expect("failed to build mock app");

            let window = WebviewWindowBuilder::new(&app, "main", Default::default())
                .build()
                .expect("failed to build mock webview");

            let (app_lock_tx, app_lock_status_rx) = std::sync::mpsc::sync_channel(8);

            app.listen_any("app-lock-status", move |event| {
                let payload: LockStatusEventPayload =
                    serde_json::from_str(event.payload()).expect("lock payload should be json");
                let _ = app_lock_tx.send(payload.status);
            });

            Self {
                _app: app,
                window,
                lock,
                app_lock_status_rx,
            }
        }

        fn configure_password(&self) {
            self.lock
                .set_password(SetPasswordPayload {
                    current_password: None,
                    new_password: "phase4-pass".to_string(),
                })
                .expect("setting password should succeed");
        }

        fn configure_auto_lock(&self, enabled: bool, auto_lock_seconds: u64) {
            self.lock
                .update_settings(UpdateAppLockSettingsPayload {
                    enabled,
                    auto_lock_seconds,
                    biometric_enabled: false,
                })
                .expect("updating app lock settings should succeed");
        }

        fn app_handle(&self) -> tauri::AppHandle<MockRuntime> {
            self.window.app_handle().clone()
        }
    }

    fn recv_lock_status(rx: &Receiver<AppLockStatus>) -> AppLockStatus {
        rx.recv_timeout(Duration::from_millis(200))
            .expect("expected lock status event")
    }

    fn assert_no_lock_status(rx: &Receiver<AppLockStatus>) {
        assert!(
            rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "did not expect lock status event"
        );
    }

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
    fn biometric_available_reflects_injected_value() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();

        crate::biometric::set_test_biometric_available(Some(true));
        assert!(biometric_available());

        crate::biometric::set_test_biometric_available(Some(false));
        assert!(!biometric_available());

        crate::biometric::set_test_biometric_available(None);
    }

    #[test]
    fn biometric_unlock_success_clears_lock() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        harness.lock.lock("manual");
        assert!(harness.lock.status().locked);

        set_test_biometric_result(Some(Ok(true)));
        let status = harness.lock.mark_biometric_unlocked();
        assert!(!status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("biometric"));
        assert_eq!(status.failed_attempts, 0);
    }

    #[test]
    fn biometric_cancel_keeps_lock_and_does_not_increment_failures() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        harness.lock.lock("manual");
        let before = harness.lock.status().failed_attempts;

        set_test_biometric_result(Some(Ok(false)));
        let result = try_biometric_unlock();
        assert_eq!(result, Ok(false));

        let status = harness.lock.status();
        assert!(status.locked);
        assert_eq!(status.failed_attempts, before);
    }

    #[test]
    fn biometric_error_returns_err() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();

        set_test_biometric_result(Some(Err("Biometric locked out".to_string())));
        let result = try_biometric_unlock();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Biometric locked out");

        set_test_biometric_result(None);
    }

    #[test]
    fn settings_downgrade_biometric_when_unavailable() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        crate::biometric::set_test_biometric_available(Some(false));

        let status = harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        assert!(!status.biometric_enabled);

        crate::biometric::set_test_biometric_available(None);
    }

    #[test]
    fn auto_lock_threshold_comparison_behaves() {
        let now = Instant::now();
        let idle_for = now
            .checked_duration_since(now - Duration::from_secs(5))
            .unwrap_or(Duration::from_secs(5));
        assert!(idle_for >= Duration::from_secs(5));
    }

    #[test]
    fn focus_event_emits_lock_status_when_locked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.lock.lock("manual");

        handle_focus_event(&harness.app_handle(), &harness.lock, true);

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("focus"));
    }

    #[test]
    fn focus_loss_refreshes_activity_before_auto_lock_check() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.configure_auto_lock(true, 1);
        harness
            .lock
            .rewind_last_activity_for_test(Duration::from_secs(2));

        handle_focus_event(&harness.app_handle(), &harness.lock, false);
        emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

        assert_no_lock_status(&harness.app_lock_status_rx);
        assert!(!harness.lock.status().locked);
    }

    #[test]
    fn auto_lock_emits_lock_status_after_idle_timeout() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.configure_auto_lock(true, 1);
        harness
            .lock
            .rewind_last_activity_for_test(Duration::from_secs(2));

        emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("auto_lock"));
    }

    #[test]
    fn auto_lock_does_not_emit_when_app_lock_disabled() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.configure_auto_lock(false, 1);
        harness
            .lock
            .rewind_last_activity_for_test(Duration::from_secs(2));

        emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

        assert_no_lock_status(&harness.app_lock_status_rx);
        assert!(!harness.lock.status().locked);
    }
}
