import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryType,
  ClipboardEntry,
  ClipboardListGroupMeta,
  ClipboardListItem,
  SearchResult,
  Tag,
} from "@/types";
import i18n from "@/i18n";

const pageSize = 50;

function isSameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function formatGroupLabel(dateValue: string) {
  const { t, locale } = i18n.global;
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  const date = new Date(dateValue);

  if (isSameDay(date, today)) return t("list.today");
  if (isSameDay(date, yesterday)) return t("list.yesterday");

  return new Intl.DateTimeFormat(locale.value, {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== today.getFullYear() ? "numeric" : undefined,
  }).format(date);
}

export const useClipboardStore = defineStore("clipboard", () => {
  const entries = ref<ClipboardEntry[]>([]);
  const totalCount = ref(0);
  const selectedCategory = ref<CategoryType>("all");
  const searchKeyword = ref("");
  const isLoading = ref(false);
  const currentPage = ref(0);
  const selectedTagId = ref<number | null>(null);
  const allTags = ref<Tag[]>([]);
  const activeEntryId = ref<number | null>(null);
  const isMultiSelectMode = ref(false);
  const selectedEntryIds = ref<number[]>([]);
  const selectionAnchorId = ref<number | null>(null);
  const pendingLoadMore = ref(false);

  const hasMore = computed(() => entries.value.length < totalCount.value);
  const selectedEntryIdSet = computed(() => new Set(selectedEntryIds.value));
  const selectedCount = computed(() => selectedEntryIds.value.length);
  const selectedEntries = computed(() => {
    const idSet = selectedEntryIdSet.value;
    return entries.value.filter((entry) => idSet.has(entry.id));
  });
  const canBatchCopy = computed(() => selectedEntries.value.some((entry) => entry.content_type !== "image"));

  const groupedEntryItems = computed<ClipboardListItem[]>(() => {
    const items: ClipboardListItem[] = [];
    let currentGroup: ClipboardListGroupMeta | null = null;

    for (const entry of entries.value) {
      const dateKey = entry.created_at.slice(0, 10);
      if (!currentGroup || currentGroup.dateKey !== dateKey) {
        currentGroup = { key: `group-${dateKey}`, dateKey, label: formatGroupLabel(entry.created_at) };
        items.push({ type: "group", key: currentGroup.key, group: currentGroup });
      }
      items.push({ type: "entry", key: `entry-${entry.id}`, entry, groupKey: currentGroup.key });
    }
    return items;
  });

  const entryIndexMap = computed(() => new Map(entries.value.map((entry, index) => [entry.id, index])));

  function reconcileSelection() {
    const idSet = new Set(entries.value.map((entry) => entry.id));
    selectedEntryIds.value = selectedEntryIds.value.filter((id) => idSet.has(id));

    if (selectionAnchorId.value !== null && !idSet.has(selectionAnchorId.value)) {
      selectionAnchorId.value = selectedEntryIds.value[0] ?? null;
    }
    if (activeEntryId.value !== null && !idSet.has(activeEntryId.value)) {
      activeEntryId.value = entries.value[0]?.id ?? null;
    }
    if (selectedEntryIds.value.length === 0) {
      isMultiSelectMode.value = false;
    }
  }

  function setEntries(nextEntries: ClipboardEntry[], reset: boolean) {
    entries.value = reset ? nextEntries : [...entries.value, ...nextEntries];
    reconcileSelection();
  }

  function resetListState() {
    currentPage.value = 0;
    pendingLoadMore.value = false;
    entries.value = [];
    activeEntryId.value = null;
    selectionAnchorId.value = null;
    clearSelection();
  }

  async function fetchEntries(reset = true) {
    if (reset) resetListState();
    isLoading.value = true;
    try {
      if (selectedCategory.value === "tags" && selectedTagId.value !== null) {
        const tagEntries = await invoke<ClipboardEntry[]>("get_entries_by_tag", { tagId: selectedTagId.value });
        setEntries(tagEntries, true);
        totalCount.value = tagEntries.length;
        activeEntryId.value = tagEntries[0]?.id ?? null;
        return;
      }

      const offset = currentPage.value * pageSize;
      const isFavoriteFilter = selectedCategory.value === "favorites" ? true : null;
      const category = selectedCategory.value === "all" || selectedCategory.value === "favorites" || selectedCategory.value === "tags"
        ? null
        : selectedCategory.value;

      const result = searchKeyword.value.trim()
        ? await invoke<SearchResult>("search_entries", { keyword: searchKeyword.value.trim(), category, isFavorite: isFavoriteFilter, limit: pageSize, offset })
        : await invoke<SearchResult>("get_entries", { limit: pageSize, offset, category, isFavorite: isFavoriteFilter });

      const nextEntries = result.entries;
      totalCount.value = result.total_count;

      setEntries(nextEntries, reset);
      if (reset) activeEntryId.value = nextEntries[0]?.id ?? null;
    } catch (e) {
      console.error("Failed to fetch entries:", e);
    } finally {
      isLoading.value = false;
    }
  }

  async function loadMore() {
    if (!hasMore.value || isLoading.value || pendingLoadMore.value) return;
    pendingLoadMore.value = true;
    currentPage.value++;
    try {
      await fetchEntries(false);
    } finally {
      pendingLoadMore.value = false;
    }
  }

  function setCategory(cat: CategoryType) {
    selectedCategory.value = cat;
    fetchEntries(true);
  }

  function setSearch(keyword: string) {
    searchKeyword.value = keyword;
    fetchEntries(true);
  }

  function setActiveEntry(id: number | null) {
    activeEntryId.value = id;
  }

  function enterMultiSelectMode(initialId?: number) {
    isMultiSelectMode.value = true;
    if (typeof initialId === "number") {
      selectionAnchorId.value = initialId;
      toggleEntrySelection(initialId, true);
    }
  }

  function exitMultiSelectMode() {
    isMultiSelectMode.value = false;
    selectionAnchorId.value = null;
    clearSelection();
  }

  function clearSelection() {
    selectedEntryIds.value = [];
  }

  function invertLoadedSelection() {
    const current = new Set(selectedEntryIds.value);
    selectedEntryIds.value = entries.value
      .map((entry) => entry.id)
      .filter((id) => !current.has(id));
    if (selectedEntryIds.value.length > 0) {
      isMultiSelectMode.value = true;
      selectionAnchorId.value = selectionAnchorId.value ?? selectedEntryIds.value[0] ?? null;
    } else {
      selectionAnchorId.value = null;
    }
  }

  function selectAllLoadedEntries() {
    if (!isMultiSelectMode.value) isMultiSelectMode.value = true;
    selectedEntryIds.value = entries.value.map((entry) => entry.id);
    selectionAnchorId.value = activeEntryId.value ?? entries.value[0]?.id ?? null;
  }

  function selectRangeTo(id: number) {
    const anchorId = selectionAnchorId.value ?? activeEntryId.value ?? id;
    const start = entryIndexMap.value.get(anchorId);
    const end = entryIndexMap.value.get(id);
    if (start === undefined || end === undefined) {
      toggleEntrySelection(id, true);
      return;
    }

    isMultiSelectMode.value = true;
    const [from, to] = start <= end ? [start, end] : [end, start];
    selectedEntryIds.value = entries.value.slice(from, to + 1).map((entry) => entry.id);
  }

  function toggleEntrySelection(id: number, force?: boolean) {
    const current = new Set(selectedEntryIds.value);
    const shouldSelect = typeof force === "boolean" ? force : !current.has(id);
    if (shouldSelect) current.add(id); else current.delete(id);
    selectedEntryIds.value = Array.from(current);
    if (shouldSelect) selectionAnchorId.value = selectionAnchorId.value ?? id;
    if (selectedEntryIds.value.length === 0) {
      isMultiSelectMode.value = false;
      selectionAnchorId.value = null;
    }
  }

  function handleEntryPrimaryAction(id: number, options?: { range?: boolean }) {
    if (isMultiSelectMode.value) {
      if (options?.range) {
        selectRangeTo(id);
      } else {
        toggleEntrySelection(id);
        selectionAnchorId.value = id;
      }
      activeEntryId.value = id;
      return;
    }

    activeEntryId.value = id;
    pasteEntry(id);
  }

  async function deleteEntry(id: number) {
    try {
      await invoke("delete_entry", { id });
      entries.value = entries.value.filter((e) => e.id !== id);
      totalCount.value = Math.max(0, totalCount.value - 1);
      reconcileSelection();
    } catch (e) {
      console.error("Failed to delete entry:", e);
    }
  }

  async function deleteSelectedEntries() {
    const ids = [...selectedEntryIds.value];
    if (ids.length === 0) return;
    try {
      const deleted = await invoke<number>("delete_entries", { ids });
      if (deleted > 0) {
        const idSet = new Set(ids);
        entries.value = entries.value.filter((entry) => !idSet.has(entry.id));
        totalCount.value = Math.max(0, totalCount.value - deleted);
      }
      exitMultiSelectMode();
      reconcileSelection();
    } catch (e) {
      console.error("Failed to delete selected entries:", e);
    }
  }

  async function copySelectedEntries() {
    const ids = selectedEntries.value.map((entry) => entry.id);
    if (ids.length === 0) return;
    try {
      await invoke<string>("copy_entries", { ids });
    } catch (e) {
      console.error("Failed to copy selected entries:", e);
    }
  }

  async function favoriteSelectedEntries(favorite: boolean) {
    const ids = selectedEntries.value.map((entry) => entry.id);
    if (ids.length === 0) return;
    try {
      await invoke<number>("set_favorite_state_for_entries", { ids, favorite });
      const selectedIdSet = new Set(ids);
      for (const entry of entries.value) {
        if (selectedIdSet.has(entry.id)) {
          entry.is_favorite = favorite;
        }
      }
      if (selectedCategory.value === "favorites" && !favorite) {
        entries.value = entries.value.filter((entry) => entry.is_favorite);
        totalCount.value = entries.value.length;
        reconcileSelection();
      }
    } catch (e) {
      console.error("Failed to update favorite state for selected entries:", e);
    }
  }

  async function toggleFavorite(id: number) {
    try {
      const newState = await invoke<boolean>("toggle_favorite", { id });
      const entry = entries.value.find((e) => e.id === id);
      if (entry) entry.is_favorite = newState;
      if (selectedCategory.value === "favorites" && !newState) {
        entries.value = entries.value.filter((e) => e.id !== id);
        totalCount.value = Math.max(0, totalCount.value - 1);
        reconcileSelection();
      }
    } catch (e) {
      console.error("Failed to toggle favorite:", e);
    }
  }

  async function pasteEntry(id: number) {
    try {
      await invoke("paste_entry", { id });
      const entry = entries.value.find((e) => e.id === id);
      if (entry) entry.use_count++;
    } catch (e) {
      console.error("Failed to paste entry:", e);
    }
  }

  function onClipboardChanged(entry: ClipboardEntry) {
    entries.value = entries.value.filter((e) => e.hash !== entry.hash);
    entries.value.unshift(entry);
    totalCount.value++;
    if (activeEntryId.value === null) activeEntryId.value = entry.id;
    reconcileSelection();
  }

  async function fetchAllTags() {
    try {
      allTags.value = await invoke<Tag[]>("get_all_tags");
    } catch (e) {
      console.error("Failed to fetch tags:", e);
    }
  }

  function selectTag(tagId: number) {
    selectedTagId.value = tagId;
    selectedCategory.value = "tags";
    fetchEntries(true);
  }

  function clearTagFilter() {
    selectedTagId.value = null;
  }

  return {
    entries,
    totalCount,
    selectedCategory,
    searchKeyword,
    isLoading,
    hasMore,
    selectedTagId,
    allTags,
    activeEntryId,
    isMultiSelectMode,
    selectedEntryIds,
    selectedEntryIdSet,
    selectedCount,
    selectedEntries,
    canBatchCopy,
    groupedEntryItems,
    selectionAnchorId,
    fetchEntries,
    loadMore,
    setCategory,
    setSearch,
    setActiveEntry,
    enterMultiSelectMode,
    exitMultiSelectMode,
    clearSelection,
    invertLoadedSelection,
    selectAllLoadedEntries,
    selectRangeTo,
    toggleEntrySelection,
    handleEntryPrimaryAction,
    deleteEntry,
    deleteSelectedEntries,
    copySelectedEntries,
    favoriteSelectedEntries,
    toggleFavorite,
    pasteEntry,
    onClipboardChanged,
    fetchAllTags,
    selectTag,
    clearTagFilter,
  };
});
