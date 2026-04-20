import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ClipboardEntry, SearchResult, CategoryType, Tag } from "@/types";

export const useClipboardStore = defineStore("clipboard", () => {
  const entries = ref<ClipboardEntry[]>([]);
  const totalCount = ref(0);
  const selectedCategory = ref<CategoryType>("all");
  const searchKeyword = ref("");
  const isLoading = ref(false);
  const currentPage = ref(0);
  const pageSize = 50;
  const selectedTagId = ref<number | null>(null);
  const allTags = ref<Tag[]>([]);

  const hasMore = computed(
    () => entries.value.length < totalCount.value
  );

  async function fetchEntries(reset = true) {
    if (reset) {
      currentPage.value = 0;
      entries.value = [];
    }

    isLoading.value = true;
    try {
      // Handle tag-based filtering
      if (selectedCategory.value === "tags" && selectedTagId.value !== null) {
        const tagEntries = await invoke<ClipboardEntry[]>("get_entries_by_tag", {
          tagId: selectedTagId.value,
        });
        entries.value = tagEntries;
        totalCount.value = tagEntries.length;
        return;
      }

      const offset = currentPage.value * pageSize;
      const category =
        selectedCategory.value === "all" || selectedCategory.value === "favorites" || selectedCategory.value === "tags"
          ? null
          : selectedCategory.value;

      let result: SearchResult;

      if (searchKeyword.value.trim()) {
        result = await invoke<SearchResult>("search_entries", {
          keyword: searchKeyword.value.trim(),
          category,
          limit: pageSize,
          offset,
        });
      } else {
        result = await invoke<SearchResult>("get_entries", {
          limit: pageSize,
          offset,
          category,
        });
      }

      if (reset) {
        entries.value = result.entries;
      } else {
        entries.value.push(...result.entries);
      }
      totalCount.value = result.total_count;
    } catch (e) {
      console.error("Failed to fetch entries:", e);
    } finally {
      isLoading.value = false;
    }
  }

  async function loadMore() {
    if (!hasMore.value || isLoading.value) return;
    currentPage.value++;
    await fetchEntries(false);
  }

  function setCategory(cat: CategoryType) {
    selectedCategory.value = cat;
    fetchEntries(true);
  }

  function setSearch(keyword: string) {
    searchKeyword.value = keyword;
    fetchEntries(true);
  }

  async function deleteEntry(id: number) {
    try {
      await invoke("delete_entry", { id });
      entries.value = entries.value.filter((e) => e.id !== id);
      totalCount.value = Math.max(0, totalCount.value - 1);
    } catch (e) {
      console.error("Failed to delete entry:", e);
    }
  }

  async function toggleFavorite(id: number) {
    try {
      const newState = await invoke<boolean>("toggle_favorite", { id });
      const entry = entries.value.find((e) => e.id === id);
      if (entry) {
        entry.is_favorite = newState;
      }
    } catch (e) {
      console.error("Failed to toggle favorite:", e);
    }
  }

  async function pasteEntry(id: number) {
    try {
      await invoke("paste_entry", { id });
      const entry = entries.value.find((e) => e.id === id);
      if (entry) {
        entry.use_count++;
      }
    } catch (e) {
      console.error("Failed to paste entry:", e);
    }
  }

  function onClipboardChanged(entry: ClipboardEntry) {
    // Remove existing entry with same hash if present (dedup on frontend)
    entries.value = entries.value.filter((e) => e.hash !== entry.hash);
    // Prepend new entry
    entries.value.unshift(entry);
    totalCount.value++;
  }

  // --- Tag management ---

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
    fetchEntries,
    loadMore,
    setCategory,
    setSearch,
    deleteEntry,
    toggleFavorite,
    pasteEntry,
    onClipboardChanged,
    fetchAllTags,
    selectTag,
    clearTagFilter,
  };
});
