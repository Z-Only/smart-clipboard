[English](README.md) | [中文](README.zh-CN.md)

# 智能剪贴板管理器

一款基于 **Tauri 2** + **Vue 3** + **Rust** 构建的跨平台轻量级智能剪贴板管理工具。它在后台运行，自动捕获和分类剪贴板内容，并在本地保护、快速检索与跨设备同步之间提供平衡。

- 官网 / 文档站：https://z-only.github.io/smart-clipboard/
- 仓库地址：https://github.com/Z-Only/smart-clipboard

## 发布状态 — v2.4.0 数据库静态加密

当前仓库在托管更新器、剪贴板历史、智能增强、模板、局域网同步、WebDAV 云同步、Phase 4 访问安全和原生生物识别集成的基础上，新增了 **数据库静态加密**。

### v2.4.0 亮点

- **可选数据库加密**：使用 AES-256-GCM 对剪贴板条目内容进行应用层加密
- **安全密钥管理**：加密密钥存储在系统密钥链中（macOS Keychain / Windows 凭据管理器 / Linux Secret Service）
- **透明加解密**：写入时加密、读取时解密，前端无需感知加密状态
- **一键迁移**：随时启用或关闭加密，所有现有条目自动迁移
- **加密设置 UI**：在设置面板中切换加密开关并监控迁移状态

## 功能特性

- **剪贴板历史** -- 自动捕获复制内容并去重
- **智能分类** -- 自动将内容分类为链接、邮箱、代码、JSON、文件路径、颜色、电话、地址、图片和文本
- **全文搜索** -- 基于 SQLite FTS5 的快速搜索
- **分类筛选** -- 按内容类型浏览历史记录
- **全局快捷键** -- `Cmd/Ctrl + Shift + V` 切换剪贴板面板
- **系统托盘** -- 在后台运行并提供托盘控制
- **收藏功能** -- 固定常用条目，防止自动清理
- **可配置** -- 最大记录数、保留天数、排除应用、监控间隔、敏感内容过期时间
- **开机自启** -- 可选择系统登录时自动启动
- **外观模式** -- 系统 / 浅色 / 深色模式
- **主题颜色** -- 6 种内置主题颜色
- **多语言支持** -- 支持中英文界面
- **敏感信息检测** -- 自动识别密码、API Key、Token、JWT、连接串等，并支持自动过期
- **内容转换** -- 一键进行大小写、编码、格式化等文本转换
- **标签管理** -- 自定义标签组织条目
- **图片剪贴板** -- 捕获并显示剪贴板图片，PNG 存储
- **使用统计** -- 展示分类分布、每日活跃度和最常使用条目
- **剪贴板模板** -- 支持 `{{placeholder}}` 语法的可复用文本模板
- **局域网同步** -- 基于 mDNS + WebSocket 的加密点对点同步
- **WebDAV 云同步** -- 带设备注册表、轮询和限流的端到端加密云同步
- **访问安全** -- 密码锁、自动锁定、受保护唤起拦截与安全解锁流程
- **托管更新器** -- 后台检查更新、镜像端点、带进度的安装包下载、签名校验和安装切换
- **数据库加密** -- 可选的 AES-256-GCM 剪贴板数据静态加密，密钥存储在系统密钥链中
- **轻量级** -- 得益于 Rust + 原生 WebView，体积小、资源占用低

## 安全模型（Phase 4）

### 保护范围

当启用应用锁时：

- 主窗口启动后默认处于锁定状态
- 托盘和全局快捷键唤起会先经过 Rust 侧访问判断
- 敏感 Tauri 命令在锁定时拒绝访问
- 锁定时前端敏感缓存会被主动清空
- 生物识别/系统认证失败会自动回退到密码方案

### 密码存储策略

- **不会明文保存密码**
- Rust 使用 **Argon2** 对密码做哈希
- 仅保存密码哈希，并通过 keyring 写入 **系统凭据存储**
- 应用配置中只保存应用锁开关、自动锁定时间、生物识别偏好等设置

### 当前平台行为

- **macOS**：支持密码锁、自动锁定、托盘/热键拦截，以及基于 LocalAuthentication 框架的原生 Touch ID 解锁
- **Windows**：支持密码锁、自动锁定、托盘/热键拦截，以及原生 Windows Hello 解锁（指纹、面容、PIN）
- **Linux**：支持密码锁、自动锁定与托盘/热键拦截；生物识别解锁回退为密码方案

## 同步概览

### 局域网同步

- 已配对设备之间的实时剪贴板同步
- X25519 密钥交换 + AES-256-GCM 加密传输
- WebSocket 心跳与断线重连
- 循环防护与去重

### WebDAV 云同步

