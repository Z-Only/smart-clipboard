# Deeper Runtime Integration Tests — Design Spec

> Status: **Approved by user** ("使用推荐方案进行推进", 2026-04-25)
> Authors: Aone Copilot via superpowers:brainstorming
> Target release: post-v2.3.0 maintenance

---

## 1. Goal

Add **invoke-level black-box integration tests** that exercise the real Tauri command pipeline of Smart Clipboard end-to-end, proving that **every protected IPC command honours the app-lock contract** under all relevant runtime states (locked, password-unlocked, biometric-unlocked, auto-locked, biometric-degraded, manually re-locked).

The current test suite already covers:

- `command_guard_tests` (commands.rs) — 6 invoke tests, but only spot-checks `get_entries`, `create_template`, `get_templates`, plus the lock/unlock/set-password commands themselves.
- `template_guard_tests` (templates/commands.rs) — 2 tests against a `FakeTemplateLock`, **not** through the real `AppLockManager` and **not** through `invoke`.
- `wakeup_tests` (hotkey.rs) — solid `TestHarness` for hotkey/tray wakeup interception.
- `security::tests` — focus events, auto-lock timer, biometric injection.

What's missing and what this spec adds:

1. Systematic invoke-level coverage of **every guarded command family** (clipboard CRUD, statistics, tags, sync, WebDAV, templates) under both locked and unlocked states.
2. A **state-flow** test that walks `password_set → manual lock → password unlock → idle/auto-lock → biometric unlock → manual lock again` and asserts the guard contract at each transition.
3. Explicit **biometric degradation** invoke test (toolkit unavailable / errors → password fallback succeeds, failed-attempts counter behaves).
4. Explicit **non-guarded command** allowlist test (e.g. `get_app_lock_status`, `unlock_app`, `quit_app`, `get_config`/`update_config`, `transform_content`, `get_template_placeholders`) — these MUST keep working while locked, and the test pins this contract so a future "everything should be guarded" refactor can't silently break unlock UX.
5. Wakeup-interception tests for **both** hotkey and tray paths under additional scenarios (auto-locked, biometric-enabled, biometric-failed) reusing the existing `wakeup_tests` harness style.

---

## 2. Non-Goals

- **No production code changes** to commands, security, biometric, hotkey, lib, or templates modules. This is a test-only delta. If a test exposes a real bug, file it as a separate finding; do NOT silently patch production code under the test PR.
- No new public API surface. Test helpers stay `pub(crate)` or `#[cfg(test)]`.
- No async runtime work (the `attach_lock_runtime` background timer is intentionally out of scope — covered indirectly by calling `emit_auto_lock_if_needed` synchronously, the same pattern security::tests already use).
- No Cargo `tests/` integration binary. Tauri's `mock_builder` plus `pub(crate)` test hooks make in-crate `#[cfg(test)]` modules the simpler, established choice.
- No changes to frontend / Vue tests.

---

## 3. Architecture

### 3.1 File layout (option **B** from brainstorm — independent test module tree)

```
src-tauri/src/
├── lib.rs                                     (modify: add `#[cfg(test)] mod integration_tests;`)
├── integration_tests/
│   ├── mod.rs                                 (declares submodules + re-exports)
│   ├── harness.rs                             (shared TestHarness, TestDir, TestKeyringGuard,
│   │                                           invoke<T>(), invoke_request(), TEST_SERIAL)
│   ├── locked_rejection.rs                    (Scenario 1: every guarded command -> "App is locked")
│   ├── unlock_flow.rs                         (Scenario 2: password + biometric unlock paths)
│   ├── auto_lock.rs                           (Scenario 3: auto-lock then re-reject)
│   ├── biometric_degradation.rs               (Scenario 4: biometric unavailable / error fallback)
│   ├── wakeup_interception.rs                 (Scenario 5: hotkey + tray under more states)
│   ├── template_guard.rs                      (Scenario 6: real-AppLockManager-backed template invoke)
│   ├── state_transitions.rs                   (Scenario 7: full flow walk)
│   └── unguarded_allowlist.rs                 (BONUS: pin the not-guarded-by-design contract)
```

Each submodule is `#[cfg(test)] mod xxx;` inside `integration_tests/mod.rs`, which is itself `#[cfg(test)]`. This keeps zero impact on release binary.

### 3.2 Shared `harness.rs`

Centralises everything currently duplicated between `command_guard_tests`, `wakeup_tests`, `security::tests`:

