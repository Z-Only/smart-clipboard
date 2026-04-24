# Native Biometric Integration — Design Spec

**Date:** 2026-04-24
**Status:** Approved
**Author:** Aone Copilot + 蝉雨

---

## 1. Overview

Replace the current macOS convenience path (osascript admin password prompt) with fully native biometric authentication using Touch ID on macOS and Windows Hello on Windows. The existing AppleScript-based "biometric" unlock is actually a system administrator password dialog — not real biometric authentication.

### Goals

- Provide genuine Touch ID authentication on macOS via the LocalAuthentication framework
- Provide Windows Hello authentication (fingerprint, face, PIN) on Windows via WinRT APIs
- Maintain the existing IPC contract so the frontend requires zero changes
- Preserve the current password-based unlock as the universal fallback

### Non-Goals

- Linux biometric support (fprintd/polkit) — deferred to a future iteration
- System password fallback within the biometric flow — biometric-only policy
- Changes to the frontend components or IPC commands

---

## 2. Architecture

### Approach: In-Place Refactor

Create a new `src-tauri/src/biometric.rs` module that encapsulates all platform-specific biometric FFI. The module follows the same `#[cfg(target_os)]` conditional compilation pattern already used by `platform.rs`.

```
src-tauri/src/
  ├── biometric.rs        ← NEW: platform-specific biometric FFI
  │    ├── #[cfg(macos)]    → LAContext (objc2-local-authentication)
  │    ├── #[cfg(windows)]  → UserConsentVerifier (windows crate)
  │    └── #[cfg(other)]    → stub (false / Err)
  ├── security.rs          ← calls biometric:: instead of inline FFI
  ├── commands.rs          ← unchanged IPC contract
  └── platform.rs          ← existing pattern reference
```

### Public API

The new module exposes exactly two public functions, matching the signatures of the functions being replaced in `security.rs`:

```rust
/// Returns true if the device has biometric hardware AND the user has
/// enrolled at least one biometric credential (fingerprint, face, etc.).
pub fn biometric_available() -> bool

/// Prompts the user for biometric authentication.
/// Returns Ok(true) on success, Ok(false) if the user cancels,
/// and Err(message) on failure (lockout, FFI error, etc.).
pub fn try_biometric_unlock() -> Result<bool, String>
```

---

## 3. Platform Implementations

### 3.1 macOS — Touch ID (LocalAuthentication)

