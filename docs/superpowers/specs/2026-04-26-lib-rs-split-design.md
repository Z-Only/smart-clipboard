# lib.rs 拆分重构 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

`src-tauri/src/lib.rs` 当前有 649 行，承载以下职责：

- 16 个 `pub mod` 声明
- `AppDataDir` 公共结构体
- 3 个窗口/锁状态辅助函数
- `run()` 入口函数（367 行），包含：
  - `invoke_handler` 命令注册（77 行）
  - `.setup()` 初始化逻辑（日志、配置、数据库、加密、同步、WebDAV、安全、热键、托盘、窗口事件等）
  - 剪贴板监听处理循环（170 行）：排除应用检测、图片/文本分类、哈希去重、加密、数据库写入、事件广播、LAN/WebDAV 同步推送
  - 初始清理任务
- `runtime_tests` 测试模块（211 行）

主要问题：

- `run()` 函数过长（367 行），职责过多
- 剪贴板监听处理循环是独立的关注点，与应用初始化混在一起
- setup 闭包体过长，难以快速定位特定初始化逻辑
- 测试模块占文件总行数的 32%，挤压了业务代码的可读性

## 2. 拆分方案

将 `lib.rs` 中的大块逻辑提取到 2 个新子模块，测试移至独立文件：

| 模块               | 文件路径                         | 职责                                                                       | 估计行数 |
| ------------------ | -------------------------------- | -------------------------------------------------------------------------- | -------- |
| `lib.rs`           | `src-tauri/src/lib.rs`           | 模块声明、AppDataDir、辅助函数、精简的 `run()` 入口                        | ~120     |
| `app_setup.rs`     | `src-tauri/src/app_setup.rs`     | setup 闭包主体：日志、配置、数据库、管理器初始化、热键、托盘、窗口事件绑定 | ~120     |
| `monitor.rs`       | `src-tauri/src/monitor.rs`       | 剪贴板监听处理循环：排除应用、分类、去重、加密、入库、广播、同步           | ~180     |
| `runtime_tests.rs` | `src-tauri/src/runtime_tests.rs` | runtime_tests 模块（TestHarness 及 3 个测试）                              | ~210     |

### 2.1 app_setup.rs

提取 `.setup()` 闭包的主体为独立函数：

```rust
pub(crate) fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 日志插件初始化
    // 配置管理器初始化
    // 数据库初始化
    // 加密管理器初始化
    // 同步管理器初始化
    // WebDAV 管理器初始化
    // 安全锁运行时挂载
    // 热键设置
    // 托盘设置
    // 窗口事件绑定
    // 图片目录创建
    // 初始锁状态广播 & 更新器状态广播
    // 启动剪贴板监听
    // 初始清理任务
    Ok(())
}
```

### 2.2 monitor.rs

提取剪贴板监听处理循环为独立函数：

```rust
pub(crate) fn start_clipboard_monitor(
    app_handle: AppHandle,
    db: Arc<Database>,
    config_manager: Arc<ConfigManager>,
    encryption_manager: Arc<EncryptionManager>,
    sync_manager: SyncManager,
    webdav_manager: WebDavSyncManager,
    images_dir: PathBuf,
    monitor_interval_ms: u64,
)
```

### 2.3 lib.rs 精简后

```rust
// 模块声明（不变）
// AppDataDir 结构体（不变）
// 3 个辅助函数（不变）
// run() 精简为: Builder + invoke_handler + .setup(app_setup::setup_app) + .run()
```

## 3. 非目标

- 不改变任何命令的签名、行为或返回值
- 不修改 store/manager 层代码
- 不新增或移除任何 tauri command
- 不修改前端调用
- 不修改 `main.rs`

## 4. 对已有测试的影响

- `runtime_tests` 模块移至 `src-tauri/src/runtime_tests.rs`，通过 `#[cfg(test)] mod runtime_tests;` 引入
- 测试内部的 `use super::*` 改为 `use crate::*` 引用
- `integration_tests` 模块不受影响
- 前端测试不受影响

## 5. 验收标准

- `lib.rs` 行数 ≤ 150 行
- `cargo build` 编译通过
- `cargo test` 所有测试通过
- `cargo clippy -- -D warnings` 无警告
- `pnpm run test:web` 前端测试仍全部通过
