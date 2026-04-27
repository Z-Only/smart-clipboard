use std::path::Path;
use std::sync::Mutex;

use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection, Result};

use super::migrations;
use super::models::{
    CategoryCount, ClipboardEntry, ClusterSummary, DayCount, PairedDevice, SearchQuery,
    SearchResult, Statistics, Tag, TagSuggestion, Template,
};
use super::search_pinyin::{build_fts_match_expr, build_pinyin_fields, normalize_search_keyword};

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = if db_path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            if let Some(parent) = std::path::Path::new(db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            Connection::open(db_path)?
        };
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<i64> {
        self.insert_entry_with_encrypted_flag(entry, false)
    }

    pub fn insert_entry_with_encrypted_flag(
        &self,
        entry: &ClipboardEntry,
        is_encrypted: bool,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let (pinyin_full, pinyin_initials) = build_pinyin_fields(&entry.content);
        conn.execute(
            "INSERT INTO clipboard_entries (content, content_type, category, hash, source_app, is_favorite, is_sensitive, pinyin_full, pinyin_initials, use_count, created_at, updated_at, expires_at, source_device, is_encrypted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                entry.content,
                entry.content_type,
                entry.category,
                entry.hash,
                entry.source_app,
                entry.is_favorite as i32,
                entry.is_sensitive as i32,
                pinyin_full,
                pinyin_initials,
                entry.use_count,
                entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                entry.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                entry.expires_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                entry.source_device,
                is_encrypted as i32,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Option<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at, source_device
             FROM clipboard_entries WHERE hash = ?1",
        )?;
        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_entry(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_hashes(&self) -> Result<std::collections::HashSet<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT hash FROM clipboard_entries")?;
        let hashes = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(hashes)
    }

    pub fn update_use_count(&self, hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().naive_local();
        conn.execute(
            "UPDATE clipboard_entries SET use_count = use_count + 1, updated_at = ?1, created_at = ?1 WHERE hash = ?2",
            params![now.format("%Y-%m-%d %H:%M:%S").to_string(), hash],
        )?;
        Ok(())
    }

    pub fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let conn = self.conn.lock().unwrap();

        if let Some(ref keyword) = query.keyword {
            if !keyword.trim().is_empty() {
                return search_fts(&conn, keyword, query);
            }
        }

        get_entries_inner(&conn, query)
    }

    pub fn get_entries(
        &self,
        limit: i64,
        offset: i64,
        category: Option<&str>,
        is_favorite: Option<bool>,
    ) -> Result<SearchResult> {
        let query = SearchQuery {
            keyword: None,
            category: category.map(|s| s.to_string()),
            is_favorite,
            limit,
            offset,
        };
        let conn = self.conn.lock().unwrap();
        get_entries_inner(&conn, &query)
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete an entry and clean up associated image file if it's an image entry.
    pub fn delete_entry_with_cleanup(&self, id: i64, _app_data_dir: &Path) -> Result<()> {
        if let Some(entry) = self.get_entry_by_id(id)? {
            if entry.content_type == "image" {
                let image_path = Path::new(&entry.content);
                if image_path.exists() {
                    if let Err(e) = std::fs::remove_file(image_path) {
                        log::warn!("Failed to delete image file {:?}: {}", image_path, e);
                    }
                }
            }
        }
        self.delete_entry(id)
    }

    pub fn delete_entries_with_cleanup(&self, ids: &[i64], app_data_dir: &Path) -> Result<i64> {
        let mut deleted = 0;
        for id in ids {
            self.delete_entry_with_cleanup(*id, app_data_dir)?;
            deleted += 1;
        }
        Ok(deleted)
    }

    pub fn get_entries_by_ids(&self, ids: &[i64]) -> Result<Vec<ClipboardEntry>> {
        ids.iter()
            .filter_map(|id| self.get_entry_by_id(*id).ok().flatten())
            .collect::<Vec<_>>()
            .pipe(Ok)
    }

    pub fn merge_entries_content(&self, ids: &[i64]) -> Result<String> {
        let entries = self.get_entries_by_ids(ids)?;
        Ok(entries
            .into_iter()
            .filter(|entry| entry.content_type != "image")
            .map(|entry| entry.content.trim().to_string())
            .filter(|content| !content.is_empty())
            .collect::<Vec<_>>()
            .join(
                "

",
            ))
    }

    pub fn toggle_favorite(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_entries SET is_favorite = CASE WHEN is_favorite = 0 THEN 1 ELSE 0 END WHERE id = ?1",
            params![id],
        )?;
        let new_state: bool = conn.query_row(
            "SELECT is_favorite FROM clipboard_entries WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(new_state)
    }

    pub fn set_favorite_state_for_entries(&self, ids: &[i64], favorite: bool) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let mut updated = 0;
        for id in ids {
            updated += tx.execute(
                "UPDATE clipboard_entries SET is_favorite = ?1 WHERE id = ?2",
                params![if favorite { 1 } else { 0 }, id],
            )? as i64;
        }
        tx.commit()?;
        Ok(updated)
    }

    pub fn delete_expired(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().naive_local();
        let count = conn.execute(
            "DELETE FROM clipboard_entries WHERE expires_at IS NOT NULL AND expires_at < ?1 AND is_favorite = 0",
            params![now.format("%Y-%m-%d %H:%M:%S").to_string()],
        )?;
        Ok(count as i64)
    }

    pub fn delete_oldest_beyond_limit(&self, max_count: i64) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM clipboard_entries WHERE is_favorite = 0 AND id NOT IN (
                SELECT id FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1
            )",
            params![max_count],
        )?;
        Ok(count as i64)
    }

    pub fn get_entry_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
            row.get(0)
        })
    }

    pub fn get_entry_by_id(&self, id: i64) -> Result<Option<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at, source_device
             FROM clipboard_entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_entry(row)?))
        } else {
            Ok(None)
        }
    }

    // --- Statistics methods ---

    pub fn get_statistics(&self) -> Result<Statistics> {
        let conn = self.conn.lock().unwrap();

        let total_entries: i64 =
            conn.query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
                row.get(0)
            })?;

        let total_favorites: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE is_favorite = 1",
            [],
            |row| row.get(0),
        )?;

        // Entries by category
        let mut cat_stmt = conn.prepare(
            "SELECT category, COUNT(*) as count FROM clipboard_entries GROUP BY category ORDER BY count DESC",
        )?;
        let entries_by_category: Vec<CategoryCount> = cat_stmt
            .query_map([], |row| {
                Ok(CategoryCount {
                    category: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Entries by day (last 30 days)
        let mut day_stmt = conn.prepare(
            "SELECT DATE(created_at) as date, COUNT(*) as count FROM clipboard_entries GROUP BY DATE(created_at) ORDER BY date DESC LIMIT 30",
        )?;
        let entries_by_day: Vec<DayCount> = day_stmt
            .query_map([], |row| {
                Ok(DayCount {
                    date: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Most used entries (top 10)
        let mut most_stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at, source_device FROM clipboard_entries ORDER BY use_count DESC LIMIT 10",
        )?;
        let most_used: Vec<ClipboardEntry> = most_stmt
            .query_map([], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(Statistics {
            total_entries,
            total_favorites,
            entries_by_category,
            entries_by_day,
            most_used,
            storage_size_bytes: 0, // Will be set by the command
        })
    }

    pub fn get_paired_devices(&self) -> Result<Vec<PairedDevice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, public_key, local_public_key, shared_secret, last_seen_at, is_active, paired_at, fingerprint FROM paired_devices ORDER BY last_seen_at DESC, paired_at DESC",
        )?;
        let devices = stmt
            .query_map([], |row| {
                Ok(PairedDevice {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    device_name: row.get(1)?,
                    host: row.get(2)?,
                    address: row.get(2)?,
                    ip: row.get(2)?,
                    port: row.get(3)?,
                    status: "unknown".to_string(),
                    public_key: row.get(4)?,
                    local_public_key: row.get(5)?,
                    shared_secret: row.get(6)?,
                    last_seen_at: row
                        .get::<_, Option<String>>(7)?
                        .map(|v| parse_datetime(&v))
                        .transpose()?,
                    is_active: row.get::<_, i32>(8)? != 0,
                    enabled: row.get::<_, i32>(8)? != 0,
                    sync_enabled: row.get::<_, i32>(8)? != 0,
                    paired_at: parse_datetime(&row.get::<_, String>(9)?)?,
                    fingerprint: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(devices)
    }

    pub fn upsert_paired_device(&self, device: &PairedDevice) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO paired_devices (id, name, host, port, public_key, local_public_key, shared_secret, last_seen_at, is_active, paired_at, fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               host = excluded.host,
               port = excluded.port,
               public_key = excluded.public_key,
               local_public_key = excluded.local_public_key,
               shared_secret = excluded.shared_secret,
               last_seen_at = excluded.last_seen_at,
               is_active = excluded.is_active,
               fingerprint = excluded.fingerprint",
            params![
                device.id,
                device.name,
                device.host,
                device.port,
                device.public_key,
                device.local_public_key,
                device.shared_secret,
                device.last_seen_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                device.is_active as i32,
                device.paired_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                device.fingerprint,
            ],
        )?;
        Ok(())
    }

    pub fn find_paired_device(&self, device_id: &str) -> Result<Option<PairedDevice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, public_key, local_public_key, shared_secret, last_seen_at, is_active, paired_at, fingerprint FROM paired_devices WHERE id = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![device_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(PairedDevice {
                id: row.get(0)?,
                name: row.get(1)?,
                device_name: row.get(1)?,
                host: row.get(2)?,
                address: row.get(2)?,
                ip: row.get(2)?,
                port: row.get(3)?,
                status: "unknown".to_string(),
                public_key: row.get(4)?,
                local_public_key: row.get(5)?,
                shared_secret: row.get(6)?,
                last_seen_at: row
                    .get::<_, Option<String>>(7)?
                    .map(|v| parse_datetime(&v))
                    .transpose()?,
                is_active: row.get::<_, i32>(8)? != 0,
                enabled: row.get::<_, i32>(8)? != 0,
                sync_enabled: row.get::<_, i32>(8)? != 0,
                paired_at: parse_datetime(&row.get::<_, String>(9)?)?,
                fingerprint: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn unpair_device(&self, device_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM paired_devices WHERE id = ?1",
            params![device_id],
        )?;
        Ok(())
    }

    pub fn set_paired_device_active(&self, device_id: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        conn.execute(
            "UPDATE paired_devices SET is_active = ?1, last_seen_at = ?2 WHERE id = ?3",
            params![enabled as i32, now, device_id],
        )?;
        Ok(())
    }

    pub fn insert_sync_log(
        &self,
        entry_hash: &str,
        device_id: &str,
        direction: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        conn.execute(
            "INSERT INTO sync_log (entry_hash, device_id, direction, synced_at) VALUES (?1, ?2, ?3, ?4)",
            params![entry_hash, device_id, direction, now],
        )?;
        Ok(())
    }

    pub fn has_sync_log(&self, entry_hash: &str, device_id: &str, direction: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sync_log WHERE entry_hash = ?1 AND device_id = ?2 AND direction = ?3",
            params![entry_hash, device_id, direction],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn has_received_entry(&self, entry_hash: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sync_log WHERE entry_hash = ?1 AND direction = 'received'",
            params![entry_hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // --- Tag management methods ---

    pub fn create_tag(&self, name: &str) -> Result<Tag> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO tags (name) VALUES (?1)", params![name])?;
        let id = conn.last_insert_rowid();
        Ok(Tag {
            id: Some(id),
            name: name.to_string(),
        })
    }

    pub fn delete_tag(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    pub fn add_tag_to_entry(&self, entry_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
            params![entry_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_entry(&self, entry_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM entry_tags WHERE entry_id = ?1 AND tag_id = ?2",
            params![entry_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_entry_tags(&self, entry_id: i64) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name FROM tags t
             INNER JOIN entry_tags et ON t.id = et.tag_id
             WHERE et.entry_id = ?1
             ORDER BY t.name",
        )?;
        let tags = stmt
            .query_map(params![entry_id], |row| {
                Ok(Tag {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    pub fn set_tags_for_entries(&self, ids: &[i64], tag_ids: &[i64], mode: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        for entry_id in ids {
            if mode == "replace" {
                tx.execute(
                    "DELETE FROM entry_tags WHERE entry_id = ?1",
                    params![entry_id],
                )?;
            }
            for tag_id in tag_ids {
                tx.execute(
                    "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                    params![entry_id, tag_id],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_entries_by_tag(&self, tag_id: i64) -> Result<Vec<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.id, e.content, e.content_type, e.category, e.hash, e.source_app, e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at, e.expires_at, e.source_device
             FROM clipboard_entries e
             INNER JOIN entry_tags et ON e.id = et.entry_id
             WHERE et.tag_id = ?1
             ORDER BY e.created_at DESC",
        )?;
        let entries = stmt
            .query_map(params![tag_id], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    // --- Template methods ---

    pub fn create_template(&self, name: &str, content: &str, category: &str) -> Result<Template> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "INSERT INTO templates (name, content, category, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, content, category, now, now],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Template {
            id: Some(id),
            name: name.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            is_favorite: false,
            use_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_template(
        &self,
        id: i64,
        name: &str,
        content: &str,
        category: &str,
    ) -> Result<Template> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let rows = conn.execute(
            "UPDATE templates SET name = ?1, content = ?2, category = ?3, updated_at = ?4 WHERE id = ?5",
            params![name, content, category, now, id],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        // Read back to get all fields
        let mut stmt = conn.prepare(
            "SELECT id, name, content, category, is_favorite, use_count, created_at, updated_at FROM templates WHERE id = ?1",
        )?;
        stmt.query_row(params![id], row_to_template)
    }

    pub fn delete_template(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM templates WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_templates(&self, category: Option<&str>) -> Result<Vec<Template>> {
        let conn = self.conn.lock().unwrap();
        if let Some(cat) = category {
            let mut stmt = conn.prepare(
                "SELECT id, name, content, category, is_favorite, use_count, created_at, updated_at FROM templates WHERE category = ?1 ORDER BY updated_at DESC",
            )?;
            let templates = stmt
                .query_map(params![cat], row_to_template)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(templates)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, content, category, is_favorite, use_count, created_at, updated_at FROM templates ORDER BY updated_at DESC",
            )?;
            let templates = stmt
                .query_map([], row_to_template)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(templates)
        }
    }

    pub fn get_template_by_id(&self, id: i64) -> Result<Option<Template>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, content, category, is_favorite, use_count, created_at, updated_at FROM templates WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_template(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn increment_template_use_count(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE templates SET use_count = use_count + 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn get_template_categories_list(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT category FROM templates ORDER BY category")?;
        let categories = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(categories)
    }

    // --- Encryption support methods ---

    /// Count encrypted vs plaintext entries.
    pub fn count_encrypted_entries(&self) -> Result<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let encrypted: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE is_encrypted = 1",
            [],
            |row| row.get(0),
        )?;
        let plaintext: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE is_encrypted = 0",
            [],
            |row| row.get(0),
        )?;
        Ok((encrypted, plaintext))
    }

    /// Fetch all plaintext entries (id, content) for migration to encrypted.
    pub fn get_plaintext_entries_for_migration(&self) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content FROM clipboard_entries WHERE is_encrypted = 0 AND content_type != 'image'",
        )?;
        let entries = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// Fetch all encrypted entries (id, content) for migration to plaintext.
    pub fn get_encrypted_entries_for_migration(&self) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, content FROM clipboard_entries WHERE is_encrypted = 1")?;
        let entries = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// Update content and encrypted flag for a single entry (used during migration).
    pub fn update_entry_content_and_encrypted_flag(
        &self,
        id: i64,
        content: &str,
        encrypted: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_entries SET content = ?1, is_encrypted = ?2 WHERE id = ?3",
            params![content, encrypted as i32, id],
        )?;
        Ok(())
    }

    // --- Smart Search: Cluster methods ---

    pub fn get_cluster_list(&self) -> Result<Vec<ClusterSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.label, COUNT(m.entry_id) as entry_count, c.created_at, c.updated_at
             FROM entry_clusters c
             LEFT JOIN entry_cluster_members m ON c.id = m.cluster_id
             GROUP BY c.id
             ORDER BY entry_count DESC",
        )?;
        let clusters = stmt
            .query_map([], |row| {
                Ok(ClusterSummary {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    entry_count: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(clusters)
    }

    pub fn get_cluster_entries(
        &self,
        cluster_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult> {
        let conn = self.conn.lock().unwrap();
        let total_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entry_cluster_members WHERE cluster_id = ?1",
            params![cluster_id],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT e.id, e.content, e.content_type, e.category, e.hash, e.source_app,
                    e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at,
                    e.expires_at, e.source_device
             FROM clipboard_entries e
             INNER JOIN entry_cluster_members m ON e.id = m.entry_id
             WHERE m.cluster_id = ?1
             ORDER BY m.score DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let entries: Vec<ClipboardEntry> = stmt
            .query_map(params![cluster_id, limit, offset], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(SearchResult {
            entries,
            total_count,
        })
    }

    pub fn upsert_cluster(&self, label: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entry_clusters (label) VALUES (?1)",
            params![label],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_to_cluster(&self, cluster_id: i64, entry_id: i64, score: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO entry_cluster_members (cluster_id, entry_id, score) VALUES (?1, ?2, ?3)",
            params![cluster_id, entry_id, score],
        )?;
        Ok(())
    }

    pub fn remove_from_cluster(&self, cluster_id: i64, entry_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM entry_cluster_members WHERE cluster_id = ?1 AND entry_id = ?2",
            params![cluster_id, entry_id],
        )?;
        Ok(())
    }

    pub fn get_unclustered_entries(&self, limit: i64) -> Result<Vec<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app,
                    is_favorite, is_sensitive, use_count, created_at, updated_at,
                    expires_at, source_device
             FROM clipboard_entries
             WHERE id NOT IN (SELECT entry_id FROM entry_cluster_members)
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    pub fn clear_clusters(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM entry_cluster_members; DELETE FROM entry_clusters;")?;
        Ok(())
    }

    // --- Smart Search: Tag suggestion methods ---

    pub fn save_tag_suggestions(&self, entry_id: i64, suggestions: &[(i64, f64)]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM tag_suggestions WHERE entry_id = ?1",
            params![entry_id],
        )?;
        for (tag_id, confidence) in suggestions {
            conn.execute(
                "INSERT INTO tag_suggestions (entry_id, tag_id, confidence) VALUES (?1, ?2, ?3)",
                params![entry_id, tag_id, confidence],
            )?;
        }
        Ok(())
    }

    pub fn get_tag_suggestions(&self, entry_id: i64) -> Result<Vec<TagSuggestion>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ts.entry_id, t.id, t.name, ts.confidence
             FROM tag_suggestions ts
             INNER JOIN tags t ON ts.tag_id = t.id
             WHERE ts.entry_id = ?1
             ORDER BY ts.confidence DESC",
        )?;
        let suggestions = stmt
            .query_map(params![entry_id], |row| {
                Ok(TagSuggestion {
                    entry_id: row.get(0)?,
                    tag: Tag {
                        id: Some(row.get(1)?),
                        name: row.get(2)?,
                    },
                    confidence: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(suggestions)
    }

    pub fn dismiss_tag_suggestions(&self, entry_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM tag_suggestions WHERE entry_id = ?1",
            params![entry_id],
        )?;
        Ok(())
    }

    // --- Smart Search: Related entries ---

    pub fn get_entries_for_similarity(
        &self,
        exclude_id: i64,
        limit: i64,
    ) -> Result<Vec<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app,
                    is_favorite, is_sensitive, use_count, created_at, updated_at,
                    expires_at, source_device
             FROM clipboard_entries
             WHERE id != ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let entries = stmt
            .query_map(params![exclude_id, limit], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    pub fn get_tagged_entries_with_tags(&self) -> Result<Vec<(ClipboardEntry, Vec<Tag>)>> {
        let conn = self.conn.lock().unwrap();

        // Get all entry ids that have at least one tag
        let mut entry_stmt = conn.prepare(
            "SELECT DISTINCT e.id, e.content, e.content_type, e.category, e.hash, e.source_app,
                    e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at,
                    e.expires_at, e.source_device
             FROM clipboard_entries e
             INNER JOIN entry_tags et ON e.id = et.entry_id
             ORDER BY e.created_at DESC",
        )?;
        let entries: Vec<ClipboardEntry> = entry_stmt
            .query_map([], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();

        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            let entry_id = entry.id.unwrap_or(0);
            let mut tag_stmt = conn.prepare(
                "SELECT t.id, t.name FROM tags t
                 INNER JOIN entry_tags et ON t.id = et.tag_id
                 WHERE et.entry_id = ?1
                 ORDER BY t.name",
            )?;
            let tags: Vec<Tag> = tag_stmt
                .query_map(params![entry_id], |row| {
                    Ok(Tag {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
            result.push((entry, tags));
        }
        Ok(result)
    }
}

fn search_fts(conn: &Connection, keyword: &str, query: &SearchQuery) -> Result<SearchResult> {
    let normalized_keyword = normalize_search_keyword(keyword);

    if normalized_keyword.is_empty() {
        return get_entries_inner(conn, query);
    }

    let fts_query = build_fts_match_expr(keyword);
    let like_pattern = format!("%{}%", normalized_keyword.replace(' ', "%"));

    let (mut sql, mut count_sql, use_match_branch) = if fts_query.is_empty() {
        (
            String::from(
                "SELECT e.id, e.content, e.content_type, e.category, e.hash, e.source_app, e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at, e.expires_at, e.source_device
                 FROM clipboard_entries e
                 WHERE (
                     LOWER(e.content) LIKE ?1
                     OR e.pinyin_full LIKE ?1
                     OR e.pinyin_initials LIKE ?1
                 )",
            ),
            String::from(
                "SELECT COUNT(*)
                 FROM clipboard_entries e
                 WHERE (
                     LOWER(e.content) LIKE ?1
                     OR e.pinyin_full LIKE ?1
                     OR e.pinyin_initials LIKE ?1
                 )",
            ),
            false,
        )
    } else {
        (
            String::from(
                "SELECT e.id, e.content, e.content_type, e.category, e.hash, e.source_app, e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at, e.expires_at, e.source_device
                 FROM clipboard_entries e
                 WHERE (
                     e.id IN (SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH ?1)
                     OR LOWER(e.content) LIKE ?2
                     OR e.pinyin_full LIKE ?2
                     OR e.pinyin_initials LIKE ?2
                 )",
            ),
            String::from(
                "SELECT COUNT(*)
                 FROM clipboard_entries e
                 WHERE (
                     e.id IN (SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH ?1)
                     OR LOWER(e.content) LIKE ?2
                     OR e.pinyin_full LIKE ?2
                     OR e.pinyin_initials LIKE ?2
                 )",
            ),
            true,
        )
    };

    if let Some(ref cat) = query.category {
        let filter = format!(" AND e.category = '{}'", cat.replace('\'', "''"));
        sql.push_str(&filter);
        count_sql.push_str(&filter);
    }

    if let Some(fav) = query.is_favorite {
        let filter = format!(" AND e.is_favorite = {}", if fav { 1 } else { 0 });
        sql.push_str(&filter);
        count_sql.push_str(&filter);
    }

    if use_match_branch {
        sql.push_str(" ORDER BY e.created_at DESC LIMIT ?3 OFFSET ?4");

        let total_count: i64 =
            conn.query_row(&count_sql, params![fts_query, like_pattern], |row| {
                row.get(0)
            })?;

        let mut stmt = conn.prepare(&sql)?;
        let entries: Vec<ClipboardEntry> = stmt
            .query_map(
                params![fts_query, like_pattern, query.limit, query.offset],
                row_to_entry,
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(SearchResult {
            entries,
            total_count,
        })
    } else {
        sql.push_str(" ORDER BY e.created_at DESC LIMIT ?2 OFFSET ?3");

        let total_count: i64 =
            conn.query_row(&count_sql, params![like_pattern], |row| row.get(0))?;

        let mut stmt = conn.prepare(&sql)?;
        let entries: Vec<ClipboardEntry> = stmt
            .query_map(
                params![like_pattern, query.limit, query.offset],
                row_to_entry,
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(SearchResult {
            entries,
            total_count,
        })
    }
}

fn get_entries_inner(conn: &Connection, query: &SearchQuery) -> Result<SearchResult> {
    let mut sql = String::from(
        "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at, source_device
         FROM clipboard_entries WHERE 1=1",
    );
    let mut count_sql = String::from("SELECT COUNT(*) FROM clipboard_entries WHERE 1=1");

    if let Some(ref cat) = query.category {
        let filter = format!(" AND category = '{}'", cat.replace('\'', "''"));
        sql.push_str(&filter);
        count_sql.push_str(&filter);
    }

    if let Some(fav) = query.is_favorite {
        let filter = format!(" AND is_favorite = {}", if fav { 1 } else { 0 });
        sql.push_str(&filter);
        count_sql.push_str(&filter);
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT ?1 OFFSET ?2");

    let total_count: i64 = conn.query_row(&count_sql, [], |row| row.get(0))?;

    let mut stmt = conn.prepare(&sql)?;
    let entries: Vec<ClipboardEntry> = stmt
        .query_map(params![query.limit, query.offset], row_to_entry)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SearchResult {
        entries,
        total_count,
    })
}

fn parse_datetime(s: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn row_to_template(row: &rusqlite::Row) -> Result<Template> {
    Ok(Template {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        content: row.get(2)?,
        category: row.get(3)?,
        is_favorite: row.get::<_, i32>(4)? != 0,
        use_count: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_entry(row: &rusqlite::Row) -> Result<ClipboardEntry> {
    let created_str: String = row.get(9)?;
    let updated_str: String = row.get(10)?;
    let expires_str: Option<String> = row.get(11)?;
    let source_device: Option<String> = row.get(12).ok().flatten();

    let parse_dt = |s: &str| -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|_| Local::now().naive_local())
    };

    Ok(ClipboardEntry {
        id: Some(row.get(0)?),
        content: row.get(1)?,
        content_type: row.get(2)?,
        category: row.get(3)?,
        hash: row.get(4)?,
        source_app: row.get(5)?,
        is_favorite: row.get::<_, i32>(6)? != 0,
        is_sensitive: row.get::<_, i32>(7)? != 0,
        use_count: row.get(8)?,
        created_at: parse_dt(&created_str),
        updated_at: parse_dt(&updated_str),
        expires_at: expires_str.as_deref().map(parse_dt),
        source_device,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_entry(content: &str, category: &str) -> ClipboardEntry {
        let now = Local::now().naive_local();
        let hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
        ClipboardEntry {
            id: None,
            content: content.to_string(),
            content_type: "text".to_string(),
            category: category.to_string(),
            hash,
            source_app: Some("test".to_string()),
            is_favorite: false,
            is_sensitive: false,
            use_count: 1,
            created_at: now,
            updated_at: now,
            expires_at: None,
            source_device: None,
        }
    }

    use sha2::Digest;

    #[test]
    fn test_insert_and_retrieve() {
        let db = Database::new(":memory:").unwrap();
        let entry = make_entry("hello world", "text");
        let id = db.insert_entry(&entry).unwrap();
        assert!(id > 0);

        let found = db.find_by_hash(&entry.hash).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.content, "hello world");
        assert_eq!(found.category, "text");
    }

    #[test]
    fn test_deduplication() {
        let db = Database::new(":memory:").unwrap();
        let entry = make_entry("duplicate content", "text");
        db.insert_entry(&entry).unwrap();

        let found = db.find_by_hash(&entry.hash).unwrap();
        assert!(found.is_some());

        // update_use_count for dedup
        db.update_use_count(&entry.hash).unwrap();
        let found = db.find_by_hash(&entry.hash).unwrap().unwrap();
        assert_eq!(found.use_count, 2);
    }

    #[test]
    fn test_fts5_search() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("rust programming language", "text"))
            .unwrap();
        db.insert_entry(&make_entry("https://github.com", "url"))
            .unwrap();
        db.insert_entry(&make_entry("python data science", "text"))
            .unwrap();

        let result = db
            .search(&SearchQuery {
                keyword: Some("rust".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.entries[0].content, "rust programming language");
    }

    #[test]
    fn test_search_matches_pinyin_full_and_initials() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("智能剪贴板", "text")).unwrap();
        db.insert_entry(&make_entry("普通内容", "text")).unwrap();

        let full = db
            .search(&SearchQuery {
                keyword: Some("zhineng".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();
        assert_eq!(full.total_count, 1, "full={:?}", full.entries);
        assert_eq!(full.entries[0].content, "智能剪贴板");

        let initials = db
            .search(&SearchQuery {
                keyword: Some("znjtb".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();
        assert_eq!(initials.total_count, 1);
        assert_eq!(initials.entries[0].content, "智能剪贴板");
    }

    #[test]
    fn test_search_uppercase_initials() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("智能剪贴板", "text")).unwrap();

        let result = db
            .search(&SearchQuery {
                keyword: Some("ZNJTB".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.entries[0].content, "智能剪贴板");
    }

    #[test]
    fn test_search_punctuation_only_does_not_error() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("hello world", "text")).unwrap();
        db.insert_entry(&make_entry("智能剪贴板", "text")).unwrap();

        let result = db
            .search(&SearchQuery {
                keyword: Some("!!!".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_search_mixed_cn_en_contiguous_prefix() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("Hello世界", "text")).unwrap();

        let result = db
            .search(&SearchQuery {
                keyword: Some("helloshi".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.entries[0].content, "Hello世界");
    }

    #[test]
    fn test_search_pinyin_preserves_category_and_pagination() {
        let db = Database::new(":memory:").unwrap();
        for i in 0..5 {
            db.insert_entry(&make_entry(&format!("智能条目 {}", i), "text"))
                .unwrap();
        }
        db.insert_entry(&make_entry("智能链接", "url")).unwrap();

        let page = db
            .search(&SearchQuery {
                keyword: Some("zhineng".to_string()),
                category: Some("text".to_string()),
                is_favorite: None,
                limit: 2,
                offset: 1,
            })
            .unwrap();

        assert_eq!(page.total_count, 5);
        assert_eq!(page.entries.len(), 2);
        assert!(page.entries.iter().all(|entry| entry.category == "text"));
    }
    #[test]
    fn test_fts5_search_no_results() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("hello world", "text")).unwrap();

        let result = db
            .search(&SearchQuery {
                keyword: Some("nonexistent".to_string()),
                category: None,
                is_favorite: None,
                limit: 50,
                offset: 0,
            })
            .unwrap();

        assert_eq!(result.total_count, 0);
        assert!(result.entries.is_empty());
    }

    #[test]
    fn test_category_filter() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("https://example.com", "url"))
            .unwrap();
        db.insert_entry(&make_entry("user@example.com", "email"))
            .unwrap();
        db.insert_entry(&make_entry("plain text", "text")).unwrap();

        let result = db.get_entries(50, 0, Some("url"), None).unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.entries[0].category, "url");
    }

    #[test]
    fn test_pagination() {
        let db = Database::new(":memory:").unwrap();
        for i in 0..10 {
            db.insert_entry(&make_entry(&format!("entry {}", i), "text"))
                .unwrap();
        }

        let page1 = db.get_entries(3, 0, None, None).unwrap();
        assert_eq!(page1.entries.len(), 3);
        assert_eq!(page1.total_count, 10);

        let page2 = db.get_entries(3, 3, None, None).unwrap();
        assert_eq!(page2.entries.len(), 3);
        assert_ne!(page1.entries[0].id, page2.entries[0].id);
    }

    #[test]
    fn test_toggle_favorite() {
        let db = Database::new(":memory:").unwrap();
        let entry = make_entry("fav test", "text");
        let id = db.insert_entry(&entry).unwrap();

        let new_state = db.toggle_favorite(id).unwrap();
        assert!(new_state);

        let new_state = db.toggle_favorite(id).unwrap();
        assert!(!new_state);
    }

    #[test]
    fn test_delete_entry() {
        let db = Database::new(":memory:").unwrap();
        let entry = make_entry("to delete", "text");
        let id = db.insert_entry(&entry).unwrap();

        db.delete_entry(id).unwrap();
        let found = db.find_by_hash(&entry.hash).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_delete_expired() {
        let db = Database::new(":memory:").unwrap();
        let now = Local::now().naive_local();

        let mut entry = make_entry("expired entry", "text");
        entry.expires_at = Some(now - Duration::hours(1));
        db.insert_entry(&entry).unwrap();

        let mut fresh = make_entry("fresh entry", "text");
        fresh.expires_at = Some(now + Duration::hours(1));
        db.insert_entry(&fresh).unwrap();

        let deleted = db.delete_expired().unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(db.get_entry_count().unwrap(), 1);
    }

    #[test]
    fn test_delete_oldest_beyond_limit() {
        let db = Database::new(":memory:").unwrap();
        for i in 0..10 {
            let mut entry = make_entry(&format!("entry {}", i), "text");
            entry.created_at = Local::now().naive_local() + Duration::seconds(i as i64);
            db.insert_entry(&entry).unwrap();
        }

        let deleted = db.delete_oldest_beyond_limit(5).unwrap();
        assert_eq!(deleted, 5);
        assert_eq!(db.get_entry_count().unwrap(), 5);
    }

    // --- Tag management tests ---

    #[test]
    fn test_create_tag() {
        let db = Database::new(":memory:").unwrap();
        let tag = db.create_tag("work").unwrap();
        assert!(tag.id.is_some());
        assert_eq!(tag.name, "work");
    }

    #[test]
    fn test_create_duplicate_tag_fails() {
        let db = Database::new(":memory:").unwrap();
        db.create_tag("work").unwrap();
        let result = db.create_tag("work");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_tags() {
        let db = Database::new(":memory:").unwrap();
        db.create_tag("work").unwrap();
        db.create_tag("personal").unwrap();
        db.create_tag("code").unwrap();

        let tags = db.get_all_tags().unwrap();
        assert_eq!(tags.len(), 3);
        // Tags are ordered by name
        assert_eq!(tags[0].name, "code");
        assert_eq!(tags[1].name, "personal");
        assert_eq!(tags[2].name, "work");
    }

    #[test]
    fn test_delete_tag() {
        let db = Database::new(":memory:").unwrap();
        let tag = db.create_tag("to_delete").unwrap();
        let id = tag.id.unwrap();

        db.delete_tag(id).unwrap();
        let tags = db.get_all_tags().unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_add_and_get_entry_tags() {
        let db = Database::new(":memory:").unwrap();
        let entry_id = db
            .insert_entry(&make_entry("tagged content", "text"))
            .unwrap();
        let tag1 = db.create_tag("tag1").unwrap();
        let tag2 = db.create_tag("tag2").unwrap();

        db.add_tag_to_entry(entry_id, tag1.id.unwrap()).unwrap();
        db.add_tag_to_entry(entry_id, tag2.id.unwrap()).unwrap();

        let tags = db.get_entry_tags(entry_id).unwrap();
        assert_eq!(tags.len(), 2);
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"tag1"));
        assert!(names.contains(&"tag2"));
    }

    #[test]
    fn test_add_tag_to_entry_idempotent() {
        let db = Database::new(":memory:").unwrap();
        let entry_id = db
            .insert_entry(&make_entry("tagged content", "text"))
            .unwrap();
        let tag = db.create_tag("tag1").unwrap();
        let tag_id = tag.id.unwrap();

        db.add_tag_to_entry(entry_id, tag_id).unwrap();
        // Adding again should not fail (INSERT OR IGNORE)
        db.add_tag_to_entry(entry_id, tag_id).unwrap();

        let tags = db.get_entry_tags(entry_id).unwrap();
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_remove_tag_from_entry() {
        let db = Database::new(":memory:").unwrap();
        let entry_id = db
            .insert_entry(&make_entry("tagged content", "text"))
            .unwrap();
        let tag = db.create_tag("removable").unwrap();
        let tag_id = tag.id.unwrap();

        db.add_tag_to_entry(entry_id, tag_id).unwrap();
        assert_eq!(db.get_entry_tags(entry_id).unwrap().len(), 1);

        db.remove_tag_from_entry(entry_id, tag_id).unwrap();
        assert_eq!(db.get_entry_tags(entry_id).unwrap().len(), 0);
    }

    #[test]
    fn test_get_entries_by_tag() {
        let db = Database::new(":memory:").unwrap();
        let id1 = db.insert_entry(&make_entry("entry one", "text")).unwrap();
        let id2 = db.insert_entry(&make_entry("entry two", "text")).unwrap();
        let _id3 = db.insert_entry(&make_entry("entry three", "text")).unwrap();

        let tag = db.create_tag("shared").unwrap();
        let tag_id = tag.id.unwrap();

        db.add_tag_to_entry(id1, tag_id).unwrap();
        db.add_tag_to_entry(id2, tag_id).unwrap();

        let entries = db.get_entries_by_tag(tag_id).unwrap();
        assert_eq!(entries.len(), 2);
        let ids: Vec<i64> = entries.iter().map(|e| e.id.unwrap()).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_delete_entry_cascades_tag_associations() {
        let db = Database::new(":memory:").unwrap();
        let entry_id = db
            .insert_entry(&make_entry("will be deleted", "text"))
            .unwrap();
        let tag = db.create_tag("cascade_test").unwrap();
        let tag_id = tag.id.unwrap();

        db.add_tag_to_entry(entry_id, tag_id).unwrap();
        assert_eq!(db.get_entry_tags(entry_id).unwrap().len(), 1);

        // Delete the entry - cascade should remove from entry_tags
        db.delete_entry(entry_id).unwrap();

        // The tag itself should still exist
        let tags = db.get_all_tags().unwrap();
        assert_eq!(tags.len(), 1);

        // But no entries should be associated with the tag
        let entries = db.get_entries_by_tag(tag_id).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_delete_image_entry_with_cleanup() {
        let db = Database::new(":memory:").unwrap();
        let now = Local::now().naive_local();

        // Create a temporary image file
        let tmp_dir = std::env::temp_dir().join("smart_clipboard_test");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let img_path = tmp_dir.join("test_image.png");
        std::fs::write(&img_path, b"fake png data").unwrap();
        assert!(img_path.exists());

        let hash = format!("{:x}", sha2::Sha256::digest(b"test image bytes"));
        let entry = ClipboardEntry {
            id: None,
            content: img_path.to_string_lossy().to_string(),
            content_type: "image".to_string(),
            category: "image".to_string(),
            hash,
            source_app: None,
            is_favorite: false,
            is_sensitive: false,
            use_count: 1,
            created_at: now,
            updated_at: now,
            expires_at: None,
            source_device: None,
        };
        let id = db.insert_entry(&entry).unwrap();

        // Delete with cleanup should remove the file
        db.delete_entry_with_cleanup(id, &tmp_dir).unwrap();

        // Entry should be gone from DB
        let found = db.get_entry_by_id(id).unwrap();
        assert!(found.is_none());

        // Image file should be deleted
        assert!(!img_path.exists());

        // Clean up temp dir
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_delete_text_entry_with_cleanup_no_file_delete() {
        let db = Database::new(":memory:").unwrap();
        let entry = make_entry("just text", "text");
        let id = db.insert_entry(&entry).unwrap();

        // delete_entry_with_cleanup should work for text entries without touching files
        let tmp_dir = std::env::temp_dir();
        db.delete_entry_with_cleanup(id, &tmp_dir).unwrap();

        let found = db.get_entry_by_id(id).unwrap();
        assert!(found.is_none());
    }

    // --- Statistics tests ---

    #[test]
    fn test_get_statistics_empty_db() {
        let db = Database::new(":memory:").unwrap();
        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_favorites, 0);
        assert!(stats.entries_by_category.is_empty());
        assert!(stats.entries_by_day.is_empty());
        assert!(stats.most_used.is_empty());
        assert_eq!(stats.storage_size_bytes, 0);
    }

    #[test]
    fn test_get_statistics_total_counts() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("entry 1", "text")).unwrap();
        db.insert_entry(&make_entry("entry 2", "url")).unwrap();
        let id3 = db.insert_entry(&make_entry("entry 3", "code")).unwrap();
        db.toggle_favorite(id3).unwrap();

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.total_favorites, 1);
    }

    #[test]
    fn test_get_statistics_entries_by_category() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("url1", "url")).unwrap();
        db.insert_entry(&make_entry("url2", "url")).unwrap();
        db.insert_entry(&make_entry("text1", "text")).unwrap();
        db.insert_entry(&make_entry("code1", "code")).unwrap();

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.entries_by_category.len(), 3);
        // Should be ordered by count DESC
        assert_eq!(stats.entries_by_category[0].category, "url");
        assert_eq!(stats.entries_by_category[0].count, 2);
    }

    #[test]
    fn test_get_statistics_entries_by_day() {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("today entry 1", "text"))
            .unwrap();
        db.insert_entry(&make_entry("today entry 2", "url"))
            .unwrap();

        let stats = db.get_statistics().unwrap();
        assert!(!stats.entries_by_day.is_empty());
        // All entries created "now" should be on the same day
        assert_eq!(stats.entries_by_day[0].count, 2);
    }

    #[test]
    fn test_get_statistics_most_used() {
        let db = Database::new(":memory:").unwrap();
        let e1 = make_entry("popular", "text");
        db.insert_entry(&e1).unwrap();
        // Bump use_count by updating multiple times
        db.update_use_count(&e1.hash).unwrap();
        db.update_use_count(&e1.hash).unwrap();

        db.insert_entry(&make_entry("less popular", "text"))
            .unwrap();

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.most_used.len(), 2);
        // Most used should come first
        assert_eq!(stats.most_used[0].content, "popular");
        assert_eq!(stats.most_used[0].use_count, 3); // 1 initial + 2 updates
    }

    #[test]
    fn test_get_statistics_most_used_limit_10() {
        let db = Database::new(":memory:").unwrap();
        for i in 0..15 {
            db.insert_entry(&make_entry(&format!("entry {}", i), "text"))
                .unwrap();
        }

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.most_used.len(), 10);
    }

    #[test]
    fn test_delete_tag_cascades_associations() {
        let db = Database::new(":memory:").unwrap();
        let entry_id = db
            .insert_entry(&make_entry("tagged content", "text"))
            .unwrap();
        let tag = db.create_tag("will_be_deleted").unwrap();
        let tag_id = tag.id.unwrap();

        db.add_tag_to_entry(entry_id, tag_id).unwrap();
        assert_eq!(db.get_entry_tags(entry_id).unwrap().len(), 1);

        // Delete the tag - cascade should remove from entry_tags
        db.delete_tag(tag_id).unwrap();

        // Entry should have no tags
        let tags = db.get_entry_tags(entry_id).unwrap();
        assert!(tags.is_empty());
    }

    // --- Template management tests ---

    #[test]
    fn test_create_template() {
        let db = Database::new(":memory:").unwrap();
        let tpl = db
            .create_template("greeting", "Hello {{name}}!", "general")
            .unwrap();
        assert!(tpl.id.is_some());
        assert_eq!(tpl.name, "greeting");
        assert_eq!(tpl.content, "Hello {{name}}!");
        assert_eq!(tpl.category, "general");
        assert_eq!(tpl.use_count, 0);
        assert!(!tpl.is_favorite);
    }

    #[test]
    fn test_create_duplicate_template_name_fails() {
        let db = Database::new(":memory:").unwrap();
        db.create_template("greeting", "Hello!", "general").unwrap();
        let result = db.create_template("greeting", "Hi!", "general");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_template_by_id() {
        let db = Database::new(":memory:").unwrap();
        let tpl = db.create_template("test", "content", "general").unwrap();
        let id = tpl.id.unwrap();

        let found = db.get_template_by_id(id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "test");
        assert_eq!(found.content, "content");
    }

    #[test]
    fn test_get_template_by_id_not_found() {
        let db = Database::new(":memory:").unwrap();
        let found = db.get_template_by_id(9999).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_update_template() {
        let db = Database::new(":memory:").unwrap();
        let tpl = db
            .create_template("old_name", "old content", "general")
            .unwrap();
        let id = tpl.id.unwrap();

        let updated = db
            .update_template(id, "new_name", "new content", "work")
            .unwrap();
        assert_eq!(updated.name, "new_name");
        assert_eq!(updated.content, "new content");
        assert_eq!(updated.category, "work");
    }

    #[test]
    fn test_update_template_not_found() {
        let db = Database::new(":memory:").unwrap();
        let result = db.update_template(9999, "name", "content", "general");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_template() {
        let db = Database::new(":memory:").unwrap();
        let tpl = db
            .create_template("to_delete", "content", "general")
            .unwrap();
        let id = tpl.id.unwrap();

        db.delete_template(id).unwrap();
        let found = db.get_template_by_id(id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_get_templates_all() {
        let db = Database::new(":memory:").unwrap();
        db.create_template("tpl1", "content1", "general").unwrap();
        db.create_template("tpl2", "content2", "work").unwrap();
        db.create_template("tpl3", "content3", "general").unwrap();

        let all = db.get_templates(None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_templates_by_category() {
        let db = Database::new(":memory:").unwrap();
        db.create_template("tpl1", "content1", "general").unwrap();
        db.create_template("tpl2", "content2", "work").unwrap();
        db.create_template("tpl3", "content3", "general").unwrap();

        let general = db.get_templates(Some("general")).unwrap();
        assert_eq!(general.len(), 2);
        for t in &general {
            assert_eq!(t.category, "general");
        }

        let work = db.get_templates(Some("work")).unwrap();
        assert_eq!(work.len(), 1);
        assert_eq!(work[0].name, "tpl2");
    }

    #[test]
    fn test_get_templates_empty_category() {
        let db = Database::new(":memory:").unwrap();
        db.create_template("tpl1", "content1", "general").unwrap();

        let empty = db.get_templates(Some("nonexistent")).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_increment_template_use_count() {
        let db = Database::new(":memory:").unwrap();
        let tpl = db.create_template("counter", "content", "general").unwrap();
        let id = tpl.id.unwrap();
        assert_eq!(tpl.use_count, 0);

        db.increment_template_use_count(id).unwrap();
        db.increment_template_use_count(id).unwrap();
        db.increment_template_use_count(id).unwrap();

        let updated = db.get_template_by_id(id).unwrap().unwrap();
        assert_eq!(updated.use_count, 3);
    }

    #[test]
    fn test_get_template_categories_list() {
        let db = Database::new(":memory:").unwrap();
        db.create_template("tpl1", "c", "work").unwrap();
        db.create_template("tpl2", "c", "general").unwrap();
        db.create_template("tpl3", "c", "email").unwrap();
        db.create_template("tpl4", "c", "work").unwrap();

        let categories = db.get_template_categories_list().unwrap();
        assert_eq!(categories, vec!["email", "general", "work"]);
    }

    #[test]
    fn test_get_template_categories_list_empty() {
        let db = Database::new(":memory:").unwrap();
        let categories = db.get_template_categories_list().unwrap();
        assert!(categories.is_empty());
    }

    #[test]
    fn test_templates_ordered_by_updated_at_desc() {
        let db = Database::new(":memory:").unwrap();
        let t1 = db.create_template("first", "c1", "general").unwrap();
        let _t2 = db.create_template("second", "c2", "general").unwrap();
        // Update first template to make its updated_at more recent
        db.increment_template_use_count(t1.id.unwrap()).unwrap();

        let all = db.get_templates(None).unwrap();
        assert_eq!(all.len(), 2);
        // The first template was updated most recently
        assert_eq!(all[0].name, "first");
        assert_eq!(all[1].name, "second");
    }
}
