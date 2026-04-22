use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::security::{enforce_window_access, AppLockManager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn setup_hotkey<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut: Shortcut = "CommandOrControl+Shift+V".parse()?;

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_window(app_handle);
            }
        })?;

    Ok(())
}

pub fn toggle_window<R: Runtime>(app_handle: &AppHandle<R>) {
    let Some(window) = app_handle.get_webview_window("main") else {
        return;
    };
    handle_toggle_request(app_handle, window.is_visible().unwrap_or(false));
}

pub(crate) fn handle_toggle_request<R: Runtime>(app_handle: &AppHandle<R>, window_visible: bool) {
    let Some(lock_manager) = app_handle.try_state::<std::sync::Arc<AppLockManager>>() else {
        if let Some(window) = app_handle.get_webview_window("main") {
            if window_visible {
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
        if window_visible {
            let _ = window.hide();
            lock_manager.record_activity();
        } else {
            enforce_window_access(app_handle, &lock_manager, "shortcut");
        }
    }
}

#[cfg(test)]
mod wakeup_tests {
    use super::{handle_toggle_request, toggle_window};
    use crate::config::ConfigManager;
    use crate::security::{self, AppLockManager, AppLockStatus, SetPasswordPayload};
    use crate::tray::handle_tray_menu_event;
    use serde::Deserialize;
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, SyncSender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tauri::test::{self, MockRuntime};
    use tauri::{App, Listener, Manager, WebviewWindow, WebviewWindowBuilder};

    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should move forward")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "smart-clipboard-wakeup-tests-{}-{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("failed to create temp test dir");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct TestKeyringGuard;

    impl TestKeyringGuard {
        fn new() -> Self {
            security::install_test_keyring_store();
            security::set_test_biometric_result(None);
            Self
        }
    }

    impl Drop for TestKeyringGuard {
        fn drop(&mut self) {
            security::set_test_biometric_result(None);
            security::reset_test_keyring_store();
        }
    }

    #[derive(Debug, Deserialize)]
    struct LockStatusEventPayload {
        status: AppLockStatus,
    }

    struct TestHarness {
        _app: App<MockRuntime>,
        window: WebviewWindow<MockRuntime>,
        lock: Arc<AppLockManager>,
        window_shown_rx: Receiver<()>,
        open_settings_rx: Receiver<()>,
        app_lock_status_rx: Receiver<AppLockStatus>,
    }

    impl TestHarness {
        fn new(base_dir: PathBuf) -> Self {
            let config = Arc::new(ConfigManager::new(base_dir));
            let lock = Arc::new(AppLockManager::new(config));

            let app = test::mock_builder()
                .manage(lock.clone())
                .build(test::mock_context(test::noop_assets()))
                .expect("failed to build mock app");

            let window = WebviewWindowBuilder::new(&app, "main", Default::default())
                .build()
                .expect("failed to build mock webview");

            let (window_shown_tx, window_shown_rx) = std::sync::mpsc::sync_channel(8);
            let (open_settings_tx, open_settings_rx) = std::sync::mpsc::sync_channel(8);
            let (app_lock_tx, app_lock_status_rx) = std::sync::mpsc::sync_channel(8);

            app.listen_any("window-shown", move |_| {
                let _ = send_unit(&window_shown_tx);
            });
            app.listen_any("open-settings", move |_| {
                let _ = send_unit(&open_settings_tx);
            });
            app.listen_any("app-lock-status", move |event| {
                let payload: LockStatusEventPayload =
                    serde_json::from_str(event.payload()).expect("lock payload should be json");
                let _ = app_lock_tx.send(payload.status);
            });

            Self {
                _app: app,
                window,
                lock,
                window_shown_rx,
                open_settings_rx,
                app_lock_status_rx,
            }
        }

        fn configure_password(&self) {
            self.lock
                .set_password(SetPasswordPayload {
                    current_password: None,
                    new_password: "phase4-pass".to_string(),
                })
                .expect("setting password should succeed");
        }

        fn app_handle(&self) -> tauri::AppHandle<MockRuntime> {
            self.window.app_handle().clone()
        }
    }

    fn send_unit(sender: &SyncSender<()>) -> Result<(), ()> {
        sender.send(()).map_err(|_| ())
    }

    fn recv_unit(rx: &Receiver<()>) {
        rx.recv_timeout(Duration::from_millis(200))
            .expect("expected event to be emitted");
    }

    fn recv_lock_status(rx: &Receiver<AppLockStatus>) -> AppLockStatus {
        rx.recv_timeout(Duration::from_millis(200))
            .expect("expected app lock status event")
    }

    fn assert_no_unit(rx: &Receiver<()>) {
        assert!(
            rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "did not expect event"
        );
    }

    #[test]
    fn hotkey_shows_window_and_emits_window_shown_when_unlocked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        handle_toggle_request(&harness.app_handle(), false);

        recv_unit(&harness.window_shown_rx);
        assert_no_unit(&harness.window_shown_rx);
    }

    #[test]
    fn hotkey_emits_lock_status_when_locked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.lock.lock("manual");
        handle_toggle_request(&harness.app_handle(), false);

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("shortcut"));
        assert_no_unit(&harness.window_shown_rx);
    }

    #[test]
    fn hotkey_hides_visible_window() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        toggle_window(&harness.app_handle());

        assert_no_unit(&harness.window_shown_rx);
    }

    #[test]
    fn tray_show_menu_emits_window_shown_when_unlocked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        handle_tray_menu_event(&harness.app_handle(), "show");

        recv_unit(&harness.window_shown_rx);
    }

    #[test]
    fn tray_show_menu_emits_lock_status_when_locked() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());
        harness.configure_password();
        harness.lock.lock("manual");
        handle_tray_menu_event(&harness.app_handle(), "show");

        let status = recv_lock_status(&harness.app_lock_status_rx);
        assert!(status.locked);
        assert_eq!(status.unlock_reason.as_deref(), Some("tray_menu"));
        assert_no_unit(&harness.window_shown_rx);
    }

    #[test]
    fn tray_settings_menu_emits_open_settings_event() {
        let _serial = TEST_SERIAL.lock().unwrap();
        let _keyring = TestKeyringGuard::new();
        let temp_dir = TestDir::new();
        let harness = TestHarness::new(temp_dir.path.clone());

        handle_tray_menu_event(&harness.app_handle(), "settings");

        recv_unit(&harness.open_settings_rx);
        assert_no_unit(&harness.open_settings_rx);
        assert_no_unit(&harness.window_shown_rx);
    }
}
