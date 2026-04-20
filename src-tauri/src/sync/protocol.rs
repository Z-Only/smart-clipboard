use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SyncMessage {
    Hello {
        device_id: String,
        device_name: String,
        protocol_version: u32,
        port: u16,
    },
    HelloAck {
        device_id: String,
        accepted: bool,
        reason: Option<String>,
        protocol_version: u32,
    },
    Ping {
        ts: i64,
    },
    Pong {
        ts: i64,
    },
    Disconnect {
        reason: String,
    },
    ClipboardSyncPlaceholder {
        entry_hash: String,
        timestamp: i64,
    },
    SyncAck {
        entry_hash: String,
        accepted: bool,
    },
}

impl SyncMessage {
    pub fn to_text(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_text(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}
