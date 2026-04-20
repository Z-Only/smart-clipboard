use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardChange {
    pub content: String,
    pub content_type: String,
    pub source_app: Option<String>,
    pub image_data: Option<ImageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
}
