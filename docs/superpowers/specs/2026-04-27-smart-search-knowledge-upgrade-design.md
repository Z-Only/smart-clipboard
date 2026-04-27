# Smart Search & Knowledge Organization Upgrade — Design Spec

## Context

Smart Clipboard Manager v2.8.0 has matured into a full-featured clipboard tool with history, classification, templates, sync, security, plugins, batch operations, virtual scroll, and quick-paste. The current search system is built on SQLite FTS5 with pinyin augmentation, providing solid keyword matching for both Chinese and English content.

However, as users accumulate hundreds or thousands of clipboard entries, the core friction shifts from "can I find it by keyword?" to "can I find it when I don't remember the exact words?" and "can I see related things grouped together?". Every major productivity tool (Notion, Alfred, Raycast) has moved toward smarter organization and retrieval. This upgrade closes that gap.

## Approach: Hybrid (Phase A + Phase B Extension Point)

**Phase A (this spec):** Lightweight, zero-dependency smart features — content similarity, auto-tag suggestions, entry clustering, related-entry recommendations, and search relevance scoring. All implemented in pure Rust using n-gram and TF-IDF techniques.

**Phase B (future, out of scope):** Optional semantic/vector search via embedded ONNX model, exposed through the existing plugin system. Phase A designs its interfaces to be compatible with Phase B, but Phase B is NOT part of this implementation.

## Goals

1. Help users discover related clipboard entries without knowing exact keywords
2. Automatically suggest tags based on content similarity to already-tagged entries
3. Group similar entries into clusters visible in a new "Smart Groups" view
4. Show "Related entries" when viewing any single entry
5. Improve search result ranking with relevance scoring beyond FTS5 default
6. Design similarity/scoring interfaces that a future vector-search plugin can implement

## Non-Goals

- Embedding model integration (Phase B)
- Cloud-based AI/LLM features
- Changing the existing FTS5 search behavior (additive only)
- Cross-device cluster sync (clusters are local-only)
- Real-time clustering (batch/on-demand is sufficient)

---

## Architecture

### New Rust Module: `analyzer/similarity.rs`

A pure-Rust text similarity engine with zero external crate dependencies (uses only std + existing `regex`/`sha2`). Provides:

1. **N-gram tokenizer**: Character-level bigrams and trigrams for short texts, word-level for longer content
2. **TF-IDF scoring**: Lightweight term-frequency / inverse-document-frequency for corpus-aware relevance
3. **Jaccard similarity**: Set-based similarity metric for tag suggestion
4. **Cosine similarity**: Vector-space similarity on TF-IDF vectors for clustering and recommendations

### Similarity Trait (Phase B Extension Point)

```rust
/// Trait for content similarity scoring.
/// Phase A: implemented by `NgramSimilarityEngine`.
/// Phase B: a plugin could provide a `VectorSimilarityEngine` wrapper.
pub trait SimilarityScorer: Send + Sync {
    /// Score similarity between two content strings. Returns 0.0..=1.0.
    fn score(&self, content_a: &str, content_b: &str) -> f64;

    /// Score similarity of `query` against a batch of candidates.
    /// Returns Vec<(candidate_index, score)> sorted by score descending.
    fn score_batch(&self, query: &str, candidates: &[&str]) -> Vec<(usize, f64)>;

    /// Return the engine name for logging/diagnostics.
    fn engine_name(&self) -> &str;
}
```

This trait is the Phase B extension point. The plugin system's `content-processor` hook pattern can be extended to register alternative `SimilarityScorer` implementations.

### New Storage: Cluster Tables

```sql
-- Cluster definitions
CREATE TABLE IF NOT EXISTS entry_clusters (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT NOT NULL,           -- auto-generated or user-edited label
    created_at  DATETIME DEFAULT (datetime('now', 'localtime')),
    updated_at  DATETIME DEFAULT (datetime('now', 'localtime'))
);

-- Cluster membership (many-to-many)
CREATE TABLE IF NOT EXISTS entry_cluster_members (
    cluster_id  INTEGER NOT NULL REFERENCES entry_clusters(id) ON DELETE CASCADE,
    entry_id    INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
    score       REAL NOT NULL DEFAULT 0.0,  -- similarity score to cluster centroid
    PRIMARY KEY (cluster_id, entry_id)
);

CREATE INDEX IF NOT EXISTS idx_cluster_members_entry
    ON entry_cluster_members(entry_id);
```

### New Storage: Tag Suggestion Cache

```sql
-- Cached tag suggestions to avoid recomputing on every view
CREATE TABLE IF NOT EXISTS tag_suggestions (
    entry_id    INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
    tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    confidence  REAL NOT NULL DEFAULT 0.0,  -- similarity confidence 0.0..1.0
    created_at  DATETIME DEFAULT (datetime('now', 'localtime')),
    PRIMARY KEY (entry_id, tag_id)
);
```

