# Native Biometric Integration Design Spec

## Overview

Upgrade the current macOS biometric convenience unlock path from the existing `osascript`-based admin prompt to a real native `LocalAuthentication` / Touch ID integration.

This work keeps the current app-lock model intact:

- password remains the primary guaranteed unlock method
- biometric unlock remains optional
- biometric failures must safely fall back to password
- sensitive access checks stay enforced in Rust

The goal of this package is to deliver a more native macOS unlock experience without broadening the scope to Windows Hello or Linux PAM in the same iteration.

## Current Problem

The current macOS implementation treats biometric availability as always `true` on macOS and attempts biometric unlock through an AppleScript administrator-privilege flow. That path has several problems:

- it is not actually backed by `LocalAuthentication` / Touch ID
- availability is not based on runtime device capability
- failure semantics are too coarse for a polished lock-screen UX
- the current `unlock_app(password?, prefer_biometric?)` command shape couples password and biometric paths too tightly

The roadmap item for native biometric integration is specifically about replacing this convenience path with a real platform-native bridge.

## Goals

- replace the current macOS biometric convenience path with a native `LocalAuthentication` bridge
- keep password unlock as the reliable fallback for all users
- make biometric availability reflect real runtime capability, not platform assumption alone
- support automatic biometric prompting on every lock-screen entry point by default
- allow users to disable automatic biometric prompting while keeping manual biometric unlock available
- enforce the two agreed protections:
  - debounce concurrent biometric attempts
  - after one automatic attempt fails or is canceled during the current lock session, do not auto-retry again in that same session
- preserve the existing Rust-side security boundary, guarded commands, and lock-state event flow

## Non-Goals

- adding Windows Hello support in this iteration
- adding Linux PAM or other Linux biometric support in this iteration
- redesigning the overall app-lock UX beyond the minimal changes needed for native biometric support
- changing password storage or replacing the existing Argon2 + keyring approach
- changing idle detection, tray interception, or guarded-command behavior outside the needs of this feature

## Recommended Approach

Implement a native macOS biometric bridge directly inside the Rust/Tauri backend, exposed through a small platform abstraction module and consumed by the existing app-lock runtime.

This is preferred over a helper executable or third-party plugin because it keeps the security-sensitive logic inside the current process, aligns with the existing `security.rs` ownership boundary, minimizes packaging complexity, and gives the project direct control over availability checks, result mapping, and test seams.

## Architecture

```text
Frontend LockScreen / Security Store
        │
        ├─ password unlock command
        └─ biometric unlock command
                 │
                 ▼
          commands.rs
                 │
                 ▼
           security.rs
                 │
                 ▼
          biometric.rs
                 │
                 ├─ macOS: LocalAuthentication bridge
                 └─ other platforms: unavailable stub
```

## Backend Design

### New biometric module

Add `src-tauri/src/biometric.rs` as the single backend abstraction for biometric capability and authentication.

It will expose a small API surface:

```rust
pub enum BiometricAuthOutcome {
    Success,
    Canceled,
    Failed,
    Unavailable,
    Error(String),
}

pub fn biometric_available() -> bool;
pub fn authenticate_with_biometrics(reason: &str) -> BiometricAuthOutcome;
```

Behavior by platform:

- macOS: call native `LocalAuthentication` APIs and map the native result into `BiometricAuthOutcome`
- Windows/Linux/other: return `Unavailable`

This keeps platform-specific code out of `security.rs` and makes the macOS bridge replaceable without disturbing the app-lock runtime.

### macOS native bridge

On macOS, the biometric module will use a native bridge around `LocalAuthentication`, specifically:

- create an evaluation context
- check whether biometric authentication can be evaluated
- issue an authentication request with an app-specific reason string such as `Unlock Smart Clipboard`
- wait for the result and map it into one of the defined outcomes

The implementation detail may use Objective-C interop or a thin native bridge layer, but the rest of the application should only depend on the stable Rust interface above.

