# Phase 3: Clipboard Templates Design Spec

## Overview

Clipboard Templates allow users to create reusable text snippets with parameterized placeholders. When a template is used, the user fills in placeholder values through a dialog, and the rendered result is copied to clipboard.

## Data Model

### SQLite Schema

```sql
CREATE TABLE templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT DEFAULT 'general',
    is_favorite INTEGER DEFAULT 0,
    use_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_templates_category ON templates(category);
CREATE INDEX idx_templates_name ON templates(name);
```

### Placeholder Syntax

- Format: `{{placeholder_name}}` 
- Names: alphanumeric + underscores, case-insensitive
- Same placeholder used multiple times = filled once, applied everywhere
- Example: `Hello {{name}}, welcome to {{company}}. {{name}}, your account is ready.`

## Backend (Rust)

### Module: `src-tauri/src/templates/`

**`mod.rs`** - Module exports

**`engine.rs`** - Template engine
- `extract_placeholders(content: &str) -> Vec<String>` - Extract unique placeholder names via regex `\{\{(\w+)\}\}`
- `render(content: &str, values: &HashMap<String, String>) -> String` - Replace placeholders with values

**`commands.rs`** - Tauri IPC commands
- `create_template(name, content, category) -> Template`
- `update_template(id, name, content, category) -> Template`
- `delete_template(id) -> bool`
- `get_templates(category?) -> Vec<Template>`
- `get_template(id) -> Template`
- `use_template(id, values: HashMap<String, String>) -> String` - Render + copy to clipboard + increment use_count
- `get_template_categories() -> Vec<String>`

### Database Migration

Add migration in `storage/migrations.rs` to create the `templates` table. Follow existing migration pattern (version check + CREATE IF NOT EXISTS).

## Frontend (Vue 3)

### Components

**`TemplateList.vue`** - Template list panel (sidebar view)
- Shows all templates grouped by category
- Search/filter by name
- Create new template button
- Click to use, right-click for edit/delete

**`TemplateEditor.vue`** - Create/edit dialog
- Name input
- Category selector (existing categories + create new)
- Content textarea with placeholder highlighting
- Live preview showing extracted placeholders
- Save/Cancel buttons

**`TemplateFillDialog.vue`** - Placeholder fill dialog
- Shows when user clicks "Use" on a template
- Input field for each unique placeholder
- Live preview of rendered result
- "Copy to Clipboard" button

### Store: `src/stores/templateStore.ts`

Pinia store managing template state:
- `templates: Template[]`
- `categories: string[]`
- `selectedCategory: string | null`
- Actions: `fetchTemplates()`, `createTemplate()`, `updateTemplate()`, `deleteTemplate()`, `useTemplate()`

### Navigation

Add "Templates" entry to sidebar navigation (below Tags, above Statistics). Use `FileText` icon from lucide-vue-next.

### i18n

Add keys to both `en.json` and `zh-CN.json`:
- `templates.title`, `templates.create`, `templates.edit`, `templates.delete`
- `templates.name`, `templates.content`, `templates.category`, `templates.placeholder`
- `templates.use`, `templates.fill_placeholders`, `templates.preview`
- `templates.no_templates`, `templates.confirm_delete`

## UI/UX Flow

1. User navigates to Templates panel via sidebar
2. Creates template with `{{placeholders}}` in content
3. When needing the template, clicks "Use"
4. Fill dialog appears with inputs for each placeholder
5. Live preview shows rendered text
6. Click "Copy" → rendered text copied to clipboard, use_count incremented

## Constraints

- Template names must be unique
- Maximum 50 placeholders per template
- Content max length: 10,000 characters
- Category names max: 50 characters
