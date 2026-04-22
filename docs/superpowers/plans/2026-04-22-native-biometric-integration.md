# Native Biometric Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current macOS `osascript`-based biometric convenience path with a native `LocalAuthentication` / Touch ID bridge, and add one automatic biometric attempt per lock session with password fallback.

**Architecture:** Add a focused Rust biometric bridge in `src-tauri/src/biometric.rs`, keep lock-state authority in `security.rs`, and split password unlock from biometric unlock at the command layer. Extend the lock status payload so the frontend can drive auto-prompt behavior from lock events alone, then update the Vue store and lock screen to auto-trigger Touch ID exactly once per new lock session while preserving manual retry and password fallback.

**Tech Stack:** Rust (Tauri 2, serde, objc2-local-authentication, block2), Vue 3 + TypeScript + Pinia + Vitest

---

## File Structure

### New Files

- `src-tauri/src/biometric.rs` — macOS biometric bridge, cross-platform stub, and test override seam
- `src/stores/securityStore.test.ts` — store-level regression tests for password errors vs biometric hint handling
- `src/components/LockScreen.test.ts` — lock-screen auto-prompt session guard tests

### Modified Rust Files

- `src-tauri/Cargo.toml` — add target-specific macOS Objective-C / LocalAuthentication dependencies
- `src-tauri/src/lib.rs` — export the biometric module and register the new biometric unlock command
- `src-tauri/src/security.rs` — add `biometric_auto_prompt_enabled`, `lock_session_id`, runtime session tracking, and updated tests
- `src-tauri/src/commands.rs` — split password unlock from biometric unlock and add invoke-level tests

### Modified Frontend Files

- `src/types/security.ts` — add new config/status fields and structured biometric response types
- `src/stores/securityStore.ts` — separate password vs biometric actions and keep biometric hint state out of the password error channel
- `src/components/LockScreen.vue` — auto-prompt biometrics once per lock session and keep manual retry available
- `src/components/SettingsPanel.vue` — expose the new auto-prompt toggle next to biometric unlock settings
- `src/i18n/locales/en.ts` — add lock-screen hint and settings labels
- `src/i18n/locales/zh-CN.ts` — add Chinese copies for the new strings
- `README.md` — document native macOS biometric integration and mark the roadmap item complete
- `README.zh-CN.md` — same documentation updates in Chinese

---

### Task 1: Add the Native Biometric Bridge Module

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/biometric.rs`
- Test: `src-tauri/src/biometric.rs`

- [ ] **Step 1: Write the failing bridge tests**

Add this test block to the new `src-tauri/src/biometric.rs` file first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_wins_for_authenticate_with_biometrics() {
        set_test_biometric_outcome(Some(BiometricAuthOutcome::Canceled));
        assert_eq!(
            authenticate_with_biometrics("Unlock Smart Clipboard"),
            BiometricAuthOutcome::Canceled
        );
        set_test_biometric_outcome(None);
    }

    #[test]
    fn non_macos_stub_reports_unavailable() {
        #[cfg(not(target_os = "macos"))]
        {
            assert!(!biometric_available());
            assert_eq!(
                authenticate_with_biometrics("Unlock Smart Clipboard"),
                BiometricAuthOutcome::Unavailable
            );
        }
    }
}
```

- [ ] **Step 2: Run the new Rust tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml test_override_wins_for_authenticate_with_biometrics -- --exact`

Expected: FAIL because `src-tauri/src/biometric.rs` and the referenced symbols do not exist yet.

- [ ] **Step 3: Add the macOS bridge dependencies and module implementation**

Update `src-tauri/Cargo.toml` by adding the target-specific macOS dependencies:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
block2 = "0.6"
objc2 = "0.6"
objc2-foundation = { version = "0.3.2", default-features = false, features = ["NSString", "NSError"] }
objc2-local-authentication = { version = "0.3.2", default-features = false, features = ["std", "block2", "LAContext", "LAError"] }
```

Export the module from `src-tauri/src/lib.rs` near the existing module list:

```rust
pub mod biometric;
```

Create `src-tauri/src/biometric.rs` with this implementation skeleton:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum BiometricAuthOutcome {
    Success,
    Canceled,
    Failed,
    Unavailable,
    Error(String),
}

