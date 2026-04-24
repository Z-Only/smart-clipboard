use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS clipboard_entries (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            content     TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'text',
            category    TEXT NOT NULL DEFAULT 'text',
            hash        TEXT NOT NULL UNIQUE,
            source_app  TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            is_sensitive INTEGER NOT NULL DEFAULT 0,
            pinyin_full TEXT NOT NULL DEFAULT '',
            pinyin_initials TEXT NOT NULL DEFAULT '',
            use_count   INTEGER NOT NULL DEFAULT 1,
            created_at  DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at  DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
            expires_at  DATETIME
        );

        CREATE INDEX IF NOT EXISTS idx_entries_hash ON clipboard_entries(hash);
        CREATE INDEX IF NOT EXISTS idx_entries_category ON clipboard_entries(category);
        CREATE INDEX IF NOT EXISTS idx_entries_created_at ON clipboard_entries(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_entries_favorite ON clipboard_entries(is_favorite);
        ",
    )?;

    // Create FTS5 virtual table if it doesn't exist
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts USING fts5(
            content, category, source_app,
            content='clipboard_entries',
            content_rowid='id'
        );
        ",
    )?;

    // Create triggers to keep FTS in sync
    // We use IF NOT EXISTS pattern by checking sqlite_master
    let trigger_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='trigger' AND name='entries_ai'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !trigger_exists {
        conn.execute_batch(
            "
            CREATE TRIGGER entries_ai AFTER INSERT ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(rowid, content, category, source_app)
                VALUES (new.id, new.content, new.category, new.source_app);
            END;

            CREATE TRIGGER entries_ad AFTER DELETE ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app)
                VALUES ('delete', old.id, old.content, old.category, old.source_app);
            END;

            CREATE TRIGGER entries_au AFTER UPDATE ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app)
                VALUES ('delete', old.id, old.content, old.category, old.source_app);
                INSERT INTO clipboard_fts(rowid, content, category, source_app)
                VALUES (new.id, new.content, new.category, new.source_app);
            END;
            ",
        )?;
    }

    // Enable foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Tag management tables
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tags (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS entry_tags (
            entry_id INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
            tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (entry_id, tag_id)
        );
        ",
    )?;

    // Sync management tables
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS paired_devices (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL DEFAULT '127.0.0.1',
            port INTEGER NOT NULL DEFAULT 23456,
            public_key BLOB,
            local_public_key BLOB,
            shared_secret BLOB,
            last_seen_at DATETIME,
            is_active INTEGER NOT NULL DEFAULT 1,
            paired_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS sync_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_hash TEXT NOT NULL,
            device_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            synced_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (device_id) REFERENCES paired_devices(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_sync_log_hash ON sync_log(entry_hash);
        CREATE INDEX IF NOT EXISTS idx_sync_log_device ON sync_log(device_id);
        ",
    )?;

    let paired_columns: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(paired_devices)")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();
        columns
    };

    if !paired_columns
        .iter()
        .any(|column| column == "local_public_key")
    {
        conn.execute_batch("ALTER TABLE paired_devices ADD COLUMN local_public_key BLOB;")?;
    }

    if !paired_columns.iter().any(|column| column == "fingerprint") {
        conn.execute_batch("ALTER TABLE paired_devices ADD COLUMN fingerprint TEXT;")?;
    }

    // Add source_device column for sync origin tracking
    let entry_columns: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(clipboard_entries)")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();
        columns
    };

    if !entry_columns.iter().any(|c| c == "source_device") {
        conn.execute_batch("ALTER TABLE clipboard_entries ADD COLUMN source_device TEXT;")?;
    }

    if !entry_columns.iter().any(|c| c == "is_encrypted") {
        conn.execute_batch(
            "ALTER TABLE clipboard_entries ADD COLUMN is_encrypted INTEGER NOT NULL DEFAULT 0;",
        )?;
    }

    if !entry_columns.iter().any(|c| c == "pinyin_full") {
        conn.execute_batch(
            "ALTER TABLE clipboard_entries ADD COLUMN pinyin_full TEXT NOT NULL DEFAULT '';",
        )?;
    }

    if !entry_columns.iter().any(|c| c == "pinyin_initials") {
        conn.execute_batch(
            "ALTER TABLE clipboard_entries ADD COLUMN pinyin_initials TEXT NOT NULL DEFAULT '';",
        )?;
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_entries_pinyin_full ON clipboard_entries(pinyin_full);",
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_entries_pinyin_initials ON clipboard_entries(pinyin_initials);"
    )?;

    {
        use crate::storage::search_pinyin::build_pinyin_fields;

        let mut stmt = conn.prepare(
            "SELECT id, content FROM clipboard_entries WHERE pinyin_full = '' AND pinyin_initials = ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        let pending: Vec<(i64, String, String)> = rows
            .filter_map(|r| r.ok())
            .map(|(id, content)| {
                let (full, initials) = build_pinyin_fields(&content);
                (id, full, initials)
            })
            .collect();
        drop(stmt);

        for (id, full, initials) in pending {
            conn.execute(
                "UPDATE clipboard_entries SET pinyin_full = ?1, pinyin_initials = ?2 WHERE id = ?3",
                [full, initials, id.to_string()],
            )?;
        }
    }

    // Template management table
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'general',
            is_favorite INTEGER NOT NULL DEFAULT 0,
            use_count INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_templates_category ON templates(category);
        CREATE INDEX IF NOT EXISTS idx_templates_name ON templates(name);
        ",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_pinyin_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        conn.execute_batch(
            "
            INSERT INTO clipboard_entries (
                content, content_type, category, hash, created_at, updated_at, pinyin_full, pinyin_initials
            ) VALUES (
                '智能剪贴板', 'text', 'text', 'legacy-hash', datetime('now', 'localtime'), datetime('now', 'localtime'), '', ''
            );
            "
        ).unwrap();

        run_migrations(&conn).unwrap();

        let (full, initials): (String, String) = conn
            .query_row(
                "SELECT pinyin_full, pinyin_initials FROM clipboard_entries WHERE hash = 'legacy-hash'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(full, "zhinengjiantieban");
        assert_eq!(initials, "znjtb");
    }
}
