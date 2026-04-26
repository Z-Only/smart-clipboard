use std::sync::Arc;

use tauri::State;

use crate::config::ConfigManager;
use crate::updater::UpdaterManager;
use crate::AppDataDir;

#[tauri::command]
pub async fn get_updater_status(
    updater: State<'_, Arc<UpdaterManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    Ok(updater.get_status())
}

#[tauri::command]
pub async fn check_for_updates_now<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let updater_config = config.get().updater;
    let target = crate::updater::current_target();
    let status =
        updater.check_now_with_fallible_fetcher(&updater_config, false, target, |url: &str| {
            let handle = tokio::runtime::Handle::current();
            match handle.block_on(crate::updater::http::fetch_text(url)) {
                Ok(body) => Ok(Some(body)),
                Err(err) => Err(err),
            }
        })?;
    updater.emit_status(&app);
    Ok(status)
}

#[tauri::command]
pub async fn download_available_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    config: State<'_, Arc<ConfigManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let updater_config = config.get().updater;
    let target = crate::updater::current_target();
    let status =
        updater.check_now_with_fallible_fetcher(&updater_config, false, target, |url: &str| {
            let handle = tokio::runtime::Handle::current();
            match handle.block_on(crate::updater::http::fetch_text(url)) {
                Ok(body) => Ok(Some(body)),
                Err(err) => Err(err),
            }
        })?;

    if status.phase != crate::updater::UpdaterPhase::UpdateAvailable {
        updater.emit_status(&app);
        return Ok(status);
    }

    let manifest_body = crate::updater::http::fetch_text(crate::updater::CANONICAL_MANIFEST_URL)
        .await
        .map_err(|e| format!("Failed to refetch manifest: {e}"))?;
    let app_handle = app.clone();
    let download_status = updater.download_update_with_handlers_and_progress(
        &app_data_dir.0,
        &updater_config,
        &manifest_body,
        target,
        crate::updater::CANONICAL_MANIFEST_URL,
        |asset_url: &str| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(crate::updater::http::fetch_bytes_with_progress(
                asset_url,
                |_| {},
            ))
        },
        crate::updater::verify::verify_downloaded_artifact,
        |progress: f64| {
            let mut status = updater.get_status();
            status.phase = if progress >= 1.0 {
                crate::updater::UpdaterPhase::ReadyToInstall
            } else {
                crate::updater::UpdaterPhase::Downloading
            };
            status.download_progress = Some(progress);
            updater.set_status(status);
            updater.emit_status(&app_handle);
        },
    )?;
    updater.emit_status(&app);
    Ok(download_status)
}

#[tauri::command]
pub async fn install_pending_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let status = updater.install_pending()?;
    updater.emit_status(&app);
    Ok(status)
}

#[tauri::command]
pub async fn discard_pending_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    updater: State<'_, Arc<UpdaterManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<crate::updater::UpdaterStatus, String> {
    let status = updater.discard_pending(&app_data_dir.0)?;
    updater.emit_status(&app);
    Ok(status)
}
