# lib.rs 拆分重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 649 行的 lib.rs 拆分为 1 个精简入口 + 2 个功能子模块 + 1 个独立测试文件，降低单文件复杂度，同时保持所有命令路径和行为完全不变。

**Architecture:** 按关注点拆分：剪贴板监听处理循环提取为 monitor.rs，setup 闭包主体提取为 app_setup.rs，runtime_tests 迁移为独立文件。lib.rs 精简为模块声明 + 辅助函数 + 精简的 run() 入口。

**Tech Stack:** Rust, Tauri v2

---

## File Map

- **Create:** `src-tauri/src/monitor.rs` — 剪贴板监听处理循环
- **Create:** `src-tauri/src/app_setup.rs` — setup 初始化逻辑
- **Create:** `src-tauri/src/runtime_tests.rs` — runtime_tests 测试模块
- **Modify:** `src-tauri/src/lib.rs` — 精简为入口模块

## Task 1: 创建 monitor.rs

**Files:** Create `src-tauri/src/monitor.rs`

- [ ] **Step 1:** 提取 L274-422 的剪贴板监听处理循环为 `start_clipboard_monitor()` 函数
- [ ] **Step 2:** 包含排除应用检测、图片/文本分类、哈希去重、加密、数据库写入、事件广播、LAN/WebDAV 同步推送

## Task 2: 创建 app_setup.rs

**Files:** Create `src-tauri/src/app_setup.rs`

- [ ] **Step 1:** 提取 `.setup()` 闭包主体为 `setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>>`
- [ ] **Step 2:** 包含日志初始化、配置管理器、数据库、加密管理器、同步管理器、WebDAV、安全锁、热键、托盘、窗口事件绑定、图片目录、监听启动、初始清理

## Task 3: 改造 lib.rs

**Files:** Modify `src-tauri/src/lib.rs`

- [ ] **Step 1:** 新增 `mod app_setup;` 和 `mod monitor;` 声明
- [ ] **Step 2:** 移除已提取到子模块的代码
- [ ] **Step 3:** `run()` 精简为 `Builder + invoke_handler + .setup(app_setup::setup_app) + .run()`
- [ ] **Step 4:** 移除不再需要的 import

## Task 4: 迁移 runtime_tests

**Files:** Create `src-tauri/src/runtime_tests.rs`

- [ ] **Step 1:** 将 runtime_tests 模块内容移至独立文件
- [ ] **Step 2:** 调整 import 路径（`use super::*` → `use crate::*`）
- [ ] **Step 3:** lib.rs 中改为 `#[cfg(test)] mod runtime_tests;`

## Task 5: 全量验证

- [ ] **Step 1:** 运行 `cargo build --manifest-path src-tauri/Cargo.toml` 确认编译通过
- [ ] **Step 2:** 运行 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` 确认无警告
- [ ] **Step 3:** 运行 `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1` 确认测试通过
- [ ] **Step 4:** 运行 `pnpm run typecheck` 确认前端类型检查通过
- [ ] **Step 5:** 运行 `pnpm run test:web` 确认前端测试通过
- [ ] **Step 6:** 确认 lib.rs 行数 ≤ 150