- `TEST_SERIAL: Mutex<()>` — held by every test (the static-mutex serialisation requirement).
- `TestDir` — temp dir RAII.
- `TestKeyringGuard` — `install_test_keyring_store()` + biometric reset on Drop.
- `TestHarness` — `tauri::test::mock_builder()` wired up with `ConfigManager`, `AppLockManager`, `Database`, optional `SyncManager` / `WebDavSyncManager` stubs, and **all guarded commands** registered in `invoke_handler!`. Exposes `app_handle()`, `webview()`, `lock`, `config`, `db`, plus event channels for `app-lock-status`, `window-shown`, `open-settings`.
- `invoke<T: DeserializeOwned>(harness, cmd, body) -> Result<T, Value>` — the same helper already in `command_guard_tests`, lifted to one place.
- `seed_default_entry(db)`, `configure_password(harness)`, `enable_biometric(harness)`, `set_biometric_result(...)`, etc. — small ergonomic helpers.

Sync and WebDAV managers are constructed in **offline mode** (no network); the tests only verify that `require_unlocked` runs first and that the underlying call returns whatever the manager would return (for our purposes, an `Err` from a real call is fine — we only care that the **guard** verdict is "App is locked" vs "anything else", never that sync actually completes).

### 3.3 Test Strategy

For each guarded command, we use a **two-pole assertion pattern**:

```rust
// Locked pole
let err = invoke::<RetType>(&h, "cmd_name", json!({ ...args... })).expect_err("locked must reject");
assert_eq!(err, json!("App is locked"));

// Unlocked pole
unlock_with_password(&h);
let _ = invoke::<RetType>(&h, "cmd_name", json!({ ...args... }));
// Either Ok(...) for read-only commands, or Err(business_error) for commands that need
// real sync/network. Critically, the error must NOT be "App is locked".
```

The full guarded-command list (extracted from `commands.rs` + `templates/commands.rs`):

**Clipboard / entries (10):**
`get_entries`, `search_entries`, `delete_entry`, `delete_entries`, `copy_entries`, `set_favorite_state_for_entries`, `toggle_favorite`, `get_entry_count`, `get_statistics`, `paste_entry`

**Tags (7):**
`create_tag`, `delete_tag`, `get_all_tags`, `add_tag_to_entry`, `remove_tag_from_entry`, `set_tags_for_entries`, `get_entry_tags`, `get_entries_by_tag`

**LAN sync (7):**
`get_sync_status`, `get_sync_config`, `update_sync_config`, `get_discovered_devices`, `get_paired_devices`, `pair_device`, `unpair_device`, `toggle_device_sync`

**WebDAV (7):**
`webdav_connect`, `webdav_disconnect`, `webdav_get_status`, `webdav_get_config`, `webdav_update_config`, `webdav_trigger_sync`, `webdav_remove_device`

**Templates (7):**
`create_template`, `update_template`, `delete_template`, `get_templates`, `get_template`, `use_template`, `get_template_categories`

For Scenario 1 (`locked_rejection.rs`) we use a **table-driven** test (single `#[test]` iterating a `&[(&str, Value)]` slice) so all ~38 commands stay maintainable in one file, and a single failure points to the offending command name. Scenario 2 mirrors the table for the unlocked pole.

