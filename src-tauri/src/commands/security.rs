use std::sync::Arc;

use tauri::State;

use crate::security::{
    self, AppLockManager, AppLockStatus, SetPasswordPayload, UnlockPayload,
    UpdateAppLockSettingsPayload,
};

#[tauri::command]
pub async fn get_app_lock_status(
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    Ok(lock.status())
}

#[tauri::command]
pub async fn set_app_lock_password<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: SetPasswordPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.set_password(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn update_app_lock_settings<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UpdateAppLockSettingsPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.update_settings(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn lock_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    let status = lock.lock("manual");
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn unlock_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UnlockPayload,
) -> Result<AppLockStatus, String> {
    let prefer_biometric = payload.prefer_biometric.unwrap_or(false);
    if prefer_biometric && lock.status().biometric_enabled {
        match security::try_biometric_unlock() {
            Ok(true) => {
                let status = lock.mark_biometric_unlocked();
                security::emit_lock_state(&app, &lock);
                return Ok(status);
            }
            Ok(false) | Err(_) => {
                // Fall through to password unlock.
            }
        }
    }

    let password = payload
        .password
        .ok_or_else(|| "Password is required".to_string())?;
    match lock.verify_password(&password) {
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
