use std::fs;
use std::path::{Path, PathBuf};

pub fn pending_version_dir(app_data_dir: &Path, version: &str) -> PathBuf {
    app_data_dir.join("updates").join("pending").join(version)
}

pub fn write_downloaded_artifact(
    app_data_dir: &Path,
    version: &str,
    asset_url: &str,
    bytes: &[u8],
    signature: &str,
) -> Result<(String, String), String> {
    let dir = pending_version_dir(app_data_dir, version);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let file_name = asset_url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("update.bin");
    let artifact_path = dir.join(file_name);
    let signature_path = dir.join(format!("{file_name}.sig"));

    fs::write(&artifact_path, bytes).map_err(|e| e.to_string())?;
    fs::write(&signature_path, signature.as_bytes()).map_err(|e| e.to_string())?;

    Ok((
        artifact_path.to_string_lossy().to_string(),
        signature_path.to_string_lossy().to_string(),
    ))
}

pub fn clear_pending_version_dir(app_data_dir: &Path, version: &str) -> Result<(), String> {
    let dir = pending_version_dir(app_data_dir, version);
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
