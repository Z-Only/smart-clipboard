use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardChange {
    pub content: String,
    pub content_type: String,
    pub source_app: Option<String>,
}
