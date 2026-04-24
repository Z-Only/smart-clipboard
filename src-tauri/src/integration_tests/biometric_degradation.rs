//! Scenario 4: Biometric error/unavailable/cancel paths and their degradation behavior.

use serde_json::json;

use super::harness::{
    invoke, invoke_raw, lock_serial, TestDir, TestHarness, TestKeyringGuard, TEST_PASSWORD,
};
use crate::security::{self, AppLockStatus};
use crate::storage::SearchResult;

#[test]
fn biometric_error_falls_back_to_password_via_invoke() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.seed_default_entry();
    harness.lock_now();
    harness.enable_biometric_in_config();

    // Simulate biometric returning an error — password fallback should still work.
    security::set_test_biometric_result(Some(Err("biometric locked out".into())));

    let status: AppLockStatus = invoke(
        &harness,
        "unlock_app",
        json!({"payload": {"password": TEST_PASSWORD, "prefer_biometric": true}}),
    )
    .expect("password fallback should succeed even when biometric errors");

    assert!(!status.locked);

    // Access must be restored.
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("post-fallback access should be restored");
    assert_eq!(result.total_count, 1);
}

#[test]
fn biometric_unavailable_downgrades_settings_via_invoke() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.configure_password();

    // Mark biometric as not available on this machine.
    crate::biometric::set_test_biometric_available(Some(false));

    let status: AppLockStatus = invoke(
        &harness,
        "update_app_lock_settings",
        json!({"payload": {"enabled": true, "auto_lock_seconds": 0, "biometric_enabled": true}}),
    )
    .expect("update_app_lock_settings should succeed");

    assert!(
        !status.biometric_enabled,
        "biometric must be downgraded to false when hardware unavailable"
    );
}

#[test]
fn biometric_cancel_then_no_password_returns_error() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();
    harness.enable_biometric_in_config();

    // Biometric returns Ok(false) = user cancelled the prompt.
    security::set_test_biometric_result(Some(Ok(false)));

    let err = invoke_raw(
        &harness,
        "unlock_app",
        json!({"payload": {"password": null, "prefer_biometric": true}}),
    )
    .expect_err("biometric cancel + no password must error");

    let err_str = format!("{err}");
    assert!(
        err_str.contains("assword") || err_str.contains("required") || err_str.contains("cancel"),
        "expected password-required or cancellation error, got: {err}"
    );

    assert!(
        harness.lock.status().locked,
        "app must remain locked after biometric cancel with no password"
    );
}
