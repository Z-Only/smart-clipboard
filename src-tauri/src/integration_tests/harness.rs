//! Shared test infrastructure for invoke-level integration tests.
//!
//! All `#[test]` functions in this module tree MUST acquire `TEST_SERIAL`
//! first — the keyring mock store and the biometric injection slots are
//! process-global statics shared across every test in the binary.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Local;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{self, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{App, Listener, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::config::ConfigManager;
use crate::security::{
    self, AppLockManager, AppLockStatus, SetPasswordPayload, UpdateAppLockSettingsPayload,
};
use crate::storage::{ClipboardEntry, Database};
use crate::sync::webdav::{WebDavConfig, WebDavSyncManager};
use crate::sync::SyncManager;
use crate::updater::UpdaterManager;

pub(crate) static TEST_SERIAL: Mutex<()> = Mutex::new(());

pub(crate) fn lock_serial() -> MutexGuard<'static, ()> {
    TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) const TEST_PASSWORD: &str = "phase4-pass";

// ---------------------------------------------------------------------------
// Temp dir RAII
// ---------------------------------------------------------------------------

pub(crate) struct TestDir {
    pub path: PathBuf,
}

impl TestDir {
    pub fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should move forward")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "smart-clipboard-integration-{}-{}",
            std::process::id(),
            unique
        ));
        std::fs::create_dir_all(&path).expect("failed to create temp dir");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Keyring + biometric isolation guard
// ---------------------------------------------------------------------------

pub(crate) struct TestKeyringGuard;

impl TestKeyringGuard {
    pub fn new() -> Self {
        security::install_test_keyring_store();
        security::set_test_biometric_result(None);
        crate::biometric::set_test_biometric_available(None);
        Self
    }
}

impl Drop for TestKeyringGuard {
    fn drop(&mut self) {
        security::set_test_biometric_result(None);
        crate::biometric::set_test_biometric_available(None);
        security::reset_test_keyring_store();
    }
}

// ---------------------------------------------------------------------------
// TestHarness
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct LockStatusEventPayload {
    status: AppLockStatus,
}

pub(crate) struct TestHarness {
    pub _app: App<MockRuntime>,
    pub webview: WebviewWindow<MockRuntime>,
    pub config: Arc<ConfigManager>,
    pub lock: Arc<AppLockManager>,
    pub db: Arc<Database>,
    pub lock_status_rx: std::sync::mpsc::Receiver<AppLockStatus>,
    pub window_shown_rx: std::sync::mpsc::Receiver<()>,
}

impl TestHarness {
    pub fn new(base_dir: &Path) -> Self {
        let config = Arc::new(ConfigManager::new(base_dir.to_path_buf()));
        let lock = Arc::new(AppLockManager::new(config.clone()));
        let db = Arc::new(
            Database::new(&base_dir.join("clipboard.db").to_string_lossy())
                .expect("failed to initialize database"),
        );

        let app_data_dir = Arc::new(crate::AppDataDir(base_dir.to_path_buf()));

        let sync_manager = SyncManager::new(db.clone(), config.clone());
        let local = sync_manager.local_device_info();
        let webdav_manager = WebDavSyncManager::new(
            db.clone(),
            WebDavConfig::default(),
            &local.device_id,
            &local.device_name,
            &local.public_key,
        );

        let updater_manager = Arc::new(UpdaterManager::new("0.0.0-test".to_string()));
        let encryption_manager =
            Arc::new(crate::encryption::EncryptionManager::new(config.clone()));

        let app = test::mock_builder()
            .manage(config.clone())
            .manage(lock.clone())
            .manage(db.clone())
            .manage(app_data_dir)
            .manage(sync_manager)
            .manage(webdav_manager)
            .manage(updater_manager)
            .manage(encryption_manager)
            .invoke_handler(tauri::generate_handler![
                crate::commands::security::get_app_lock_status,
                crate::commands::security::set_app_lock_password,
                crate::commands::security::update_app_lock_settings,
                crate::commands::security::lock_app,
                crate::commands::security::unlock_app,
                crate::commands::clipboard::get_entries,
                crate::commands::clipboard::search_entries,
                crate::commands::clipboard::delete_entry,
                crate::commands::clipboard::delete_entries,
                crate::commands::clipboard::copy_entries,
                crate::commands::clipboard::set_favorite_state_for_entries,
                crate::commands::clipboard::toggle_favorite,
                crate::commands::clipboard::get_entry_count,
                crate::commands::clipboard::get_statistics,
                crate::commands::clipboard::paste_entry,
                crate::commands::tags::create_tag,
                crate::commands::tags::delete_tag,
                crate::commands::tags::get_all_tags,
                crate::commands::tags::add_tag_to_entry,
                crate::commands::tags::remove_tag_from_entry,
                crate::commands::tags::set_tags_for_entries,
                crate::commands::tags::get_entry_tags,
                crate::commands::tags::get_entries_by_tag,
                crate::commands::sync::get_sync_status,
                crate::commands::sync::get_sync_config,
                crate::commands::sync::update_sync_config,
                crate::commands::sync::get_discovered_devices,
                crate::commands::sync::get_paired_devices,
                crate::commands::sync::pair_device,
                crate::commands::sync::unpair_device,
                crate::commands::sync::toggle_device_sync,
                crate::commands::sync::webdav_connect,
                crate::commands::sync::webdav_disconnect,
                crate::commands::sync::webdav_get_status,
                crate::commands::sync::webdav_get_config,
                crate::commands::sync::webdav_update_config,
                crate::commands::sync::webdav_trigger_sync,
                crate::commands::sync::webdav_remove_device,
                crate::templates::commands::create_template,
                crate::templates::commands::update_template,
                crate::templates::commands::delete_template,
                crate::templates::commands::get_templates,
                crate::templates::commands::get_template,
                crate::templates::commands::use_template,
                crate::templates::commands::get_template_categories,
                crate::templates::commands::get_template_placeholders,
                crate::commands::config::get_config,
                crate::commands::config::update_config,
                crate::commands::transform::transform_content,
                crate::commands::updater::get_updater_status,
            ])
            .build(test::mock_context(test::noop_assets()))
            .expect("failed to build mock app");

        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("failed to build mock webview");

        let (lock_status_tx, lock_status_rx) = std::sync::mpsc::sync_channel(32);
        let (window_shown_tx, window_shown_rx) = std::sync::mpsc::sync_channel(8);

        app.listen_any("app-lock-status", move |event| {
            let payload: LockStatusEventPayload =
                serde_json::from_str(event.payload()).expect("lock-status payload must be JSON");
            let _ = lock_status_tx.send(payload.status);
        });
        app.listen_any("window-shown", move |_| {
            let _ = window_shown_tx.send(());
        });

        Self {
            _app: app,
            webview,
            config,
            lock,
            db,
            lock_status_rx,
            window_shown_rx,
        }
    }

