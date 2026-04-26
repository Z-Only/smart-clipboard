# 测试覆盖全面提升（Store + 组件） Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 templateStore、securityStore 补充测试覆盖至 ≥70%，为 14 个零覆盖组件编写基础测试，整体覆盖率阈值提升至 35/20/30/35。

**Architecture:** 遵循项目已有测试模式（vi.mock Tauri IPC、createPinia、动态 import store、@vue/test-utils mount），按功能域分组编写测试。不修改业务代码，仅新增/修改测试文件和调整覆盖率阈值。

**Tech Stack:** Vitest, Pinia, Vue 3, TypeScript, @vue/test-utils

---

## File Map

- **Modify:** `tests/unit/templateStore.test.ts` — 补充 fetchTemplates/fetchCategories/updateTemplate/deleteTemplate/setCategory/filteredTemplates
- **Modify:** `tests/unit/securityStore.test.ts` — 补充 updateSettings/enableEncryption/disableEncryption/refreshEncryption/refresh
- **Create:** `tests/unit/SearchBar.test.ts`
- **Create:** `tests/unit/CategoryFilter.test.ts`
- **Create:** `tests/unit/LockScreen.test.ts`
- **Create:** `tests/unit/EntryCard.test.ts`
- **Create:** `tests/unit/TagPicker.test.ts`
- **Create:** `tests/unit/BatchTagDialog.test.ts`
- **Create:** `tests/unit/DeviceCard.test.ts`
- **Create:** `tests/unit/PairConfirmDialog.test.ts`
- **Create:** `tests/unit/TemplateFillDialog.test.ts`
- **Create:** `tests/unit/StatisticsPanel.test.ts`
- **Create:** `tests/unit/TemplateEditor.test.ts`
- **Create:** `tests/unit/TemplateList.test.ts`
- **Create:** `tests/unit/ConflictLogPanel.test.ts`
- **Create:** `tests/unit/ConflictResolveDialog.test.ts`
- **Modify:** `vite.config.ts` — 提升覆盖率阈值

## Task 1: templateStore 测试补充

## Task 2: securityStore 测试补充

## Task 3-16: 14 个零覆盖组件基础测试

## Task 17: 提升覆盖率阈值并全量验证
