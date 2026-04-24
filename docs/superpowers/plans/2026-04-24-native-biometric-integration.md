# Native Biometric Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fake osascript-based biometric unlock with native Touch ID (macOS) and Windows Hello (Windows) via platform FFI, keeping the existing IPC contract and frontend unchanged.

**Architecture:** A new `src-tauri/src/biometric.rs` module encapsulates all platform-specific biometric FFI using `#[cfg(target_os)]` conditional compilation. The existing `biometric_available()` and `try_biometric_unlock()` functions in `security.rs` are deleted and replaced with calls to the new module. The frontend, IPC commands, and `AppLockStatus` type remain unchanged.

**Tech Stack:** Rust, `objc2-local-authentication` (macOS LAContext), `windows` crate with `Security_Credentials_UI` feature (Windows Hello UserConsentVerifier), Tauri 2.

**Spec:** `docs/superpowers/specs/2026-04-24-native-biometric-integration-design.md`

---

## File Structure

| File                         | Action     | Responsibility                                                                                             |
| ---------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/biometric.rs` | **Create** | Platform-specific biometric FFI: `biometric_available()`, `try_biometric_unlock()`, test injection helpers |
| `src-tauri/src/security.rs`  | **Modify** | Remove inline biometric functions and `TEST_BIOMETRIC_RESULT`, add `use crate::biometric` and re-export    |
| `src-tauri/src/lib.rs`       | **Modify** | Add `pub mod biometric;` declaration                                                                       |
| `src-tauri/Cargo.toml`       | **Modify** | Add platform-conditional dependencies                                                                      |

---

### Task 1: Add Platform Dependencies to Cargo.toml

**Files:**

- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add macOS and Windows biometric dependencies**

Add the following at the end of `src-tauri/Cargo.toml`, before `[dev-dependencies]`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2-local-authentication = "0.3"
objc2-foundation = "0.3"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Security_Credentials_UI",
    "Foundation",
] }
```

- [ ] **Step 2: Verify the dependency resolves**

Run: `cd src-tauri && cargo check 2>&1 | tail -20`
Expected: Compilation succeeds (dependencies download and resolve)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build: add native biometric dependencies for macOS and Windows"
```

---

### Task 2: Create biometric.rs with Test Infrastructure and Stub

**Files:**

- Create: `src-tauri/src/biometric.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `src-tauri/src/biometric.rs` with test injection and stub**

```rust
//! Platform-specific biometric authentication.
//!
//! Provides native Touch ID (macOS) and Windows Hello (Windows) integration.
//! Other platforms return a safe stub that reports biometric as unavailable.

use std::sync::Mutex;

#[cfg(test)]
static TEST_BIOMETRIC_AVAILABLE: Mutex<Option<bool>> = Mutex::new(None);

#[cfg(test)]
static TEST_BIOMETRIC_RESULT: Mutex<Option<Result<bool, String>>> = Mutex::new(None);

#[cfg(test)]
pub(crate) fn set_test_biometric_available(val: Option<bool>) {
    *TEST_BIOMETRIC_AVAILABLE.lock().unwrap() = val;
}

#[cfg(test)]
pub(crate) fn set_test_biometric_result(result: Option<Result<bool, String>>) {
    *TEST_BIOMETRIC_RESULT.lock().unwrap() = result;
}

/// Returns `true` if the device has biometric hardware **and** the user has
/// enrolled at least one biometric credential (fingerprint, face, etc.).
pub fn biometric_available() -> bool {
    #[cfg(test)]
    if let Some(val) = *TEST_BIOMETRIC_AVAILABLE.lock().unwrap() {
        return val;
    }

    platform_biometric_available()
}

/// Prompts the user for biometric authentication via the native OS dialog.
///
/// Returns `Ok(true)` on success, `Ok(false)` if the user cancels, and
/// `Err(message)` on failure (lockout, FFI error, etc.).
pub fn try_biometric_unlock() -> Result<bool, String> {
    #[cfg(test)]
    if let Some(result) = TEST_BIOMETRIC_RESULT.lock().unwrap().clone() {
        return result;
    }

    platform_try_biometric_unlock()
}

// ---------------------------------------------------------------------------
// macOS — Touch ID via LocalAuthentication framework
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn platform_biometric_available() -> bool {
    use objc2_local_authentication::{LAContext, LAPolicy};

    let context = LAContext::new();
    let mut error = None;
    context.canEvaluatePolicy_error(
        LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
        &mut error,
    )
}

#[cfg(target_os = "macos")]
fn platform_try_biometric_unlock() -> Result<bool, String> {
    use objc2_foundation::NSString;
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::mpsc;

    let context = LAContext::new();
    let reason = NSString::from_str("Unlock Smart Clipboard");

    let (tx, rx) = mpsc::channel();

    unsafe {
        context.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
            &reason,
            &objc2_foundation::block2::RcBlock::new(move |success: bool, error| {
                if success {
                    let _ = tx.send(Ok(true));
                } else if let Some(err) = error {
                    let code = err.code();
                    // LAError.userCancel == -2, LAError.appCancel == -9
                    if code == -2 || code == -9 {
                        let _ = tx.send(Ok(false));
                    } else {
                        let desc = err.localizedDescription().to_string();
                        let _ = tx.send(Err(desc));
                    }
                } else {
                    let _ = tx.send(Ok(false));
                }
            }),
        );
    }

    rx.recv().map_err(|e| format!("Biometric channel error: {e}"))?
}

// ---------------------------------------------------------------------------
// Windows — Windows Hello via UserConsentVerifier
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn platform_biometric_available() -> bool {
    use windows::Security::Credentials::UI::{
        UserConsentVerifier, UserConsentVerifierAvailability,
    };

    UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|op| op.get())
        .map(|availability| availability == UserConsentVerifierAvailability::Available)
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn platform_try_biometric_unlock() -> Result<bool, String> {
    use windows::core::HSTRING;
    use windows::Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier,
    };

    let message = HSTRING::from("Unlock Smart Clipboard");

    let result = UserConsentVerifier::RequestVerificationAsync(&message)
        .and_then(|op| op.get())
        .map_err(|e| format!("Windows Hello error: {e}"))?;

    match result {
        UserConsentVerificationResult::Verified => Ok(true),
        UserConsentVerificationResult::Canceled => Ok(false),
        other => Err(format!("Windows Hello denied: {:?}", other)),
    }
}

// ---------------------------------------------------------------------------
// Stub — unsupported platforms
// ---------------------------------------------------------------------------

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_biometric_available() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_try_biometric_unlock() -> Result<bool, String> {
    Ok(false)
}
```

