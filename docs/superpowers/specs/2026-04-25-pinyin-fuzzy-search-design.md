# 拼音模糊搜索 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-25
- **版本**：v1.0
- **状态**：Reviewed & Refined（方案 A）

## 1. 背景

Smart Clipboard 的全文搜索目前基于 SQLite FTS5，索引列为 `content / category / source_app`。对于中文条目，用户只能输入原文汉字才能命中，无法通过拼音（全拼）或首字母缩写快速检索，体验与日常"拼音输入习惯"不符。

## 2. 目标

让用户在搜索栏输入以下任何形式都能命中中文条目"智能剪贴板"：

- 原文：`智能` / `剪贴板`
- 全拼：`zhineng` / `zhinengjiantieban` / `zhi`（前缀）
- 首字母缩写：`znjtb` / `znj`（前缀）
- 极短输入：`zn`、`z`（通过 LIKE 兜底）
- 大小写无关：`ZNJTB` 等同于 `znjtb`
- 混合中英：`hello sj` 命中 `Hello世界`

**非目标**：

- 不做同音字 / 音近字扩展（如 `zhi` 不需映射到 `chi`）
- 不做拼音分词消歧（多音字统一取第一个读音，由 `pinyin` crate 默认行为决定）
- 不改变前端 API；不引入新的 Tauri 命令
- 不处理图片条目（`content_type = 'image'` 的 content 是文件路径，其拼音生成已由现有 `build_pinyin_fields` 处理，保持原行为）

## 3. 现状（Already Done）—— 基于真实代码核对

本次需求的前置工作大量已完成，**这是增量迭代**，不是从零开始。以下描述已对照实际源码（非假设）：

### 3.1 已实现能力

| 模块                       | 位置                                                             | 现状（实测）                                                                                                                                                                                                                                                           |
| -------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| pinyin crate               | `src-tauri/Cargo.toml`                                           | `pinyin = "0.11"`，启用 `plain` feature                                                                                                                                                                                                                                |
| 拼音生成                   | `src-tauri/src/storage/search_pinyin.rs::build_pinyin_fields`    | 输入字符串 → `(pinyin_full, pinyin_initials)`。例：`"智能剪贴板"` → `("zhinengjiantieban", "znjtb")`；`"Hello世界123"` → `("helloshijie123", "hellosj123")`                                                                                                            |
| 关键词归一化               | `search_pinyin.rs::normalize_search_keyword`                     | **保留汉字原样**（不转拼音），把 ASCII 转小写，去非字母数字非拼音字符。例：`" ZN-JT!!  "` → `"znjt"`；`"智能 JT"` → `"智能 jt"`                                                                                                                                        |
| 数据表列                   | `clipboard_entries.pinyin_full / pinyin_initials`                | 已加列 + B-Tree 索引 `idx_entries_pinyin_full / idx_entries_pinyin_initials`，DEFAULT `''`                                                                                                                                                                             |
| 老库列回填                 | `storage/migrations.rs`                                          | 若列为空则调用 `build_pinyin_fields` 回填（已存在，无需改动）                                                                                                                                                                                                          |
| 写入链路                   | `Database::insert_entry_with_encrypted_flag` (database.rs:44-73) | 插入时调用 `build_pinyin_fields`，写入 `pinyin_full / pinyin_initials` 两列                                                                                                                                                                                            |
| 既有单元测试               | `search_pinyin.rs::tests`                                        | 已覆盖 3 个场景：中文生成、中英混合、归一化                                                                                                                                                                                                                            |
| **既有搜索 SQL（关键！）** | `database.rs::search_fts` (行 760-828)                           | 已是 **FTS MATCH + 三列 LIKE OR 融合** 的单路径 SQL：`WHERE (id IN (SELECT rowid FROM clipboard_fts WHERE MATCH ?1) OR LOWER(content) LIKE ?2 OR pinyin_full LIKE ?2 OR pinyin_initials LIKE ?2)`。意味着**拼音 LIKE 搜索已经生效**；欠缺的只是 FTS MATCH 侧没用拼音列 |
| 既有 FTS5 虚表             | `migrations.rs::run_migrations` (行 33-74)                       | `CREATE VIRTUAL TABLE clipboard_fts USING fts5(content, category, source_app, content='clipboard_entries', content_rowid='id')` —— **只有 3 列，无拼音列**。三个触发器 `entries_ai/ad/au` 同步这 3 列                                                                  |

### 3.2 本次真正要做的增量

