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

    Ok(())
}
