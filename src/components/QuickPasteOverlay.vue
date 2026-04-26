<template>
  <Teleport to="body">
    <div
      v-if="isActive"
      class="fixed inset-0 z-50 flex items-start justify-center pt-16"
      @click.self="$emit('dismiss')"
      @keydown="handleKeydown"
    >
      <div
        ref="panelRef"
        class="w-[380px] max-h-[480px] rounded-xl border border-border bg-background shadow-2xl flex flex-col overflow-hidden"
        tabindex="-1"
      >
        <!-- Header -->
        <div
          class="flex items-center justify-between px-4 py-2.5 border-b border-border bg-muted/30"
        >
          <span class="text-sm font-medium">{{ $t('quickPaste.title') }}</span>
          <span class="text-xs text-muted-foreground">{{ $t('quickPaste.dismiss') }}</span>
        </div>

        <!-- Entry list -->
        <div
          v-if="entries.length === 0"
          class="flex items-center justify-center py-10 text-sm text-muted-foreground"
        >
          {{ $t('quickPaste.empty') }}
        </div>
        <div v-else class="flex-1 overflow-y-auto">
          <button
            v-for="(entry, index) in entries"
            :key="entry.id"
            class="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-accent/50"
            :class="{ 'bg-accent': index === activeIndex }"
            @click="$emit('paste', entry.id)"
            @mouseenter="activeIndex = index"
          >
            <span
              class="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-primary/10 text-xs font-semibold text-primary"
            >
              {{ index + 1 }}
            </span>
            <span class="text-sm shrink-0">{{ categoryIcon(entry.category) }}</span>
            <span class="flex-1 truncate text-sm">{{ displayContent(entry) }}</span>
            <span class="shrink-0 text-[10px] text-muted-foreground">{{
              relativeTime(entry.created_at)
            }}</span>
          </button>
        </div>

        <!-- Footer -->
        <div class="border-t border-border px-4 py-2 text-xs text-muted-foreground">
          {{ $t('quickPaste.typeToSearch') }}
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ClipboardEntry } from '@/types';
import { CATEGORIES } from '@/types';

const props = defineProps<{
  entries: ClipboardEntry[];
  isActive: boolean;
}>();

const emit = defineEmits<{
  paste: [entryId: number];
  dismiss: [];
  search: [text: string];
}>();

const { t } = useI18n();

const activeIndex = ref(0);
const panelRef = ref<HTMLDivElement | null>(null);

watch(
  () => props.isActive,
  async (active) => {
    if (active) {
      activeIndex.value = 0;
      await nextTick();
      panelRef.value?.focus();
    }
  },
);

function categoryIcon(category: string): string {
  const found = CATEGORIES.find((c) => c.key === category);
  return found?.icon ?? '📋';
}

function displayContent(entry: ClipboardEntry): string {
  if (entry.content_type === 'image') return t('quickPaste.imageEntry');
  return entry.content.replace(/\n/g, ' ').slice(0, 80);
}

function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSeconds = Math.floor((now - then) / 1000);
  if (diffSeconds < 60) return t('entry.justNow');
  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return t('entry.minutesAgo', { n: diffMinutes });
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return t('entry.hoursAgo', { n: diffHours });
  const diffDays = Math.floor(diffHours / 24);
  return t('entry.daysAgo', { n: diffDays });
}

function handleKeydown(event: KeyboardEvent) {
  const key = event.key;

  // Number keys 1-9 → paste corresponding entry
  if (key >= '1' && key <= '9') {
    const index = parseInt(key) - 1;
    if (index < props.entries.length) {
      event.preventDefault();
      emit('paste', props.entries[index].id);
    }
    return;
  }

  switch (key) {
    case 'Escape':
      event.preventDefault();
      emit('dismiss');
      break;
    case 'Enter':
      event.preventDefault();
      if (props.entries.length > 0) {
        emit('paste', props.entries[activeIndex.value].id);
      }
      break;
    case 'ArrowDown':
      event.preventDefault();
      activeIndex.value = Math.min(activeIndex.value + 1, props.entries.length - 1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      activeIndex.value = Math.max(activeIndex.value - 1, 0);
      break;
    default:
      // Any printable character → transition to search
      if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault();
        emit('search', key);
      }
      break;
  }
}
</script>
