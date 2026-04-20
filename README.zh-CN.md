[English](README.md) | [中文](README.zh-CN.md)

# 智能剪贴板管理器

一款基于 **Tauri 2** + **Vue 3** + **Rust** 构建的跨平台轻量级智能剪贴板管理工具。它在后台运行，自动捕获和分类剪贴板内容，提供即时搜索和检索功能。

## 第三阶段进展 — 局域网剪贴板同步

当前仓库已经包含 **Phase 3 局域网同步** 的完整实现，支持**实时剪贴板内容同步**。新的剪贴板条目会自动加密并发送到所有已连接的配对设备。

### 同步架构

```
设备 A                                设备 B
┌─────────────────┐                  ┌─────────────────┐
│ 剪贴板监听       │                  │ 剪贴板监听       │
│       ↓         │                  │       ↓         │
│   数据库写入     │                  │   数据库写入     │
│       ↓         │                  │       ↓         │
│ 广播同步条目     │                  │ 广播同步条目     │
│       ↓         │                  │       ↓         │
│ SyncManager     │◄── WebSocket ──►│ SyncManager     │
│  ├─ 加密        │   (AES-256-GCM) │  ├─ 解密        │
│  ├─ 发送        │                  │  ├─ 去重        │
│  └─ 同步日志    │                  │  ├─ 存储        │
│                 │                  │  └─ 同步日志    │
└─────────────────┘                  └─────────────────┘
```

### 已实现功能

- **实时剪贴板同步**：新的文本条目自动发送到已配对设备
- **同步过滤**：可配置 `auto_sync`（自动同步）、`sync_images`（同步图片）、`sync_sensitive`（同步敏感内容）和 1MB 负载限制
- **循环防护**：从其他设备接收的条目不会被重新广播
- **去重机制**：内容哈希 + sync_log 双重检查，防止重复条目
- **来源追踪**：同步条目标记 `source_device` 以追踪来源设备
- **端到端加密**：X25519 密钥交换 + AES-256-GCM 逐消息加密
- **mDNS 发现**：通过 `_smartclip._tcp.local.` 自动发现局域网设备
- **WebSocket 传输**：自动连接、ping/pong 心跳、断线重连退避
- **同步管理面板**：设备管理、配对、按设备开关同步

### 尚未实现

- 双端确认配对流程 UI（当前自动接受已知设备）
- 图片内容同步（当前仅同步文本；除非启用 `sync_images`，否则跳过图片）
- 高级冲突处理（当前双端各保留自己的版本）

## 功能特性

- **剪贴板历史** -- 自动捕获所有复制的文本，支持去重
- **智能分类** -- 自动将内容分类为链接、邮箱、代码、JSON、文件路径、颜色、电话、地址和纯文本
- **全文搜索** -- 基于 SQLite FTS5 的毫秒级搜索
- **分类筛选** -- 按内容类型浏览剪贴板历史
- **全局快捷键** -- `Cmd/Ctrl + Shift + V` 切换剪贴板面板
- **系统托盘** -- 通过托盘图标安静地在后台运行
- **收藏功能** -- 固定常用条目，防止自动清理
- **可配置** -- 最大记录数、保留天数、排除应用、监控间隔
- **开机自启** -- 可选择在系统登录时自动启动
- **外观模式** -- 系统/浅色/深色模式，自动检测系统偏好
- **主题颜色** -- 6 种内置主题色：锌灰、蓝色、绿色、玫瑰、橙色、紫罗兰
- **多语言支持** -- 支持中英文界面
- **敏感信息检测** -- 自动识别密码、API 密钥、令牌、JWT 和连接字符串，支持自动过期
- **内容转换** -- 12 种一键文本转换（大小写、编码、格式化等）
- **标签管理** -- 自定义标签组织条目，支持按标签筛选
- **图片剪贴板** -- 捕获并显示剪贴板图片，PNG 格式存储
- **使用统计** -- 仪表盘展示分类分布、每日活跃度和最常使用条目
- **剪贴板模板** -- 支持 `{{占位符}}` 语法的可重用文本模板，使用时弹出填写对话框
- **轻量级** -- 约 5MB 二进制文件，得益于 Rust + 原生 WebView，CPU/内存占用极低

