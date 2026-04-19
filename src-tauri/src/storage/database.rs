use std::sync::Mutex;

use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection, Result};

use super::migrations;
use super::models::{ClipboardEntry, SearchQuery, SearchResult};

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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.content,
                entry.content_type,
                entry.category,
                entry.hash,
                entry.source_app,
                entry.is_favorite as i32,
                entry.is_sensitive as i32,
                entry.use_count,
                entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                entry.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                entry.expires_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Option<ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at
             FROM clipboard_entries WHERE hash = ?1",
        )?;
        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_entry(row)?))
        } else {
            Ok(None)
        }
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
    ) -> Result<SearchResult> {
        let query = SearchQuery {
            keyword: None,
            category: category.map(|s| s.to_string()),
            is_favorite: None,
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
            "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at
             FROM clipboard_entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_entry(row)?))
        } else {
            Ok(None)
        }
    }
}

fn search_fts(conn: &Connection, keyword: &str, query: &SearchQuery) -> Result<SearchResult> {
    // Escape FTS5 special characters and build match expression
    let fts_query = keyword
        .replace('"', "\"\"")
        .split_whitespace()
        .map(|w| format!("\"{}\"", w))
        .collect::<Vec<_>>()
        .join(" ");

    let mut sql = String::from(
        "SELECT e.id, e.content, e.content_type, e.category, e.hash, e.source_app, e.is_favorite, e.is_sensitive, e.use_count, e.created_at, e.updated_at, e.expires_at
         FROM clipboard_entries e
         INNER JOIN clipboard_fts f ON e.id = f.rowid
         WHERE clipboard_fts MATCH ?1",
    );
    let mut count_sql = String::from(
        "SELECT COUNT(*)
         FROM clipboard_entries e
         INNER JOIN clipboard_fts f ON e.id = f.rowid
         WHERE clipboard_fts MATCH ?1",
    );

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

    sql.push_str(" ORDER BY e.created_at DESC LIMIT ?2 OFFSET ?3");

    let total_count: i64 = conn.query_row(&count_sql, params![fts_query], |row| row.get(0))?;

    let mut stmt = conn.prepare(&sql)?;
    let entries: Vec<ClipboardEntry> = stmt
        .query_map(params![fts_query, query.limit, query.offset], |row| {
            row_to_entry(row)
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SearchResult {
        entries,
        total_count,
    })
}

fn get_entries_inner(conn: &Connection, query: &SearchQuery) -> Result<SearchResult> {
    let mut sql = String::from(
        "SELECT id, content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at
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
        .query_map(params![query.limit, query.offset], |row| row_to_entry(row))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SearchResult {
        entries,
        total_count,
    })
}

fn row_to_entry(row: &rusqlite::Row) -> Result<ClipboardEntry> {
    let created_str: String = row.get(9)?;
    let updated_str: String = row.get(10)?;
    let expires_str: Option<String> = row.get(11)?;

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

        let result = db.get_entries(50, 0, Some("url")).unwrap();
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

        let page1 = db.get_entries(3, 0, None).unwrap();
        assert_eq!(page1.entries.len(), 3);
        assert_eq!(page1.total_count, 10);

        let page2 = db.get_entries(3, 3, None).unwrap();
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
}