1. **FTS5 虚表扩列**：把 `pinyin_full / pinyin_initials` 加进 `clipboard_fts` 并 rebuild（FTS5 不支持 ALTER ADD COLUMN，须 drop+recreate）
2. **同步改造三个触发器**：`entries_ai / entries_ad / entries_au` 的 INSERT 列从 3 列扩到 5 列
3. **改造 FTS MATCH 表达式构造**（`search_fts` 里的 `fts_query` 部分）：从简单"双引号包词、空格连接"升级为"每个 token 生成 `(content:w* OR pinyin_full:w* OR pinyin_initials:w*)`"的多列前缀查询。**LIKE 三列 OR 保持不变**（现状已在兜底短关键词，本次不改变该行为；但需在规格中明确它是“子串匹配”而非“压缩后跨段跳跃匹配”）
4. **处理 FTS MATCH 可能为空的情况**：当 token 全部过短或纯标点时，动态去掉 SQL 里的 `id IN (...MATCH ?1)` 子句，只保留 LIKE OR 链，避免 FTS5 报语法错
5. **前端 placeholder 提示**：`SearchBar.vue` 模板**已经**走 `$t('search.placeholder')`（行 18 已核对），本次**只改 locale 文案**，不动 .vue 文件

### 3.3 不改动清单（明确边界）

- **不改** `build_pinyin_fields / normalize_search_keyword` 的现有行为（已有测试保护）
- **不改** `row_to_entry` 的 SELECT 字段顺序和列索引（保持 13 列，不把 `pinyin_*` 读回 struct）
- **不改** `clipboard_entries` 表结构、索引、回填逻辑
- **不改** `insert_entry_with_encrypted_flag` 的拼音写入逻辑
- **不改** `.worktrees/feature/runtime-integration-tests/` 下的旧副本文件（这是隔离的 git worktree，与主分支独立）
- **不改** `get_entries_inner`（无关键词分支）
- **不改** `SearchBar.vue` 模板结构（只动 locale 文案）

## 4. 技术方案（方案 A）

### 4.1 总体思路

**保留现有的"FTS MATCH + 三列 LIKE OR 融合"单路径 SQL 结构**，只做两个原地升级：

1. 把 `pinyin_full / pinyin_initials` 加入 FTS5 虚表索引（含触发器与 rebuild）
2. 把 `search_fts` 里构造 `fts_query` 字符串的逻辑从"简单分词加双引号"升级为"每个 ASCII token 生成 `(content:w* OR pinyin_full:w* OR pinyin_initials:w*)`，汉字 token 生成 `content:"汉字"`"

LIKE OR 三列不动。短关键词（1 字符）自然通过现有 LIKE OR 分支兜底，无需新增"LIKE-only 分支"。当 FTS MATCH 经过构造后仍为空字符串（如 keyword 全是标点），动态改写 SQL 去掉 MATCH 子句，只保留 LIKE OR，避免 FTS5 对空 MATCH 报语法错。

### 4.2 数据模型

**保持不变：**

- `clipboard_entries` 表结构（已有 `pinyin_full / pinyin_initials`，DEFAULT `''`）
- B-Tree 索引 `idx_entries_pinyin_full / idx_entries_pinyin_initials`（LIKE 仍受益）
- `row_to_entry` 的 SELECT 字段集合与列索引（保持 13 列）

**变更：**

- `clipboard_fts` 虚表从 3 列（`content, category, source_app`）扩到 5 列，追加 `pinyin_full, pinyin_initials`
- 三个触发器 `entries_ai / entries_ad / entries_au` 的 INSERT 列表从 3 列同步扩到 5 列

（等价 SQL 形式见 §4.3 代码片段，避免与迁移说明重复。）

### 4.3 迁移策略

FTS5 虚表不支持 `ALTER TABLE ... ADD COLUMN`，采用 **drop-and-rebuild** 策略，且要求幂等 + 向后兼容。

#### 4.3.1 幂等判断依据（已实测）

`PRAGMA table_info('clipboard_fts')` 对 FTS5 虚表**正常返回列列表**（实测 `CREATE VIRTUAL TABLE t USING fts5(a,b,c); PRAGMA table_info(t);` 输出 `0|a||0||0 / 1|b||0||0 / 2|c||0||0`），故判断条件可靠：收集第 2 列为列名的 `Vec<String>`，若已包含 `pinyin_full` 则迁移无需执行。

#### 4.3.2 严格执行顺序（关键！）

SQLite 里 `entries_ai/ad/au` 触发器**定义在 `clipboard_entries` 表上**（非 FTS 虚表上），`DROP TABLE clipboard_fts` 不会自动级联 drop 触发器；若旧触发器残留、且其 INSERT 列数（3 列）与新虚表（5 列）不符，下一次 INSERT 就会报 `table clipboard_fts has 5 columns but 3 values were supplied`。因此必须按严格顺序执行：

