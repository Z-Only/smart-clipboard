<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 px-4"
    @click.self="$emit('close')"
  >
    <div class="w-full max-w-md rounded-2xl border border-border bg-card shadow-2xl">
      <div class="border-b border-border px-5 py-4">
        <h3 class="text-base font-semibold">{{ $t('tags.batchTitle') }}</h3>
        <p class="mt-1 text-xs text-muted-foreground">{{ $t('tags.batchHint', { count }) }}</p>
      </div>

      <div class="space-y-3 px-5 py-4">
        <div class="flex items-center gap-2">
          <input
            v-model="newTagName"
            class="flex-1 h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none focus:ring-1 focus:ring-ring"
            :placeholder="$t('tags.createTag')"
            @keydown.enter.prevent="handleCreate"
          />
          <button
            class="rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:opacity-90"
            @click="handleCreate"
          >
            {{ $t('tags.createAction') }}
          </button>
        </div>

        <div class="max-h-64 space-y-1 overflow-y-auto rounded-md border border-border p-2">
          <label
            v-for="tag in allTags"
            :key="tag.id"
            class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent"
          >
            <input
              type="checkbox"
              class="h-4 w-4 accent-primary"
              :checked="selectedTagIds.has(tag.id)"
              @change="toggleTag(tag.id)"
            />
            <span class="truncate">{{ tag.name }}</span>
          </label>
          <div v-if="allTags.length === 0" class="py-6 text-center text-xs text-muted-foreground">
            {{ $t('tags.noTags') }}
          </div>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2 border-t border-border px-5 py-4">
        <button
          class="rounded-md border border-border px-3 py-2 text-xs hover:bg-accent"
          @click="$emit('close')"
        >
          {{ $t('templates.cancel') }}
        </button>
        <button
          class="rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:opacity-90"
          @click="submit"
        >
          {{ $t('tags.applyBatch') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Tag } from "@/types";

const props = defineProps<{
  isOpen: boolean;
  count: number;
}>();

const emit = defineEmits<{
  close: [];
  apply: [tagIds: number[]];
}>();

const allTags = ref<Tag[]>([]);
const selectedTagIds = ref<Set<number>>(new Set());
const newTagName = ref("");

async function loadTags() {
  allTags.value = await invoke<Tag[]>("get_all_tags");
}

function toggleTag(tagId: number) {
  const next = new Set(selectedTagIds.value);
  if (next.has(tagId)) next.delete(tagId);
  else next.add(tagId);
  selectedTagIds.value = next;
}

async function handleCreate() {
  const name = newTagName.value.trim();
  if (!name) return;
  const tag = await invoke<Tag>("create_tag", { name });
  allTags.value = [...allTags.value, tag].sort((a, b) => a.name.localeCompare(b.name));
  const next = new Set(selectedTagIds.value);
  next.add(tag.id);
  selectedTagIds.value = next;
  newTagName.value = "";
}

function submit() {
  emit("apply", Array.from(selectedTagIds.value));
}

watch(
  () => props.isOpen,
  async (open) => {
    if (!open) return;
    selectedTagIds.value = new Set();
    newTagName.value = "";
    await loadTags();
  },
  { immediate: true }
);
</script>
