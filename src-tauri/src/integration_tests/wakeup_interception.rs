//! Scenario 5: Hotkey and tray wakeup paths while locked (including auto-lock
//! and biometric-enabled states).

use std::time::Duration;

use super::harness::{lock_serial, TestDir, TestHarness, TestKeyringGuard};
use crate::hotkey::handle_toggle_request;
use crate::security;

#[test]
fn hotkey_intercepted_after_auto_lock() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.configure_password();
    harness.configure_auto_lock(1);

    // Simulate idle-timeout auto-lock.
    harness
        .lock
        .rewind_last_activity_for_test(Duration::from_secs(2));
    security::emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);
    harness.drain_lock_status();

    handle_toggle_request(&harness.app_handle(), false);

    let status = harness
        .lock_status_rx
        .recv_timeout(Duration::from_millis(500))
        .expect("hotkey while auto-locked must emit app-lock-status");
    assert!(status.locked);
}

#[test]
fn hotkey_intercepted_when_biometric_enabled_but_locked() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();
    harness.enable_biometric_in_config();
    harness.drain_lock_status();

    // Even with biometric enabled, hotkey while locked must NOT auto-attempt
    // biometric — it should just emit lock status and let frontend drive the
    // unlock UI.
    security::set_test_biometric_result(Some(Ok(true)));
    handle_toggle_request(&harness.app_handle(), false);

    let status = harness
        .lock_status_rx
        .recv_timeout(Duration::from_millis(500))
        .expect("hotkey must emit lock status even when biometric enabled");
    assert!(status.locked);
}
