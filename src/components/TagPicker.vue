<template>
  <div class="relative" ref="pickerRef">
    <button
      class="p-1 rounded hover:bg-background/80 text-muted-foreground"
      @click.stop="togglePicker"
      :title="t('tags.addTag')"
    >
      <svg
        class="h-3.5 w-3.5"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M12 5v14M5 12h14" />
      </svg>
    </button>
    <div
      v-if="isOpen"
      class="absolute right-0 top-full mt-1 z-50 min-w-[200px] max-h-[240px] rounded-md border bg-popover p-2 text-popover-foreground shadow-md flex flex-col gap-1"
    >
      <div class="flex items-center gap-1 mb-1">
        <input
          ref="inputRef"
          v-model="newTagName"
          class="flex-1 h-7 rounded-sm border bg-transparent px-2 text-xs outline-none focus:ring-1 focus:ring-ring"
          :placeholder="t('tags.createTag')"
          @keydown.enter.prevent="handleCreateTag"
          @click.stop
        />
        <button
          v-if="newTagName.trim()"
          class="h-7 px-2 rounded-sm bg-primary text-primary-foreground text-xs hover:bg-primary/90"
          @click.stop="handleCreateTag"
        >
          +
        </button>
      </div>
      <div class="overflow-y-auto flex flex-col gap-0.5" v-if="allTags.length > 0">
        <label
          v-for="tag in allTags"
          :key="tag.id"
          class="flex items-center gap-2 px-2 py-1 rounded-sm text-xs hover:bg-accent cursor-pointer"
          @click.stop
        >
          <input
            type="checkbox"
            :checked="isTagAssociated(tag.id)"
            @change="toggleTag(tag.id)"
            class="h-3 w-3 rounded border-muted-foreground accent-primary"
          />
          <span class="truncate flex-1">{{ tag.name }}</span>
        </label>
      </div>
      <div v-else class="text-xs text-muted-foreground text-center py-2">
        {{ t('tags.noTags') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import type { Tag } from '@/types';

const { t } = useI18n();

const props = defineProps<{
  entryId: number;
}>();

const emit = defineEmits<{
  tagsChanged: [tags: Tag[]];
}>();

const isOpen = ref(false);
const pickerRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const newTagName = ref('');
const allTags = ref<Tag[]>([]);
const entryTagIds = ref<Set<number>>(new Set());

function isTagAssociated(tagId: number): boolean {
  return entryTagIds.value.has(tagId);
}

async function loadTags() {
  try {
    allTags.value = await invoke<Tag[]>('get_all_tags');
  } catch (e) {
    console.error('Failed to load tags:', e);
  }
}

async function loadEntryTags() {
  try {
    const tags = await invoke<Tag[]>('get_entry_tags', { entryId: props.entryId });
    entryTagIds.value = new Set(tags.map((t) => t.id));
    emit('tagsChanged', tags);
  } catch (e) {
    console.error('Failed to load entry tags:', e);
  }
}

async function toggleTag(tagId: number) {
  try {
    if (entryTagIds.value.has(tagId)) {
      await invoke('remove_tag_from_entry', { entryId: props.entryId, tagId });
      entryTagIds.value.delete(tagId);
    } else {
      await invoke('add_tag_to_entry', { entryId: props.entryId, tagId });
      entryTagIds.value.add(tagId);
    }
    // Force reactivity
    entryTagIds.value = new Set(entryTagIds.value);
    // Emit updated tags
    const currentTags = allTags.value.filter((t) => entryTagIds.value.has(t.id));
    emit('tagsChanged', currentTags);
  } catch (e) {
    console.error('Failed to toggle tag:', e);
  }
}

async function handleCreateTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  try {
    const tag = await invoke<Tag>('create_tag', { name });
    allTags.value.push(tag);
    // Auto-associate with current entry
    await invoke('add_tag_to_entry', { entryId: props.entryId, tagId: tag.id });
    entryTagIds.value.add(tag.id);
    entryTagIds.value = new Set(entryTagIds.value);
    newTagName.value = '';
    const currentTags = allTags.value.filter((t) => entryTagIds.value.has(t.id));
    emit('tagsChanged', currentTags);
  } catch (e) {
    console.error('Failed to create tag:', e);
  }
}

function togglePicker() {
  isOpen.value = !isOpen.value;
  if (isOpen.value) {
    loadTags();
    loadEntryTags();
  }
}

function handleClickOutside(event: MouseEvent) {
  if (pickerRef.value && !pickerRef.value.contains(event.target as Node)) {
    isOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
});
</script>
