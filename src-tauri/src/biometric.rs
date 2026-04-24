//! Platform-specific biometric authentication.
//!
//! Provides native Touch ID (macOS) and Windows Hello (Windows) integration.
//! Other platforms return a safe stub that reports biometric as unavailable.

#[cfg(test)]
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
    {
        if let Some(val) = *TEST_BIOMETRIC_AVAILABLE.lock().unwrap() {
            return val;
        }
    }

    platform_biometric_available()
}

/// Prompts the user for biometric authentication via the native OS dialog.
///
/// Returns `Ok(true)` on success, `Ok(false)` if the user cancels, and
/// `Err(message)` on failure (lockout, FFI error, etc.).
pub fn try_biometric_unlock() -> Result<bool, String> {
    #[cfg(test)]
    {
        if let Some(result) = TEST_BIOMETRIC_RESULT.lock().unwrap().clone() {
            return result;
        }
    }

    platform_try_biometric_unlock()
}

// ---------------------------------------------------------------------------
// macOS — Touch ID via LocalAuthentication framework
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn platform_biometric_available() -> bool {
    use objc2_local_authentication::{LAContext, LAPolicy};

    let context = unsafe { LAContext::new() };
    unsafe {
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_ok()
    }
}

#[cfg(target_os = "macos")]
fn platform_try_biometric_unlock() -> Result<bool, String> {
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::mpsc;

    let context = unsafe { LAContext::new() };
    let reason = NSString::from_str("Unlock Smart Clipboard");

    let (tx, rx) = mpsc::channel();

    unsafe {
        context.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
            &reason,
            &block2::RcBlock::new(move |success: Bool, error: *mut NSError| {
                if success.as_bool() {
                    let _ = tx.send(Ok(true));
                } else if let Some(err) = error.as_ref() {
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

    rx.recv()
        .map_err(|e| format!("Biometric channel error: {e}"))?
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
    use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier};

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
