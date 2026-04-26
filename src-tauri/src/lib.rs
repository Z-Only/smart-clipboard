pub mod analyzer;
mod app_setup;
pub mod biometric;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod encryption;
pub mod hotkey;
mod monitor;
pub mod platform;
pub mod plugins;
pub mod security;
pub mod storage;
pub mod sync;
pub mod templates;
pub mod tray;
pub mod updater;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod runtime_tests;

use std::path::PathBuf;

use tauri::{AppHandle, Runtime, WebviewWindow};

use security::AppLockManager;

/// Managed state holding the app data directory path.
pub struct AppDataDir(pub PathBuf);

pub(crate) fn emit_initial_lock_state<R: Runtime>(
    app_handle: &AppHandle<R>,
    lock_manager: &AppLockManager,
) {
    security::emit_lock_state(app_handle, lock_manager);
}

pub(crate) fn handle_main_window_focus<R: Runtime>(
    app_handle: &AppHandle<R>,
    lock_manager: &AppLockManager,
    focused: bool,
) {
    if focused {
        security::emit_lock_state(app_handle, lock_manager);
    }
}

pub(crate) fn handle_main_window_close<R: Runtime>(
    window: &WebviewWindow<R>,
    lock_manager: &AppLockManager,
) {
    let _ = window.hide();
    lock_manager.record_activity();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            // clipboard commands
            commands::clipboard::get_entries,
            commands::clipboard::search_entries,
            commands::clipboard::delete_entry,
            commands::clipboard::delete_entries,
            commands::clipboard::copy_entries,
            commands::clipboard::set_favorite_state_for_entries,
            commands::clipboard::toggle_favorite,
            commands::clipboard::get_entry_count,
            commands::clipboard::get_statistics,
            commands::clipboard::paste_entry,
            // updater commands
            commands::updater::get_updater_status,
            commands::updater::check_for_updates_now,
            commands::updater::download_available_update,
            commands::updater::install_pending_update,
            commands::updater::discard_pending_update,
            // config commands
            commands::config::quit_app,
            commands::config::get_config,
            commands::config::update_config,
            commands::config::get_autostart_enabled,
            commands::config::set_autostart_enabled,
            // transform commands
            commands::transform::transform_content,
            // plugin commands
            plugins::commands::list_plugins,
            plugins::commands::set_plugin_enabled,
            plugins::commands::list_plugin_transforms,
            plugins::commands::apply_plugin_transform,
            // tag commands
            commands::tags::create_tag,
            commands::tags::delete_tag,
            commands::tags::get_all_tags,
            commands::tags::add_tag_to_entry,
            commands::tags::remove_tag_from_entry,
            commands::tags::get_entry_tags,
            commands::tags::set_tags_for_entries,
            commands::tags::get_entries_by_tag,
            // sync commands
            commands::sync::get_sync_status,
            commands::sync::get_sync_config,
            commands::sync::update_sync_config,
            commands::sync::get_discovered_devices,
            commands::sync::get_paired_devices,
            commands::sync::pair_device,
            commands::sync::unpair_device,
            commands::sync::toggle_device_sync,
            // template commands
            templates::commands::create_template,
            templates::commands::update_template,
            templates::commands::delete_template,
            templates::commands::get_templates,
            templates::commands::get_template,
            templates::commands::use_template,
            templates::commands::get_template_categories,
            templates::commands::get_template_placeholders,
            // webdav commands
            commands::sync::webdav_connect,
            commands::sync::webdav_disconnect,
            commands::sync::webdav_get_status,
            commands::sync::webdav_get_config,
            commands::sync::webdav_update_config,
            commands::sync::webdav_trigger_sync,
            commands::sync::webdav_remove_device,
            // security commands
            commands::security::get_app_lock_status,
            commands::security::set_app_lock_password,
            commands::security::update_app_lock_settings,
            commands::security::lock_app,
            commands::security::unlock_app,
            // encryption commands
            commands::sync::get_encryption_status,
            commands::sync::enable_encryption,
            commands::sync::disable_encryption,
        ])
        .setup(|app| app_setup::setup_app(app))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
