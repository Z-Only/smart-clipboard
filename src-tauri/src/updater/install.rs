use std::path::{Path, PathBuf};
use std::process::Command;

pub fn validate_pending_artifact_paths(
    artifact_path: &str,
    signature_path: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let artifact = PathBuf::from(artifact_path);
    let signature = PathBuf::from(signature_path);
    if !artifact.exists() {
        return Err("Pending update artifact file is missing".to_string());
    }
    if !signature.exists() {
        return Err("Pending update signature file is missing".to_string());
    }
    Ok((artifact, signature))
}

fn is_supported_installer_artifact(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    name.ends_with(".app.tar.gz")
        || name.ends_with(".tar.gz")
        || name.ends_with(".zip")
        || name.ends_with(".msi")
        || name.ends_with(".exe")
        || name.ends_with(".AppImage")
        || name.ends_with(".deb")
        || name.ends_with(".rpm")
}

pub fn perform_install_handoff(artifact_path: &Path, signature_path: &Path) -> Result<(), String> {
    let _ = signature_path;
    if !is_supported_installer_artifact(artifact_path) {
        return Err(format!(
            "Unsupported installer artifact: {}",
            artifact_path.display()
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(artifact_path)
            .status()
            .map_err(|e| format!("Failed to launch installer via open: {e}"))?;
        if !status.success() {
            return Err(format!("Installer launch failed with status: {status}"));
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", &artifact_path.to_string_lossy()])
            .status()
            .map_err(|e| format!("Failed to launch installer via start: {e}"))?;
        if !status.success() {
            return Err(format!("Installer launch failed with status: {status}"));
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("xdg-open")
            .arg(artifact_path)
            .status()
            .map_err(|e| format!("Failed to launch installer via xdg-open: {e}"))?;
        if !status.success() {
            return Err(format!("Installer launch failed with status: {status}"));
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Unsupported platform for installer handoff".to_string())
}
