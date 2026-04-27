pub mod clipboard;
pub mod config;
pub mod security;
pub mod smart;
pub mod sync;
pub mod tags;
pub mod transform;
pub mod updater;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use tauri::State;

use crate::encryption::EncryptionManager;
use crate::security::AppLockManager;
use crate::storage::{ClipboardEntry, SearchResult};

pub(crate) fn require_unlocked(lock: &State<'_, Arc<AppLockManager>>) -> Result<(), String> {
    lock.ensure_unlocked()
}

pub(crate) fn decrypt_entries(encryption: &EncryptionManager, entries: &mut [ClipboardEntry]) {
    for entry in entries.iter_mut() {
        if let Ok(decrypted) = encryption.decrypt_content(&entry.content) {
            entry.content = decrypted;
        }
    }
}

pub(crate) fn decrypt_search_result(encryption: &EncryptionManager, result: &mut SearchResult) {
    decrypt_entries(encryption, &mut result.entries);
}

// Re-export all command functions so that `commands::xxx` paths remain valid.
pub use clipboard::{
    copy_entries, delete_entries, delete_entry, get_entries, get_entry_count, get_recent_entries,
    get_statistics, paste_entry, search_entries, set_favorite_state_for_entries, toggle_favorite,
};
pub use config::{
    get_autostart_enabled, get_config, quit_app, set_autostart_enabled, update_config,
};
pub use security::{
    get_app_lock_status, lock_app, set_app_lock_password, unlock_app, update_app_lock_settings,
};
pub use smart::{
    accept_tag_suggestion, dismiss_tag_suggestions, get_cluster_entries, get_clusters,
    get_related_entries, get_tag_suggestions, trigger_recluster,
};
pub use sync::{
    disable_encryption, enable_encryption, get_discovered_devices, get_encryption_status,
    get_paired_devices, get_sync_config, get_sync_status, pair_device, toggle_device_sync,
    unpair_device, update_sync_config, webdav_connect, webdav_disconnect, webdav_get_config,
    webdav_get_status, webdav_remove_device, webdav_trigger_sync, webdav_update_config,
};
pub use tags::{
    add_tag_to_entry, create_tag, delete_tag, get_all_tags, get_entries_by_tag, get_entry_tags,
    remove_tag_from_entry, set_tags_for_entries,
};
pub use transform::transform_content;
pub use updater::{
    check_for_updates_now, discard_pending_update, download_available_update, get_updater_status,
    install_pending_update,
};
