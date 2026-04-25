# 拼音模糊搜索 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Smart Clipboard 增加可维护、可测试的中文拼音全文搜索能力，让中文条目可通过汉字、全拼、首字母和短前缀命中，同时保持现有 API、排序和 LIKE 兜底行为不变。

**Architecture:** 在现有 SQLite FTS5 + LIKE 融合查询结构上做增量升级：扩展 `clipboard_fts` 虚表纳入 `pinyin_full / pinyin_initials`，并在搜索层新增纯函数构造多列前缀 MATCH 表达式。迁移逻辑保持幂等，搜索逻辑根据 MATCH 表达式是否为空动态切换 SQL 形态，避免空 MATCH 报错，同时保留现有 LIKE 子串兜底语义。

**Tech Stack:** Rust, rusqlite, SQLite FTS5, pinyin crate, Vue 3, vue-i18n, Tauri

---

## File Map

- **Modify:** `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/search_pinyin.rs`
  - 责任：新增纯函数 `sanitize_ascii_token`、`sanitize_cjk_token`、`build_fts_match_expr` 及其单元测试。
- **Modify:** `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/migrations.rs`
  - 责任：在 `run_migrations` 中追加幂等的 FTS5 扩列/rebuild 迁移；补充迁移测试。
- **Modify:** `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`
  - 责任：集成新的 MATCH 构造逻辑，并在空 MATCH 时动态退化到 LIKE-only 变体；补充搜索集成测试。
- **Modify:** `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts`
  - 责任：更新中文搜索占位文案。
- **Modify:** `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts`
  - 责任：更新英文搜索占位文案。
- **Verify only:** `/Users/chanyu/AIProjects/smart-clipboard/docs/superpowers/specs/2026-04-25-pinyin-fuzzy-search-design.md`
  - 责任：作为实现对照，不在实现阶段继续扩写需求。

## Task 1: 纯函数搜索表达式与单元测试

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/search_pinyin.rs`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/search_pinyin.rs`

- [ ] **Step 1: 先写失败的单元测试，定义 token 清洗与 MATCH 表达式行为**

```rust
#[test]
fn sanitize_ascii_strips_special_chars() {
    assert_eq!(sanitize_ascii_token("Zn*Jt\""), "znjt");
}

#[test]
fn sanitize_ascii_keeps_alnum_lowercased() {
    assert_eq!(sanitize_ascii_token("Hello123"), "hello123");
}

#[test]
fn sanitize_ascii_drops_cjk() {
    assert_eq!(sanitize_ascii_token("智能"), "");
}

#[test]
fn sanitize_cjk_keeps_hanzi_only() {
    assert_eq!(sanitize_cjk_token("智能ABC123"), "智能");
}

#[test]
fn build_match_expr_empty_when_only_punct() {
    assert_eq!(build_fts_match_expr("!!! ???"), "");
}

#[test]
fn build_match_expr_ascii_multicol_prefix() {
    assert_eq!(
        build_fts_match_expr("znjtb"),
        "(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"
    );
}

#[test]
fn build_match_expr_short_ascii_skipped() {
    assert_eq!(build_fts_match_expr("z"), "");
}

#[test]
fn build_match_expr_cjk_token_uses_content_phrase() {
    assert_eq!(build_fts_match_expr("智能"), "content:\"智能\"");
}

#[test]
fn build_match_expr_mixed_tokens_and_joined() {
    assert_eq!(
        build_fts_match_expr("hello 智能"),
        "(content:\"hello\"* OR pinyin_full:\"hello\"* OR pinyin_initials:\"hello\"*) AND content:\"智能\""
    );
}

#[test]
fn build_match_expr_uppercase_ascii_normalized() {
    assert_eq!(
        build_fts_match_expr("ZNJTB"),
        "(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"
    );
}
```

- [ ] **Step 2: 运行该测试文件，确认新增测试先失败**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test sanitize_ascii_strips_special_chars --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: FAIL，报 `cannot find function 'sanitize_ascii_token'` 或等价未实现错误。

- [ ] **Step 3: 用最小实现新增纯函数**