- 跨网络端到端加密剪贴板同步
- 基于 Argon2id 的口令派生密钥
- AES-256-GCM 文件加密
- 基于 ETag 的冲突处理
- 设备注册表与可配置轮询

## 截图

可在官网截图页查看最新界面预览：https://z-only.github.io/smart-clipboard/guide/screenshots

## 技术栈

| 层级       | 技术                                                                               |
| ---------- | ---------------------------------------------------------------------------------- |
| 前端       | Vue 3 + TypeScript + Tailwind CSS + shadcn-vue                                     |
| 后端       | Rust                                                                               |
| 框架       | Tauri 2                                                                            |
| 数据库     | SQLite with FTS5（通过 rusqlite）                                                  |
| 剪贴板     | arboard                                                                            |
| 本地安全   | argon2 + aes-gcm + keyring + LocalAuthentication (macOS) + Windows Hello (Windows) |
| 局域网发现 | mdns-sd                                                                            |
| 国际化     | vue-i18n                                                                           |

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

1. 启动应用 —— 默认最小化到系统托盘
2. 在任意应用中复制文本或图片
3. 按 `Cmd+Shift+V`（macOS）或 `Ctrl+Shift+V`（Windows/Linux）打开剪贴板面板
4. 搜索、筛选、打标签，或点击条目进行粘贴
5. 使用模板快速填充可复用片段
6. 打开同步面板管理局域网同步或 WebDAV 云同步
7. 在设置中配置应用锁、自动锁定以及其他偏好
8. 锁定后使用密码解锁；在支持的平台上可尝试更便捷的生物识别/系统认证解锁

## 项目结构

```text
smart-clipboard/
├── src/                          # Vue 3 前端
│   ├── components/               # UI 组件
│   ├── composables/              # Vue 组合式函数
│   ├── i18n/                     # 国际化
│   ├── stores/                   # Pinia 状态管理
│   └── types/                    # TypeScript 类型定义
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── analyzer/             # 内容分类与敏感检测
│       ├── clipboard/            # 剪贴板监听
│       ├── storage/              # SQLite + FTS5 数据库层
│       ├── sync/                 # 局域网同步 + WebDAV 云同步
│       ├── templates/            # 剪贴板模板引擎与命令
│       ├── security.rs           # 应用锁、解锁与访问控制运行时
│       ├── encryption.rs         # AES-256-GCM 数据库加密引擎
│       ├── commands.rs           # 主要 Tauri IPC 命令
│       ├── config.rs             # 配置管理
│       ├── hotkey.rs             # 全局快捷键处理
│       ├── tray.rs               # 系统托盘集成
│       └── lib.rs                # 应用入口
└── docs/                         # 设计文档
```

## 路线图

### 已完成

- [x] **Phase 1 -- MVP**：剪贴板监控、存储、分类、搜索界面、快捷键、托盘、设置
- [x] **Phase 2 -- 智能增强**：敏感检测、内容转换、标签、图片、使用统计
- [x] **模板能力**：支持参数化占位符的可复用剪贴板模板
- [x] **Phase 3 -- 同步**：局域网同步、设备配对、加密 WebSocket 传输、WebDAV 云同步
- [x] **Phase 4 -- 访问安全**：应用锁、安全密码存储、启动解锁门禁、托盘/热键拦截、自动锁定与命令守卫

### 计划中 / 未来预期

- [x] **原生生物识别集成**：原生 Touch ID (macOS) 和 Windows Hello (Windows)，通过平台 FFI 实现
- [x] **更深层运行时集成测试**：增加基于 invoke 边界的黑盒测试，验证锁定/解锁状态下的真实命令行为
- [x] **本地数据库静态加密**：使用 AES-256-GCM 加密剪贴板数据，密钥通过系统密钥链安全管理
- [ ] **高级同步冲突处理**：提供更智能的合并 / 冲突解决策略
- [ ] **插件 / 扩展系统**：支持用户扩展自动化与转换能力
- [ ] **更强的平台级安全增强**：例如更准确的系统空闲检测与更好的原生解锁体验

## 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目基于 MIT 许可证开源 —— 详见 [LICENSE](LICENSE) 文件。

## 更新配置

托管更新依赖 Tauri updater 构建产物与发布附件。

发布所需 Secrets：

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

运行时 updater 公钥读取顺序：

1. `SMART_CLIPBOARD_UPDATER_PUBLIC_KEY` 环境变量
2. `src-tauri/tauri.conf.json` 中的 `plugins.updater.pubkey`

当前状态：

- `sha256:` 签名可作为开发期回退校验方案
- `minisign:` 签名的运行时公钥接线已完成，但最终验签实现仍待补齐
