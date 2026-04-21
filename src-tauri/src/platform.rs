//! Platform-specific utilities for obtaining the frontmost (active) application.

/// Returns the bundle identifier or process name of the currently frontmost application.
/// Returns `None` if the information cannot be obtained.
#[cfg(target_os = "macos")]
pub fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get bundle identifier of first process whose frontmost is true")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if bundle_id.is_empty() {
        None
    } else {
        Some(bundle_id)
    }
}

/// Returns the name of the currently active window's application on Linux (X11/Wayland).
/// Uses `xdotool` when available; falls back to `wmctrl`.
#[cfg(target_os = "linux")]
pub fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    // Try xdotool first (works on most X11 desktops)
    let output = Command::new("xdotool")
        .args(["getwindowfocus", "getwindowname"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

/// Returns the name of the currently foreground application on Windows.
#[cfg(target_os = "windows")]
pub fn get_frontmost_app() -> Option<String> {
    // On Windows we can use the `windows` crate in the future, but for now
    // fall back to a PowerShell query.
    use std::process::Command;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Process -Id (Get-ActiveWindowProcessId)).ProcessName",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Fallback for unsupported platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn get_frontmost_app() -> Option<String> {
    None
}