#[cfg(test)]
static TEST_BIOMETRIC_OUTCOME: std::sync::Mutex<Option<BiometricAuthOutcome>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(crate) fn set_test_biometric_outcome(outcome: Option<BiometricAuthOutcome>) {
    *TEST_BIOMETRIC_OUTCOME.lock().unwrap() = outcome;
}

#[cfg(target_os = "macos")]
pub fn biometric_available() -> bool {
    use objc2_local_authentication::{LAContext, LAPolicy};

    unsafe {
        let context = LAContext::new();
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_ok()
    }
}

#[cfg(not(target_os = "macos"))]
pub fn biometric_available() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn map_error(error: &objc2_foundation::NSError) -> BiometricAuthOutcome {
    use objc2_local_authentication::LAError;

    match LAError(error.code()) {
        LAError::AuthenticationFailed => BiometricAuthOutcome::Failed,
        LAError::UserCancel
        | LAError::UserFallback
        | LAError::SystemCancel
        | LAError::AppCancel => BiometricAuthOutcome::Canceled,
        LAError::BiometryNotAvailable
        | LAError::BiometryNotEnrolled
        | LAError::BiometryLockout
        | LAError::PasscodeNotSet => BiometricAuthOutcome::Unavailable,
        _ => BiometricAuthOutcome::Error(error.localizedDescription().to_string()),
    }
}

#[cfg(target_os = "macos")]
pub fn authenticate_with_biometrics(reason: &str) -> BiometricAuthOutcome {
    #[cfg(test)]
    if let Some(outcome) = TEST_BIOMETRIC_OUTCOME.lock().unwrap().clone() {
        return outcome;
    }

    use block2::RcBlock;
    use objc2_foundation::NSString;
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::mpsc::sync_channel;

    unsafe {
        let context = LAContext::new();
        context.setLocalizedFallbackTitle(Some(&NSString::from_str("")));

        if context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_err()
        {
            return BiometricAuthOutcome::Unavailable;
        }

        let (tx, rx) = sync_channel(1);
        let reply = RcBlock::new(move |success: bool, error: *mut objc2_foundation::NSError| {
            let outcome = if success {
                BiometricAuthOutcome::Success
            } else if let Some(error) = unsafe { error.as_ref() } {
                map_error(error)
            } else {
                BiometricAuthOutcome::Failed
            };
            let _ = tx.send(outcome);
        });

        context.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
            &NSString::from_str(reason),
            &reply,
        );

        rx.recv().unwrap_or_else(|_| {
            BiometricAuthOutcome::Error("Biometric reply channel closed".to_string())
        })
    }
}

#[cfg(not(target_os = "macos"))]
pub fn authenticate_with_biometrics(_: &str) -> BiometricAuthOutcome {
    #[cfg(test)]
    if let Some(outcome) = TEST_BIOMETRIC_OUTCOME.lock().unwrap().clone() {
        return outcome;
    }

    BiometricAuthOutcome::Unavailable
}
```

- [ ] **Step 4: Run the bridge tests again to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml test_override_wins_for_authenticate_with_biometrics -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the bridge scaffold**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/biometric.rs
git commit -m "feat: add native biometric bridge scaffold"
```

---

### Task 2: Extend Lock Config and Runtime State for Session-Based Auto Prompting

**Files:**

- Modify: `src-tauri/src/security.rs`
- Test: `src-tauri/src/security.rs`

- [ ] **Step 1: Write the failing security runtime tests**

Add these tests inside the existing `#[cfg(test)] mod tests` block in `src-tauri/src/security.rs`:

```rust
    #[test]
    fn lock_session_id_increments_only_on_new_lock_transition() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        let first = harness.lock.lock("manual");
        assert!(first.locked);
        assert_eq!(first.lock_session_id, 1);

        let second = harness.lock.lock("focus");
        assert!(second.locked);
        assert_eq!(second.lock_session_id, 1);

        let unlocked = harness.lock.mark_biometric_unlocked();
        assert!(!unlocked.locked);

        let third = harness.lock.lock("manual");
        assert!(third.locked);
        assert_eq!(third.lock_session_id, 2);
    }

    #[test]
    fn status_exposes_auto_prompt_preference() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let config = Arc::new(ConfigManager::new(temp_dir.path.clone()));
        let manager = AppLockManager::new(config.clone());

        let mut app_config = config.get();
        app_config.app_lock.biometric_auto_prompt_enabled = false;
        config.update(app_config).expect("failed to update config");

        let status = manager.status();
        assert!(!status.biometric_auto_prompt_enabled);
        assert_eq!(status.lock_session_id, 0);
    }
```

