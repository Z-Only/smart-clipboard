//! Scenario 2: Unlock via password restores access; wrong password increments
//! failed_attempts and keeps the app locked.

use serde_json::json;

use super::harness::{
    invoke, invoke_raw, lock_serial, manual_lock, unlock_with_password, TestDir, TestHarness,
    TestKeyringGuard,
};
use crate::security::AppLockStatus;
use crate::storage::SearchResult;

#[test]
fn correct_password_unlocks_and_restores_access() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.seed_default_entry();
    harness.lock_now();

    // Locked — get_entries must fail.
    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "get_entries must be rejected while locked"
    );

    // Unlock with the correct password.
    unlock_with_password(&harness);

    // Now the same call must succeed.
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("get_entries must succeed after unlock");

    assert_eq!(result.total_count, 1);
}

#[test]
fn wrong_password_increments_failed_attempts() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();

    let err = invoke_raw(
        &harness,
        "unlock_app",
        json!({"payload": {"password": "wrong-pass", "prefer_biometric": false}}),
    )
    .expect_err("wrong password must return an error");

    let err_str = err.as_str().unwrap_or("");
    assert!(
        err_str.contains("Invalid") || err_str.contains("invalid") || err_str.contains("password"),
        "expected invalid-password error, got: {err}"
    );

    let status: AppLockStatus = invoke(&harness, "get_app_lock_status", json!({}))
        .expect("get_app_lock_status should always work");
    assert!(status.locked, "app must remain locked after wrong password");
    assert!(
        status.failed_attempts >= 1,
        "failed_attempts must be >= 1, got {}",
        status.failed_attempts
    );
}

#[test]
fn lock_unlock_lock_cycle_maintains_consistency() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.seed_default_entry();
    harness.lock_now();

    // Unlock → re-lock → unlock again.
    unlock_with_password(&harness);
    manual_lock(&harness);

    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "get_entries must fail after re-lock"
    );

    unlock_with_password(&harness);

    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("get_entries must succeed after second unlock");
    assert_eq!(result.total_count, 1);
}
