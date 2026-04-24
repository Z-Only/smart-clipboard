//! Scenario 8: Unguarded commands remain accessible even while the app is locked.

use super::harness::{
    invoke_raw, lock_serial, unguarded_commands, TestDir, TestHarness, TestKeyringGuard,
};

#[test]
fn all_unguarded_commands_succeed_while_locked() {
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

    for (cmd, body) in unguarded_commands() {
        if let Err(err) = invoke_raw(&harness, cmd, body) {
            failures.push(format!("{cmd} was rejected (should be unguarded): {err}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Unguarded commands that were unexpectedly rejected while locked:\n{}",
        failures.join("\n")
    );
}