The **unguarded allowlist** (Scenario 8) pins:
`get_app_lock_status`, `set_app_lock_password`, `update_app_lock_settings`, `lock_app`, `unlock_app` (lock-management surface), `quit_app`, `get_config`, `update_config`, `get_autostart_enabled`, `set_autostart_enabled`, `transform_content` (pure), `get_template_placeholders` (pure), `get_updater_status`, `check_for_updates_now`, `download_available_update`, `install_pending_update`, `discard_pending_update`. While locked these MUST not return `"App is locked"`. (They may legitimately return other errors when their dependencies aren't available — we only assert the **negative**.)

### 3.4 Auto-lock testing

Reuses `AppLockManager::rewind_last_activity_for_test(Duration)` plus a manual `manager.check_auto_lock()` call (matching `security::tests`). No real timers. This avoids flakiness and keeps tests fast.

### 3.5 Biometric injection

Reuses existing `crate::biometric::set_test_biometric_available(Some(false))` and `crate::security::set_test_biometric_result(Some(Err(...)))` hooks. No new test hooks needed.

### 3.6 Wakeup interception

Reuses `hotkey::handle_toggle_request` and `tray::handle_tray_menu_event` directly — same pattern as existing `wakeup_tests`. New cases:

- Hotkey while **auto-locked** → emits `app-lock-status` with `unlock_reason="shortcut"`, no `window-shown`.
- Hotkey while biometric-enabled but biometric returns error → still locked, hotkey still intercepted (because the guard is enforced **before** any biometric attempt).
- Tray "show" while auto-locked → same shape as above with `unlock_reason="tray_menu"`.

---

## 4. Error handling & test contracts

| Situation                                               | Expected return                                                                                     | Asserted by                                                                                           |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Locked + guarded cmd                                    | `Err(json!("App is locked"))`                                                                       | locked_rejection.rs (table)                                                                           |
| Unlocked + guarded cmd                                  | `Ok(_)` OR `Err(_)` where `err != "App is locked"`                                                  | unlock_flow.rs (table)                                                                                |
| Wrong password                                          | `Err(json!("Incorrect password"))`; `failed_attempts` increments; `unlock_reason="failed_password"` | already covered by existing `wrong_password_keeps_app_locked_and_tracks_failed_attempts` (kept as-is) |
| Biometric error + valid password                        | `Ok(status)` with `unlock_reason="password"`, `failed_attempts==0`                                  | biometric_degradation.rs                                                                              |
| Biometric unavailable + biometric_enabled write attempt | `update_settings` downgrades `biometric_enabled` to `false`                                         | biometric_degradation.rs (mirrors security::tests but via invoke)                                     |
| Auto-lock fired then sensitive cmd                      | second `get_entries` returns `"App is locked"`                                                      | auto_lock.rs                                                                                          |
| Hotkey while locked                                     | no `window-shown` event; one `app-lock-status` with `unlock_reason="shortcut"`                      | wakeup_interception.rs                                                                                |

---

## 5. Testing strategy summary

- **TDD discipline (rigid skill):** for each new test file: write ONE failing test → `cargo test -p smart-clipboard <test_name> -- --test-threads=1` → see RED → write helper or extend harness as needed → see GREEN → commit. Repeat per scenario.
- **Serialisation:** every `#[test]` in the new module **must** acquire `harness::TEST_SERIAL` first (the static keyring/biometric mutexes are global). Forgetting this is the most likely failure mode — harness exposes a `let _serial = lock_serial();` ergonomic helper to make it one line.
- **Run command:** `cd src-tauri && cargo test -- --test-threads=1` (also exposed as `pnpm run test:rust`). All existing tests must continue to pass.

---

## 6. Risks & mitigations

| Risk                                                                                                                                                                                                                                                                                                         | Mitigation                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `commands.rs` cyclical visibility — `command_guard_tests` is `#[cfg(test)] mod` _inside_ commands.rs, so it sees `super::*`. New `integration_tests/` is a sibling module of `commands` under `lib.rs`, so it must access commands via `crate::commands::xxx`. All command fns are already `pub` — verified. | No code change needed; just import paths in test files.                                                                                                                                 |
| `template_guard_tests` (FakeTemplateLock) becomes redundant.                                                                                                                                                                                                                                                 | Leave it in place. Don't delete in this PR. (Out of scope; the new `template_guard.rs` adds real-AppLockManager invoke coverage _alongside_ it.)                                        |
| WebDAV manager constructor needs a network endpoint?                                                                                                                                                                                                                                                         | Constructed with empty `WebDavConfig`; harness asserts only the guard verdict. Confirmed in commands.rs that it doesn't connect at construction time. To be re-verified in Plan Task 0. |
| Sync manager `set_app_handle` not called → `attach_lock_runtime` never fires the timer.                                                                                                                                                                                                                      | Intentional — auto-lock tests call `check_auto_lock()` synchronously, no timer needed.                                                                                                  |
| Test count balloons from 30→100+                                                                                                                                                                                                                                                                             | Acceptable for a "deeper integration tests" feature. Run-time stays under 30s with `--test-threads=1` because tauri mock builds are cheap. To be measured in Plan Task FINAL.           |
| `lib.rs` exposes private items as `pub(crate)` for tests                                                                                                                                                                                                                                                     | The new `mod integration_tests;` is `#[cfg(test)]` and lives under `crate::`, so it already sees `pub(crate)` items. No visibility loosening needed.                                    |

---

## 7. Acceptance criteria

1. `cd src-tauri && cargo test -- --test-threads=1` — all tests pass, **including ≥30 new tests** across the 8 new files.
2. Every `#[tauri::command]` in `commands.rs` and `templates/commands.rs` is referenced by name in either the **locked-rejection table** or the **unguarded allowlist**. (Audit task in Plan: enumerate via `grep` and diff.)
3. No production source files (`commands.rs`, `security.rs`, `biometric.rs`, `hotkey.rs`, `templates/commands.rs`, `lib.rs`) lose any existing test or behaviour. The only `lib.rs` edit is one `#[cfg(test)] mod integration_tests;` line.
4. README "Roadmap → Planned / Future Improvements" flips `[ ] **Deeper runtime integration tests**` to `[x]`.
5. `pnpm run test:rust` (which is the project's canonical Rust test command) passes locally.
6. No new `clippy` warnings introduced (`cd src-tauri && cargo clippy --all-targets -- -D warnings`).

---

## 8. Out-of-scope follow-ups (for a future spec)

- Cross-platform CI matrix (current CI only runs one OS at a time — biometric stub differs by `cfg(target_os)`).
- Property-based fuzzing of the lock state machine (proptest).
- Real LAN sync integration test with two in-process peers.
- Migrating `template_guard_tests::FakeTemplateLock` away once `template_guard.rs` proves equivalent coverage.
