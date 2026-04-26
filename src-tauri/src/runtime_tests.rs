use crate::config::ConfigManager;
use crate::security::{
    self, emit_auto_lock_if_needed, AppLockManager, AppLockStatus, SetPasswordPayload,
    UpdateAppLockSettingsPayload,
};
use crate::{emit_initial_lock_state, handle_main_window_close, handle_main_window_focus};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::test::{self, MockRuntime};
use tauri::{App, Listener, Manager, WebviewWindow, WebviewWindowBuilder};

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
            "smart-clipboard-lib-runtime-tests-{}-{}",
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
        security::install_test_keyring_store();
        security::set_test_biometric_result(None);
        Self
    }
}

impl Drop for TestKeyringGuard {
    fn drop(&mut self) {
        security::set_test_biometric_result(None);
        security::reset_test_keyring_store();
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
        Self::with_lock(lock)
    }

    fn new_startup_locked(base_dir: PathBuf) -> Self {
        let config = Arc::new(ConfigManager::new(base_dir));
        let bootstrap = AppLockManager::new(config.clone());
        bootstrap
            .set_password(SetPasswordPayload {
                current_password: None,
                new_password: "phase4-pass".to_string(),
            })
            .expect("setting password should succeed");
        bootstrap
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: false,
            })
            .expect("enabling app lock should succeed");

        let lock = Arc::new(AppLockManager::new(config));
        Self::with_lock(lock)
    }

    fn with_lock(lock: Arc<AppLockManager>) -> Self {
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
fn startup_locked_app_emits_current_lock_status() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = TestHarness::new_startup_locked(temp_dir.path.clone());

    emit_initial_lock_state(&harness.app_handle(), &harness.lock);

    let status = recv_lock_status(&harness.app_lock_status_rx);
    assert!(status.enabled);
    assert!(status.locked);
    assert_eq!(status.unlock_reason.as_deref(), Some("startup"));
}

#[test]
fn window_focus_emits_current_lock_status_when_unlocked() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = TestHarness::new(temp_dir.path.clone());
    harness.configure_password();

    handle_main_window_focus(&harness.app_handle(), &harness.lock, true);

    let status = recv_lock_status(&harness.app_lock_status_rx);
    assert!(status.enabled);
    assert!(!status.locked);
    assert_eq!(status.unlock_reason.as_deref(), Some("password_set"));
}

#[test]
fn window_close_refreshes_activity_before_auto_lock_check() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = TestHarness::new(temp_dir.path.clone());
    harness.configure_password();
    harness.configure_auto_lock(true, 1);
    harness
        .lock
        .rewind_last_activity_for_test(Duration::from_secs(2));

    handle_main_window_close(&harness.window, &harness.lock);
    emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

    assert_no_lock_status(&harness.app_lock_status_rx);
    assert!(!harness.lock.status().locked);
}