```
function migrate_fts_to_include_pinyin(conn):
    # 步骤 0：幂等早退
    cols = PRAGMA table_info('clipboard_fts')
    if cols 已经包含 pinyin_full: return

    # 步骤 1：先删旧触发器（必须在删 FTS 虚表前完成）
    DROP TRIGGER IF EXISTS entries_ai
    DROP TRIGGER IF EXISTS entries_ad
    DROP TRIGGER IF EXISTS entries_au

    # 步骤 2：删旧 FTS 虚表（content-less 虚表删除不影响 clipboard_entries 基表）
    DROP TABLE IF EXISTS clipboard_fts

    # 步骤 3：建新 5 列 FTS 虚表
    CREATE VIRTUAL TABLE clipboard_fts USING fts5(
        content, category, source_app, pinyin_full, pinyin_initials,
        content='clipboard_entries', content_rowid='id')

    # 步骤 4：建新 5 列触发器（三个，每个都写 5 列）
    CREATE TRIGGER entries_ai AFTER INSERT ON clipboard_entries
        BEGIN
            INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
            VALUES (new.id, new.content, new.category, new.source_app, new.pinyin_full, new.pinyin_initials);
        END
    CREATE TRIGGER entries_ad AFTER DELETE ON clipboard_entries
        BEGIN
            INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app, pinyin_full, pinyin_initials)
            VALUES ('delete', old.id, old.content, old.category, old.source_app, old.pinyin_full, old.pinyin_initials);
        END
    CREATE TRIGGER entries_au AFTER UPDATE ON clipboard_entries
        BEGIN
            INSERT INTO clipboard_fts(clipboard_fts, rowid, content, category, source_app, pinyin_full, pinyin_initials)
            VALUES ('delete', old.id, old.content, old.category, old.source_app, old.pinyin_full, old.pinyin_initials);
            INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
            VALUES (new.id, new.content, new.category, new.source_app, new.pinyin_full, new.pinyin_initials);
        END

    # 步骤 5：全量 rebuild FTS 索引
    INSERT INTO clipboard_fts(rowid, content, category, source_app, pinyin_full, pinyin_initials)
    SELECT id, content, category, source_app, pinyin_full, pinyin_initials FROM clipboard_entries
```

#### 4.3.3 在 `run_migrations` 中的插入位置

该迁移步骤必须放在 `migrations.rs::run_migrations` 当前**倒数第二段**（templates 表建表语句之前）—— 即紧跟在"回填 `pinyin_full/pinyin_initials` 空值"代码块之后，确保 rebuild 时列已有值。同时现存 `CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts` 和 `CREATE TRIGGER` 段落保持不变（首次全新部署时它们建的是旧 3 列结构，然后立刻被本迁移步骤 drop 并 recreate 成 5 列 —— 略有冗余但保证迁移路径对"全新库"和"老版本库"都幂等正确）。

#### 4.3.4 性能与回滚

**性能评估**：默认上限 5000 条，DROP + CREATE + 5000 行 INSERT 全部在单事务里，实测毫秒级；WAL 模式下不阻塞读。

**回滚性**：FTS 是派生索引。若代码 revert，下次启动因 FTS 列数不匹配会再次触发 drop-rebuild，业务数据不受影响。

### 4.4 搜索逻辑

> **本节修订说明（2026-04-25 自审补充）**：原稿对 `LIKE` 兜底的命中示例有一处语义写得过满，容易让实现者误以为现有 `%keyword%` 模式支持“跳字母压缩匹配”。实际上当前设计保持 `normalize_search_keyword(keyword)` + `%...%` 的普通子串匹配，因此规格中所有示例必须与这一语义保持一致。

**总入口 `Database::search` 不变**（行 107-117）：keyword 为空或全空白时走 `get_entries_inner`，否则走 `search_fts`。

`search_fts` 保持"**FTS MATCH ? 子查询 OR 三列 LIKE ?**"的单路径 SQL 结构（见 §3.1 的现状 SQL），只做三处原地升级：

1. 用新 helper `build_fts_match_expr(keyword)` 生成 `?1` 的值，替换现有 "把每个词加双引号、空格连接" 的简单做法
2. 当 `build_fts_match_expr` 返回空字符串时（例如 keyword 仅包含标点 `!!!`），**动态构造 SQL**：去掉 `e.id IN (SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH ?1)` 这一支路和对应参数，只保留 LIKE OR 三条
3. LIKE 表达式和 `like_pattern` 的构造逻辑（`normalize_search_keyword` + `.replace(' ', "%")` 外包 `%..%`）保持不变

#### 4.4.1 新 helper `build_fts_match_expr(keyword: &str) -> String`

位置：`src-tauri/src/storage/search_pinyin.rs`，与 `build_pinyin_fields / normalize_search_keyword` 同模块（纯函数，无 DB 依赖，便于单元测试）。

伪代码（注意：不调用 `normalize_search_keyword`，避免破坏对汉字的大小写/标点处理与 FTS5 unicode61 分词的配合）：