```rust
use pinyin::ToPinyin;

pub(super) fn sanitize_ascii_token(token: &str) -> String {
    token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(super) fn sanitize_cjk_token(token: &str) -> String {
    token.chars().filter(|c| c.to_pinyin().is_some()).collect()
}

pub fn build_fts_match_expr(keyword: &str) -> String {
    let mut clauses = Vec::new();

    for word in keyword.split_whitespace() {
        let ascii_safe = sanitize_ascii_token(word);
        let han_safe = sanitize_cjk_token(word);

        if ascii_safe.chars().count() >= 2 {
            clauses.push(format!(
                "(content:\"{s}\"* OR pinyin_full:\"{s}\"* OR pinyin_initials:\"{s}\"*)",
                s = ascii_safe
            ));
        }

        if !han_safe.is_empty() {
            clauses.push(format!("content:\"{}\"", han_safe));
        }
    }

    clauses.join(" AND ")
}
```

- [ ] **Step 4: 运行 `search_pinyin.rs` 相关测试，确认全部通过**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test --manifest-path src-tauri/Cargo.toml search_pinyin -- --nocapture
```

Expected: PASS，原有 3 个测试 + 新增 10 个测试全部通过。

- [ ] **Step 5: 提交本任务**

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && git add src-tauri/src/storage/search_pinyin.rs && git commit -m "feat(search): add pinyin fts query builders"
```

## Task 2: FTS5 扩列迁移与迁移测试

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/migrations.rs`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/migrations.rs`

- [ ] **Step 1: 先写失败的迁移测试，锁定 FTS 虚表扩列与幂等行为**

```rust
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
    ).unwrap();

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
        .query_row("SELECT COUNT(*) FROM pragma_table_info('clipboard_fts')", [], |row| row.get(0))
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
```

- [ ] **Step 2: 运行迁移测试，确认先失败**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test fts_rebuild_includes_pinyin_columns --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: FAIL，因当前 `clipboard_fts` 仍只有 3 列，或 `MATCH 'pinyin_full:...'` 报列不存在。

- [ ] **Step 3: 在 `run_migrations` 中追加幂等 FTS 扩列迁移**

```rust
let fts_columns: Vec<String> = {
    let mut stmt = conn.prepare("PRAGMA table_info(clipboard_fts)")?;
    stmt.query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect()
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
```

- [ ] **Step 4: 运行迁移相关测试，确认通过**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test migration --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: PASS，原有 pinyin 回填测试 + 新增 3 个迁移测试全部通过。

- [ ] **Step 5: 提交本任务**

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && git add src-tauri/src/storage/migrations.rs && git commit -m "feat(search): rebuild fts with pinyin columns"
```

## Task 3: 搜索 SQL 改造与集成测试

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/search_pinyin.rs`（仅 import/export 若需要）
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`

- [ ] **Step 1: 先写失败的集成测试，锁定拼音搜索、短词兜底和空 MATCH 降级行为**

```rust
#[cfg(test)]
mod search_integration_tests {
    use super::*;
    use crate::models::{ClipboardEntry, SearchQuery};
    use std::collections::BTreeSet;

    fn make_entry(content: &str, hash: &str) -> ClipboardEntry {
        ClipboardEntry {
            id: None,
            content: content.to_string(),
            content_type: "text".to_string(),
            source: None,
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
            expires_at: None,
            category: "text".to_string(),
            tags: None,
            metadata: None,
            is_favorite: false,
            hash: hash.to_string(),
            use_count: 1,
            source_device: None,
        }
    }