### Data Flow

```
New entry arrives (clipboard monitor)
    → classify() (existing)
    → detect_sensitive() (existing)
    → store in DB (existing)
    → [NEW] compute_tag_suggestions(entry)
        → find entries with tags that are similar
        → store suggestions in tag_suggestions table
    → [NEW] assign_to_cluster(entry)
        → compare against existing cluster centroids
        → if similarity > threshold → add to cluster
        → else → mark as unclustered (will be picked up in next batch)

User opens "Smart Groups" view
    → frontend requests cluster list with entry counts
    → user clicks a cluster → shows entries in that cluster

User views a single entry
    → frontend requests related entries
    → backend: find_related(entry_id, limit=5)
        → uses SimilarityScorer to find top-5 most similar entries

Batch re-cluster (manual trigger or periodic)
    → load all unclustered entries
    → run agglomerative clustering with similarity threshold
    → create/update clusters
    → auto-label clusters from most common category + keywords

Search with relevance scoring
    → existing FTS5 search returns candidates
    → [NEW] re-rank candidates using TF-IDF similarity to query
    → return re-ranked results
```

---

## Component Design

### Backend (Rust)

#### 1. `analyzer/similarity.rs` (New)

Core similarity computation module.

**Public API:**

```rust
pub struct NgramSimilarityEngine {
    ngram_size: usize,  // default 3 for trigrams
}

impl SimilarityScorer for NgramSimilarityEngine { ... }

/// Lightweight TF-IDF index for a small corpus (in-memory).
pub struct TfIdfIndex {
    documents: Vec<HashMap<String, f64>>,  // term → tf-idf weight per doc
    idf: HashMap<String, f64>,             // term → idf
}

impl TfIdfIndex {
    pub fn build(contents: &[&str]) -> Self;
    pub fn query_similarity(&self, query: &str) -> Vec<(usize, f64)>;
}
```

#### 2. `analyzer/clustering.rs` (New)

Entry clustering logic.

**Public API:**

```rust
pub struct ClusterEngine {
    scorer: Arc<dyn SimilarityScorer>,
    similarity_threshold: f64,  // default 0.3
    max_cluster_size: usize,    // default 50
}

impl ClusterEngine {
    pub fn assign_entry(&self, entry_content: &str, existing_clusters: &[ClusterSummary]) -> Option<i64>;
    pub fn recluster(&self, entries: &[(i64, String)]) -> Vec<ClusterResult>;
    pub fn generate_label(&self, entry_contents: &[&str], category: &str) -> String;
}
```

#### 3. `analyzer/suggestions.rs` (New)

Tag suggestion logic.

**Public API:**

```rust
pub struct TagSuggester {
    scorer: Arc<dyn SimilarityScorer>,
    min_confidence: f64,  // default 0.25
    max_suggestions: usize,  // default 3
}

impl TagSuggester {
    pub fn suggest_tags(
        &self,
        entry_content: &str,
        tagged_entries: &[(String, Vec<Tag>)],  // (content, tags) of already-tagged entries
    ) -> Vec<TagSuggestion>;
}

pub struct TagSuggestion {
    pub tag: Tag,
    pub confidence: f64,
}
```

#### 4. `storage/database.rs` additions

New query methods (added to existing `Database` impl):

- `get_cluster_list() -> Vec<ClusterSummary>` — list all clusters with entry count
- `get_cluster_entries(cluster_id, limit, offset) -> SearchResult` — entries in a cluster
- `upsert_cluster(label) -> i64` — create or get cluster
- `add_to_cluster(cluster_id, entry_id, score)` — add entry to cluster
- `remove_from_cluster(cluster_id, entry_id)` — remove entry from cluster
- `get_unclustered_entries(limit) -> Vec<ClipboardEntry>` — entries not in any cluster
- `save_tag_suggestions(entry_id, suggestions)` — persist tag suggestions
- `get_tag_suggestions(entry_id) -> Vec<TagSuggestion>` — retrieve suggestions
- `find_related_entries(entry_id, limit) -> Vec<(ClipboardEntry, f64)>` — related by similarity
- `clear_clusters()` — reset all clusters (for re-clustering)

#### 5. New Tauri IPC commands

In `commands/` (new file `commands/smart.rs`):

- `get_clusters` — list clusters
- `get_cluster_entries` — entries in a cluster
- `trigger_recluster` — manual re-cluster trigger
- `get_tag_suggestions` — suggestions for an entry
- `accept_tag_suggestion` — apply a suggested tag
- `dismiss_tag_suggestion` — dismiss a suggestion
- `get_related_entries` — related entries for a given entry

