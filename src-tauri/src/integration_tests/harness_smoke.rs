//! Smoke test proving the TestHarness boots and a simple invoke round-trips.

use serde_json::json;

use super::harness::{invoke, lock_serial, TestDir, TestHarness, TestKeyringGuard};
use crate::security::AppLockStatus;

#[test]
fn harness_boots_and_lock_status_round_trips() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    let status: AppLockStatus = invoke(&harness, "get_app_lock_status", json!({}))
        .expect("get_app_lock_status should succeed on a fresh harness");

    assert!(!status.locked, "fresh harness should be unlocked");
    assert!(
        !status.configured,
        "fresh harness should have no password configured"
    );
}
