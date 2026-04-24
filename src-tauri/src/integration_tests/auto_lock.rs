//! Scenario 3: Auto-lock fires after idle timeout and re-rejects guarded commands.

use std::time::Duration;

use serde_json::json;

use super::harness::{invoke, invoke_raw, lock_serial, TestDir, TestHarness, TestKeyringGuard};
use crate::security::{self};
use crate::storage::SearchResult;

#[test]
fn auto_lock_re_rejects_sensitive_commands_after_idle_timeout() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.seed_default_entry();
    harness.configure_password();
    harness.configure_auto_lock(1);

    // Initially unlocked — sensitive call must succeed.
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("unlocked get_entries should succeed");
    assert_eq!(result.total_count, 1);

    // Force activity rewind, trigger auto-lock.
    harness
        .lock
        .rewind_last_activity_for_test(Duration::from_secs(2));
    security::emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

    let status = harness.lock.status();
    assert!(status.locked, "app must be locked after auto-lock fires");

    // Now the same call must be rejected.
    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "get_entries must be rejected after auto-lock"
    );
}

#[test]
fn auto_lock_does_not_fire_when_disabled() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.configure_password();
    // auto_lock_seconds = 0 means disabled
    harness.configure_auto_lock(0);

    harness
        .lock
        .rewind_last_activity_for_test(Duration::from_secs(2));
    security::emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);

    let status = harness.lock.status();
    assert!(!status.locked, "disabled auto-lock must not lock the app");
}