## 截图

*即将推出*

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3 + TypeScript + Tailwind CSS + shadcn-vue |
| 后端 | Rust |
| 框架 | Tauri 2 |
| 数据库 | SQLite with FTS5 (通过 rusqlite) |
| 剪贴板 | arboard |
| 国际化 | vue-i18n |

## 快速开始

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install) (1.77+)
- [Node.js](https://nodejs.org/) (18+)
- [pnpm](https://pnpm.io/)
- [Tauri](https://v2.tauri.app/start/prerequisites/) 的平台特定依赖

### 开发

```bash
# 克隆仓库
git clone https://github.com/Z-Only/smart-clipboard.git
cd smart-clipboard

# 安装依赖
pnpm install

# 以开发模式运行
pnpm tauri dev
```

### 构建

```bash
# 构建生产版本
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 使用方法

1. 启动应用 -- 它会最小化到系统托盘
2. 在任意应用中复制文本
3. 按 `Cmd+Shift+V` (macOS) 或 `Ctrl+Shift+V` (Windows/Linux) 打开剪贴板面板
4. 搜索、按分类筛选，或点击条目粘贴
5. 收藏条目以永久保留
6. 右键点击托盘图标可快速访问和设置
7. 在设置中切换语言（中文/英文）
8. 在设置中选择外观模式（系统/浅色/深色）和主题颜色
9. 右键条目可进行文本转换（URL 编码、Base64、JSON 格式化等）
10. 使用自定义标签组织条目
11. 复制图片 -- 它们也会出现在剪贴板历史中
12. 点击柱状图图标查看使用统计
13. 点击文档图标管理和使用剪贴板模板

## 项目结构

```
smart-clipboard/
├── src/                          # Vue 3 前端
│   ├── components/               # UI 组件
│   ├── composables/              # Vue 组合式函数
│   ├── i18n/                     # 国际化
│   │   ├── locales/              # 语言文件 (en, zh-CN)
│   │   └── index.ts              # i18n 配置
│   ├── stores/                   # Pinia 状态管理
│   └── types/                    # TypeScript 类型定义
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── analyzer/             # 内容分类器（正则规则）
│       ├── clipboard/            # 剪贴板监控（arboard 轮询）
│       ├── storage/              # SQLite + FTS5 数据库层
│       ├── commands.rs           # Tauri IPC 命令
│       ├── config.rs             # 配置管理
│       ├── hotkey.rs             # 全局快捷键
│       ├── tray.rs               # 系统托盘
│       └── lib.rs                # 应用入口
└── docs/                         # 设计文档
```

## 路线图

- [x] **第一阶段 -- MVP**：剪贴板监控、存储、分类、搜索界面、快捷键、托盘、设置
- [x] **国际化**：多语言支持（中文、英文）
- [x] **主题系统**：外观模式切换（系统/浅色/深色）及 6 种主题颜色
- [x] **第二阶段 -- 智能增强**：敏感内容检测、内容转换、标签管理、图片支持、使用统计
- [x] **剪贴板模板**：支持参数化占位符的可重用文本模板
- [ ] **第三阶段 -- 同步与高级功能**：局域网同步、端到端加密云同步、插件系统

## 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目基于 MIT 许可证开源 -- 详见 [LICENSE](LICENSE) 文件。

## 致谢

- [Tauri](https://tauri.app/) -- 跨平台应用框架
- [Vue.js](https://vuejs.org/) -- 前端框架
- [vue-i18n](https://vue-i18n.intlify.dev/) -- Vue.js 国际化方案
- [shadcn-vue](https://www.shadcn-vue.com/) -- UI 组件库
- [arboard](https://github.com/1Password/arboard) -- 跨平台剪贴板库
- [rusqlite](https://github.com/rusqlite/rusqlite) -- Rust 的 SQLite 绑定
