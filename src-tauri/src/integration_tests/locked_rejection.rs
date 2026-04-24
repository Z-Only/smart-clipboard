//! Scenario 1: Every guarded command returns an error when the app is locked.

use super::harness::{
    guarded_commands, invoke_raw, lock_serial, TestDir, TestHarness, TestKeyringGuard,
};

#[test]
fn all_guarded_commands_reject_while_locked() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();
    assert!(
        harness.lock.status().locked,
        "precondition: app must be locked"
    );

    let mut failures = Vec::new();

    for (cmd, body) in guarded_commands() {
        match invoke_raw(&harness, cmd, body) {
            Ok(val) => {
                failures.push(format!("{cmd} succeeded (expected rejection): {val}"));
            }
            Err(err) => {
                let msg = err.as_str().unwrap_or("");
                if !msg.contains("locked") && !msg.contains("Locked") && !msg.contains("LOCKED") {
                    failures.push(format!("{cmd} returned error but not a lock error: {err}"));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Guarded commands that did NOT reject while locked:\n{}",
        failures.join("\n")
    );
}
