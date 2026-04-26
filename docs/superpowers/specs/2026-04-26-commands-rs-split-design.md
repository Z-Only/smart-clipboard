# commands.rs 拆分重构 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

`src-tauri/src/commands.rs` 当前有 1242 行，承载 52 个 `#[tauri::command]` 函数 + 1 个 transform 子模块 + 1 个集成测试模块。7 个功能域（安全/剪贴板/更新器/配置/标签/转换/同步+加密）全部混合在同一个文件中。

主要问题：

- 文件过大（1242 行），远超 Rust 单文件推荐的 300~500 行
- 功能域混合：安全、剪贴板 CRUD、更新器、同步等互不相关的逻辑放在一起
- 导航困难：需要反复滚动才能找到目标命令
- 项目中 plugins 和 templates 已经有独立的 `commands` 子模块作为范例

## 2. 拆分方案

将 `commands.rs` 拆分为 **1 个入口模块 + 6 个功能子模块**：

| 模块                    | 文件路径 | 职责                                 | 估计行数 |
| ----------------------- | -------- | ------------------------------------ | -------- |
| `commands/mod.rs`       | 入口     | re-export 所有命令 + 共享辅助函数    | ~30      |
| `commands/security.rs`  | 安全锁   | 5 个锁/密码命令                      | ~80      |
| `commands/clipboard.rs` | 剪贴板   | 10 个条目 CRUD 命令                  | ~200     |
| `commands/updater.rs`   | 更新器   | 5 个更新器命令                       | ~110     |
| `commands/config.rs`    | 配置     | 4+1 个配置/自启/quit 命令            | ~40      |
| `commands/tags.rs`      | 标签     | 8 个标签命令                         | ~100     |
| `commands/sync.rs`      | 同步     | 12 个 P2P+WebDAV 命令 + 3 个加密命令 | ~200     |

transform 子模块保持为 `commands/transform.rs`（从 `pub mod transform` 内联块提取为独立文件）。

测试模块 `command_guard_tests` 移至 `commands/tests.rs`。

### 2.1 共享辅助函数

以下函数被多个子模块共用，放在 `commands/mod.rs` 中：

```rust
pub(crate) fn require_unlocked(lock: &State<'_, Arc<AppLockManager>>) -> Result<(), String>
pub(crate) fn decrypt_entries(encryption: &EncryptionManager, entries: &mut [ClipboardEntry])
pub(crate) fn decrypt_search_result(encryption: &EncryptionManager, result: &mut SearchResult)
```

### 2.2 lib.rs 修改

`lib.rs` 中的 `generate_handler!` 宏调用路径从 `commands::xxx` 不变（因为子模块的命令会通过 `mod.rs` re-export）。

## 3. 非目标

- 不改变任何命令的签名、行为或返回值
- 不修改 store/manager 层代码
- 不新增或移除任何 tauri command
- 不修改前端调用

## 4. 对已有测试的影响

- `command_guard_tests` 模块将移至 `commands/tests.rs`，import 路径需要调整
- Rust 集成测试（`integration_tests`）通过 `crate::commands::xxx` 调用，re-export 后路径不变
- 前端测试 mock `invoke` 调用字符串命令名，不受影响

## 5. 验收标准

- `commands.rs` 被替换为 `commands/` 目录，每个子模块 ≤ 250 行
- `cargo build` 编译通过
- `cargo test` 所有测试通过（包括 command_guard_tests）
- `cargo clippy` 无新增警告
- `pnpm run test:web` 前端测试仍全部通过
- `lib.rs` 中 `generate_handler!` 的所有命令路径正确