```
fn build_fts_match_expr(keyword: &str) -> String:
    clauses: Vec<String> = []
    for word in keyword.split_whitespace():        # 按空格切 token，保留汉字原样
        ascii_safe = sanitize_ascii_token(word)    # 见 §4.4.2，只保留 [a-z0-9] 并转小写
        han_safe   = sanitize_cjk_token(word)      # 见 §4.4.2，只保留汉字（to_pinyin().is_some()）

        # ASCII 子 token ≥ 2 字符：三列前缀 MATCH（覆盖原文英文 + 拼音全拼 + 首字母）
        if ascii_safe.chars().count() >= 2:
            clauses.push(format!(
                "(content:\"{s}\"* OR pinyin_full:\"{s}\"* OR pinyin_initials:\"{s}\"*)",
                s = ascii_safe))
        # 注意：ascii_safe 长度 0 或 1 时都不加入 MATCH，由调用方 LIKE OR 兜底

        # CJK 子 token 非空：content 列精确短语 MATCH（FTS5 unicode61 把连续汉字当独立 token）
        # ascii_safe 与 han_safe 的输出字符集天然不相交（前者 ASCII，后者汉字），
        # 故无需比较二者是否相等；只要 han_safe 非空就加入 content 精确匹配。
        # 当一个 word 同时含 ASCII 和汉字（如 "abc智能"）时，会分别 push 两个 clause，
        # 在 §4.4.1 末尾用 AND 连接，语义为"该条记录必须同时匹配 abc 前缀 AND 含'智能'"，
        # 这符合用户输入复合词时的直觉。
        if !han_safe.is_empty():
            clauses.push(format!("content:\"{}\"", han_safe))

    clauses.join(" AND ")   # 空 Vec → 返回空字符串 ""
```

**关键决策**：

- **不 normalize keyword**：`normalize_search_keyword` 把 "ZN-JT" 变成 "znjt"、把 "智能 JT" 变成 "智能 jt"，但我们只需要做"按空格分词 + 逐 token 过滤非法字符"，原始 token 已够用，避免行为偏差。
- **ASCII token 大小写**：`sanitize_ascii_token` 内部统一 `to_ascii_lowercase()`，保证 `ZNJTB` 与 pinyin 列里的 `znjtb` 匹配（`pinyin_full / pinyin_initials` 列在写入时已是小写）。
- **长度判断用 `chars().count()`**：避免 `str::len()` 在汉字上返回字节数的 bug；`sanitize_ascii_token` 输出纯 ASCII，`.len()` 与 `chars().count()` 等价，但统一用 `chars().count()` 保持风格一致。
- **多 clause 用 AND 连接**：符合用户"缩小范围"的直觉（FTS5 中 AND 对单条记录要求全部子句成立）。
- **"词内复合"语义**：一个 word 含 ASCII + 汉字时（如 `abc智能`），产生两条 clause 并以 AND 连接，语义上要求该条记录同时包含 `abc` 前缀和"智能"短语 —— 实现简单且符合直觉，不特别处理。

#### 4.4.2 新 helper `sanitize_ascii_token` / `sanitize_cjk_token`

位置：同 `search_pinyin.rs`，私有可见（`pub(super)`）。

```rust
/// 只保留 [a-z0-9]，ASCII 字母统一小写。
/// 用于匹配 content / pinyin_full / pinyin_initials 三列中的拉丁数字部分。
fn sanitize_ascii_token(token: &str) -> String:
    token.chars()
         .filter(|c| c.is_ascii_alphanumeric())
         .map(|c| c.to_ascii_lowercase())
         .collect()

/// 只保留 pinyin crate 能识别的字符（汉字）。
/// 用于汉字 token 的 content 列精确匹配。
fn sanitize_cjk_token(token: &str) -> String:
    token.chars()
         .filter(|c| c.to_pinyin().is_some())
         .collect()
```

**为什么不需要双引号转义函数**：外层 `build_fts_match_expr` 在拼接时已经把 `{safe}` 放进 `"..."` 里；而 `sanitize_*` 保证里面只出现 `[a-z0-9]` 或汉字，不可能含有 `"` / `*` / `:` 等 FTS5 保留字符，**天然防注入**。这比"先生成任意字符串再做 escape"更简单、更安全。

#### 4.4.3 `search_fts` 改造（位置：`database.rs:761`）

改造点（保持其他逻辑不变）：

