use tauri::{AppHandle, Emitter, Manager};

use crate::security::{enforce_window_access, AppLockManager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn setup_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut: Shortcut = "CommandOrControl+Shift+V".parse()?;

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_window(app_handle);
            }
        })?;

    Ok(())
}

pub fn toggle_window(app_handle: &AppHandle) {
    let Some(lock_manager) = app_handle.try_state::<std::sync::Arc<AppLockManager>>() else {
        if let Some(window) = app_handle.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = app_handle.emit("window-shown", ());
            }
        }
        return;
    };

    if let Some(window) = app_handle.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            lock_manager.record_activity();
        } else {
            enforce_window_access(app_handle, &lock_manager, "shortcut");
        }
    }
}