    pub fn app_handle(&self) -> tauri::AppHandle<MockRuntime> {
        self.webview.app_handle().clone()
    }

    pub fn configure_password(&self) {
        self.lock
            .set_password(SetPasswordPayload {
                current_password: None,
                new_password: TEST_PASSWORD.to_string(),
            })
            .expect("setting password should succeed");
    }

    pub fn configure_auto_lock(&self, auto_lock_seconds: u64) {
        self.lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds,
                biometric_enabled: false,
            })
            .expect("updating app lock settings should succeed");
    }

    pub fn enable_biometric_in_config(&self) {
        let mut cfg = self.config.get();
        cfg.app_lock.enabled = true;
        cfg.app_lock.biometric_enabled = true;
        self.config
            .update(cfg)
            .expect("failed to enable biometric in config");
    }

    pub fn lock_now(&self) {
        self.configure_password();
        self.lock.lock("manual");
    }

    pub fn seed_default_entry(&self) -> i64 {
        let now = Local::now().naive_local();
        self.db
            .insert_entry(&ClipboardEntry {
                id: None,
                content: "alpha entry".to_string(),
                content_type: "text".to_string(),
                category: "text".to_string(),
                hash: format!(
                    "hash-alpha-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ),
                source_app: Some("integration-tests".to_string()),
                is_favorite: false,
                is_sensitive: false,
                use_count: 0,
                created_at: now,
                updated_at: now,
                expires_at: None,
                source_device: None,
            })
            .expect("failed to seed clipboard entry")
    }

    pub fn drain_lock_status(&self) {
        while self.lock_status_rx.try_recv().is_ok() {}
    }
}

// ---------------------------------------------------------------------------
// Invoke helpers
// ---------------------------------------------------------------------------

fn invoke_request(cmd: &str, body: Value) -> InvokeRequest {
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().expect("valid tauri url"),
        body: InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: test::INVOKE_KEY.to_string(),
    }
}

pub(crate) fn invoke<T: DeserializeOwned>(
    harness: &TestHarness,
    cmd: &str,
    body: Value,
) -> Result<T, Value> {
    test::get_ipc_response(&harness.webview, invoke_request(cmd, body)).map(|response| {
        response
            .deserialize::<T>()
            .expect("command response should deserialize")
    })
}

pub(crate) fn invoke_raw(harness: &TestHarness, cmd: &str, body: Value) -> Result<Value, Value> {
    test::get_ipc_response(&harness.webview, invoke_request(cmd, body))
        .map(|r| r.deserialize::<Value>().unwrap_or(Value::Null))
}

pub(crate) fn unlock_with_password(harness: &TestHarness) {
    let status: AppLockStatus = invoke(
        harness,
        "unlock_app",
        json!({"payload": {"password": TEST_PASSWORD, "prefer_biometric": false}}),
    )
    .expect("password unlock should succeed");
    assert!(!status.locked);
    harness.drain_lock_status();
}