The bridge should prefer the biometrics-only policy equivalent to `DeviceOwnerAuthenticationWithBiometrics`, not the broader device-owner-auth policy. That keeps the native prompt aligned with the product contract: biometric unlock is attempted natively, and password fallback remains the app's own lock-screen password flow rather than a macOS system-password sheet.

### Security runtime changes

`security.rs` remains the source of truth for lock state, password state, and guarded access, but its biometric responsibilities change:

- replace the current hard-coded macOS availability behavior with `biometric::biometric_available()`
- remove the current AppleScript-based biometric unlock path
- keep `mark_biometric_unlocked()` as the canonical state transition for successful biometric unlock
- add explicit lock-session tracking so the frontend can safely implement one automatic attempt per lock session

## Configuration Model

Extend `AppLockConfig` with a new field:

```rust
pub biometric_auto_prompt_enabled: bool
```

Default behavior:

- default `false` for `biometric_enabled`
- default `true` for `biometric_auto_prompt_enabled`

Resulting semantics:

- if biometric unlock is enabled, users automatically get the simpler default behavior
- users can later turn off auto-prompt without disabling biometric unlock entirely
- config remains backward compatible because old config files deserialize with the new field defaulted

Frontend TypeScript config types must mirror this new field.

## Lock-State Model

Extend `AppLockStatus` with:

```rust
pub lock_session_id: u64
```

Rules:

- increment `lock_session_id` only when the app transitions from unlocked to locked
- do not increment when repeated wake/focus/tray events occur while already locked
- preserve the current `unlock_reason` semantics for user-facing copy

This ID is the contract that allows the frontend to distinguish a genuinely new lock session from repeated event noise or component remounts.

## Command Design

Split password and biometric unlock into separate commands.

### Password unlock

Keep `unlock_app`, but narrow it to password-only behavior:

- input: `password`
- success: unlock, emit status, clear failed attempt count
- failure: stay locked, increment failed attempt count, emit status

### Biometric unlock

Add a dedicated command:

```rust
unlock_with_biometric
```

Behavior:

- check whether biometric unlock is enabled and available
- call `authenticate_with_biometrics`
- return a structured result that includes both the biometric outcome and the current lock status
- on `Success`, unlock and emit status
- on `Canceled`, `Failed`, `Unavailable`, or `Error`, remain locked and return the current locked status without treating it as a password error
- do not increment password failed-attempt counters for biometric outcomes

This separation avoids overloading password errors and makes the frontend auto-prompt flow easier to reason about and test.

A suitable response shape is:

```rust
pub struct BiometricUnlockResponse {
    pub outcome: BiometricAuthOutcome,
    pub status: AppLockStatus,
}
```

## Frontend Design

### Settings panel

Keep the existing biometric enable toggle and add a second dependent toggle:

- `Biometric unlock`
- `Automatically prompt on lock screen`

Rules:

- the auto-prompt toggle is only interactive when biometric unlock is enabled and the platform is currently available
- if the backend reports biometric unavailable, the saved config must resolve to `biometric_enabled = false`
- the UI should reflect backend truth after save instead of inferring availability on its own

### Lock screen

The lock screen keeps both manual unlock methods:

- password input and submit button
- biometric button when enabled and available

New auto-prompt behavior:

- when the lock screen enters a new `lock_session_id`
- and the status says `locked`
- and biometric is enabled
- and auto-prompt is enabled
- and biometric is available
- then the frontend fires one automatic biometric attempt

### Protection rules

The frontend must enforce the two agreed rules per `lock_session_id`:

1. debounce
   Only one biometric attempt may be in progress at a time for the current lock session.

2. fail once, stop auto-retrying
   If the automatic biometric attempt is canceled, fails, or errors, the frontend must not automatically trigger biometric again within the same `lock_session_id`.

Manual biometric retries remain allowed even after the automatic flow has failed for the session.

### Error and hint handling

Password errors and biometric interruptions should no longer share the same error channel.

Expected UX:

- password failure keeps the existing error treatment and failed-attempt count
- biometric cancel/failure/error is derived from the biometric command response rather than the password error state
- biometric cancel/failure/error shows a lighter informational hint such as `Biometric unlock was not completed. Use your password or try again.`
- biometric failure must never be rendered as `Incorrect password`