| 原                                                                                                           | 改为                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `let fts_query = keyword.replace('"', "\"\"").split_whitespace().map(\|w\| format!("\"{}\"", w)).join(" ");` | `let fts_query = build_fts_match_expr(keyword);`                                                                                                                                                                                                                                                                        |
| 固定 SQL 含 `e.id IN (SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH ?1) OR ...`                  | 根据 `fts_query.is_empty()` 分两种形态动态拼接：<br>**情况 A**（非空）：保持现状 SQL 与参数 `params![fts_query, like_pattern, ...]`<br>**情况 B**（空）：去掉 MATCH 子查询与 `?1` 参数，`WHERE (LOWER(e.content) LIKE ?1 OR e.pinyin_full LIKE ?1 OR e.pinyin_initials LIKE ?1)`，参数改为 `params![like_pattern, ...]` |
| `normalize_search_keyword(keyword).is_empty()` 时提前返回 `get_entries_inner`                                | 保留现状（兼容原有空归一化短路行为；此时 MATCH 与 LIKE 都无意义）                                                                                                                                                                                                                                                       |

排序仍是 `ORDER BY e.created_at DESC`（现状未用 BM25，本次也不引入，避免扩大改动范围）。

LIKE 部分的 `like_pattern` 构造保持不变（`format!("%{}%", normalize_search_keyword(keyword).replace(' ', "%"))`），这是现状里让"短关键词 / 1 字符 / 混合大小写"能命中的兜底关键，不需要新增独立 `search_like` 函数。

#### 4.4.4 已删除

（原稿曾提议新增 `search_like` 独立分支 —— 基于 §3.1 核对后发现现有 `search_fts` 的 LIKE OR 链**已经**承担了短关键词兜底职责，新增独立分支会重复逻辑并改变排序语义，故本节作废。）

### 4.5 前端改动

**不改** `src/components/SearchBar.vue`（第 18 行已使用 `:placeholder="$t('search.placeholder')"`）。

**只改两份 locale 文件**（路径已实测确认，非 `src/i18n/index.ts` 里嵌入）：

| 文件                        | 键路径               | 现状（实测）            | 改为                                             |
| --------------------------- | -------------------- | ----------------------- | ------------------------------------------------ |
| `src/i18n/locales/zh-CN.ts` | `search.placeholder` | `'搜索剪贴板...'`       | `'搜索内容 / 拼音 / 首字母（如 znjtb）'`         |
| `src/i18n/locales/en.ts`    | `search.placeholder` | `'Search clipboard...'` | `'Search text / pinyin / initials (e.g. znjtb)'` |

`src/i18n/index.ts` 只是聚合入口（756 B，实测只做 `createI18n` 装配），不含 placeholder 字面量，**不改**。

无其他前端改动。搜索调用链 `useSearch → invoke('search_entries') → Database::search` 对用户透明。

## 5. 组件边界与接口

| 组件                               | 职责                                                                                  | 依赖                        |
| ---------------------------------- | ------------------------------------------------------------------------------------- | --------------------------- |
| `search_pinyin.rs`                 | 拼音生成 / 关键词归一化 / FTS MATCH 表达式构造 / token 白名单过滤（纯函数）           | `pinyin` crate              |
| `migrations.rs::run_migrations`    | 追加"FTS 虚表扩列 + 触发器 5 列重建 + 全量 rebuild"幂等迁移步骤                       | `rusqlite`                  |
| `database.rs::search_fts`          | 原地调用 `build_fts_match_expr` 生成 MATCH 值；根据是否为空动态拼接 SQL；参数随之变化 | `search_pinyin`、`rusqlite` |
| `SearchBar.vue` + `i18n/locales/*` | UI 占位文案更新为"原文 / 拼音 / 首字母"                                               | `vue-i18n`                  |

独立性验证：

- `search_pinyin` 新增函数无 DB 依赖，可纯单元测试
- `migrations` 用 `Connection::open_in_memory()` 可完整跑一遍迁移 + 查 FTS 虚表列
- `search_fts` 改造后行为由 in-memory `Database::new(":memory:")` + 插入固定数据集 + `db.search(...)` 断言覆盖
- 前端改动只涉及 i18n 键值，不需要新增组件测试

## 6. 错误处理

- **FTS5 MATCH 语法错误**：由 `sanitize_ascii_token / sanitize_cjk_token` 的白名单（只保留 `[a-z0-9]` 或 `ch.to_pinyin().is_some()` 的字符）保证放入 `"..."` 的片段不含 FTS5 保留字符（`"` `*` `:` `^` `-`），从源头消除语法错。若仍意外失败，`rusqlite::Error` 会冒泡到上层 Tauri command 并返回 `Err(String)`，与现状一致
- **MATCH 为空字符串**：按 §4.4.3 走"情况 B"去 MATCH 子查询形态，不传入空字符串（SQLite FTS5 对空 MATCH 会报 "fts5: syntax error"）
- **迁移失败**：`run_migrations` 返回 `Err`，`Database::new` 把错传播到 `lib.rs::run` 里的 `.expect("Failed to initialize database")` 并 panic。此行为与现有迁移一致，不引入新的错误路径
- **LIKE 兜底的空结果**是正常业务结果，不是错误

## 7. 测试策略（TDD）

所有新增测试必须先写失败测试、观察失败原因、再写最小实现通过，遵循 `test-driven-development` skill 的红绿重构节奏。

