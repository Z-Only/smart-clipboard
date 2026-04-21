<template>
  <div class="flex h-full min-h-0 flex-col">
    <BatchTagDialog
      :is-open="showBatchTagDialog"
      :count="selectedCount"
      @close="showBatchTagDialog = false"
      @apply="handleApplyBatchTags"
    />
    <div v-if="isMultiSelectMode" class="flex items-center justify-between gap-3 border-b border-border px-3 py-2 text-sm">
      <div class="flex items-center gap-2">
        <span class="font-medium">{{ $t('list.selectedCount', { count: selectedCount }) }}</span>
      </div>
      <div class="flex items-center gap-2">
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent disabled:opacity-50" :disabled="selectedCount === 0" @click="showBatchTagDialog = true">{{ $t('list.batchTag') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent disabled:opacity-50" :disabled="selectedCount === 0" @click="store.favoriteSelectedEntries(true)">{{ $t('list.batchFavorite') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent disabled:opacity-50" :disabled="selectedCount === 0" @click="store.favoriteSelectedEntries(false)">{{ $t('list.batchUnfavorite') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent disabled:opacity-50" :disabled="!canBatchCopy" @click="store.copySelectedEntries">{{ $t('list.batchCopy') }}</button>
        <button class="rounded-md border border-destructive/20 px-2.5 py-1 text-xs text-destructive transition-colors hover:bg-destructive/10 disabled:opacity-50" :disabled="selectedCount === 0" @click="store.deleteSelectedEntries">{{ $t('list.batchDelete') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent" @click="store.selectAllLoadedEntries">{{ $t('list.selectAllLoaded') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent" @click="store.invertLoadedSelection">{{ $t('list.invertSelection') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent" @click="store.clearSelection">{{ $t('list.clearSelection') }}</button>
        <button class="rounded-md border border-border px-2.5 py-1 text-xs transition-colors hover:bg-accent" @click="store.exitMultiSelectMode">{{ $t('list.exitMultiSelect') }}</button>
      </div>
    </div>

    <div class="flex items-center justify-between gap-3 border-b border-border px-3 py-2 text-xs text-muted-foreground">
      <span>{{ stickyGroupLabel || $t('list.virtualizedHint') }}</span>
      <button class="rounded-md border border-border px-2.5 py-1 transition-colors hover:bg-accent" @click="toggleMultiSelect">
        {{ isMultiSelectMode ? $t('list.exitMultiSelect') : $t('list.multiSelect') }}
      </button>
    </div>

    <div v-if="isLoading && entries.length === 0" class="flex items-center justify-center h-40 text-muted-foreground">
      <svg class="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" /></svg>
      {{ $t('list.loading') }}
    </div>

    <div v-else-if="entries.length === 0" class="flex flex-col items-center justify-center h-40 text-muted-foreground">
      <svg class="h-10 w-10 mb-2 opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="4" x="8" y="2" rx="1" ry="1" /><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" /></svg>
      <span class="text-sm">{{ $t('list.noEntries') }}</span>
      <span class="text-xs mt-1">{{ $t('list.noEntriesHint') }}</span>
    </div>

    <div v-else ref="viewportRef" class="relative flex-1 overflow-y-auto" @scroll="handleScroll">
      <div v-if="stickyGroupLabel" class="pointer-events-none sticky top-0 z-20 bg-background/95 px-3 py-1.5 backdrop-blur-sm">
        <span class="text-xs font-medium text-muted-foreground">{{ stickyGroupLabel }}</span>
      </div>
      <div :style="{ height: `${totalHeight}px`, position: 'relative' }">
        <div v-for="item in visibleItems" :key="item.key" :style="{ position: 'absolute', top: `${item.offset}px`, left: '0', right: '0' }">
          <div v-if="item.type === 'group'" :ref="(el) => measureVirtualItem(item.key, el)" class="px-3 py-1.5">
            <span class="text-xs font-medium text-muted-foreground">{{ item.group.label }}</span>
          </div>
          <div v-else :ref="(el) => measureVirtualItem(item.key, el)">
            <EntryCard
              :entry="item.entry"
              :is-selected="activeEntryId === item.entry.id"
              :show-checkbox="isMultiSelectMode"
              :is-checked="selectedEntryIdSet.has(item.entry.id)"
              @select="handleSelectPayload"
              @toggle-check="store.toggleEntrySelection"
              @toggle-favorite="store.toggleFavorite"
              @delete="store.deleteEntry"
            />
          </div>
        </div>
      </div>
      <div v-if="hasMore" ref="sentinelRef" class="h-6" />
      <div v-if="isLoading && entries.length > 0" class="py-3 text-center text-xs text-muted-foreground">{{ $t('list.loadingMore') }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import EntryCard from "./EntryCard.vue";
import BatchTagDialog from "./BatchTagDialog.vue";
import { useClipboardStore } from "@/stores/clipboardStore";
import type { ClipboardListItem } from "@/types";

const store = useClipboardStore();
const { entries, isLoading, hasMore, activeEntryId, isMultiSelectMode, selectedCount, selectedEntryIdSet, canBatchCopy, groupedEntryItems } = storeToRefs(store);
const showBatchTagDialog = ref(false);
const viewportRef = ref<HTMLElement | null>(null);
const sentinelRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(0);
let observer: IntersectionObserver | null = null;
const resizeObservers = new Map<string, ResizeObserver>();
const GROUP_HEIGHT = 34;
const ENTRY_HEIGHT = 112;
const OVERSCAN = 8;

const layoutItems = computed(() => {
  let offset = 0;
  return groupedEntryItems.value.map((item) => {
    const fallback = item.type === "group" ? GROUP_HEIGHT : ENTRY_HEIGHT;
    const height = store.getVirtualItemHeight(item.key, fallback);
    const laidOut = { ...item, height, offset };
    offset += height;
    return laidOut;
  });
});
const totalHeight = computed(() => {
  const items = layoutItems.value;
  if (!items.length) return 0;
  const last = items[items.length - 1];
  return last.offset + last.height;
});
const visibleItems = computed(() => {
  const start = Math.max(0, scrollTop.value - OVERSCAN * ENTRY_HEIGHT);
  const end = scrollTop.value + viewportHeight.value + OVERSCAN * ENTRY_HEIGHT;
  return layoutItems.value.filter((item) => item.offset + item.height >= start && item.offset <= end) as Array<ClipboardListItem & { offset: number; height: number }>;
});
const stickyGroupLabel = computed(() => {
  const current = [...layoutItems.value].reverse().find((item) => item.type === "group" && item.offset <= scrollTop.value + 1);
  return current?.type === "group" ? current.group.label : groupedEntryItems.value[0]?.type === "group" ? groupedEntryItems.value[0].group.label : "";
});
function updateViewportMetrics() { if (viewportRef.value) viewportHeight.value = viewportRef.value.clientHeight; }
function ensureActiveVisible() {
  if (!viewportRef.value || activeEntryId.value === null) return;
  const target = layoutItems.value.find((item) => item.type === "entry" && item.entry.id === activeEntryId.value);
  if (!target) return;
  const top = target.offset; const bottom = top + target.height; const viewTop = viewportRef.value.scrollTop; const viewBottom = viewTop + viewportRef.value.clientHeight;
  if (top < viewTop) viewportRef.value.scrollTop = top; else if (bottom > viewBottom) viewportRef.value.scrollTop = bottom - viewportRef.value.clientHeight;
}
function handleSelect(id: number, range = false) { store.handleEntryPrimaryAction(id, { range }); }
function handleSelectPayload(payload: { id: number; shiftKey: boolean }) { handleSelect(payload.id, payload.shiftKey); }
function toggleMultiSelect() { if (isMultiSelectMode.value) store.exitMultiSelectMode(); else store.enterMultiSelectMode(activeEntryId.value ?? entries.value[0]?.id); }
function measureVirtualItem(key: string, el: Element | { $el?: Element | null } | null) {
  const target = el instanceof HTMLElement ? el : (el as { $el?: Element | null } | null)?.$el;
  if (!(target instanceof HTMLElement)) return;
  store.setVirtualItemHeight(key, Math.ceil(target.getBoundingClientRect().height));
  if (!resizeObservers.has(key) && typeof ResizeObserver !== "undefined") {
    const ro = new ResizeObserver(() => {
      store.setVirtualItemHeight(key, Math.ceil(target.getBoundingClientRect().height));
    });
    ro.observe(target);
    resizeObservers.set(key, ro);
  }
}
async function handleApplyBatchTags(payload: { tagIds: number[]; mode: "append" | "replace" }) { await store.applyTagsToSelectedEntries(payload.tagIds, payload.mode); showBatchTagDialog.value = false; }
function handleScroll() { if (viewportRef.value) scrollTop.value = viewportRef.value.scrollTop; }
function handleKeydown(e: KeyboardEvent) {
  if (entries.value.length === 0) return;
  const currentIndex = entries.value.findIndex((entry) => entry.id === activeEntryId.value);
  if (e.key === "ArrowDown") {
    e.preventDefault(); const nextIndex = Math.min((currentIndex >= 0 ? currentIndex : -1) + 1, entries.value.length - 1); const nextId = entries.value[nextIndex]?.id ?? null;
    if (nextId !== null && e.shiftKey && isMultiSelectMode.value) store.selectRangeTo(nextId); store.setActiveEntry(nextId); ensureActiveVisible();
  } else if (e.key === "ArrowUp") {
    e.preventDefault(); const nextIndex = Math.max(currentIndex <= 0 ? 0 : currentIndex - 1, 0); const nextId = entries.value[nextIndex]?.id ?? null;
    if (nextId !== null && e.shiftKey && isMultiSelectMode.value) store.selectRangeTo(nextId); store.setActiveEntry(nextId); ensureActiveVisible();
  } else if (e.key === "Enter" && activeEntryId.value !== null) {
    e.preventDefault(); store.handleEntryPrimaryAction(activeEntryId.value, { range: e.shiftKey });
  } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
    e.preventDefault(); store.selectAllLoadedEntries();
  } else if (e.key === "Escape" && isMultiSelectMode.value) {
    e.preventDefault(); store.exitMultiSelectMode();
  }
}
onMounted(() => {
  updateViewportMetrics();
  window.addEventListener("resize", updateViewportMetrics);
  window.addEventListener("keydown", handleKeydown);
  observer = new IntersectionObserver((es) => { if (es[0]?.isIntersecting) store.loadMore(); }, { threshold: 0.1 });
  if (sentinelRef.value) observer.observe(sentinelRef.value);
});
watch(sentinelRef, async (el, prev) => { if (prev) observer?.unobserve(prev); await nextTick(); if (el) observer?.observe(el); });
watch(() => groupedEntryItems.value.length, async () => { await nextTick(); updateViewportMetrics(); });
onUnmounted(() => {
  window.removeEventListener("resize", updateViewportMetrics);
  window.removeEventListener("keydown", handleKeydown);
  observer?.disconnect();
  for (const ro of resizeObservers.values()) ro.disconnect();
  resizeObservers.clear();
});
</script>