- [ ] **Step 2: Register the module in `src-tauri/src/lib.rs`**

Add `pub mod biometric;` after the existing `pub mod analyzer;` line:

```rust
pub mod analyzer;
pub mod biometric;
pub mod clipboard;
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | tail -20`
Expected: Compilation succeeds

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/biometric.rs src-tauri/src/lib.rs
git commit -m "feat: add biometric module with native Touch ID and Windows Hello support"
```

---

### Task 3: Migrate security.rs to Use biometric Module

**Files:**

- Modify: `src-tauri/src/security.rs`

- [ ] **Step 1: Remove `TEST_BIOMETRIC_RESULT` static from `security.rs`**

Delete line 15 from `security.rs`:

```rust
#[cfg(test)]
static TEST_BIOMETRIC_RESULT: Mutex<Option<Result<bool, String>>> = Mutex::new(None);
```

- [ ] **Step 2: Add biometric re-exports to `security.rs`**

Add the following after the existing `use crate::config::ConfigManager;` line (line 9):

```rust
// Delegate biometric functions to the dedicated module and re-export for
// backward compatibility with commands.rs.
pub use crate::biometric::try_biometric_unlock;
use crate::biometric;
```

- [ ] **Step 3: Replace `biometric_available()` function body in `security.rs`**

Delete the existing `biometric_available()` function (lines 337–347):

```rust
pub fn biometric_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
```

Replace with:

```rust
pub fn biometric_available() -> bool {
    biometric::biometric_available()
}
```

- [ ] **Step 4: Delete the `try_biometric_unlock()` function from `security.rs`**

Delete the entire `try_biometric_unlock()` function (lines 349–382):

```rust
pub fn try_biometric_unlock() -> Result<bool, String> {
    #[cfg(test)]
    if let Some(result) = TEST_BIOMETRIC_RESULT.lock().unwrap().clone() {
        return result;
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let script = r#"
try
  do shell script "echo biometric-check" with prompt "Unlock Smart Clipboard" with administrator privileges
  return "ok"
on error errMsg number errNum
  error errMsg number errNum
end try
"#;
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to start biometric prompt: {e}"))?;
        if output.status.success() {
            Ok(true)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}
```

This function is now provided by the `pub use crate::biometric::try_biometric_unlock;` re-export added in Step 2.

- [ ] **Step 5: Update `set_test_biometric_result` helper in `security.rs`**

Delete the existing `set_test_biometric_result` function (around line 293):

```rust
#[cfg(test)]
pub(crate) fn set_test_biometric_result(result: Option<Result<bool, String>>) {
    *TEST_BIOMETRIC_RESULT.lock().unwrap() = result;
}
```

Replace with a delegation to the biometric module:

```rust
#[cfg(test)]
pub(crate) fn set_test_biometric_result(result: Option<Result<bool, String>>) {
    crate::biometric::set_test_biometric_result(result);
}
```

- [ ] **Step 6: Update `TestKeyringGuard` in `security.rs` tests**

The `TestKeyringGuard::new()` and `Drop` already call `set_test_biometric_result(None)` — these will now delegate to the biometric module via the updated helper. No further changes needed.

- [ ] **Step 7: Verify compilation and tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -30`
Expected: All existing tests pass

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/security.rs
git commit -m "refactor: migrate biometric functions from security.rs to biometric module"
```

---

### Task 4: Add Biometric-Specific Unit Tests

**Files:**

- Modify: `src-tauri/src/security.rs` (test module)

- [ ] **Step 1: Add test for biometric availability injection**

Add the following test to the `#[cfg(test)] mod tests` block in `security.rs`, after the existing `biometric_availability_is_boolean_contract` test:

```rust
    #[test]
    fn biometric_available_reflects_injected_value() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();

        crate::biometric::set_test_biometric_available(Some(true));
        assert!(biometric_available());

        crate::biometric::set_test_biometric_available(Some(false));
        assert!(!biometric_available());

        crate::biometric::set_test_biometric_available(None);
    }
```

- [ ] **Step 2: Add test for successful biometric unlock**

```rust
    #[test]
    fn biometric_unlock_success_clears_lock() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        harness.lock.lock("manual");
        assert!(harness.lock.status().locked);

        set_test_biometric_result(Some(Ok(true)));
        let status = harness.lock.mark_biometric_unlocked();
        assert!(!status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("biometric"));
        assert_eq!(status.failed_attempts, 0);
    }
```

- [ ] **Step 3: Add test for biometric user cancel preserves lock**

```rust
    #[test]
    fn biometric_cancel_keeps_lock_and_does_not_increment_failures() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        harness.lock.lock("manual");
        let before = harness.lock.status().failed_attempts;

        set_test_biometric_result(Some(Ok(false)));
        let result = try_biometric_unlock();
        assert_eq!(result, Ok(false));

        let status = harness.lock.status();
        assert!(status.locked);
        assert_eq!(status.failed_attempts, before);
    }
```

- [ ] **Step 4: Add test for biometric error returns Err**

```rust
    #[test]
    fn biometric_error_returns_err() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();

        set_test_biometric_result(Some(Err("Biometric locked out".to_string())));
        let result = try_biometric_unlock();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Biometric locked out");

        set_test_biometric_result(None);
    }
```

- [ ] **Step 5: Add test for settings auto-downgrade when biometric unavailable**

```rust
    #[test]
    fn settings_downgrade_biometric_when_unavailable() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();

        crate::biometric::set_test_biometric_available(Some(false));

        let status = harness
            .lock
            .update_settings(UpdateAppLockSettingsPayload {
                enabled: true,
                auto_lock_seconds: 0,
                biometric_enabled: true,
            })
            .unwrap();

        assert!(!status.biometric_enabled);

        crate::biometric::set_test_biometric_available(None);
    }
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -30`
Expected: All tests pass, including the 5 new biometric tests

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/security.rs
git commit -m "test: add unit tests for native biometric integration"
```

---

### Task 5: Update Documentation and Final Verification

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Update the Security Model section in README.md**

Find the current platform behavior section:

```markdown
### Current platform behavior

- **macOS**: Password lock, auto-lock, tray/hotkey interception, and a system-auth convenience unlock path are available
- **Windows / Linux**: Password lock, auto-lock, and tray/hotkey interception are available; biometric unlock currently falls back to password-only behavior
```

Replace with:

```markdown
### Current platform behavior

- **macOS**: Password lock, auto-lock, tray/hotkey interception, and native Touch ID unlock via LocalAuthentication framework
- **Windows**: Password lock, auto-lock, tray/hotkey interception, and native Windows Hello unlock (fingerprint, face, PIN)
- **Linux**: Password lock, auto-lock, and tray/hotkey interception are available; biometric unlock falls back to password-only behavior
```

- [ ] **Step 2: Update the Roadmap section in README.md**

Find:

```markdown
- [ ] **Native biometric integration**: Replace the current macOS convenience path with a fully native Touch ID / LocalAuthentication bridge and expand platform coverage where possible
```

Replace with:

```markdown
- [x] **Native biometric integration**: Native Touch ID (macOS) and Windows Hello (Windows) via platform FFI
```

- [ ] **Step 3: Update the Tech Stack table in README.md**

Find:

```markdown
| Local security | argon2 + keyring |
```

Replace with:

```markdown
| Local security | argon2 + keyring + LocalAuthentication (macOS) + Windows Hello (Windows) |
```

- [ ] **Step 4: Run the full quality gate**

Run: `cd /Users/chanyu/AIProjects/smart-clipboard && pnpm run check 2>&1 | tail -40`
Expected: All checks pass (format, lint, typecheck, tests)

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: update README for native biometric integration"
```
