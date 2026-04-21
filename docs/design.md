# Smart Clipboard Manager - 跨平台智能剪贴板管理器

## Context

剪贴板是日常使用最频繁的系统功能之一，但系统自带的剪贴板只保留最近一条记录，且无法搜索、分类或跨设备同步。本项目旨在构建一个轻量、智能、跨平台的剪贴板管理器，解决以下痛点：

- 复制内容被覆盖后无法找回
- 频繁复制相同内容（如代码片段、地址、账号）
- 无法按类型快速检索历史内容
- 多设备间剪贴板无法同步

---

## 一、核心功能设计

### 1. 剪贴板历史管理

- **自动捕获**：后台监听系统剪贴板变化，自动记录所有复制内容
- **支持类型**：纯文本、富文本、图片、文件路径、代码片段
- **去重策略**：相同内容不重复存储，仅更新时间戳和使用频次
- **容量管理**：可配置保留条数（默认 5000 条）和过期时间（默认 30 天）
- **置顶/收藏**：常用内容可收藏为永久保留，不受清理策略影响

### 2. 智能分类与识别

利用规则引擎 + 轻量 NLP 自动识别内容类型：

| 类别     | 识别方式                  | 示例                         |
| -------- | ------------------------- | ---------------------------- |
| URL      | 正则匹配 `https?://...`   | 网页链接                     |
| Email    | 正则匹配 `xxx@xxx.xxx`    | 邮箱地址                     |
| 电话号码 | 正则匹配各国号码格式      | 手机号/座机                  |
| 代码片段 | 语法关键词检测 + 缩进分析 | Python/JS/SQL 等             |
| 文件路径 | 路径格式检测              | `/usr/local/...` 或 `C:\...` |
| 颜色值   | 正则匹配 `#hex` / `rgb()` | 设计色值                     |
| JSON/XML | 结构化格式检测            | API 响应数据                 |
| 地址     | 关键词 + 模式匹配         | 快递地址                     |
| 普通文本 | 兜底分类                  | 其他内容                     |

### 3. 快速搜索与检索

- **全文搜索**：基于 SQLite FTS5 实现毫秒级全文检索
- **按类型过滤**：点击分类标签快速筛选
- **模糊匹配**：支持拼音首字母、模糊关键词搜索
- **时间线浏览**：按日期分组展示历史记录

### 4. 快捷操作

- **全局快捷键**：`Cmd/Ctrl + Shift + V` 唤起剪贴板面板
- **快速粘贴**：选中后自动粘贴到当前活动窗口
- **批量操作**：多选后合并粘贴、批量删除
- **内容转换**：一键转换大小写、去除格式、URL 编码/解码、JSON 格式化

### 5. 跨设备同步（可选，Phase 2）

- **端到端加密**：所有同步内容使用 AES-256-GCM 加密
- **同步方式**：
  - 局域网直连（mDNS 发现 + WebSocket）
  - 云端中继（WebDAV，兼容坚果云/Nextcloud/群晖等主流网盘）
- **冲突处理**：以最新时间戳为准，保留两端记录

### 6. 安全与隐私

- **敏感内容检测**：自动识别密码、密钥、Token 等敏感信息
- **排除规则**：可配置排除特定应用（如密码管理器、银行 App）的复制内容
- **自动清理**：敏感内容可设置自动过期时间（如 5 分钟）
- **数据加密**：本地数据库使用 SQLCipher 加密存储
- **锁定功能**：应用可设置密码/生物识别锁定

---

## 二、技术架构

### 技术选型

```
┌─────────────────────────────────────────────┐
│              Frontend (WebView)              │
│          Vue 3 + TypeScript + Tailwind       │
│            (Shadcn-vue 组件库)                │
├─────────────────────────────────────────────┤
│              Tauri Bridge (IPC)              │
├─────────────────────────────────────────────┤
│              Backend (Rust Core)             │
│  ┌──────────┬──────────┬──────────────────┐ │
│  │Clipboard │ Storage  │ Content          │ │
│  │Monitor   │ Engine   │ Analyzer         │ │
│  │(arboard) │(SQLite)  │(regex + heuristic│ │
│  └──────────┴──────────┴──────────────────┘ │
├─────────────────────────────────────────────┤
│           OS (macOS / Windows / Linux)       │
└─────────────────────────────────────────────┘
```