#### 6. Configuration additions

In `config.rs`, new fields:

```rust
pub smart_search_enabled: bool,           // default true
pub cluster_similarity_threshold: f64,    // default 0.3
pub tag_suggestion_min_confidence: f64,   // default 0.25
pub max_related_entries: u8,              // default 5
```

### Frontend (Vue 3)

#### 1. `SmartGroupsPanel.vue` (New)

A new panel (alongside Templates, Sync, Statistics) showing auto-generated clusters.

**Layout:**

```
┌─────────────────────────────────────┐
│  Smart Groups           [Refresh ↻] │
├─────────────────────────────────────┤
│  📁 Code Snippets (23)        →     │
│  📁 URLs & Links (18)         →     │
│  📁 Meeting Notes (12)        →     │
│  📁 API Keys & Configs (8)    →     │
│  📁 Email Addresses (6)       →     │
│  📁 Unclustered (45)          →     │
├─────────────────────────────────────┤
│  Last updated: 2 min ago            │
└─────────────────────────────────────┘
```

Clicking a cluster navigates to a filtered view showing cluster entries.

#### 2. `TagSuggestionBadge.vue` (New)

Small inline component shown on `EntryCard.vue` when tag suggestions exist.

**Layout:**

```
┌────────────────────────────────────────┐
│ 📋 Meeting notes from standup...       │
│ text · 2 min ago                       │
│ 💡 Suggested: [work] [meeting] [+]  ✕  │
└────────────────────────────────────────┘
```

- Clicking a suggestion chip applies the tag
- Clicking ✕ dismisses all suggestions for this entry
- Clicking [+] opens the existing TagPicker

#### 3. `RelatedEntries.vue` (New)

A collapsible section shown below an entry's detail/expanded view.

**Layout:**

```
┌────────────────────────────────────────┐
│ ▼ Related (3)                          │
│   87% │ Similar meeting notes from...  │
│   74% │ Standup action items fo...     │
│   61% │ Weekly sync agenda...          │
└────────────────────────────────────────┘
```

#### 4. Store additions: `smartStore.ts` (New)

```typescript
// New Pinia store for smart search features
export const useSmartStore = defineStore('smart', () => {
  const clusters = ref<Cluster[]>([]);
  const tagSuggestions = ref<Map<number, TagSuggestion[]>>(new Map());
  const relatedEntries = ref<Map<number, RelatedEntry[]>>(new Map());
  const isReclustering = ref(false);

  async function fetchClusters(): Promise<void>;
  async function fetchClusterEntries(clusterId: number): Promise<ClipboardEntry[]>;
  async function triggerRecluster(): Promise<void>;
  async function fetchTagSuggestions(entryId: number): Promise<TagSuggestion[]>;
  async function acceptTagSuggestion(entryId: number, tagId: number): Promise<void>;
  async function dismissTagSuggestions(entryId: number): Promise<void>;
  async function fetchRelatedEntries(entryId: number): Promise<RelatedEntry[]>;
});
```

#### 5. Types additions

```typescript
interface Cluster {
  id: number;
  label: string;
  entryCount: number;
  createdAt: string;
  updatedAt: string;
}

interface TagSuggestion {
  tag: Tag;
  confidence: number;
}

interface RelatedEntry {
  entry: ClipboardEntry;
  score: number;
}
```

---

## Search Relevance Enhancement

The current FTS5 search returns results ordered by SQLite's internal BM25 ranking. This upgrade adds a **re-ranking pass**:

1. FTS5 returns candidate set (up to 3× requested limit)
2. Build a mini TF-IDF index from the candidates
3. Score each candidate against the query using TF-IDF cosine similarity
4. Combine FTS5 rank (normalized) and TF-IDF score: `final = 0.6 * fts_rank + 0.4 * tfidf_score`
5. Sort by final score, return requested limit

This happens entirely in Rust, adds minimal latency (< 5ms for 150 candidates), and produces noticeably better ordering for ambiguous queries.

---

## Performance Considerations

- **Similarity computation on insert**: Single entry vs. tagged entries is O(T) where T = number of tagged entries. With trigram sets pre-cached, this takes < 2ms for T=100.
- **Clustering**: Batch re-cluster is O(N²) pairwise comparisons. For N=1000, this takes ~200ms. For N=5000+, we use sampling (random 1000 entries per cluster candidate). Runs in background thread, never blocks UI.
- **Memory**: TF-IDF index for 1000 entries uses ~2-4MB RAM. Built on-demand, dropped after use.
- **Storage**: Two new tables add negligible disk overhead. Tag suggestions are pruned when entries are deleted (CASCADE).

---

## Security Considerations