Also update the existing `configure_auto_lock` helper to compile once the new field is introduced:

```rust
        fn configure_auto_lock(&self, enabled: bool, auto_lock_seconds: u64) {
            self.lock
                .update_settings(UpdateAppLockSettingsPayload {
                    enabled,
                    auto_lock_seconds,
                    biometric_enabled: false,
                    biometric_auto_prompt_enabled: true,
                })
                .expect("updating app lock settings should succeed");
        }
```

- [ ] **Step 2: Run the targeted security tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lock_session_id_increments_only_on_new_lock_transition -- --exact`

Expected: FAIL because `lock_session_id` and `biometric_auto_prompt_enabled` are not part of the security types yet.

- [ ] **Step 3: Implement the new config/status fields and lock-session logic**

Update the top of `src-tauri/src/security.rs` like this:

```rust
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppLockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_lock_seconds: u64,
    #[serde(default)]
    pub biometric_enabled: bool,
    #[serde(default = "default_true")]
    pub biometric_auto_prompt_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLockStatus {
    pub enabled: bool,
    pub configured: bool,
    pub locked: bool,
    pub biometric_available: bool,
    pub biometric_enabled: bool,
    pub biometric_auto_prompt_enabled: bool,
    pub auto_lock_seconds: u64,
    pub unlock_reason: Option<String>,
    pub failed_attempts: u32,
    pub lock_session_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppLockSettingsPayload {
    pub enabled: bool,
    pub auto_lock_seconds: u64,
    pub biometric_enabled: bool,
    pub biometric_auto_prompt_enabled: bool,
}
```

Extend the runtime state and status builder:

```rust
struct AppLockRuntimeState {
    locked: bool,
    last_unlock_at: Option<Instant>,
    last_activity_at: Instant,
    unlock_reason: Option<String>,
    failed_attempts: u32,
    lock_session_id: u64,
}

AppLockStatus {
    enabled: cfg.enabled,
    configured: password_hash_exists(),
    locked: runtime.locked,
    biometric_available: biometric::biometric_available(),
    biometric_enabled: cfg.biometric_enabled,
    biometric_auto_prompt_enabled: cfg.biometric_auto_prompt_enabled,
    auto_lock_seconds: cfg.auto_lock_seconds,
    unlock_reason: runtime.unlock_reason.clone(),
    failed_attempts: runtime.failed_attempts,
    lock_session_id: runtime.lock_session_id,
}
```

Update `update_settings` and `lock`:

```rust
        cfg.app_lock = AppLockConfig {
            enabled: payload.enabled,
            auto_lock_seconds: payload.auto_lock_seconds,
            biometric_enabled: payload.biometric_enabled && biometric::biometric_available(),
            biometric_auto_prompt_enabled: payload.biometric_auto_prompt_enabled,
        };
```

```rust
    pub fn lock(&self, reason: &str) -> AppLockStatus {
        let cfg = self.config.get().app_lock;
        let mut runtime = self.runtime.lock().unwrap();
        if cfg.enabled && password_hash_exists() {
            if !runtime.locked {
                runtime.lock_session_id = runtime.lock_session_id.saturating_add(1);
            }
            runtime.locked = true;
            runtime.unlock_reason = Some(reason.to_string());
        }
        drop(runtime);
        self.status()
    }
```

Also change all direct biometric calls in this file to use `crate::biometric`.

- [ ] **Step 4: Run the security tests again to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lock_session_id_increments_only_on_new_lock_transition -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the lock-state changes**

```bash
git add src-tauri/src/security.rs
git commit -m "feat: track lock sessions for biometric auto prompt"
```

---

### Task 3: Split Biometric Unlock Into Its Own Tauri Command

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write the failing invoke-level tests**

Add these tests to the existing command test module in `src-tauri/src/commands.rs`:

```rust
    #[test]
    fn biometric_unlock_success_returns_unlocked_status() {
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
        biometric::set_test_biometric_outcome(Some(BiometricAuthOutcome::Success));

        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let response: BiometricUnlockResponse =
            invoke(&harness, "unlock_with_biometric", json!({}))
                .expect("biometric unlock should succeed");

        assert_eq!(response.outcome, BiometricAuthOutcome::Success);
        assert!(!response.status.locked);
        assert_eq!(response.status.unlock_reason.as_deref(), Some("biometric"));
    }

    #[test]
    fn biometric_unlock_canceled_keeps_app_locked_without_password_error_count() {
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
        biometric::set_test_biometric_outcome(Some(BiometricAuthOutcome::Canceled));

        let _: AppLockStatus =
            invoke(&harness, "lock_app", json!({})).expect("manual lock should succeed");

        let response: BiometricUnlockResponse =
            invoke(&harness, "unlock_with_biometric", json!({}))
                .expect("biometric cancel should still return a response");

        assert_eq!(response.outcome, BiometricAuthOutcome::Canceled);
        assert!(response.status.locked);
        assert_eq!(response.status.failed_attempts, 0);
    }
```

- [ ] **Step 2: Run the targeted command tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml biometric_unlock_success_returns_unlocked_status -- --exact`

Expected: FAIL because `unlock_with_biometric` and `BiometricUnlockResponse` do not exist yet.

- [ ] **Step 3: Implement the command split**

In `src-tauri/src/commands.rs`, replace the old mixed unlock payload/flow with these types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockPayload {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BiometricUnlockResponse {
    pub outcome: biometric::BiometricAuthOutcome,
    pub status: AppLockStatus,
}
```

Keep `unlock_app` password-only:

```rust
#[tauri::command]
pub async fn unlock_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UnlockPayload,
) -> Result<AppLockStatus, String> {
    match lock.verify_password(&payload.password) {
        Ok(status) => {
            security::emit_lock_state(&app, &lock);
            Ok(status)
        }
        Err(err) => {
            let _ = lock.handle_failed_unlock();
            security::emit_lock_state(&app, &lock);
            Err(err)
        }
    }
}
```

Add the new biometric command:

```rust
#[tauri::command]
pub async fn unlock_with_biometric<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<BiometricUnlockResponse, String> {
    let status = lock.status();
    if !status.biometric_enabled || !status.biometric_available {
        return Ok(BiometricUnlockResponse {
            outcome: biometric::BiometricAuthOutcome::Unavailable,
            status,
        });
    }

    let outcome = biometric::authenticate_with_biometrics("Unlock Smart Clipboard");
    let status = match &outcome {
        biometric::BiometricAuthOutcome::Success => {
            let status = lock.mark_biometric_unlocked();
            security::emit_lock_state(&app, &lock);
            status
        }
        _ => lock.status(),
    };

    Ok(BiometricUnlockResponse { outcome, status })
}
```

Register the new command in `src-tauri/src/lib.rs`:

```rust
            commands::unlock_app,
            commands::unlock_with_biometric,
```

- [ ] **Step 4: Run the command tests again to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml biometric_unlock_success_returns_unlocked_status -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the command split**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: split biometric unlock into dedicated tauri command"
```

---

### Task 4: Update Frontend Types and the Security Store

**Files:**

- Modify: `src/types/security.ts`
- Modify: `src/stores/securityStore.ts`
- Create: `src/stores/securityStore.test.ts`
- Test: `src/stores/securityStore.test.ts`

- [ ] **Step 1: Write the failing store tests**

Create `src/stores/securityStore.test.ts` with these tests:

```ts
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useSecurityStore } from '@/stores/securityStore';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const lockedStatus = {
  enabled: true,
  configured: true,
  locked: true,
  biometric_available: true,
  biometric_enabled: true,
  biometric_auto_prompt_enabled: true,
  auto_lock_seconds: 30,
  unlock_reason: 'manual',
  failed_attempts: 0,
  lock_session_id: 7,
};

describe('securityStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('keeps biometric interruption out of the password error channel', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      outcome: { kind: 'canceled' },
      status: lockedStatus,
    });

    const store = useSecurityStore();
    await store.unlockWithBiometric();

    expect(store.error).toBeNull();
    expect(store.biometricHint).toBe(
      'Biometric unlock was not completed. Use your password or try again.',
    );
    expect(store.status.locked).toBe(true);
  });

  it('still stores incorrect password in the password error channel', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('Incorrect password');

    const store = useSecurityStore();
    await expect(store.unlockWithPassword('wrong-pass')).rejects.toBe('Incorrect password');

    expect(store.error).toContain('Incorrect password');
    expect(store.biometricHint).toBeNull();
  });
});
```

- [ ] **Step 2: Run the store tests to verify they fail**

Run: `pnpm exec vitest run src/stores/securityStore.test.ts`

Expected: FAIL because the store still exposes the old mixed `unlock` action and has no `biometricHint`.

- [ ] **Step 3: Implement the new types and store actions**

Update `src/types/security.ts`:

```ts
export interface AppLockConfig {
  enabled: boolean;
  auto_lock_seconds: number;
  biometric_enabled: boolean;
  biometric_auto_prompt_enabled: boolean;
}

export interface AppLockStatus {
  enabled: boolean;
  configured: boolean;
  locked: boolean;
  biometric_available: boolean;
  biometric_enabled: boolean;
  biometric_auto_prompt_enabled: boolean;
  auto_lock_seconds: number;
  unlock_reason: string | null;
  failed_attempts: number;
  lock_session_id: number;
}

export type BiometricAuthOutcome =
  | { kind: 'success' }
  | { kind: 'canceled' }
  | { kind: 'failed' }
  | { kind: 'unavailable' }
  | { kind: 'error'; message: string };

export interface BiometricUnlockResponse {
  outcome: BiometricAuthOutcome;
  status: AppLockStatus;
}
```

Update `src/stores/securityStore.ts`:

```ts
import type { AppLockStatus, BiometricUnlockResponse } from '@/types/security';

interface State {
  status: AppLockStatus;
  loading: boolean;
  error: string | null;
  biometricHint: string | null;
  initialized: boolean;
}

const defaultStatus: AppLockStatus = {
  enabled: false,
  configured: false,
  locked: false,
  biometric_available: false,
  biometric_enabled: false,
  biometric_auto_prompt_enabled: true,
  auto_lock_seconds: 0,
  unlock_reason: null,
  failed_attempts: 0,
  lock_session_id: 0,
};

function biometricHintForOutcome(outcome: BiometricUnlockResponse['outcome']) {
  return outcome.kind === 'success'
    ? null
    : 'Biometric unlock was not completed. Use your password or try again.';
}
```

Replace the old mixed unlock action with:

```ts
    async unlockWithPassword(password: string) {
      this.loading = true;
      this.error = null;
      this.biometricHint = null;
      try {
        this.status = await invoke<AppLockStatus>('unlock_app', {
          payload: { password },
        });
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async unlockWithBiometric() {
      this.loading = true;
      this.error = null;
      try {
        const response = await invoke<BiometricUnlockResponse>('unlock_with_biometric');
        this.status = response.status;
        this.biometricHint = biometricHintForOutcome(response.outcome);
        return response;
      } finally {
        this.loading = false;
      }
    },
```

Also update `updateSettings` to include `biometric_auto_prompt_enabled`.

- [ ] **Step 4: Run the store tests again to verify they pass**

Run: `pnpm exec vitest run src/stores/securityStore.test.ts`

Expected: PASS

- [ ] **Step 5: Commit the store changes**

```bash
git add src/types/security.ts src/stores/securityStore.ts src/stores/securityStore.test.ts
git commit -m "feat: separate biometric hint handling in security store"
```

---

### Task 5: Add Lock-Screen Auto Prompting and the Settings Toggle

**Files:**

- Modify: `src/components/LockScreen.vue`
- Modify: `src/components/SettingsPanel.vue`
- Modify: `src/i18n/locales/en.ts`
- Modify: `src/i18n/locales/zh-CN.ts`
- Create: `src/components/LockScreen.test.ts`
- Test: `src/components/LockScreen.test.ts`

- [ ] **Step 1: Write the failing lock-screen tests**

Create `src/components/LockScreen.test.ts` like this:

```ts
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import { reactive, nextTick } from 'vue';
import LockScreen from '@/components/LockScreen.vue';
import en from '@/i18n/locales/en';

const mockSecurityStore = reactive({
  status: {
    enabled: true,
    configured: true,
    locked: true,
    biometric_available: true,
    biometric_enabled: true,
    biometric_auto_prompt_enabled: true,
    auto_lock_seconds: 30,
    unlock_reason: 'manual',
    failed_attempts: 0,
    lock_session_id: 1,
  },
  error: null as string | null,
  biometricHint: null as string | null,
  unlockWithPassword: vi.fn(),
  unlockWithBiometric: vi.fn().mockResolvedValue({
    outcome: { kind: 'canceled' },
    status: {
      enabled: true,
      configured: true,
      locked: true,
      biometric_available: true,
      biometric_enabled: true,
      biometric_auto_prompt_enabled: true,
      auto_lock_seconds: 30,
      unlock_reason: 'manual',
      failed_attempts: 0,
      lock_session_id: 1,
    },
  }),
});

vi.mock('@/stores/securityStore', () => ({
  useSecurityStore: () => mockSecurityStore,
}));

describe('LockScreen', () => {
  beforeEach(() => {
    mockSecurityStore.unlockWithPassword.mockReset();
    mockSecurityStore.unlockWithBiometric.mockClear();
    mockSecurityStore.status.lock_session_id = 1;
    mockSecurityStore.status.biometric_auto_prompt_enabled = true;
  });

  it('auto-prompts only once for the same lock session', async () => {
    const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
    mount(LockScreen, { global: { plugins: [i18n] } });
    await flushPromises();

    expect(mockSecurityStore.unlockWithBiometric).toHaveBeenCalledTimes(1);

    mockSecurityStore.status.unlock_reason = 'focus';
    await nextTick();
    await flushPromises();

    expect(mockSecurityStore.unlockWithBiometric).toHaveBeenCalledTimes(1);
  });

  it('triggers again for a brand new lock session only', async () => {
    const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
    mount(LockScreen, { global: { plugins: [i18n] } });
    await flushPromises();

    mockSecurityStore.status.lock_session_id = 2;
    await nextTick();
    await flushPromises();

    expect(mockSecurityStore.unlockWithBiometric).toHaveBeenCalledTimes(2);
  });
});
```

- [ ] **Step 2: Run the lock-screen tests to verify they fail**

Run: `pnpm exec vitest run src/components/LockScreen.test.ts`

Expected: FAIL because `LockScreen.vue` still uses the old `security.unlock(null, true)` flow and has no lock-session auto-prompt watcher.

- [ ] **Step 3: Implement the new lock-screen flow, settings toggle, and translations**

Update the lock strings in `src/i18n/locales/en.ts`:

```ts
    biometricAutoPrompt: 'Automatically prompt on lock screen',
    biometricAutoPromptHint:
      'Try Touch ID once each time a new lock screen session begins',
    biometricRetryHint: 'Biometric unlock was not completed. Use your password or try again.',
```

Add the Chinese strings in `src/i18n/locales/zh-CN.ts`:

```ts
    biometricAutoPrompt: '锁屏时自动发起生物识别',
    biometricAutoPromptHint: '每次进入新的锁屏会话时自动尝试一次 Touch ID',
    biometricRetryHint: '生物识别未完成，请输入密码或再次尝试。',
```

Update the local config type and defaults in `src/components/SettingsPanel.vue`:

```ts
interface AppLockConfig {
  enabled: boolean;
  auto_lock_seconds: number;
  biometric_enabled: boolean;
  biometric_auto_prompt_enabled: boolean;
}

  app_lock: {
    enabled: false,
    auto_lock_seconds: 0,
    biometric_enabled: false,
    biometric_auto_prompt_enabled: true,
  },
```

Add the dependent toggle under the existing biometric section:

```vue
          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('lock.biometricAutoPrompt') }}</label>
              <p class="text-xs text-muted-foreground">
                {{ $t('lock.biometricAutoPromptHint') }}
              </p>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :disabled="!security.status.biometric_available || !form.app_lock.biometric_enabled"
              :class="
                form.app_lock.biometric_auto_prompt_enabled
                  ? 'bg-primary'
                  : 'bg-input disabled:opacity-50'
              "
              @click="
                form.app_lock.biometric_auto_prompt_enabled =
                  !form.app_lock.biometric_auto_prompt_enabled
              "
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="
                  form.app_lock.biometric_auto_prompt_enabled ? 'translate-x-4' : 'translate-x-0'
                "
              />
            </button>
          </div>
```

Update `src/components/LockScreen.vue`:

```ts
import { computed, ref, watch } from 'vue';

const autoAttemptedSessionId = ref<number | null>(null);
const autoAttemptInFlight = ref(false);

async function submitPassword() {
  try {
    await security.unlockWithPassword(password.value);
    password.value = '';
  } catch {
    password.value = '';
  }
}

async function unlockWithBiometric() {
  try {
    await security.unlockWithBiometric();
    password.value = '';
  } catch {
    // biometric interruptions are reflected through store state, not thrown password errors
  }
}

watch(
  () =>
    [
      security.status.locked,
      security.status.lock_session_id,
      security.status.biometric_available,
      security.status.biometric_enabled,
      security.status.biometric_auto_prompt_enabled,
    ] as const,
  async ([locked, sessionId, available, enabled, autoPromptEnabled]) => {
    if (!locked || !available || !enabled || !autoPromptEnabled) return;
    if (autoAttemptInFlight.value || autoAttemptedSessionId.value === sessionId) return;

    autoAttemptInFlight.value = true;
    autoAttemptedSessionId.value = sessionId;
    try {
      await security.unlockWithBiometric();
    } finally {
      autoAttemptInFlight.value = false;
    }
  },
  { immediate: true },
);
```

And render the biometric hint separately from the password error:

```vue
<p v-if="security.error" class="text-sm text-destructive">{{ security.error }}</p>
<p v-else-if="security.biometricHint" class="text-sm text-muted-foreground">
          {{ $t('lock.biometricRetryHint') }}
        </p>
```

- [ ] **Step 4: Run the lock-screen tests again to verify they pass**

Run: `pnpm exec vitest run src/components/LockScreen.test.ts`

Expected: PASS

- [ ] **Step 5: Commit the UI changes**

```bash
git add src/components/LockScreen.vue src/components/SettingsPanel.vue src/components/LockScreen.test.ts src/i18n/locales/en.ts src/i18n/locales/zh-CN.ts
git commit -m "feat: auto prompt biometrics once per lock session"
```

---

### Task 6: Update Documentation and Run End-to-End Verification

**Files:**

- Modify: `README.md`
- Modify: `README.zh-CN.md`

- [ ] **Step 1: Update the README copy**

Change the macOS biometric language in `README.md`:

```md
- **Native biometric unlock**: On macOS, Touch ID now uses a native `LocalAuthentication` bridge, with password fallback on interruption or failure
```

Update the platform behavior section:

```md
- **macOS**: Password lock, auto-lock, tray/hotkey interception, and native Touch ID / `LocalAuthentication` unlock are available
- **Windows / Linux**: Password lock, auto-lock, and tray/hotkey interception are available; biometric unlock remains password-only for now
```

Mark the roadmap item complete:

```md
- [x] **Native biometric integration**: Replace the current macOS convenience path with a fully native Touch ID / LocalAuthentication bridge and expand platform coverage where possible
```

Mirror the same meaning in `README.zh-CN.md`:

```md
- **原生生物识别解锁**：在 macOS 上通过原生 `LocalAuthentication` / Touch ID 解锁，失败或中断时回退到应用密码
```

- [ ] **Step 2: Run the targeted verification commands**

Run these commands in order:

```bash
cargo test --manifest-path src-tauri/Cargo.toml biometric_unlock_success_returns_unlocked_status -- --exact
cargo test --manifest-path src-tauri/Cargo.toml lock_session_id_increments_only_on_new_lock_transition -- --exact
pnpm exec vitest run src/stores/securityStore.test.ts src/components/LockScreen.test.ts
```

Expected:

- first Rust command: PASS
- second Rust command: PASS
- Vitest command: PASS

- [ ] **Step 3: Run the full quality gate**

Run: `pnpm run check`

Expected: PASS with zero formatting, lint, typecheck, web test, and rust test failures.

- [ ] **Step 4: Commit the docs and final verification pass**

```bash
git add README.md README.zh-CN.md
git commit -m "docs: update native biometric integration roadmap status"
```

---

## Self-Review

### Spec Coverage

- native macOS bridge: Task 1
- runtime capability detection: Tasks 1 and 2
- `biometric_auto_prompt_enabled`: Tasks 2, 4, and 5
- `lock_session_id` and once-per-session auto prompt: Tasks 2 and 5
- separate password vs biometric command paths: Task 3
- biometric hint vs password error separation: Tasks 4 and 5
- README / roadmap completion: Task 6

### Placeholder Scan

- No placeholder markers remain.
- All commands are concrete and reference exact file paths already present in the repo.

### Type Consistency

- Rust and TypeScript use the same field names: `biometric_auto_prompt_enabled`, `lock_session_id`
- The frontend action names are consistent across the plan: `unlockWithPassword`, `unlockWithBiometric`
- The Tauri response name is consistent across the plan: `BiometricUnlockResponse`
