<template>
  <ScrollArea class="flex-1">
    <div v-if="isLoading && entries.length === 0" class="flex items-center justify-center h-40 text-muted-foreground">
      <svg class="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
      Loading...
    </div>

    <div v-else-if="entries.length === 0" class="flex flex-col items-center justify-center h-40 text-muted-foreground">
      <svg class="h-10 w-10 mb-2 opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
        stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <rect width="8" height="4" x="8" y="2" rx="1" ry="1" />
        <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
      </svg>
      <span class="text-sm">No entries yet</span>
      <span class="text-xs mt-1">Copy something to get started</span>
    </div>

    <div v-else ref="listRef" class="flex flex-col">
      <template v-for="(group, date) in groupedEntries" :key="date">
        <div class="sticky top-0 z-10 bg-background/95 backdrop-blur-sm px-3 py-1.5">
          <span class="text-xs font-medium text-muted-foreground">{{ date }}</span>
        </div>
        <EntryCard
          v-for="entry in group"
          :key="entry.id"
          :entry="entry"
          :is-selected="selectedIndex >= 0 && entries[selectedIndex]?.id === entry.id"
          @select="handleSelect"
          @toggle-favorite="store.toggleFavorite"
          @delete="store.deleteEntry"
        />
      </template>

      <div v-if="hasMore" ref="sentinelRef" class="flex items-center justify-center py-4 text-muted-foreground text-xs">
        <span v-if="isLoading">Loading more...</span>
        <span v-else>&nbsp;</span>
      </div>
    </div>
  </ScrollArea>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { storeToRefs } from "pinia";
import { ScrollArea } from "@/components/ui/scroll-area";
import EntryCard from "./EntryCard.vue";
import { useClipboardStore } from "@/stores/clipboardStore";

const store = useClipboardStore();
const { entries, isLoading, hasMore } = storeToRefs(store);

const selectedIndex = ref(-1);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

const groupedEntries = computed(() => {
  const groups: Record<string, typeof entries.value> = {};
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  for (const entry of entries.value) {
    const date = new Date(entry.created_at);
    let label: string;

    if (isSameDay(date, today)) {
      label = "Today";
    } else if (isSameDay(date, yesterday)) {
      label = "Yesterday";
    } else {
      label = date.toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
        year: date.getFullYear() !== today.getFullYear() ? "numeric" : undefined,
      });
    }

    if (!groups[label]) groups[label] = [];
    groups[label].push(entry);
  }

  return groups;
});

function isSameDay(a: Date, b: Date) {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function handleSelect(id: number) {
  store.pasteEntry(id);
}

function handleKeydown(e: KeyboardEvent) {
  if (entries.value.length === 0) return;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, entries.value.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
  } else if (e.key === "Enter" && selectedIndex.value >= 0) {
    e.preventDefault();
    const entry = entries.value[selectedIndex.value];
    if (entry) store.pasteEntry(entry.id);
  }
}

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);

  // Infinite scroll observer
  observer = new IntersectionObserver(
    (es) => {
      if (es[0]?.isIntersecting) {
        store.loadMore();
      }
    },
    { threshold: 0.1 }
  );

  if (sentinelRef.value) {
    observer.observe(sentinelRef.value);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
  observer?.disconnect();
});

defineExpose({ selectedIndex });
</script>