### 7.1 `search_pinyin.rs::tests` 新增单元测试

函数签名最终形态：`build_fts_match_expr(&str) -> String`、`sanitize_ascii_token(&str) -> String`、`sanitize_cjk_token(&str) -> String`。

| #   | 测试名                                           | 输入           | 期望（基于 §4.4 设计）                                                                                |
| --- | ------------------------------------------------ | -------------- | ----------------------------------------------------------------------------------------------------- |
| U1  | `sanitize_ascii_strips_special_chars`            | `"Zn*Jt\""`    | `"znjt"`                                                                                              |
| U2  | `sanitize_ascii_keeps_alnum_lowercased`          | `"Hello123"`   | `"hello123"`                                                                                          |
| U3  | `sanitize_ascii_drops_cjk`                       | `"智能"`       | `""`                                                                                                  |
| U4  | `sanitize_cjk_keeps_hanzi_only`                  | `"智能ABC123"` | `"智能"`                                                                                              |
| U5  | `build_match_expr_empty_when_only_punct`         | `"!!! ???"`    | `""`                                                                                                  |
| U6  | `build_match_expr_ascii_multicol_prefix`         | `"znjtb"`      | `"(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"`                      |
| U7  | `build_match_expr_short_ascii_skipped`           | `"z"`          | `""`（1 字符不加入 MATCH，由调用方 LIKE 兜底）                                                        |
| U8  | `build_match_expr_cjk_token_uses_content_phrase` | `"智能"`       | `"content:\"智能\""`                                                                                  |
| U9  | `build_match_expr_mixed_tokens_and_joined`       | `"hello 智能"` | `"(content:\"hello\"* OR pinyin_full:\"hello\"* OR pinyin_initials:\"hello\"*) AND content:\"智能\""` |
| U10 | `build_match_expr_uppercase_ascii_normalized`    | `"ZNJTB"`      | `"(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"`                      |

### 7.2 `migrations.rs::tests` 新增迁移测试

使用 `Connection::open_in_memory()`：

- **`fts_rebuild_includes_pinyin_columns`**（模拟升级老库）
  1. 手动建老版本 schema：`clipboard_entries` 全表 + 老 3 列 `clipboard_fts` + 老 3 列 `entries_ai/ad/au` 触发器（复制现有 migrations.rs 里的建表/建触发器 SQL 即可）
  2. 通过 `INSERT INTO clipboard_entries` 写入 `"智能剪贴板"`（此时老 trigger 会把 3 列写进老 FTS）
  3. 调用 `run_migrations(&conn)` —— 新增迁移步骤会 drop 并 recreate FTS 5 列 + 5 列触发器 + rebuild
  4. 断言：
     - `PRAGMA table_info(clipboard_fts)` 返回 5 列，其中包含 `pinyin_full`、`pinyin_initials`
     - `SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH 'pinyin_full:zhineng*'` 返回前面插入的 rowid
     - `SELECT rowid FROM clipboard_fts WHERE clipboard_fts MATCH 'pinyin_initials:znjtb'` 也能命中

- **`migration_is_idempotent`**
  1. 连续调用 `run_migrations(&conn)` 两次
  2. 断言：不报错；FTS 虚表列数仍为 5；能正常做一次 `MATCH 'pinyin_initials:znjtb'` 查询

- **`post_migration_insert_updates_fts_pinyin_columns`**（回归新 trigger）
  1. 调用 `run_migrations`
  2. `INSERT INTO clipboard_entries (... pinyin_full='shijie', pinyin_initials='sj' ...)` 写一条新 entry
  3. 断言 `MATCH 'pinyin_full:shijie*'` 能命中该新 entry rowid（证明新 `entries_ai` 触发器写了 5 列）

### 7.3 `database.rs` 搜索集成测试

**自审修正**：原稿中的 `T8` 用例把 `hellosj` 作为可命中 `Hello世界` 的示例，这与当前保留的 `LIKE` 语义不一致。`Hello世界` 的 `pinyin_full` 为 `helloshijie`，其中并不包含连续子串 `hellosj`；而 FTS 前缀匹配同样不会把 `hello` 与 `shijie` 自动压缩成 `hellosj`。因此该示例必须改为“可连续前缀命中的输入”，例如 `helloshi`。

**放置位置**：`src-tauri/src/storage/database.rs` 末尾的 `#[cfg(test)] mod search_integration_tests`，避免新建独立文件（减少 module 声明改动）。

**测试夹具**：在 `Database::new(":memory:")` 之后，通过公开 API `db.insert_entry(&entry)` 插入三条：

| id  | content       | content_type | category | hash |
| --- | ------------- | ------------ | -------- | ---- |
| 1   | `Hello World` | text         | text     | `h1` |
| 2   | `智能剪贴板`  | text         | text     | `h2` |
| 3   | `Hello世界`   | text         | text     | `h3` |

