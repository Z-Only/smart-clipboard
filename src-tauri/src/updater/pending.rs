use std::fs;
use std::path::{Path, PathBuf};

use crate::updater::types::PendingUpdateRecord;

pub fn pending_state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("updates").join("pending.json")
}

pub fn write_pending_update(
    app_data_dir: &Path,
    record: &PendingUpdateRecord,
) -> Result<(), String> {
    let path = pending_state_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn read_pending_update(app_data_dir: &Path) -> Result<Option<PendingUpdateRecord>, String> {
    let path = pending_state_path(app_data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let record = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(Some(record))
}

pub fn clear_pending_update(app_data_dir: &Path) -> Result<(), String> {
    let path = pending_state_path(app_data_dir);
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        clear_pending_update, pending_state_path, read_pending_update, write_pending_update,
    };
    use crate::updater::types::PendingUpdateRecord;

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
    fn pending_state_round_trips_and_clears() {
        let dir = tempdir().unwrap();
        let record = sample_record();
        write_pending_update(dir.path(), &record).unwrap();

        let path = pending_state_path(dir.path());
        assert!(path.exists());
        let restored = read_pending_update(dir.path()).unwrap();
        assert_eq!(restored, Some(record.clone()));

        clear_pending_update(dir.path()).unwrap();
        assert_eq!(read_pending_update(dir.path()).unwrap(), None);
        assert!(!path.exists());
    }

    #[test]
    fn read_pending_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let pending_dir = dir.path().join("updates").join("pending");
        fs::create_dir_all(&pending_dir).unwrap();
        assert_eq!(read_pending_update(dir.path()).unwrap(), None);
    }
}