**为什么选 Tauri + Rust：**

- 跨平台：一套代码编译到 macOS、Windows、Linux
- 轻量：打包体积 ~5MB（vs Electron ~150MB）
- 性能：Rust 后端处理剪贴板监听和内容分析，极低 CPU/内存占用
- 安全：Rust 内存安全 + Tauri 最小权限模型
- 原生体验：使用系统 WebView，无需捆绑 Chromium

### 项目结构

```
smart-clipboard/
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs              # 入口
│   │   ├── clipboard/
│   │   │   ├── mod.rs
│   │   │   ├── monitor.rs       # 剪贴板监听（轮询/事件）
│   │   │   └── types.rs         # ClipboardEntry 数据结构
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── database.rs      # SQLite 数据库操作
│   │   │   ├── migrations.rs    # 数据库迁移
│   │   │   └── models.rs        # 数据模型
│   │   ├── analyzer/
│   │   │   ├── mod.rs
│   │   │   ├── classifier.rs    # 内容分类器
│   │   │   ├── patterns.rs      # 正则规则库
│   │   │   └── sensitive.rs     # 敏感信息检测
│   │   ├── commands.rs          # Tauri IPC 命令
│   │   ├── hotkey.rs            # 全局快捷键
│   │   ├── tray.rs              # 系统托盘
│   │   └── config.rs            # 配置管理
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                          # Vue 3 前端
│   ├── App.vue
│   ├── components/
│   │   ├── ClipboardList.vue    # 历史列表（虚拟滚动）
│   │   ├── SearchBar.vue        # 搜索栏
│   │   ├── CategoryFilter.vue   # 分类过滤
│   │   ├── EntryCard.vue        # 单条记录卡片
│   │   ├── PreviewPanel.vue     # 内容预览
│   │   └── SettingsPanel.vue    # 设置面板
│   ├── composables/
│   │   ├── useClipboard.ts      # 剪贴板组合式函数
│   │   └── useSearch.ts         # 搜索逻辑
│   ├── stores/
│   │   └── clipboardStore.ts    # Pinia 状态管理
│   └── styles/
│       └── globals.css
├── package.json
└── README.md
```

### 核心数据模型

```sql
-- 剪贴板条目表
CREATE TABLE clipboard_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    content     TEXT NOT NULL,           -- 文本内容或图片 base64
    content_type TEXT NOT NULL,          -- text, image, file
    category    TEXT DEFAULT 'text',     -- url, email, code, phone, etc.
    hash        TEXT NOT NULL UNIQUE,    -- 内容哈希，用于去重
    source_app  TEXT,                    -- 来源应用
    is_favorite INTEGER DEFAULT 0,      -- 是否收藏
    is_sensitive INTEGER DEFAULT 0,     -- 是否敏感
    use_count   INTEGER DEFAULT 1,      -- 使用次数
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at  DATETIME                -- 过期时间（可选）
);

-- 全文搜索虚拟表
CREATE VIRTUAL TABLE clipboard_fts USING fts5(
    content, category, source_app,
    content='clipboard_entries',
    content_rowid='id'
);

-- 标签表（用户自定义标签）
CREATE TABLE tags (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE
);

CREATE TABLE entry_tags (
    entry_id    INTEGER REFERENCES clipboard_entries(id),
    tag_id      INTEGER REFERENCES tags(id),
    PRIMARY KEY (entry_id, tag_id)
);
```

### 关键 Rust 模块设计

