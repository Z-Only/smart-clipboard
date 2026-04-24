use std::sync::Mutex;

use semver::Version;
use tauri::{AppHandle, Emitter};

pub mod download;
pub mod http;
pub mod install;
pub mod manifest;
pub mod mirrors;
pub mod pending;
pub mod policy;
pub mod types;
pub mod verify;

pub use types::{PendingUpdateRecord, UpdaterPhase, UpdaterStatus};

pub const CANONICAL_MANIFEST_URL: &str =
    "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json";

pub fn current_target() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "darwin-aarch64";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "darwin-x86_64";
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "windows-x86_64";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "linux-x86_64";
    }
    #[allow(unreachable_code)]
    "unknown"
}

pub fn resolve_updater_public_key(config_value: Option<&str>) -> Option<String> {
    if let Ok(value) = std::env::var("SMART_CLIPBOARD_UPDATER_PUBLIC_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    config_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub struct UpdaterManager {
    status: Mutex<UpdaterStatus>,
}

impl UpdaterManager {
    pub fn new(current_version: String) -> Self {
        Self {
            status: Mutex::new(UpdaterStatus::idle(current_version)),
        }
    }

    pub fn get_status(&self) -> UpdaterStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn set_status(&self, status: UpdaterStatus) {
        *self.status.lock().unwrap() = status;
    }

    pub fn restore_from_disk(
        &self,
        app_data_dir: &std::path::Path,
        current_version: &str,
    ) -> Result<(), String> {
        let mut status = UpdaterStatus::idle(current_version.to_string());
        if let Some(record) = pending::read_pending_update(app_data_dir)? {
            status.phase = UpdaterPhase::ReadyToInstall;
            status.available_version = Some(record.version.clone());
            status.pending_update = Some(record);
        }
        self.set_status(status);
        Ok(())
    }

    pub fn discard_pending(&self, app_data_dir: &std::path::Path) -> Result<UpdaterStatus, String> {
        pending::clear_pending_update(app_data_dir)?;
        let current_version = self.get_status().current_version;
        let mut status = UpdaterStatus::idle(current_version);
        status.last_check_silent = false;
        self.set_status(status.clone());
        Ok(status)
    }

    pub fn check_now(
        &self,
        _config: &crate::config::UpdaterConfig,
        silent: bool,
    ) -> Result<UpdaterStatus, String> {
        let mut status = self.get_status();
        status.phase = UpdaterPhase::Checking;
        status.last_error = None;
        status.last_check_silent = silent;

        if status.pending_update.is_some() {
            status.phase = UpdaterPhase::ReadyToInstall;
            self.set_status(status.clone());
            return Ok(status);
        }

        status.phase = UpdaterPhase::UpToDate;
        self.set_status(status.clone());
        Ok(status)
    }

    pub fn check_now_with_manifest(
        &self,
        _config: &crate::config::UpdaterConfig,
        silent: bool,
        manifest_json: &str,
        target: &str,
        _manifest_url: &str,
    ) -> Result<UpdaterStatus, String> {
        let manifest = manifest::RemoteManifest::parse(manifest_json)?;
        let mut status = self.get_status();
        status.phase = UpdaterPhase::Checking;
        status.last_error = None;
        status.last_check_silent = silent;

        if status.pending_update.is_some() {
            status.phase = UpdaterPhase::ReadyToInstall;
            self.set_status(status.clone());
            return Ok(status);
        }

        let current = Version::parse(&status.current_version).map_err(|e| e.to_string())?;
        let remote = Version::parse(&manifest.version).map_err(|e| e.to_string())?;

        if manifest.platform(target).is_none() || remote <= current {
            status.phase = UpdaterPhase::UpToDate;
            status.available_version = None;
            status.available_notes = None;
            status.available_release_date = None;
            self.set_status(status.clone());
            return Ok(status);
        }

        status.phase = UpdaterPhase::UpdateAvailable;
        status.available_version = Some(manifest.version.clone());
        status.available_notes = manifest.notes.clone();
        status.available_release_date = manifest.pub_date.clone();
        self.set_status(status.clone());
        Ok(status)
    }

    pub fn check_now_with_fetcher<F>(
        &self,
        config: &crate::config::UpdaterConfig,
        silent: bool,
        target: &str,
        mut fetcher: F,
    ) -> Result<UpdaterStatus, String>
    where
        F: FnMut(&str) -> Result<Option<String>, String>,
    {
        let candidates = mirrors::resolve_candidate_urls(CANONICAL_MANIFEST_URL, &config.mirrors);
        for url in candidates {
            if let Some(body) = fetcher(&url)? {
                return self.check_now_with_manifest(config, silent, &body, target, &url);
            }
        }
        self.check_now(config, false)
    }

    pub fn check_now_with_fallible_fetcher<F>(
        &self,
        config: &crate::config::UpdaterConfig,
        silent: bool,
        target: &str,
        mut fetcher: F,
    ) -> Result<UpdaterStatus, String>
    where
        F: FnMut(&str) -> Result<Option<String>, String>,
    {
        let candidates = mirrors::resolve_candidate_urls(CANONICAL_MANIFEST_URL, &config.mirrors);
        let mut last_error: Option<String> = None;
        for url in candidates {
            match fetcher(&url) {
                Ok(Some(body)) => {
                    return self.check_now_with_manifest(config, silent, &body, target, &url);
                }
                Ok(None) => continue,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            }
        }

        if let Some(err) = last_error {
            return Err(err);
        }

        self.check_now(config, false)
    }

    pub fn download_update_with_handlers<F>(
        &self,
        app_data_dir: &std::path::Path,
        config: &crate::config::UpdaterConfig,
        manifest_json: &str,
        target: &str,
        manifest_url: &str,
        downloader: F,
    ) -> Result<UpdaterStatus, String>
    where
        F: FnMut(&str) -> Result<Vec<u8>, String>,
    {
        self.download_update_with_handlers_and_progress_and_public_key(
            app_data_dir,
            config,
            None,
            manifest_json,
            target,
            manifest_url,
            downloader,
            verify::verify_downloaded_artifact_with_public_key,
            |_| {},
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn download_update_with_handlers_and_progress<F, V, P>(
        &self,
        app_data_dir: &std::path::Path,
        config: &crate::config::UpdaterConfig,
        manifest_json: &str,
        target: &str,
        manifest_url: &str,
        downloader: F,
        verifier: V,
        progress: P,
    ) -> Result<UpdaterStatus, String>
    where
        F: FnMut(&str) -> Result<Vec<u8>, String>,
        V: Fn(&[u8], &str) -> Result<(), String>,
        P: FnMut(f64),
    {
        self.download_update_with_handlers_and_progress_and_public_key(
            app_data_dir,
            config,
            None,
            manifest_json,
            target,
            manifest_url,
            downloader,
            move |bytes, signature, _public_key| verifier(bytes, signature),
            progress,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn download_update_with_handlers_and_progress_and_public_key<F, V, P>(
        &self,
        app_data_dir: &std::path::Path,
        _config: &crate::config::UpdaterConfig,
        updater_public_key: Option<&str>,
        manifest_json: &str,
        target: &str,
        manifest_url: &str,
        mut downloader: F,
        verifier: V,
        mut progress: P,
    ) -> Result<UpdaterStatus, String>
    where
        F: FnMut(&str) -> Result<Vec<u8>, String>,
        V: Fn(&[u8], &str, Option<&str>) -> Result<(), String>,
        P: FnMut(f64),
    {
        let manifest = manifest::RemoteManifest::parse(manifest_json)?;
        let platform = manifest
            .platform(target)
            .ok_or_else(|| format!("No manifest platform entry for {target}"))?;

        let mut status = self.get_status();
        status.phase = UpdaterPhase::Downloading;
        status.download_progress = Some(0.0);
        self.set_status(status);
        progress(0.0);

        let bytes = downloader(&platform.url)?;
        let (artifact_path, signature_path) = download::write_downloaded_artifact(
            app_data_dir,
            &manifest.version,
            &platform.url,
            &bytes,
            &platform.signature,
        )?;

        if let Err(err) = verifier(&bytes, &platform.signature, updater_public_key) {
            let _ = download::clear_pending_version_dir(app_data_dir, &manifest.version);
            let _ = pending::clear_pending_update(app_data_dir);
            return Err(err);
        }

        let pending = PendingUpdateRecord {
            version: manifest.version.clone(),
            release_date: manifest.pub_date.clone(),
            current_version: self.get_status().current_version,
            notes: manifest.notes.clone(),
            artifact_path,
            signature_path,
            canonical_asset_url: platform.url.clone(),
            source_asset_url: platform.url.clone(),
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };
        pending::write_pending_update(app_data_dir, &pending)?;

        let mut status = self.get_status();
        status.phase = UpdaterPhase::ReadyToInstall;
        status.available_version = Some(manifest.version.clone());
        status.available_notes = manifest.notes.clone();
        status.available_release_date = manifest.pub_date.clone();
        status.pending_update = Some(pending);
        status.download_progress = Some(1.0);
        self.set_status(status.clone());
        progress(1.0);
        let _ = manifest_url;
        Ok(status)
    }

    pub fn install_pending(&self) -> Result<UpdaterStatus, String> {
        let app_data_dir = std::path::Path::new("");
        self.install_pending_with_handler_and_cleanup(
            app_data_dir,
            install::perform_install_handoff,
        )
    }

    pub fn install_pending_with_handler<F>(&self, installer: F) -> Result<UpdaterStatus, String>
    where
        F: Fn(&std::path::Path, &std::path::Path) -> Result<(), String>,
    {
        let app_data_dir = std::path::Path::new("");
        self.install_pending_with_handler_and_cleanup(app_data_dir, installer)
    }

    pub fn install_pending_with_handler_and_cleanup<F>(
        &self,
        app_data_dir: &std::path::Path,
        installer: F,
    ) -> Result<UpdaterStatus, String>
    where
        F: Fn(&std::path::Path, &std::path::Path) -> Result<(), String>,
    {
        let mut status = self.get_status();
        let pending = status
            .pending_update
            .clone()
            .ok_or_else(|| "No pending update available".to_string())?;

        let (artifact_path, signature_path) = install::validate_pending_artifact_paths(
            &pending.artifact_path,
            &pending.signature_path,
        )?;

        installer(&artifact_path, &signature_path)?;

        if !app_data_dir.as_os_str().is_empty() {
            let _ = pending::clear_pending_update(app_data_dir);
        }
        status.pending_update = None;
        status.download_progress = None;
        status.phase = UpdaterPhase::Installing;
        self.set_status(status.clone());
        Ok(status)
    }

    pub fn emit_status<R: tauri::Runtime>(&self, app: &AppHandle<R>) {
        let _ = app.emit("updater-status-changed", self.get_status());
    }
}

#[cfg(test)]
mod updater_manager_tests {
    use tempfile::tempdir;

    use super::{pending, PendingUpdateRecord, UpdaterManager, UpdaterPhase};

    fn sample_record() -> PendingUpdateRecord {
        PendingUpdateRecord {
            version: "2.2.0".to_string(),
            release_date: Some("2026-04-23T10:30:00Z".to_string()),
            current_version: "2.1.0".to_string(),
            notes: Some("notes".to_string()),
            artifact_path: "/tmp/installer".to_string(),
            signature_path: "/tmp/installer.sig".to_string(),
            canonical_asset_url: "https://github.com/org/repo/releases/download/v2.2.0/app"
                .to_string(),
            source_asset_url:
                "https://mirror.example/https://github.com/org/repo/releases/download/v2.2.0/app"
                    .to_string(),
            downloaded_at: "2026-04-23T10:35:00Z".to_string(),
        }
    }

    #[test]
    fn runtime_public_key_resolver_prefers_environment_override() {
        std::env::set_var("SMART_CLIPBOARD_UPDATER_PUBLIC_KEY", "ENV-KEY");
        let value = super::resolve_updater_public_key(Some("CONFIG-KEY"));
        std::env::remove_var("SMART_CLIPBOARD_UPDATER_PUBLIC_KEY");
        assert_eq!(value.as_deref(), Some("ENV-KEY"));
    }

    #[test]
    fn runtime_public_key_resolver_falls_back_to_config_value() {
        std::env::remove_var("SMART_CLIPBOARD_UPDATER_PUBLIC_KEY");
        let value = super::resolve_updater_public_key(Some("CONFIG-KEY"));
        assert_eq!(value.as_deref(), Some("CONFIG-KEY"));
    }

    #[test]
    fn verifier_reports_missing_public_key_for_minisign_scheme() {
        let bytes = vec![1_u8, 2, 3, 4];
        let error = super::verify::verify_downloaded_artifact_with_public_key(
            &bytes,
            "minisign:abcdef",
            None,
        )
        .unwrap_err();
        assert!(error.contains("Missing updater public key"));
    }

    #[test]
    fn verifier_rejects_invalid_minisign_public_key() {
        let bytes = vec![1_u8, 2, 3, 4];
        let error = super::verify::verify_downloaded_artifact_with_public_key(
            &bytes,
            "minisign:abcdef",
            Some("NOT-A-VALID-KEY"),
        )
        .unwrap_err();
        assert!(error.contains("Invalid updater public key"));
    }

    #[test]
    fn verifier_rejects_invalid_minisign_signature_payload() {
        let bytes = vec![1_u8, 2, 3, 4];
        let validish_key = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
        let error = super::verify::verify_downloaded_artifact_with_public_key(
            &bytes,
            "minisign:abcdef",
            Some(validish_key),
        )
        .unwrap_err();
        assert!(error.contains("Invalid minisign signature"));
    }

    #[test]
    fn verifier_scheme_dispatches_sha256_signatures() {
        let bytes = vec![1_u8, 2, 3, 4];
        let signature = "sha256:9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a";
        assert!(super::verify::verify_downloaded_artifact(&bytes, signature).is_ok());
    }

    #[test]
    fn verifier_rejects_unknown_signature_schemes() {
        let bytes = vec![1_u8, 2, 3, 4];
        let error =
            super::verify::verify_downloaded_artifact(&bytes, "unknown:abcdef").unwrap_err();
        assert!(error.contains("Unsupported signature scheme"));
    }

    #[test]
    fn restore_from_disk_promotes_pending_update_to_ready_to_install() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let record = sample_record();
        pending::write_pending_update(dir.path(), &record).unwrap();

        manager.restore_from_disk(dir.path(), "2.1.0").unwrap();

        let status = manager.get_status();
        assert_eq!(status.phase, UpdaterPhase::ReadyToInstall);
        assert_eq!(status.pending_update, Some(record));
    }

    #[test]
    fn install_handoff_rejects_unknown_artifact_extensions() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("app.unknown");
        let signature = dir.path().join("app.unknown.sig");
        std::fs::write(&artifact, b"artifact").unwrap();
        std::fs::write(&signature, b"signature").unwrap();

        let error =
            crate::updater::install::perform_install_handoff(&artifact, &signature).unwrap_err();
        assert!(error.contains("Unsupported installer artifact"));
    }

    #[test]
    fn install_pending_requires_existing_artifact_files() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        manager.set_status(crate::updater::UpdaterStatus {
            phase: crate::updater::UpdaterPhase::ReadyToInstall,
            current_version: "2.1.0".to_string(),
            available_version: Some("2.2.0".to_string()),
            available_notes: Some("notes".to_string()),
            available_release_date: Some("2026-04-23T10:30:00Z".to_string()),
            pending_update: Some(PendingUpdateRecord {
                version: "2.2.0".to_string(),
                release_date: Some("2026-04-23T10:30:00Z".to_string()),
                current_version: "2.1.0".to_string(),
                notes: Some("notes".to_string()),
                artifact_path: "/nonexistent/artifact".to_string(),
                signature_path: "/nonexistent/artifact.sig".to_string(),
                canonical_asset_url: "https://github.com/x".to_string(),
                source_asset_url: "https://mirror/x".to_string(),
                downloaded_at: "2026-04-23T10:35:00Z".to_string(),
            }),
            download_progress: Some(1.0),
            last_error: None,
            last_check_silent: false,
        });

        let error = manager
            .install_pending_with_handler(|_artifact, _signature| Ok(()))
            .unwrap_err();
        assert!(
            error.contains("artifact file is missing")
                || error.contains("signature file is missing")
        );
    }

    #[test]
    fn install_handoff_success_clears_pending_metadata_and_runtime_pending_record() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("app.tar.gz");
        let signature = dir.path().join("app.tar.gz.sig");
        std::fs::write(&artifact, b"artifact").unwrap();
        std::fs::write(&signature, b"signature").unwrap();

        let pending = PendingUpdateRecord {
            version: "2.2.0".to_string(),
            release_date: Some("2026-04-23T10:30:00Z".to_string()),
            current_version: "2.1.0".to_string(),
            notes: Some("notes".to_string()),
            artifact_path: artifact.to_string_lossy().to_string(),
            signature_path: signature.to_string_lossy().to_string(),
            canonical_asset_url: "https://github.com/x".to_string(),
            source_asset_url: "https://mirror/x".to_string(),
            downloaded_at: "2026-04-23T10:35:00Z".to_string(),
        };
        pending::write_pending_update(dir.path(), &pending).unwrap();

        let manager = UpdaterManager::new("2.1.0".to_string());
        manager.set_status(crate::updater::UpdaterStatus {
            phase: crate::updater::UpdaterPhase::ReadyToInstall,
            current_version: "2.1.0".to_string(),
            available_version: Some("2.2.0".to_string()),
            available_notes: Some("notes".to_string()),
            available_release_date: Some("2026-04-23T10:30:00Z".to_string()),
            pending_update: Some(pending),
            download_progress: Some(1.0),
            last_error: None,
            last_check_silent: false,
        });

        let status = manager
            .install_pending_with_handler_and_cleanup(
                dir.path(),
                |_artifact_path, _signature_path| Ok(()),
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::Installing);
        assert!(status.pending_update.is_none());
        assert_eq!(pending::read_pending_update(dir.path()).unwrap(), None);
    }

    #[test]
    fn install_pending_handoff_marks_installing_when_files_exist() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("app.tar.gz");
        let signature = dir.path().join("app.tar.gz.sig");
        std::fs::write(&artifact, b"artifact").unwrap();
        std::fs::write(&signature, b"signature").unwrap();

        let manager = UpdaterManager::new("2.1.0".to_string());
        manager.set_status(crate::updater::UpdaterStatus {
            phase: crate::updater::UpdaterPhase::ReadyToInstall,
            current_version: "2.1.0".to_string(),
            available_version: Some("2.2.0".to_string()),
            available_notes: Some("notes".to_string()),
            available_release_date: Some("2026-04-23T10:30:00Z".to_string()),
            pending_update: Some(PendingUpdateRecord {
                version: "2.2.0".to_string(),
                release_date: Some("2026-04-23T10:30:00Z".to_string()),
                current_version: "2.1.0".to_string(),
                notes: Some("notes".to_string()),
                artifact_path: artifact.to_string_lossy().to_string(),
                signature_path: signature.to_string_lossy().to_string(),
                canonical_asset_url: "https://github.com/x".to_string(),
                source_asset_url: "https://mirror/x".to_string(),
                downloaded_at: "2026-04-23T10:35:00Z".to_string(),
            }),
            download_progress: Some(1.0),
            last_error: None,
            last_check_silent: false,
        });

        let status = manager
            .install_pending_with_handler(
                |artifact_path: &std::path::Path, signature_path: &std::path::Path| {
                    assert!(artifact_path.exists());
                    assert!(signature_path.exists());
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(status.phase, UpdaterPhase::Installing);
        assert!(status.pending_update.is_none());
    }

    #[test]
    fn discard_pending_clears_persisted_pending_state() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        pending::write_pending_update(dir.path(), &sample_record()).unwrap();
        manager.restore_from_disk(dir.path(), "2.1.0").unwrap();

        let status = manager.discard_pending(dir.path()).unwrap();

        assert_eq!(status.phase, UpdaterPhase::Idle);
        assert!(status.pending_update.is_none());
        assert_eq!(pending::read_pending_update(dir.path()).unwrap(), None);
    }

    #[test]
    fn manager_uses_configured_public_key_for_minisign_verification_path() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig::default();
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "minisign:abcdef",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;

        let error = manager
            .download_update_with_handlers_and_progress_and_public_key(
                dir.path(),
                &config,
                Some("PUBLIC-KEY"),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_asset_url: &str| Ok(vec![1_u8, 2, 3, 4]),
                super::verify::verify_downloaded_artifact_with_public_key,
                |_progress: f64| {},
            )
            .unwrap_err();

        assert!(
            error.contains("Invalid updater public key")
                || error.contains("Invalid minisign signature")
                || error.contains("Minisign verification failed")
        );
    }

    #[test]
    fn invalid_signature_cleans_partial_pending_files() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "invalid-signature",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;

        let error = manager
            .download_update_with_handlers_and_progress(
                dir.path(),
                &crate::config::UpdaterConfig::default(),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_asset_url: &str| Ok(vec![1_u8, 2, 3]),
                |_bytes: &[u8], _signature: &str| Err("signature verification failed".to_string()),
                |_progress: f64| {},
            )
            .unwrap_err();

        assert!(error.contains("signature verification failed"));
        assert_eq!(pending::read_pending_update(dir.path()).unwrap(), None);
        let pending_dir = dir.path().join("updates").join("pending").join("2.2.0");
        assert!(!pending_dir.exists());
    }

    #[test]
    fn progress_hook_can_observe_intermediate_progress_updates() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sha256:9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;
        let mut seen = Vec::new();

        let _ = manager
            .download_update_with_handlers_and_progress(
                dir.path(),
                &crate::config::UpdaterConfig::default(),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_asset_url: &str| Ok(vec![1_u8, 2, 3, 4]),
                |_bytes: &[u8], _signature: &str| Ok(()),
                |progress: f64| {
                    seen.push(progress);
                },
            )
            .unwrap();

        assert!(seen.len() >= 2);
        assert!((seen[0] - 0.0).abs() < f64::EPSILON);
        assert!((seen[seen.len() - 1] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_callback_updates_status_while_downloading() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sha256:9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;
        let mut seen_statuses = Vec::new();

        let status = manager
            .download_update_with_handlers_and_progress(
                dir.path(),
                &crate::config::UpdaterConfig::default(),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_asset_url: &str| Ok(vec![1_u8, 2, 3, 4]),
                |_bytes: &[u8], _signature: &str| Ok(()),
                |progress: f64| {
                    let status = manager.get_status();
                    seen_statuses.push((status.phase.clone(), status.download_progress, progress));
                },
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::ReadyToInstall);
        assert!(seen_statuses.iter().any(|(phase, value, progress)| {
            *phase == UpdaterPhase::Downloading
                && *value == Some(0.0)
                && (*progress - 0.0).abs() < f64::EPSILON
        }));
        assert!(seen_statuses.iter().any(|(phase, value, progress)| {
            *phase == UpdaterPhase::ReadyToInstall
                && *value == Some(1.0)
                && (*progress - 1.0).abs() < f64::EPSILON
        }));
    }

    #[test]
    fn download_updates_progress_and_phase() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sha256:039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;
        let mut progress_events = Vec::new();

        let status = manager
            .download_update_with_handlers_and_progress(
                dir.path(),
                &crate::config::UpdaterConfig::default(),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_asset_url: &str| Ok(vec![1_u8, 2, 3, 4]),
                |_bytes: &[u8], _signature: &str| Ok(()),
                |progress: f64| progress_events.push(progress),
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::ReadyToInstall);
        assert!(progress_events
            .iter()
            .any(|value| (*value - 0.0).abs() < f64::EPSILON));
        assert!(progress_events
            .iter()
            .any(|value| (*value - 1.0).abs() < f64::EPSILON));
    }

    #[test]
    fn download_update_writes_pending_record_and_marks_ready() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig::default();
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sha256:039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;

        let status = manager
            .download_update_with_handlers(
                dir.path(),
                &config,
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_url: &str| Ok(vec![1_u8, 2, 3]),
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::ReadyToInstall);
        assert_eq!(status.available_version.as_deref(), Some("2.2.0"));
        let pending = status.pending_update.expect("pending update record");
        assert!(std::path::Path::new(&pending.artifact_path).exists());
        assert!(std::path::Path::new(&pending.signature_path).exists());
        let restored = pending::read_pending_update(dir.path())
            .unwrap()
            .expect("persisted pending");
        assert_eq!(restored.version, "2.2.0");
        assert_eq!(restored.notes.as_deref(), Some("Bug fixes"));
    }

    #[test]
    fn download_update_replaces_existing_pending_version() {
        let dir = tempdir().unwrap();
        let manager = UpdaterManager::new("2.1.0".to_string());
        let old = sample_record();
        pending::write_pending_update(dir.path(), &old).unwrap();
        manager.restore_from_disk(dir.path(), "2.1.0").unwrap();

        let manifest = r#"{
          "version": "2.3.0",
          "notes": "New release",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sha256:06df4f7e1394f1c57cc6583fba4d8060a5a66f4f4771c14aeff6b9af8a28c9b3",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.3.0/app.tar.gz"
            }
          }
        }"#;

        let status = manager
            .download_update_with_handlers(
                dir.path(),
                &crate::config::UpdaterConfig::default(),
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
                |_url: &str| Ok(vec![9_u8, 8, 7]),
            )
            .unwrap();

        let pending = status.pending_update.expect("pending update record");
        assert_eq!(pending.version, "2.3.0");
        let restored = pending::read_pending_update(dir.path())
            .unwrap()
            .expect("persisted pending");
        assert_eq!(restored.version, "2.3.0");
    }

    #[test]
    fn all_failed_manifest_sources_return_error_for_manual_checks() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig {
            mirrors: vec!["https://mirror-a/{url}".to_string()],
            ..crate::config::UpdaterConfig::default()
        };

        let error = manager
            .check_now_with_fallible_fetcher(&config, false, "darwin-aarch64", |_url: &str| {
                Err("all sources failed".to_string())
            })
            .unwrap_err();

        assert!(error.contains("all sources failed"));
    }

    #[test]
    fn http_fetch_errors_when_all_manifest_sources_fail() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig {
            mirrors: vec!["https://mirror-a/{url}".to_string()],
            ..crate::config::UpdaterConfig::default()
        };

        let error = manager
            .check_now_with_fetcher(&config, false, "darwin-aarch64", |_url: &str| {
                Err("network down".to_string())
            })
            .unwrap_err();

        assert!(error.contains("network down"));
    }

    #[test]
    fn selects_first_successful_manifest_source_in_mirror_order() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig {
            mirrors: vec![
                "https://mirror-a/{url}".to_string(),
                "https://mirror-b/{url}".to_string(),
            ],
            ..crate::config::UpdaterConfig::default()
        };
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sig",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;
        let mut seen = Vec::new();
        let status = manager
            .check_now_with_fetcher(&config, false, "darwin-aarch64", |url: &str| {
                seen.push(url.to_string());
                if url.starts_with("https://mirror-b/") {
                    Ok(Some(manifest.to_string()))
                } else {
                    Ok(None)
                }
            })
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::UpdateAvailable);
        assert_eq!(seen.len(), 2);
        assert!(seen[0].starts_with("https://mirror-a/"));
        assert!(seen[1].starts_with("https://mirror-b/"));
    }

    #[test]
    fn manual_check_detects_newer_manifest_version() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig::default();
        let manifest = r#"{
          "version": "2.2.0",
          "notes": "Bug fixes",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sig",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.2.0/app.tar.gz"
            }
          }
        }"#;

        let status = manager
            .check_now_with_manifest(
                &config,
                false,
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::UpdateAvailable);
        assert_eq!(status.available_version.as_deref(), Some("2.2.0"));
        assert!(status.pending_update.is_none());
    }

    #[test]
    fn manual_check_ignores_same_or_older_manifest_version() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig::default();
        let manifest = r#"{
          "version": "2.1.0",
          "notes": "No-op",
          "pub_date": "2026-04-23T10:30:00Z",
          "platforms": {
            "darwin-aarch64": {
              "signature": "sig",
              "url": "https://github.com/Z-Only/smart-clipboard/releases/download/v2.1.0/app.tar.gz"
            }
          }
        }"#;

        let status = manager
            .check_now_with_manifest(
                &config,
                false,
                manifest,
                "darwin-aarch64",
                "https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json",
            )
            .unwrap();

        assert_eq!(status.phase, UpdaterPhase::UpToDate);
        assert!(status.available_version.is_none());
    }

    #[test]
    fn manual_check_without_pending_sets_up_to_date() {
        let manager = UpdaterManager::new("2.1.0".to_string());
        let config = crate::config::UpdaterConfig::default();

        let status = manager.check_now(&config, false).unwrap();

        assert_eq!(status.phase, UpdaterPhase::UpToDate);
        assert!(!status.last_check_silent);
        assert!(status.pending_update.is_none());
    }
}