**Crate:** `objc2-local-authentication` (published on crates.io, provides Rust bindings to Apple's LocalAuthentication framework). Fallback to `localauthentication-rs` if the primary crate has API gaps.

**Policy:** `LAPolicy::DeviceOwnerAuthenticationWithBiometrics` — biometric-only, no system password fallback within the native dialog.

**`biometric_available()`:**

1. Create an `LAContext` instance
2. Call `canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics)` with an error out-parameter
3. Return `true` only if the call succeeds (hardware present AND biometric enrolled)

**`try_biometric_unlock()`:**

1. Create an `LAContext` instance
2. Call `evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: "Unlock Smart Clipboard")`
3. This is an async callback-based API; bridge to synchronous Rust using a channel or `block_on`
4. Map results:
   - Success → `Ok(true)`
   - User cancel (error code `LAError.userCancel`) → `Ok(false)`
   - Biometric lockout (`LAError.biometryLockout`) → `Err("Biometric locked out")`
   - Other errors → `Err(error_description)`

**Removes:** The current `osascript` + AppleScript `with administrator privileges` implementation (~20 lines).

### 3.2 Windows — Windows Hello (UserConsentVerifier)

**Crate:** `windows` with feature `Security_Credentials_UI` and `Foundation`.

**`biometric_available()`:**

1. Call `UserConsentVerifier::CheckAvailabilityAsync()?.get()?`
2. Return `true` if result is `UserConsentVerifierAvailability::Available`

**`try_biometric_unlock()`:**

1. Call `UserConsentVerifier::RequestVerificationAsync("Unlock Smart Clipboard")?.get()?`
2. Map results:
   - `Verified` → `Ok(true)`
   - `Canceled` → `Ok(false)`
   - `DeviceNotPresent` / `NotConfiguredForUser` → `Err(description)`
   - `DisabledByPolicy` → `Err("Biometric disabled by policy")`
   - `RetriesExhausted` → `Err("Too many failed attempts")`

### 3.3 Other Platforms — Stub

```rust
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn biometric_available() -> bool { false }

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn try_biometric_unlock() -> Result<bool, String> { Ok(false) }
```

---

## 4. Changes to Existing Code

### 4.1 `security.rs`

- **Delete** the existing `biometric_available()` function (lines ~345-354) — hardcoded `true` on macOS, `false` elsewhere
- **Delete** the existing `try_biometric_unlock()` function (lines ~356-398) — osascript AppleScript implementation
- **Delete** the `TEST_BIOMETRIC_RESULT` static and `set_test_biometric_result()` helper (moved to `biometric.rs`)
- **Add** `use crate::biometric;` at the top
- **Replace** internal calls to `biometric_available()` with `biometric::biometric_available()`
- **Re-export** for backward compatibility: `pub use crate::biometric::try_biometric_unlock;` — this keeps `commands.rs` unchanged

### 4.2 `commands.rs`

- **No changes.** The `unlock_app` command calls `security::try_biometric_unlock()` (line 72), which will resolve to the re-exported `biometric::try_biometric_unlock()` via the `pub use` in `security.rs`.

### 4.3 `lib.rs`

- **Add** `pub mod biometric;` to the module declarations.

### 4.4 `Cargo.toml`

Add platform-conditional dependencies:

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

### 4.5 Frontend

- **No changes.** The IPC contract (`get_app_lock_status`, `unlock_app`) is unchanged. `LockScreen.vue`, `securityStore.ts`, and `security.ts` types remain as-is.

---

## 5. Error Handling and Fallback

### Error Classification

| Scenario     | Cause                                                | `biometric_available()` | `try_biometric_unlock()`                 | Frontend Behavior               |
| ------------ | ---------------------------------------------------- | ----------------------- | ---------------------------------------- | ------------------------------- |
| No hardware  | No Touch ID sensor / no Windows Hello camera         | `false`                 | N/A (button hidden)                      | Biometric button hidden         |
| Not enrolled | Hardware present, no fingerprint/face registered     | `false`                 | N/A (button hidden)                      | Biometric button hidden         |
| User cancels | Dismissed the native dialog                          | N/A                     | `Ok(false)`                              | Stay locked, no error shown     |
| Auth failed  | Wrong fingerprint/face                               | N/A                     | Native dialog handles retries internally | Native dialog shows retry       |
| Lockout      | Too many failures, OS temporarily disables biometric | N/A                     | `Err("Biometric locked out")`            | Show error, prompt for password |
| FFI error    | Framework loading failure                            | `false` (graceful)      | `Err(message)`                           | Fall back to password           |

### Fallback Flow

```
User clicks "Biometric Unlock"
  └→ try_biometric_unlock()
       ├→ Ok(true)  → mark_biometric_unlocked() → App unlocked ✅
       ├→ Ok(false) → Stay locked, user can retry or enter password
       └→ Err(_)    → Show error message, guide to password entry
```

### Design Decisions

1. **User cancel does NOT increment `failed_attempts`** — consistent with current behavior in `commands.rs`
2. **Native dialog handles retries** — macOS Touch ID and Windows Hello dialogs support multiple attempts natively; no application-level retry loop needed
3. **No caching of `biometric_available()`** — checked fresh each time, as users may add/remove biometric credentials at runtime. Cost is negligible (<1ms)

---

## 6. Testing Strategy

### Test Injection (Unit Tests)

The new `biometric.rs` module continues the existing test injection pattern from `security.rs`:

```rust
#[cfg(test)]
static TEST_BIOMETRIC_AVAILABLE: Mutex<Option<bool>> = Mutex::new(None);
#[cfg(test)]
static TEST_BIOMETRIC_RESULT: Mutex<Option<Result<bool, String>>> = Mutex::new(None);

#[cfg(test)]
pub(crate) fn set_test_biometric_available(val: Option<bool>) { ... }
#[cfg(test)]
pub(crate) fn set_test_biometric_result(result: Option<Result<bool, String>>) { ... }
```

### Test Scenarios

| #   | Scenario                   | Injection                                    | Assertion                                         |
| --- | -------------------------- | -------------------------------------------- | ------------------------------------------------- |
| 1   | Biometric available        | `set_test_biometric_available(Some(true))`   | `status.biometric_available == true`              |
| 2   | Biometric unavailable      | `set_test_biometric_available(Some(false))`  | `status.biometric_available == false`             |
| 3   | Successful unlock          | `set_test_biometric_result(Some(Ok(true)))`  | `locked == false`, `unlock_reason == "biometric"` |
| 4   | User cancel                | `set_test_biometric_result(Some(Ok(false)))` | `locked == true`, `failed_attempts` unchanged     |
| 5   | Biometric error → fallback | `set_test_biometric_result(Some(Err(...)))`  | Error propagated, password flow available         |
| 6   | Settings auto-downgrade    | available=false, enabled=true                | `biometric_enabled` set to `false` in config      |

### Migration of Existing Tests

- Move `TEST_BIOMETRIC_RESULT` and `set_test_biometric_result()` from `security.rs` to `biometric.rs`
- Update `security.rs` test imports to use `crate::biometric::set_test_biometric_result`
- Existing `securityStore.test.ts` frontend tests remain unchanged

### Platform FFI Testing

Real Touch ID / Windows Hello FFI is **not tested in CI** (requires hardware). Quality assurance:

- **Compile check:** CI runs `cargo check` per target to verify platform code compiles
- **Manual integration test:** Developer verifies real biometric dialog on local hardware
- **Thin FFI layer:** Platform code does one native API call; all business logic is in testable pure Rust

---

## 7. Dependencies

### New Dependencies

| Crate                        | Version | Platform | Purpose                       |
| ---------------------------- | ------- | -------- | ----------------------------- |
| `objc2-local-authentication` | 0.3     | macOS    | LAContext Rust bindings       |
| `objc2-foundation`           | 0.3     | macOS    | Foundation types for objc2    |
| `windows`                    | 0.58    | Windows  | UserConsentVerifier WinRT API |

### Existing Dependencies (unchanged)

- `keyring` — password hash storage (unaffected)
- `argon2` — password hashing (unaffected)

---

## 8. Files Changed

| File                         | Change Type  | Description                                                  |
| ---------------------------- | ------------ | ------------------------------------------------------------ |
| `src-tauri/src/biometric.rs` | **New**      | Platform-specific biometric FFI module                       |
| `src-tauri/src/security.rs`  | **Modified** | Remove inline biometric functions, delegate to `biometric::` |
| `src-tauri/src/lib.rs`       | **Modified** | Add `pub mod biometric;`                                     |
| `src-tauri/Cargo.toml`       | **Modified** | Add platform-conditional dependencies                        |

### Files NOT Changed

| File                            | Reason                                      |
| ------------------------------- | ------------------------------------------- |
| `src-tauri/src/commands.rs`     | IPC contract unchanged                      |
| `src/stores/securityStore.ts`   | Frontend API unchanged                      |
| `src/components/LockScreen.vue` | Already handles biometric button visibility |
| `src/types/security.ts`         | `AppLockStatus` interface unchanged         |

---

## 9. Scope and Boundaries

### In Scope

- Native Touch ID integration on macOS via LocalAuthentication
- Native Windows Hello integration via UserConsentVerifier
- Runtime hardware + enrollment detection
- Biometric-only policy (no system password in native dialog)
- Clean removal of the osascript/AppleScript workaround
- Unit tests with test injection
- Stub for unsupported platforms

### Out of Scope

- Linux biometric support (polkit/fprintd)
- Changes to the frontend UI components
- Changes to the IPC command signatures
- Database-at-rest encryption
- Biometric-protected keychain access (Secure Enclave)