```rust
// clipboard/monitor.rs - 剪贴板监听核心
pub struct ClipboardMonitor {
    interval: Duration,          // 轮询间隔（默认 500ms）
    last_hash: Option<String>,   // 上次内容哈希
    excluded_apps: Vec<String>,  // 排除的应用列表
}

impl ClipboardMonitor {
    /// 启动后台监听线程
    pub fn start(&self, tx: Sender<ClipboardEntry>) {
        // 1. 轮询系统剪贴板（arboard crate）
        // 2. 计算内容哈希，与 last_hash 比较
        // 3. 如有变化，构造 ClipboardEntry 发送到通道
        // 4. 检查来源应用是否在排除列表中
    }
}

// analyzer/classifier.rs - 内容分类
pub fn classify(content: &str) -> Category {
    // 优先级匹配链：
    // 1. URL → 2. Email → 3. 颜色值 → 4. 文件路径
    // → 5. JSON/XML → 6. 代码 → 7. 电话 → 8. 地址 → 9. 普通文本
}
```

### Rust 关键依赖

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "global-shortcut"] }
arboard = "3"           # 跨平台剪贴板访问
rusqlite = { version = "0.31", features = ["bundled", "fts5"] }
serde = { version = "1", features = ["derive"] }
sha2 = "0.10"           # 内容哈希
regex = "1"              # 内容分类规则
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
```

---

## 三、UI/UX 设计要点

### 主界面布局

```
┌──────────────────────────────────────────┐
│  搜索...                 [≡] [设] [—][×] │  <- 搜索栏 + 工具栏
├────────┬─────────────────────────────────┤
│ 全部   │  ┌─────────────────────────┐    │
│ *收藏  │  │ 今天                     │    │
│ >链接  │  │ ┌───────────────────┐   │    │
│ @邮件  │  │ │ https://github... │   │    │
│ <>代码 │  │ │ 2 分钟前 · Chrome  │   │    │
│ /文件  │  │ └───────────────────┘   │    │
│ #数字  │  │ ┌───────────────────┐   │    │
│ T文本  │  │ │ SELECT * FROM...  │   │    │
│        │  │ │ 15 分钟前 · VSCode │   │    │
│        │  │ └───────────────────┘   │    │
│        │  │ ...                     │    │
│        │  └─────────────────────────┘    │
├────────┴─────────────────────────────────┤
│ 共 1,234 条记录  ·  已使用 12.3 MB       │
└──────────────────────────────────────────┘
```

### 交互设计

- **唤起**：全局快捷键唤起后，光标自动聚焦搜索框
- **选择**：键盘 Up/Down 导航，Enter 粘贴，Esc 关闭
- **预览**：鼠标悬停显示完整内容预览
- **上下文菜单**：右键 -> 复制/收藏/删除/编辑标签
- **主题**：跟随系统深色/浅色模式

---

## 四、分阶段实施计划

### Phase 1 -- MVP（核心功能）

1. Tauri 项目初始化 + 基础窗口
2. Rust 剪贴板监听（arboard 轮询）
3. SQLite 存储 + FTS5 全文搜索
4. 内容分类器（正则规则链）
5. Vue 3 前端：历史列表 + 搜索 + 分类过滤
6. 全局快捷键唤起/隐藏
7. 系统托盘图标 + 开机启动
8. 基础设置：保留条数、排除应用

### Phase 2 -- 智能增强

9. 敏感信息检测 + 自动过期
10. 内容转换（大小写、URL 编码、JSON 格式化）
11. 收藏/标签管理
12. 图片剪贴板支持
13. 使用统计面板

### Phase 3 -- 同步与高级功能

14. 局域网设备发现（mDNS）
15. 端到端加密同步（WebDAV + WebSocket）
16. 剪贴板模板（可参数化的常用文本）
17. 插件系统（自定义分类规则 / 处理动作）

---

## 五、验证方式

1. **功能验证**：复制不同类型内容，确认自动分类正确
2. **性能验证**：5000 条记录下搜索延迟 < 50ms，内存占用 < 50MB
3. **跨平台验证**：macOS / Windows / Linux 分别编译运行
4. **快捷键验证**：全局热键在各平台正常唤起/隐藏
5. **单元测试**：内容分类器覆盖所有规则分支
6. **安全验证**：敏感内容检测准确率 > 90%