所有 `SearchQuery` 的 `limit=50, offset=0, category=None, is_favorite=None`（除 T11 外）。

| #   | 用例名                                  | keyword                            | 期望命中 hash 集合                                                                                                                         |
| --- | --------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| T1  | `matches_english_original_text`         | `hello`                            | `{h1, h3}`（FTS `content:"hello"*` 同时命中 "Hello World" 和 "Hello世界"）                                                                 |
| T2  | `matches_full_pinyin_exact`             | `zhinengjiantieban`                | `{h2}`                                                                                                                                     |
| T3  | `matches_full_pinyin_prefix`            | `zhineng`                          | `{h2}`（pinyin_full 列前缀）                                                                                                               |
| T4  | `matches_full_pinyin_short_prefix`      | `zhi`                              | `{h2}`                                                                                                                                     |
| T5  | `matches_initials_exact`                | `znjtb`                            | `{h2}`（pinyin_initials 列前缀）                                                                                                           |
| T6  | `matches_initials_prefix`               | `znj`                              | `{h2}`                                                                                                                                     |
| T7  | `matches_single_char_via_like_fallback` | `z`                                | `{h2}`（build_fts_match_expr 返回 `""` → 走"情况 B"仅 LIKE；`pinyin_initials LIKE '%z%'` 命中 `znjtb`）                                    |
| T8  | `matches_mixed_cn_en_contiguous_prefix` | `helloshi`                         | `{h3}`（`Hello世界` 的 `pinyin_full` 为 `helloshijie`，连续前缀 `helloshi` 可通过 `pinyin_full` 前缀 MATCH / LIKE 命中）                   |
| T9  | `matches_uppercase_initials`            | `ZNJTB`                            | `{h2}`（sanitize_ascii 转小写后与 `znjtb` 一致）                                                                                           |
| T10 | `no_match_returns_empty`                | `xyz`                              | `{}`（`SearchResult.entries` 为空、`total_count == 0`）                                                                                    |
| T11 | `category_filter_still_works`           | keyword=`hello`, category=`"text"` | `{h1, h3}`；再改 category=`"code"` 期望 `{}`（验证现有 category 过滤逻辑未被破坏；若后续顺手把字符串拼接改成参数化查询，也视为符合本规格） |
| T12 | `total_count_matches_entries_len`       | `hello`                            | `SearchResult.total_count == SearchResult.entries.len()`（验证 count_sql 与 sql 同 WHERE）                                                 |

**注意事项**：

- 断言时用 `entries.iter().map(|e| e.hash.clone()).collect::<BTreeSet<_>>()` 做集合比较，避免顺序耦合
- 每个测试独立 `Database::new(":memory:")`，不共享 state
- 不修改 `row_to_entry` 的 SELECT 列数，保证 `row.get(12)` 不越界

### 7.4 前端验收（手工）

在已安装的 dev 版本中：

1. 中文 locale：打开搜索栏，看到占位文案"搜索内容 / 拼音 / 首字母（如 znjtb）"
2. 英文 locale：切换为 English，看到"Search text / pinyin / initials (e.g. znjtb)"
3. 依次输入 `zn` / `znjtb` / `zhineng` / `智能` / `ZNJTB`，均能命中历史中含"智能"的中文条目
4. 对于混合中英条目 `Hello世界`，输入 `helloshi` 应命中；规格**不要求** `hellosj` 这类“压缩式混合缩写”命中
5. 输入 `xyz` 得到空列表，不报错
6. 输入 `!!!` 不报错（验证空 MATCH 降级分支）

## 8. 风险

| 风险                                                                          | 影响 | 缓解                                                                                                                                                  |
| ----------------------------------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| FTS5 特殊字符导致语法错或注入                                                 | High | `sanitize_ascii_token / sanitize_cjk_token` 白名单（只保留 `[a-z0-9]` 或 `ch.to_pinyin().is_some()`）+ 外层只拼 `"{safe}"` 格式 + U1/U2 单元测试覆盖  |
| MATCH 整体为空字符串被误传给 FTS5                                             | Med  | 调用方按 `fts_query.is_empty()` 分情况 A/B 拼 SQL，B 形态完全不含 MATCH 子查询；U5/T7 测试覆盖                                                        |
| 旧库 rebuild FTS 首次启动变慢                                                 | Low  | 默认上限 5000 条毫秒级；WAL 不阻塞读；`fts_rebuild_includes_pinyin_columns` 测试间接覆盖                                                              |
| 生僻字无拼音                                                                  | Low  | `build_pinyin_fields` 现状：非拼音非 ASCII 字符直接忽略；原文列仍可匹配                                                                               |
| 多音字只取一个读音                                                            | Low  | 由 `pinyin` crate 默认行为决定（取第一个读音），本期不处理（见 §2 非目标）                                                                            |
| 迁移与既有"pinyin_full/pinyin_initials 列回填"步骤顺序错乱导致 rebuild 出空列 | Med  | 新迁移步骤**必须**排在回填逻辑之后（见 §4.3 执行顺序）；`fts_rebuild_includes_pinyin_columns` 测试中插入的数据由回填 + rebuild 链路填充，间接验证顺序 |