- Clusters and tag suggestions follow existing security model: locked state blocks all smart commands via `require_unlocked()` guard
- If database encryption is enabled, cluster/suggestion tables are in the same encrypted database
- No data leaves the device — all computation is local

---

## Testing Strategy

### Rust unit tests

- `similarity.rs`: Trigram tokenization, Jaccard/cosine similarity scoring, edge cases (empty, identical, CJK)
- `clustering.rs`: Cluster assignment, re-clustering, label generation
- `suggestions.rs`: Tag suggestion with varying confidence thresholds
- `database.rs`: CRUD for clusters, suggestions; related entries query

### Frontend unit tests

- `SmartGroupsPanel.vue`: Render cluster list, refresh action, empty state
- `TagSuggestionBadge.vue`: Render suggestions, accept/dismiss actions
- `RelatedEntries.vue`: Render related list, empty state, collapse/expand
- `smartStore.ts`: All store actions with mocked IPC

### Integration validation

- End-to-end: copy several similar items → verify cluster forms → verify tag suggestions appear
- Performance: 1000-entry re-cluster completes in < 500ms

---

## Internationalization

All user-facing strings go through vue-i18n:

- `smart.groups` — "Smart Groups"
- `smart.refresh` — "Refresh"
- `smart.reclustering` — "Analyzing..."
- `smart.unclustered` — "Unclustered"
- `smart.lastUpdated` — "Last updated: {time}"
- `smart.related` — "Related ({count})"
- `smart.suggestedTags` — "Suggested:"
- `smart.noGroups` — "Not enough entries for smart grouping yet"
- `smart.noRelated` — "No related entries found"

---

## Phase B Extension Notes (Out of Scope)

When a vector search plugin is implemented in the future:

1. The plugin registers a `VectorSimilarityScorer` implementing the `SimilarityScorer` trait
2. Configuration switches the active scorer from `NgramSimilarityEngine` to the plugin's implementation
3. The embedding model runs inference on entry content and stores vectors in a separate table (or sidecar file)
4. All existing clustering, suggestion, and related-entry logic works unchanged — only the scoring backend differs
5. The plugin manages its own model lifecycle (download, load, unload)

This architecture means Phase A delivers real value now, and Phase B is a drop-in upgrade with zero structural changes.

---

## File Structure Summary

| Action | File                                    | Responsibility                                                                |
| ------ | --------------------------------------- | ----------------------------------------------------------------------------- |
| Create | `src-tauri/src/analyzer/similarity.rs`  | N-gram tokenizer, TF-IDF, Jaccard/cosine similarity, `SimilarityScorer` trait |
| Create | `src-tauri/src/analyzer/clustering.rs`  | Cluster assignment, batch re-cluster, label generation                        |
| Create | `src-tauri/src/analyzer/suggestions.rs` | Tag suggestion engine                                                         |
| Modify | `src-tauri/src/analyzer/mod.rs`         | Re-export new modules                                                         |
| Modify | `src-tauri/src/storage/migrations.rs`   | Add cluster and suggestion tables                                             |
| Modify | `src-tauri/src/storage/database.rs`     | Cluster/suggestion CRUD, related entries, search re-ranking                   |
| Modify | `src-tauri/src/storage/models.rs`       | New model structs                                                             |
| Create | `src-tauri/src/commands/smart.rs`       | Smart search IPC commands                                                     |
| Modify | `src-tauri/src/commands/mod.rs`         | Re-export smart commands                                                      |
| Modify | `src-tauri/src/lib.rs`                  | Register smart commands                                                       |
| Modify | `src-tauri/src/config.rs`               | Smart search config fields                                                    |
| Create | `src/components/SmartGroupsPanel.vue`   | Cluster list UI                                                               |
| Create | `src/components/TagSuggestionBadge.vue` | Inline tag suggestion chips                                                   |
| Create | `src/components/RelatedEntries.vue`     | Related entries section                                                       |
| Create | `src/stores/smartStore.ts`              | Pinia store for smart features                                                |
| Modify | `src/types/index.ts`                    | New TypeScript types                                                          |
| Modify | `src/i18n/locales/en.ts`                | English i18n keys                                                             |
| Modify | `src/i18n/locales/zh-CN.ts`             | Chinese i18n keys                                                             |
| Modify | `src/App.vue`                           | Mount SmartGroupsPanel                                                        |
| Create | `tests/unit/smartStore.test.ts`         | Store unit tests                                                              |
| Create | `tests/unit/SmartGroupsPanel.test.ts`   | Panel unit tests                                                              |
| Create | `tests/unit/TagSuggestionBadge.test.ts` | Badge unit tests                                                              |
| Create | `tests/unit/RelatedEntries.test.ts`     | Related entries unit tests                                                    |
