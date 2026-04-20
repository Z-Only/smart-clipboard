<template>
  <div class="flex flex-col gap-1 py-2">
    <button
      v-for="cat in CATEGORIES"
      :key="cat.key"
      class="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors text-left"
      :class="[
        selectedCategory === cat.key && (cat.key !== 'tags' || selectedTagId === null)
          ? 'bg-primary text-primary-foreground'
          : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
      ]"
      @click="handleCategoryClick(cat.key)"
    >
      <span class="w-5 text-center text-xs">{{ cat.icon }}</span>
      <span class="truncate">{{ $t(cat.labelKey) }}</span>
    </button>

    <!-- Tag filter section -->
    <div v-if="allTags.length > 0" class="mt-2 pt-2 border-t border-border">
      <div class="px-3 py-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
        {{ $t('tags.filterByTag') }}
      </div>
      <button
        v-for="tag in allTags"
        :key="tag.id"
        class="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors text-left w-full group"
        :class="[
          selectedCategory === 'tags' && selectedTagId === tag.id
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
        ]"
        @click="store.selectTag(tag.id)"
      >
        <span class="w-5 text-center text-xs">&#x1f3f7;&#xfe0f;</span>
        <span class="truncate flex-1">{{ tag.name }}</span>
        <span
          class="text-[10px] opacity-0 group-hover:opacity-100 hover:text-destructive"
          @click.stop="handleDeleteTag(tag.id)"
          :title="$t('tags.deleteTag')"
        >&#x2715;</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore } from "@/stores/clipboardStore";
import { CATEGORIES } from "@/types";
import type { CategoryType } from "@/types";

const store = useClipboardStore();
const { selectedCategory, selectedTagId, allTags } = storeToRefs(store);

function handleCategoryClick(key: CategoryType) {
  store.clearTagFilter();
  store.setCategory(key);
}

async function handleDeleteTag(tagId: number) {
  try {
    await invoke("delete_tag", { id: tagId });
    await store.fetchAllTags();
    // If we were filtering by this tag, reset
    if (selectedTagId.value === tagId) {
      store.clearTagFilter();
      store.setCategory("all");
    }
  } catch (e) {
    console.error("Failed to delete tag:", e);
  }
}

onMounted(() => {
  store.fetchAllTags();
});
</script>