    fn seed_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.insert_entry(&make_entry("Hello World", "h1")).unwrap();
        db.insert_entry(&make_entry("智能剪贴板", "h2")).unwrap();
        db.insert_entry(&make_entry("Hello世界", "h3")).unwrap();
        db
    }

    fn hashes(result: &SearchResult) -> BTreeSet<String> {
        result.entries.iter().map(|e| e.hash.clone()).collect()
    }

    fn base_query(keyword: &str) -> SearchQuery {
        SearchQuery {
            keyword: keyword.to_string(),
            limit: 50,
            offset: 0,
            category: None,
            is_favorite: None,
        }
    }

    #[test]
    fn matches_english_original_text() {
        let db = seed_db();
        let result = db.search(base_query("hello")).unwrap();
        assert_eq!(hashes(&result), ["h1".to_string(), "h3".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_full_pinyin_exact() {
        let db = seed_db();
        let result = db.search(base_query("zhinengjiantieban")).unwrap();
        assert_eq!(hashes(&result), ["h2".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_full_pinyin_prefix() {
        let db = seed_db();
        let result = db.search(base_query("zhineng")).unwrap();
        assert_eq!(hashes(&result), ["h2".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_initials_prefix() {
        let db = seed_db();
        let result = db.search(base_query("znj")).unwrap();
        assert_eq!(hashes(&result), ["h2".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_single_char_via_like_fallback() {
        let db = seed_db();
        let result = db.search(base_query("z")).unwrap();
        assert_eq!(hashes(&result), ["h2".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_mixed_cn_en_contiguous_prefix() {
        let db = seed_db();
        let result = db.search(base_query("helloshi")).unwrap();
        assert_eq!(hashes(&result), ["h3".to_string()].into_iter().collect());
    }

    #[test]
    fn matches_uppercase_initials() {
        let db = seed_db();
        let result = db.search(base_query("ZNJTB")).unwrap();
        assert_eq!(hashes(&result), ["h2".to_string()].into_iter().collect());
    }

    #[test]
    fn no_match_returns_empty() {
        let db = seed_db();
        let result = db.search(base_query("xyz")).unwrap();
        assert!(result.entries.is_empty());
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn punctuation_only_does_not_error() {
        let db = seed_db();
        let result = db.search(base_query("!!!")).unwrap();
        assert_eq!(result.total_count, 3);
    }

    #[test]
    fn total_count_matches_entries_len() {
        let db = seed_db();
        let result = db.search(base_query("hello")).unwrap();
        assert_eq!(result.total_count as usize, result.entries.len());
    }
}
```

- [ ] **Step 2: 运行单个集成测试，确认在实现前失败**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test matches_full_pinyin_exact --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: FAIL，因当前 FTS 未索引拼音列，或查询逻辑尚未构造多列 MATCH。

- [ ] **Step 3: 改造 `search_fts` 使用新 MATCH helper，并在空 MATCH 时降级 SQL**

```rust
use crate::storage::search_pinyin::{build_fts_match_expr, normalize_search_keyword};

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
        let total_count: i64 = conn.query_row(&count_sql, params![fts_query, like_pattern], |row| row.get(0))?;
        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(params![fts_query, like_pattern, query.limit, query.offset], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(SearchResult { entries, total_count })
    } else {
        sql.push_str(" ORDER BY e.created_at DESC LIMIT ?2 OFFSET ?3");
        let total_count: i64 = conn.query_row(&count_sql, params![like_pattern], |row| row.get(0))?;
        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(params![like_pattern, query.limit, query.offset], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(SearchResult { entries, total_count })
    }
}
```

- [ ] **Step 4: 运行搜索相关测试，确认通过**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test search --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: PASS，新增搜索集成测试与相关现有测试通过；`!!!` 查询不报 FTS 语法错误。

- [ ] **Step 5: 提交本任务**

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && git add src-tauri/src/storage/database.rs src-tauri/src/storage/search_pinyin.rs && git commit -m "feat(search): support pinyin fts matching"
```

## Task 4: i18n 文案更新与全量验证

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts`
- Verify: `/Users/chanyu/AIProjects/smart-clipboard/src/components/SearchBar.vue`

- [ ] **Step 1: 更新两份 locale 文案**

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts
search: {
  placeholder: '搜索内容 / 拼音 / 首字母（如 znjtb）',
},
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts
search: {
  placeholder: 'Search text / pinyin / initials (e.g. znjtb)',
},
```

- [ ] **Step 2: 构建前端，确认文案改动未破坏编译**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && npm run build
```

Expected: PASS。

- [ ] **Step 3: 运行 Rust 全量测试与构建，完成最终验证**

Run:

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && cargo test --manifest-path src-tauri/Cargo.toml
cd /Users/chanyu/AIProjects/smart-clipboard && cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: PASS，新增 25 个测试与现有测试全部通过，构建成功。

- [ ] **Step 4: 提交本任务**

```bash
cd /Users/chanyu/AIProjects/smart-clipboard && git add src/i18n/locales/zh-CN.ts src/i18n/locales/en.ts && git commit -m "docs(search): clarify pinyin search placeholder"
```

## Self-Review

- **Spec coverage:**
  - FTS5 扩列、触发器同步、全量 rebuild：Task 2
  - MATCH 表达式 helper 与空 MATCH 降级：Task 1 + Task 3
  - 保持 LIKE 兜底与排序语义：Task 3
  - locale 文案更新：Task 4
  - 测试策略：Task 1/2/3/4 全覆盖
- **Placeholder scan:** 已检查，无 `TODO/TBD/implement later`，每一步都给出明确代码或命令。
- **Type consistency:** helper 名称、测试名、SQL 参数位和规格中的接口名保持一致；混合中英测试使用已修订后的 `helloshi` 语义。

Plan complete and saved to `docs/superpowers/plans/2026-04-25-pinyin-fuzzy-search.md`. Recommended next step: execute it with **Subagent-Driven Development** to minimize interruption and keep quality gates in place.
