# Documentation Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update product and design documentation so it accurately reflects the latest main-branch implementation and roadmap.

**Architecture:** Treat the documentation set as three layers: system design (`docs/design.md`), repository-facing product summary (`README.md` and `README.zh-CN.md`), and user-facing site pages (`site/*`). Align terminology and roadmap structure across all three layers without changing product behavior.

**Tech Stack:** Markdown, VitePress docs, Git

---

### Task 1: Refresh the system design document

**Files:**

- Modify: `docs/design.md`
- Test: manual review diff

- [ ] **Step 1: Update implemented capability sections**

Add explicit sections for smart search, knowledge organization, and search re-ranking in `docs/design.md`, and renumber subsequent sections to keep the document structure coherent.

- [ ] **Step 2: Add an implementation progress / roadmap section**

Append a new roadmap-oriented section describing completed phases, current version value, and next directions based on the current main branch.

- [ ] **Step 3: Review the document for consistency**

Read the updated file to ensure section numbering, terminology, and roadmap wording are internally consistent.

- [ ] **Step 4: Commit**

```bash
git add docs/design.md
git commit -m "docs: refresh design document status"
```

### Task 2: Align repository-facing documentation

**Files:**

- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Test: manual review diff

- [ ] **Step 1: Rewrite roadmap sections**

Replace the outdated mixed completed/future roadmap lists with “Implemented/Next” structures that reflect the current codebase.

- [ ] **Step 2: Normalize release and feature wording**

Adjust current-version summary text so both languages consistently describe Smart Search & Knowledge Organization as a current focus rather than a future addition.

- [ ] **Step 3: Review the bilingual docs together**

Check the two README files side by side for feature parity, terminology consistency, and no accidental regressions.

- [ ] **Step 4: Commit**

```bash
git add README.md README.zh-CN.md
git commit -m "docs: align readme roadmap with current implementation"
```

### Task 3: Update site-facing product docs

**Files:**

- Modify: `site/guide/features.md`
- Modify: `site/index.md`
- Modify: `site/guide/screenshots.md`
- Test: manual review diff

- [ ] **Step 1: Add smart knowledge-organization content**

Extend the features guide with a dedicated section for Smart Groups, tag suggestions, related entries, and search re-ranking.

- [ ] **Step 2: Refresh homepage feature cards**

Update the homepage feature copy so it reflects the current smart-search and plugin-extension positioning.

- [ ] **Step 3: Note screenshot gaps explicitly**

Update the screenshots guide to acknowledge current capability coverage and list recommended future screenshots for already-implemented UI.

- [ ] **Step 4: Commit**

```bash
git add site/guide/features.md site/index.md site/guide/screenshots.md
git commit -m "docs: update site content for current feature set"
```

### Task 4: Final verification

**Files:**

- Modify: none
- Test: `git diff --check`

- [ ] **Step 1: Run whitespace / patch validation**

Run:

```bash
git diff --check
```

Expected: no output

- [ ] **Step 2: Inspect changed files summary**

Run:

```bash
git diff --stat
```

Expected: only documentation files changed

- [ ] **Step 3: Commit final doc batch if working as one changeset**

```bash
git add docs/design.md README.md README.zh-CN.md site/guide/features.md site/index.md site/guide/screenshots.md docs/superpowers/plans/2026-04-27-docs-alignment.md
git commit -m "docs: align design and roadmap with main branch"
```
