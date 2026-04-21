use serde::{Deserialize, Serialize};

use super::crypto::EncryptedPayload;

/// Payload for clipboard entry synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEntryPayload {
    pub content: String,
    pub content_type: String,
    pub category: String,
    pub hash: String,
    pub source_app: Option<String>,
    pub is_sensitive: bool,
    pub source_device: String,
    pub created_at: String,
}

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
    ClipboardSync {
        entry: SyncEntryPayload,
        sender_device_id: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_message_roundtrip() {
        let msg = SyncMessage::Ping { ts: 1234567890 };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::Ping { ts } => assert_eq!(ts, 1234567890),
            _ => panic!("Expected Ping message"),
        }
    }

    #[test]
    fn test_pong_message_roundtrip() {
        let msg = SyncMessage::Pong { ts: 1234567890 };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::Pong { ts } => assert_eq!(ts, 1234567890),
            _ => panic!("Expected Pong message"),
        }
    }

    #[test]
    fn test_pair_request_roundtrip() {
        let msg = SyncMessage::PairRequest {
            device_id: "device-123".to_string(),
            device_name: "My Device".to_string(),
            protocol_version: 1,
            port: 8080,
            public_key: "pubkey-base64".to_string(),
        };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::PairRequest {
                device_id,
                device_name,
                protocol_version,
                port,
                public_key,
            } => {
                assert_eq!(device_id, "device-123");
                assert_eq!(device_name, "My Device");
                assert_eq!(protocol_version, 1);
                assert_eq!(port, 8080);
                assert_eq!(public_key, "pubkey-base64");
            }
            _ => panic!("Expected PairRequest message"),
        }
    }

    #[test]
    fn test_clipboard_sync_roundtrip() {
        let entry = SyncEntryPayload {
            content: "Hello World".to_string(),
            content_type: "text/plain".to_string(),
            category: "text".to_string(),
            hash: "abc123".to_string(),
            source_app: Some("VSCode".to_string()),
            is_sensitive: false,
            source_device: "device-456".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let msg = SyncMessage::ClipboardSync {
            entry: entry.clone(),
            sender_device_id: "device-789".to_string(),
            timestamp: 1234567890,
        };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::ClipboardSync {
                entry,
                sender_device_id,
                timestamp,
            } => {
                assert_eq!(entry.content, "Hello World");
                assert_eq!(entry.content_type, "text/plain");
                assert_eq!(entry.category, "text");
                assert_eq!(entry.hash, "abc123");
                assert_eq!(entry.source_app, Some("VSCode".to_string()));
                assert!(!entry.is_sensitive);
                assert_eq!(entry.source_device, "device-456");
                assert_eq!(entry.created_at, "2024-01-01T00:00:00Z");
                assert_eq!(sender_device_id, "device-789");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Expected ClipboardSync message"),
        }
    }

    #[test]
    fn test_sync_entry_payload_serialization() {
        let payload = SyncEntryPayload {
            content: "Test content".to_string(),
            content_type: "text/plain".to_string(),
            category: "text".to_string(),
            hash: "hash123".to_string(),
            source_app: None,
            is_sensitive: true,
            source_device: "dev-001".to_string(),
            created_at: "2024-06-15T12:30:00Z".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: SyncEntryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Test content");
        assert_eq!(deserialized.content_type, "text/plain");
        assert_eq!(deserialized.category, "text");
        assert_eq!(deserialized.hash, "hash123");
        assert_eq!(deserialized.source_app, None);
        assert!(deserialized.is_sensitive);
        assert_eq!(deserialized.source_device, "dev-001");
        assert_eq!(deserialized.created_at, "2024-06-15T12:30:00Z");
    }

    #[test]
    fn test_disconnect_message_roundtrip() {
        let msg = SyncMessage::Disconnect {
            reason: "User initiated".to_string(),
        };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::Disconnect { reason } => {
                assert_eq!(reason, "User initiated");
            }
            _ => panic!("Expected Disconnect message"),
        }
    }

    #[test]
    fn test_hello_message_roundtrip() {
        let msg = SyncMessage::Hello {
            device_id: "hello-device".to_string(),
            device_name: "Hello Device".to_string(),
            protocol_version: 2,
            port: 9090,
        };
        let text = msg.to_text().unwrap();
        let parsed: SyncMessage = SyncMessage::from_text(&text).unwrap();
        match parsed {
            SyncMessage::Hello {
                device_id,
                device_name,
                protocol_version,
                port,
            } => {
                assert_eq!(device_id, "hello-device");
                assert_eq!(device_name, "Hello Device");
                assert_eq!(protocol_version, 2);
                assert_eq!(port, 9090);
            }
            _ => panic!("Expected Hello message"),
        }
    }
}