This keeps security feedback accurate while avoiding overly alarming messaging for normal Touch ID cancellation.

## Lock Entry Points

Automatic biometric prompting should behave consistently for every way the UI reaches the locked state:

- startup when app lock is enabled
- manual lock from settings
- idle auto-lock
- tray/hotkey/focus wakeups when the app is still locked

The contract is intentionally simple: if the user is looking at a fresh lock session and auto-prompt is enabled, the app should attempt biometric exactly once for that session.

## State Transitions

### Successful biometric unlock

1. app becomes locked and emits a new `lock_session_id`
2. frontend auto-prompts once if eligible
3. backend returns `Success`
4. `security.rs` marks biometric unlocked
5. status event unlocks the app
6. frontend reloads sensitive stores as it already does today

### Failed or canceled biometric auto-prompt

1. app becomes locked and emits a new `lock_session_id`
2. frontend auto-prompts once if eligible
3. backend returns `Canceled`, `Failed`, `Unavailable`, or `Error`
4. app stays locked
5. frontend records that auto-prompt has already been consumed for this `lock_session_id`
6. no further automatic prompts occur in this session
7. user may enter a password or manually tap the biometric button

### Manual biometric retry after auto failure

1. current lock session already consumed its auto-prompt
2. user taps the biometric button
3. frontend sends a manual biometric command
4. backend succeeds or fails independently of the auto-prompt guard
5. the current session remains locked until a successful password or biometric unlock occurs

## Testing Strategy

### Rust unit tests

Add or extend tests for:

- runtime biometric availability contract
- `lock_session_id` incrementing only on unlocked-to-locked transitions
- successful biometric unlock clearing failed attempts and setting `unlock_reason = biometric`
- biometric failure leaving the app locked without incrementing password failed-attempt count

### Rust invoke-level tests

Extend command tests to cover:

- `unlock_with_biometric` success path
- `unlock_with_biometric` failure path
- biometric failure followed by successful password unlock
- repeated `lock_app` or guarded wake events while already locked not creating a new lock session

### Frontend tests

Add targeted store/component tests for:

- auto-prompt firing once for a fresh lock session
- no duplicate auto-prompts while an attempt is already running
- no second automatic attempt after an auto failure in the same session
- manual biometric retry still working after auto failure
- password error presentation remaining separate from biometric hint presentation

## Documentation Updates

Implementation should update user-facing documentation after code lands:

- README release highlights should describe native macOS biometric integration rather than a generic convenience path
- platform behavior should explicitly say macOS uses a native Touch ID / `LocalAuthentication` bridge
- Windows and Linux should continue to be documented as password-only for now
- the roadmap entry `Native biometric integration` should be marked complete once implementation and tests are done

## Risks and Mitigations

### Native macOS bridge complexity

Risk:
Bridging `LocalAuthentication` from Rust adds platform-specific complexity.

Mitigation:
Isolate all platform-specific behavior inside `biometric.rs` with a narrow public interface and test seam.

### Over-prompting or repeated prompts

Risk:
Repeated tray/focus events or component remounts could trigger multiple Touch ID prompts.

Mitigation:
Use `lock_session_id` as the explicit session contract and enforce one automatic prompt attempt per session in the frontend.

### Incorrect error messaging

Risk:
Biometric failures could surface as password errors and confuse users.

Mitigation:
Split the command paths and separate biometric hint messaging from password error state.

### Platform drift

Risk:
The design could accidentally imply Windows/Linux biometric support before it exists.

Mitigation:
Keep non-macOS behavior stubbed as unavailable and document the scope clearly in both code and README updates.

## Implementation Scope Summary

This design intentionally stays focused on one roadmap item:

- native macOS biometric integration
- real runtime capability detection
- default automatic biometric prompt with user override
- per-lock-session debounce and no-auto-retry guards
- command and state-model adjustments needed to support those behaviors cleanly

It does not expand into broader platform coverage or unrelated access-security refactors in the same pass.
