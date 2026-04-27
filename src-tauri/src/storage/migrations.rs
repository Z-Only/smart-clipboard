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

        let fts_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(clipboard_fts)")?;
            let cols = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            cols
        };

        if !fts_columns.iter().any(|c| c == "pinyin_full") {
            conn.execute_batch(
            r#"
            DROP TRIGGER IF EXISTS entries_ai;
            DROP TRIGGER IF EXISTS entries_ad;
            DROP TRIGGER IF EXISTS entries_au;

            DROP TABLE IF EXISTS clipboard_fts;

            CREATE VIRTUAL TABLE clipboard_fts USING fts5(
                content, category, source_app, pinyin_full, pinyin_initials,
                content='clipboard_entries',
                content_rowid='id'
            );

            CREATE TRIGGER entries_ai AFTER INSERT ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
                VALUES (new.id, new.content, new.category, new.source_app, new.pinyin_full, new.pinyin_initials);
            END;

            CREATE TRIGGER entries_ad AFTER DELETE ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app, pinyin_full, pinyin_initials)
                VALUES ('delete', old.id, old.content, old.category, old.source_app, old.pinyin_full, old.pinyin_initials);
            END;

            CREATE TRIGGER entries_au AFTER UPDATE ON clipboard_entries BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app, pinyin_full, pinyin_initials)
                VALUES ('delete', old.id, old.content, old.category, old.source_app, old.pinyin_full, old.pinyin_initials);
                INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
                VALUES (new.id, new.content, new.category, new.source_app, new.pinyin_full, new.pinyin_initials);
            END;

            INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
            SELECT id, content, category, source_app, pinyin_full, pinyin_initials FROM clipboard_entries;
            "#,
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

    // Smart search: cluster and tag suggestion tables
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS entry_clusters (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            label       TEXT NOT NULL,
            created_at  DATETIME DEFAULT (datetime('now', 'localtime')),
            updated_at  DATETIME DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS entry_cluster_members (
            cluster_id  INTEGER NOT NULL REFERENCES entry_clusters(id) ON DELETE CASCADE,
            entry_id    INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
            score       REAL NOT NULL DEFAULT 0.0,
            PRIMARY KEY (cluster_id, entry_id)
        );

        CREATE INDEX IF NOT EXISTS idx_cluster_members_entry
            ON entry_cluster_members(entry_id);

        CREATE TABLE IF NOT EXISTS tag_suggestions (
            entry_id    INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
            tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            confidence  REAL NOT NULL DEFAULT 0.0,
            created_at  DATETIME DEFAULT (datetime('now', 'localtime')),
            PRIMARY KEY (entry_id, tag_id)
        );
        ",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_rebuild_includes_pinyin_columns() {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                content_type TEXT NOT NULL DEFAULT 'text',
                category TEXT NOT NULL DEFAULT 'text',
                hash TEXT NOT NULL UNIQUE,
                source_app TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                is_sensitive INTEGER NOT NULL DEFAULT 0,
                pinyin_full TEXT NOT NULL DEFAULT '',
                pinyin_initials TEXT NOT NULL DEFAULT '',
                use_count INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'localtime')),
                expires_at DATETIME
            );

            CREATE VIRTUAL TABLE clipboard_fts USING fts5(
                content, category, source_app,
                content='clipboard_entries', content_rowid='id'
            );

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
            "#,
        )
        .unwrap();

        conn.execute(
            "INSERT INTO clipboard_entries (content, content_type, category, hash, pinyin_full, pinyin_initials, created_at, updated_at) VALUES (?1, 'text', 'text', 'legacy-hash', ?2, ?3, datetime('now', 'localtime'), datetime('now', 'localtime'))",
            rusqlite::params!["智能剪贴板", "zhinengjiantieban", "znjtb"],
        ).unwrap();

        run_migrations(&conn).unwrap();

        let columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(clipboard_fts)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };

        assert!(columns.iter().any(|c| c == "pinyin_full"));
        assert!(columns.iter().any(|c| c == "pinyin_initials"));

        let rowid_by_full: i64 = conn
            .query_row(
                "SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH 'pinyin_full:zhineng*'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let rowid_by_initials: i64 = conn
            .query_row(
                "SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH 'pinyin_initials:znjtb'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(rowid_by_full, rowid_by_initials);
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let column_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('clipboard_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(column_count, 5);
    }

    #[test]
    fn post_migration_insert_updates_fts_pinyin_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO clipboard_entries (content, content_type, category, hash, pinyin_full, pinyin_initials, created_at, updated_at) VALUES (?1, 'text', 'text', 'new-hash', ?2, ?3, datetime('now', 'localtime'), datetime('now', 'localtime'))",
            rusqlite::params!["世界", "shijie", "sj"],
        ).unwrap();

        let rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH 'pinyin_full:shijie*'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(rowid > 0);
    }

    #[test]
    fn smart_search_tables_created() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify entry_clusters table exists
        let cluster_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='entry_clusters'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(cluster_exists, "entry_clusters table should exist");

        // Verify entry_cluster_members table exists
        let members_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='entry_cluster_members'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(members_exists, "entry_cluster_members table should exist");

        // Verify tag_suggestions table exists
        let suggestions_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='tag_suggestions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(suggestions_exists, "tag_suggestions table should exist");

        // Verify idx_cluster_members_entry index exists
        let index_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_cluster_members_entry'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(index_exists, "idx_cluster_members_entry index should exist");
    }

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
