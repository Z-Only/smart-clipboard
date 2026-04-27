use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: Option<i64>,
    pub name: String,
    pub content: String,
    pub category: String,
    pub is_favorite: bool,
    pub use_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub total_entries: i64,
    pub total_favorites: i64,
    pub entries_by_category: Vec<CategoryCount>,
    pub entries_by_day: Vec<DayCount>,
    pub most_used: Vec<ClipboardEntry>,
    pub storage_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: Option<i64>,
    pub content: String,
    pub content_type: String,
    pub category: String,
    pub hash: String,
    pub source_app: Option<String>,
    pub is_favorite: bool,
    pub is_sensitive: bool,
    pub use_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub source_device: Option<String>, // device_id of origin, None = local
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entries: Vec<ClipboardEntry>,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub is_favorite: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub host: String,
    pub address: String,
    pub ip: String,
    pub port: i64,
    pub version: String,
    pub status: String,
    pub last_seen_at: NaiveDateTime,
    pub is_paired: bool,
    pub enabled: bool,
    pub sync_enabled: bool,
    pub paired_at: Option<NaiveDateTime>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub host: String,
    pub address: String,
    pub ip: String,
    pub port: i64,
    pub status: String,
    pub public_key: Option<Vec<u8>>,
    pub local_public_key: Option<Vec<u8>>,
    pub shared_secret: Option<Vec<u8>>,
    pub last_seen_at: Option<NaiveDateTime>,
    pub is_active: bool,
    pub enabled: bool,
    pub sync_enabled: bool,
    pub paired_at: NaiveDateTime,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    pub state: String,
    pub paired_count: i64,
    pub online_count: i64,
    pub last_sync_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLogEntry {
    pub id: Option<i64>,
    pub entry_hash: String,
    pub device_id: String,
    pub direction: String,
    pub synced_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub id: i64,
    pub label: String,
    pub entry_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub cluster_id: i64,
    pub entry_id: i64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSuggestion {
    pub entry_id: i64,
    pub tag: Tag,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntry {
    pub entry: ClipboardEntry,
    pub score: f64,
}
