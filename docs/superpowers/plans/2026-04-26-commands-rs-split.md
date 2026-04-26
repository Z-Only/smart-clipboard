# commands.rs 拆分重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 1242 行的 commands.rs 拆分为 1 个入口模块 + 7 个功能子模块，降低单文件复杂度，同时保持所有命令路径和行为完全不变。

**Architecture:** 将 commands.rs 替换为 commands/ 目录。mod.rs 声明子模块并 re-export 所有 pub 命令函数和 transform 子模块。每个子模块按功能域独立：security、clipboard、updater、config、tags、sync、transform。测试模块随文件一起迁移。

**Tech Stack:** Rust, Tauri v2

---

## File Map

- **Delete:** `src-tauri/src/commands.rs`
- **Create:** `src-tauri/src/commands/mod.rs` — 入口，声明子模块 + re-export + 共享辅助函数
- **Create:** `src-tauri/src/commands/security.rs` — 5 个安全/锁命令
- **Create:** `src-tauri/src/commands/clipboard.rs` — 10 个剪贴板条目命令
- **Create:** `src-tauri/src/commands/updater.rs` — 5 个更新器命令
- **Create:** `src-tauri/src/commands/config.rs` — 5 个配置/自启/quit 命令
- **Create:** `src-tauri/src/commands/tags.rs` — 8 个标签命令
- **Create:** `src-tauri/src/commands/sync.rs` — 15 个同步+加密命令
- **Create:** `src-tauri/src/commands/transform.rs` — transform 子模块
- **Create:** `src-tauri/src/commands/tests.rs` — command_guard_tests

## Task 1: 创建 commands/ 目录结构和 mod.rs

## Task 2: 创建 security.rs (安全/锁命令)

## Task 3: 创建 clipboard.rs (剪贴板条目命令)

## Task 4: 创建 updater.rs (更新器命令)

## Task 5: 创建 config.rs (配置/自启/quit命令)

## Task 6: 创建 tags.rs (标签命令)

## Task 7: 创建 sync.rs (同步+加密命令)

## Task 8: 创建 transform.rs (转换子模块)

## Task 9: 创建 tests.rs (集成测试)

## Task 10: 删除旧 commands.rs 并全量验证
