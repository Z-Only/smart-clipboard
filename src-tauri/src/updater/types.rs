use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdaterPhase {
    Idle,
    Checking,
    UpdateAvailable,
    Downloading,
    ReadyToInstall,
    UpToDate,
    Installing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpdateRecord {
    pub version: String,
    pub release_date: Option<String>,
    pub current_version: String,
    pub notes: Option<String>,
    pub artifact_path: String,
    pub signature_path: String,
    pub canonical_asset_url: String,
    pub source_asset_url: String,
    pub downloaded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub phase: UpdaterPhase,
    pub current_version: String,
    pub available_version: Option<String>,
    pub available_notes: Option<String>,
    pub available_release_date: Option<String>,
    pub pending_update: Option<PendingUpdateRecord>,
    pub download_progress: Option<f64>,
    pub last_error: Option<String>,
    pub last_check_silent: bool,
}

impl UpdaterStatus {
    pub fn idle(current_version: String) -> Self {
        Self {
            phase: UpdaterPhase::Idle,
            current_version,
            available_version: None,
            available_notes: None,
            available_release_date: None,
            pending_update: None,
            download_progress: None,
            last_error: None,
            last_check_silent: false,
        }
    }
}
