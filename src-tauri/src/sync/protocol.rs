use serde::{Deserialize, Serialize};

use super::crypto::EncryptedPayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl From<EncryptedPayload> for EncryptedMessage {
    fn from(value: EncryptedPayload) -> Self {
        Self {
            nonce: value.nonce,
            ciphertext: value.ciphertext,
        }
    }
}

impl From<EncryptedMessage> for EncryptedPayload {
    fn from(value: EncryptedMessage) -> Self {
        Self {
            nonce: value.nonce,
            ciphertext: value.ciphertext,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SyncMessage {
    PairRequest {
        device_id: String,
        device_name: String,
        protocol_version: u32,
        port: u16,
        public_key: String,
    },
    PairResponse {
        device_id: String,
        device_name: String,
        accepted: bool,
        reason: Option<String>,
        protocol_version: u32,
        port: u16,
        public_key: Option<String>,
    },
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
    EncryptedPayload {
        message: EncryptedMessage,
    },
    SyncAck {
        entry_hash: String,
        accepted: bool,
    },
    KeyVerification {
        fingerprint: String,
        device_id: String,
        verified: bool,
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