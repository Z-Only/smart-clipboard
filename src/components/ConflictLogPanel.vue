<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-sm font-semibold">{{ $t('conflict.log.title') }}</h3>
        <p class="text-xs text-muted-foreground">
          {{ logCount }} {{ logCount === 1 ? 'entry' : 'entries' }}
        </p>
      </div>
      <button
        v-if="logCount > 0"
        class="inline-flex h-8 items-center rounded-md border border-border px-2.5 text-xs font-medium text-destructive transition-colors hover:bg-destructive/10"
        @click="handleClearLog"
      >
        {{ $t('conflict.log.clearAll') }}
      </button>
    </div>

    <!-- Empty state -->
    <div
      v-if="logCount === 0"
      class="rounded-xl border border-dashed border-border p-8 text-center"
    >
      <p class="text-sm text-muted-foreground">{{ $t('conflict.log.empty') }}</p>
      <p class="mt-1 text-xs text-muted-foreground">{{ $t('conflict.log.emptyHint') }}</p>
    </div>

    <!-- Log entries -->
    <div v-else class="max-h-96 space-y-2 overflow-y-auto">
      <div
        v-for="entry in sortedLog"
        :key="entry.id"
        class="group rounded-lg border border-border bg-accent/20 p-3 transition-colors hover:bg-accent/40"
      >
        <div class="flex items-start justify-between gap-2">
          <div class="min-w-0 flex-1 space-y-2">
            <!-- Content previews -->
            <div class="grid grid-cols-2 gap-2">
              <div class="min-w-0">
                <p class="text-xs font-medium text-muted-foreground">
                  {{ $t('conflict.log.entry.localContent') }}
                </p>
                <p class="truncate text-sm">{{ entry.localContentPreview }}</p>
              </div>
              <div class="min-w-0">
                <p class="text-xs font-medium text-muted-foreground">
                  {{ $t('conflict.log.entry.remoteContent') }}
                </p>
                <p class="truncate text-sm">{{ entry.remoteContentPreview }}</p>
              </div>
            </div>

            <!-- Metadata -->
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <span>
                <span class="font-medium">{{ $t('conflict.log.entry.device') }}:</span>
                {{ entry.remoteDeviceName }}
              </span>
              <span>
                <span class="font-medium">{{ $t('conflict.log.entry.strategy') }}:</span>
                {{ $t(`conflict.strategy.${entry.strategy}`) }}
              </span>
              <span
                class="inline-flex items-center rounded-full px-1.5 py-0.5 text-xs font-medium"
                :class="outcomeClass(entry.outcome)"
              >
                {{ $t(`conflict.log.outcome.${entry.outcome}`) }}
              </span>
              <span class="ml-auto">{{ formatTime(entry.resolvedAt) }}</span>
            </div>
          </div>

          <!-- Delete single entry -->
          <button
            class="shrink-0 rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover:opacity-100"
            @click="handleRemoveEntry(entry.id)"
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
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useI18n } from 'vue-i18n';
import { useConflictStore } from '@/stores/conflictStore';
import type { ConflictOutcome } from '@/types';

const { t } = useI18n();
const conflictStore = useConflictStore();
const { sortedLog, logCount } = storeToRefs(conflictStore);

function outcomeClass(outcome: ConflictOutcome): string {
  switch (outcome) {
    case 'kept-local':
      return 'bg-blue-500/10 text-blue-600 dark:text-blue-400';
    case 'kept-remote':
      return 'bg-amber-500/10 text-amber-600 dark:text-amber-400';
    case 'auto-resolved':
      return 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400';
    case 'dismissed':
      return 'bg-muted text-muted-foreground';
    default:
      return 'bg-muted text-muted-foreground';
  }
}

function formatTime(isoString: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(isoString));
  } catch {
    return isoString;
  }
}

function handleClearLog() {
  if (!confirm(t('conflict.log.clearConfirm'))) return;
  conflictStore.clearLog();
}

function handleRemoveEntry(logEntryId: string) {
  conflictStore.removeLogEntry(logEntryId);
}
</script>