pub(crate) fn manual_lock(harness: &TestHarness) {
    let _: AppLockStatus = invoke(harness, "lock_app", json!({})).expect("lock_app should succeed");
    harness.drain_lock_status();
}

pub(crate) fn recv_lock_status(harness: &TestHarness) -> AppLockStatus {
    harness
        .lock_status_rx
        .recv_timeout(Duration::from_millis(200))
        .expect("expected app-lock-status event within 200ms")
}

pub(crate) fn assert_no_lock_status(harness: &TestHarness) {
    assert!(
        harness
            .lock_status_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "did not expect app-lock-status event"
    );
}

pub(crate) fn assert_no_window_shown(harness: &TestHarness) {
    assert!(
        harness
            .window_shown_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "did not expect window-shown event"
    );
}

// ---------------------------------------------------------------------------
// Command table for table-driven tests
// ---------------------------------------------------------------------------

pub(crate) fn guarded_commands() -> Vec<(&'static str, Value)> {
    // Tauri 2 IPC converts Rust snake_case param names to camelCase JSON keys.
    vec![
        // Entries
        (
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "isFavorite": null}),
        ),
        (
            "search_entries",
            json!({"keyword": "alpha", "category": null, "isFavorite": null, "limit": 20, "offset": 0}),
        ),
        ("delete_entry", json!({"id": 999_999})),
        ("delete_entries", json!({"ids": [999_999]})),
        ("copy_entries", json!({"ids": [999_999]})),
        (
            "set_favorite_state_for_entries",
            json!({"ids": [999_999], "favorite": true}),
        ),
        ("toggle_favorite", json!({"id": 999_999})),
        ("get_entry_count", json!({})),
        ("get_statistics", json!({})),
        ("paste_entry", json!({"id": 999_999})),
        // Tags
        ("create_tag", json!({"name": "tag-x"})),
        ("delete_tag", json!({"id": 999_999})),
        ("get_all_tags", json!({})),
        ("add_tag_to_entry", json!({"entryId": 1, "tagId": 1})),
        ("remove_tag_from_entry", json!({"entryId": 1, "tagId": 1})),
        (
            "set_tags_for_entries",
            json!({"ids": [1], "tagIds": [1], "mode": "replace"}),
        ),
        ("get_entry_tags", json!({"entryId": 1})),
        ("get_entries_by_tag", json!({"tagId": 1})),
        // LAN sync
        ("get_sync_status", json!({})),
        ("get_sync_config", json!({})),
        (
            "update_sync_config",
            json!({"newConfig": {"enabled": false, "deviceName": "x", "port": 23456, "autoSync": false, "syncImages": false, "syncSensitive": false}}),
        ),
        ("get_discovered_devices", json!({})),
        ("get_paired_devices", json!({})),
        ("pair_device", json!({"deviceId": "fake-device"})),
        ("unpair_device", json!({"deviceId": "fake-device"})),
        (
            "toggle_device_sync",
            json!({"deviceId": "fake-device", "enabled": true}),
        ),
        // WebDAV
        (
            "webdav_connect",
            json!({"config": {"enabled": false, "serverUrl": "", "username": "", "password": "", "syncPassword": "", "pollIntervalSecs": 30, "syncImages": false, "syncSensitive": false, "rateLimitCapacity": 150, "rateLimitRefillMinutes": 30, "remotePath": "/SmartClipboard", "maxCloudEntries": 2000}}),
        ),
        ("webdav_disconnect", json!({})),
        ("webdav_get_status", json!({})),
        ("webdav_get_config", json!({})),
        (
            "webdav_update_config",
            json!({"newConfig": {"enabled": false, "serverUrl": "", "username": "", "password": "", "syncPassword": "", "pollIntervalSecs": 30, "syncImages": false, "syncSensitive": false, "rateLimitCapacity": 150, "rateLimitRefillMinutes": 30, "remotePath": "/SmartClipboard", "maxCloudEntries": 2000}}),
        ),
        ("webdav_trigger_sync", json!({})),
        ("webdav_remove_device", json!({"deviceId": "fake"})),
        // Templates
        (
            "create_template",
            json!({"name": "T", "content": "Hi {{n}}", "category": "general"}),
        ),
        (
            "update_template",
            json!({"id": 999_999, "name": "T", "content": "x", "category": "general"}),
        ),
        ("delete_template", json!({"id": 999_999})),
        ("get_templates", json!({"category": null})),
        ("get_template", json!({"id": 999_999})),
        ("use_template", json!({"id": 999_999, "values": {}})),
        ("get_template_categories", json!({})),
    ]
}

pub(crate) fn unguarded_commands() -> Vec<(&'static str, Value)> {
    vec![
        ("get_app_lock_status", json!({})),
        ("get_config", json!({})),
        (
            "transform_content",
            json!({"content": "hello", "transformType": "uppercase"}),
        ),
        (
            "get_template_placeholders",
            json!({"content": "Hi {{name}}"}),
        ),
        ("get_updater_status", json!({})),
    ]
}