## 9. 交付物

1. **Rust 源码改动**：
   - `src-tauri/src/storage/search_pinyin.rs`：新增 `build_fts_match_expr` + `sanitize_ascii_token` + `sanitize_cjk_token`
   - `src-tauri/src/storage/migrations.rs`：在 `run_migrations` 尾部追加 §4.3.2 的 FTS 扩列迁移步骤
   - `src-tauri/src/storage/database.rs`：改造 `search_fts` 构造 `fts_query` 的方式并按空/非空动态拼 SQL
2. **Rust 新增测试**（见 §7）：
   - `search_pinyin.rs` 内 `#[cfg(test)] mod tests`：U1-U10 共 10 个
   - `migrations.rs` 内 `#[cfg(test)] mod tests`：迁移相关 3 个
   - `database.rs` 末尾 `#[cfg(test)] mod search_integration_tests`：T1-T12 共 12 个
3. **前端改动**：
   - `src/i18n/locales/zh-CN.ts`：更新 `search.placeholder`
   - `src/i18n/locales/en.ts`：更新 `search.placeholder`
4. **文档**：本 spec + 实施计划 `docs/superpowers/plans/2026-04-25-pinyin-fuzzy-search.md`

## 10. 验收标准

- [ ] `cargo test -p smart-clipboard` 全绿，含新增 **25 个测试**（§7.1 U1-U10 共 10 个、§7.2 迁移 3 个、§7.3 搜索集成 T1-T12 共 12 个）
- [ ] `cargo build` 通过，无新增 warning
- [ ] 若仓库存在对应脚本，则执行静态检查并无新增错误（例如 Rust lint / 项目既有检查脚本）
- [ ] `npm run build`（前端）通过
- [ ] 前端现有测试通过；若仓库中不存在专门覆盖搜索占位文案或关键词透传的测试文件，则不强制新增前端测试
- [ ] 手工验证：`zn` / `znjtb` / `zhineng` / `智能` / `ZNJTB` / `!!!` / `xyz` 7 种输入行为正确（见 §7.4）
- [ ] 旧 DB 升级后无需手工 rebuild，首次启动即可拼音搜索（由 `fts_rebuild_includes_pinyin_columns` 测试间接保证）

## 11. 规格自审结论（2026-04-25）

本次按 superpowers:brainstorming 的 spec self-review 要求，重点检查了 **占位符、内部一致性、范围边界、可歧义点**，结论如下：

### 11.1 已确认完善的部分

- **范围边界清晰**：明确限定为 FTS 虚表扩列、触发器同步、MATCH 构造升级、locale 文案更新，不扩展到新 API、新命令或复杂排序。
- **与现状代码基本一致**：已核对 `search_pinyin.rs`、`migrations.rs`、`database.rs::search_fts`、`SearchBar.vue`/locale 文件，主叙述与仓库现状匹配。
- **迁移顺序说明充分**：尤其指出“先删 trigger 再删 FTS 表”，能避免列数不匹配导致的运行时错误，这一点很关键。
- **测试覆盖足够具体**：纯函数、迁移、集成、手工验收四层都已定义，实施者可直接据此落地。

### 11.2 本次修订的问题

1. **混合中英示例过度承诺**：原稿把 `hellosj` 写成能命中 `Hello世界`，但按当前保留的 `%keyword%` LIKE 与 FTS 前缀语义，这并不成立。现已改为 `helloshi` 这类连续前缀示例，并明确“不要求压缩式混合缩写匹配”。
2. **仓库相关验收项过于绑定本地脚本名**：`read_lints ...` 与指定前端测试文件在当前仓库中未证实存在，容易让实施者误以为是强制步骤。现已放宽为“若仓库存在对应脚本则执行”。
3. **状态标记补充**：将文档状态更新为 `Reviewed & Refined`，表明该 spec 已经过一轮自审修订，而不是仅停留在初稿批准状态。

### 11.3 结论

**结论：该规格已经达到可实施状态。**

剩余注意点不是设计缺失，而是实施时需要严格遵守的语义边界：

- 不要在实现中偷偷扩展为“压缩式混合缩写搜索”；
- 不要改变现有 `LIKE` 兜底和排序语义，除非另起新 spec；
- 若实施中发现 SQLite FTS5 对列限定语法存在兼容性差异，应在不改变用户可见行为的前提下微调表达式生成细节，并同步回写本 spec。
