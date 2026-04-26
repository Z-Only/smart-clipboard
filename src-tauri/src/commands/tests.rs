use crate::config::ConfigManager;
use crate::security::{self, AppLockStatus};
use crate::storage::{ClipboardEntry, Database, SearchResult, Template};
use chrono::Local;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{self, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

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
            "smart-clipboard-app-lock-command-tests-{}-{}",
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

struct TestHarness {
    _app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
    config: Arc<ConfigManager>,
}

fn create_harness(base_dir: &Path) -> TestHarness {
    let config = Arc::new(ConfigManager::new(base_dir.to_path_buf()));
    let lock = Arc::new(crate::security::AppLockManager::new(config.clone()));
    let encryption = Arc::new(crate::encryption::EncryptionManager::new(config.clone()));
    let db = Arc::new(
        Database::new(&base_dir.join("clipboard.db").to_string_lossy())
            .expect("failed to initialize database"),
    );

    let now = Local::now().naive_local();
    db.insert_entry(&ClipboardEntry {
        id: None,
        content: "alpha entry".to_string(),
        content_type: "text".to_string(),
        category: "text".to_string(),
        hash: "hash-alpha-entry".to_string(),
        source_app: Some("test-suite".to_string()),
        is_favorite: false,
        is_sensitive: false,
        use_count: 0,
        created_at: now,
        updated_at: now,
        expires_at: None,
        source_device: None,
    })
    .expect("failed to seed clipboard entry");

    let app = test::mock_builder()
        .manage(config.clone())
        .manage(lock)
        .manage(encryption)
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            super::security::get_app_lock_status,
            super::security::set_app_lock_password,
            super::security::lock_app,
            super::security::unlock_app,
            super::clipboard::get_entries,
            crate::templates::commands::create_template,
            crate::templates::commands::get_templates
        ])
        .build(test::mock_context(test::noop_assets()))
        .expect("failed to build mock app");

    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("failed to build mock webview");

    TestHarness {
        _app: app,
        webview,
        config,
    }
}

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

fn invoke<T: DeserializeOwned>(harness: &TestHarness, cmd: &str, body: Value) -> Result<T, Value> {
    test::get_ipc_response(&harness.webview, invoke_request(cmd, body)).map(
        |response: tauri::ipc::InvokeResponseBody| {
            response
                .deserialize::<T>()
                .expect("command response should deserialize")
        },
    )
}

fn enable_biometric_for_test(config: &ConfigManager) {
    let mut app_config = config.get();
    app_config.app_lock.enabled = true;
    app_config.app_lock.biometric_enabled = true;
    config
        .update(app_config)
        .expect("failed to enable biometric for test");
}

#[test]
fn locked_sensitive_command_is_rejected_via_invoke() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");

    let lock_status: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");
    assert!(lock_status.locked);

    let error = invoke::<SearchResult>(
        &harness,
        "get_entries",
        json!({
            "limit": 20,
            "offset": 0,
            "category": null,
            "is_favorite": null
        }),
    )
    .expect_err("locked app should reject sensitive commands");

    assert_eq!(error, json!("App is locked"));
}

#[test]
fn unlock_restores_sensitive_command_access() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");
    let _: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

    let unlock_status: AppLockStatus = invoke(
        &harness,
        "unlock_app",
        json!({
            "payload": {
                "password": "phase4-pass",
                "prefer_biometric": false
            }
        }),
    )
    .expect("unlock should succeed");

    assert!(!unlock_status.locked);
    assert_eq!(unlock_status.unlock_reason.as_deref(), Some("password"));

    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({
            "limit": 20,
            "offset": 0,
            "category": null,
            "is_favorite": null
        }),
    )
    .expect("unlocked app should allow sensitive commands");

    assert_eq!(result.total_count, 1);
    assert_eq!(result.entries[0].content, "alpha entry");
}

#[test]
fn wrong_password_keeps_app_locked_and_tracks_failed_attempts() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");
    let _: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

    let error = invoke::<AppLockStatus>(
        &harness,
        "unlock_app",
        json!({
            "payload": {
                "password": "wrong-pass",
                "prefer_biometric": false
            }
        }),
    )
    .expect_err("wrong password should fail");

    assert_eq!(error, json!("Incorrect password"));

    let status: AppLockStatus =
        invoke(&harness, "get_app_lock_status", json!({})).expect("status command should succeed");
    assert!(status.locked);
    assert_eq!(status.failed_attempts, 1);
    assert_eq!(status.unlock_reason.as_deref(), Some("failed_password"));
}

#[test]
fn biometric_failure_falls_back_to_password_unlock() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");

    enable_biometric_for_test(&harness.config);
    security::set_test_biometric_result(Some(Err("biometric failed".to_string())));

    let _: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

    let unlock_status: AppLockStatus = invoke(
        &harness,
        "unlock_app",
        json!({
            "payload": {
                "password": "phase4-pass",
                "prefer_biometric": true
            }
        }),
    )
    .expect("password fallback should succeed");

    assert!(!unlock_status.locked);
    assert_eq!(unlock_status.unlock_reason.as_deref(), Some("password"));
    assert_eq!(unlock_status.failed_attempts, 0);

    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({
            "limit": 20,
            "offset": 0,
            "category": null,
            "is_favorite": null
        }),
    )
    .expect("fallback unlock should restore access");

    assert_eq!(result.total_count, 1);
}

#[test]
fn locked_template_command_is_rejected_via_invoke() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");
    let _: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

    let error = invoke::<Template>(
        &harness,
        "create_template",
        json!({
            "name": "Greeting",
            "content": "Hello {{name}}",
            "category": "general"
        }),
    )
    .expect_err("locked app should reject template commands");

    assert_eq!(error, json!("App is locked"));
}

#[test]
fn unlock_restores_template_command_access() {
    let _serial = TEST_SERIAL.lock().unwrap();
    let _keyring = TestKeyringGuard::new();
    let temp_dir = TestDir::new();
    let harness = create_harness(&temp_dir.path);

    let _: AppLockStatus = invoke(
        &harness,
        "set_app_lock_password",
        json!({
            "payload": {
                "current_password": null,
                "new_password": "phase4-pass"
            }
        }),
    )
    .expect("setting password should succeed");
    let _: AppLockStatus =
        invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

    let _: AppLockStatus = invoke(
        &harness,
        "unlock_app",
        json!({
            "payload": {
                "password": "phase4-pass",
                "prefer_biometric": false
            }
        }),
    )
    .expect("unlock should succeed");

    let created: Template = invoke(
        &harness,
        "create_template",
        json!({
            "name": "Greeting",
            "content": "Hello {{name}}",
            "category": "general"
        }),
    )
    .expect("unlocked app should allow creating templates");

    assert_eq!(created.name, "Greeting");
    assert_eq!(created.content, "Hello {{name}}");
    assert_eq!(created.category, "general");

    let templates: Vec<Template> = invoke(
        &harness,
        "get_templates",
        json!({
            "category": null
        }),
    )
    .expect("unlocked app should allow listing templates");

    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "Greeting");
    assert_eq!(templates[0].content, "Hello {{name}}");
}
