//! Scenario 7: Full lock → unlock → auto-lock → unlock → manual-lock cycle
//! verifying state consistency at every boundary.

use std::time::Duration;

use serde_json::json;

use super::harness::{
    invoke, invoke_raw, lock_serial, manual_lock, unlock_with_password, TestDir, TestHarness,
    TestKeyringGuard,
};
use crate::security;
use crate::storage::SearchResult;

#[test]
fn full_state_transition_cycle() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.seed_default_entry();
    harness.configure_password();
    harness.configure_auto_lock(1);

    // Phase 1: Fresh state — unlocked, guarded commands work.
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("phase 1: get_entries must succeed while unlocked");
    assert_eq!(result.total_count, 1, "phase 1: expected 1 entry");

    // Phase 2: Manual lock.
    manual_lock(&harness);
    let status = harness.lock.status();
    assert!(status.locked, "phase 2: must be locked after manual lock");
    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "phase 2: get_entries must be rejected"
    );

    // Phase 3: Unlock.
    unlock_with_password(&harness);
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("phase 3: get_entries must succeed after unlock");
    assert_eq!(result.total_count, 1);

    // Phase 4: Auto-lock via idle timeout.
    harness
        .lock
        .rewind_last_activity_for_test(Duration::from_secs(2));
    security::emit_auto_lock_if_needed(&harness.app_handle(), &harness.lock);
    let status = harness.lock.status();
    assert!(status.locked, "phase 4: must be locked after auto-lock");
    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "phase 4: get_entries must be rejected after auto-lock"
    );

    // Phase 5: Unlock again.
    unlock_with_password(&harness);
    let result: SearchResult = invoke(
        &harness,
        "get_entries",
        json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
    )
    .expect("phase 5: get_entries must succeed after second unlock");
    assert_eq!(result.total_count, 1);

    // Phase 6: Manual lock again — verify clean cycle.
    manual_lock(&harness);
    assert!(
        invoke_raw(
            &harness,
            "get_entries",
            json!({"limit": 20, "offset": 0, "category": null, "is_favorite": null}),
        )
        .is_err(),
        "phase 6: get_entries must be rejected after final manual lock"
    );
}
