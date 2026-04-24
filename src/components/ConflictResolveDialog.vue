<template>
  <div
    v-if="conflict"
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 px-4"
    @click.self="handleDismiss"
  >
    <div
      class="w-full max-w-2xl overflow-hidden rounded-2xl border border-border bg-card shadow-2xl"
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border px-5 py-4">
        <div>
          <h3 class="text-lg font-semibold">{{ $t('conflict.resolve.title') }}</h3>
          <p class="mt-1 text-sm text-muted-foreground">
            {{ $t('conflict.resolve.description') }}
          </p>
        </div>
        <div v-if="pendingCount > 1" class="text-xs text-muted-foreground">
          {{ $t('conflict.resolve.remaining', { count: pendingCount }) }}
        </div>
      </div>

      <!-- Comparison -->
      <div class="grid grid-cols-2 gap-0 border-b border-border">
        <!-- Local Version -->
        <div class="border-r border-border p-4">
          <div class="mb-3 flex items-center gap-2">
            <span
              class="inline-flex h-5 items-center rounded-full bg-blue-500/10 px-2 text-xs font-medium text-blue-600 dark:text-blue-400"
            >
              {{ $t('conflict.resolve.localVersion') }}
            </span>
          </div>
          <div class="space-y-2">
            <div
              class="max-h-40 overflow-y-auto rounded-lg border border-border bg-accent/30 p-3 text-sm leading-relaxed break-all"
            >
              {{ conflict.localVersion.content }}
            </div>
            <div class="space-y-1 text-xs text-muted-foreground">
              <p>
                {{
                  $t('conflict.resolve.modifiedAt', {
                    time: formatTime(conflict.localVersion.updated_at),
                  })
                }}
              </p>
              <p>
                {{
                  $t('conflict.resolve.contentType', {
                    type: conflict.localVersion.content_type,
                  })
                }}
              </p>
            </div>
          </div>
        </div>

        <!-- Remote Version -->
        <div class="p-4">
          <div class="mb-3 flex items-center gap-2">
            <span
              class="inline-flex h-5 items-center rounded-full bg-amber-500/10 px-2 text-xs font-medium text-amber-600 dark:text-amber-400"
            >
              {{ $t('conflict.resolve.remoteVersion') }}
            </span>
            <span class="text-xs text-muted-foreground">
              {{ $t('conflict.resolve.fromDevice', { device: conflict.remoteDeviceName }) }}
            </span>
          </div>
          <div class="space-y-2">
            <div
              class="max-h-40 overflow-y-auto rounded-lg border border-border bg-accent/30 p-3 text-sm leading-relaxed break-all"
            >
              {{ conflict.remoteVersion.content }}
            </div>
            <div class="space-y-1 text-xs text-muted-foreground">
              <p>
                {{
                  $t('conflict.resolve.modifiedAt', {
                    time: formatTime(conflict.remoteVersion.updated_at),
                  })
                }}
              </p>
              <p>
                {{
                  $t('conflict.resolve.contentType', {
                    type: conflict.remoteVersion.content_type,
                  })
                }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center justify-between px-5 py-4">
        <button
          class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          @click="handleDismiss"
        >
          {{ $t('conflict.resolve.dismiss') }}
        </button>
        <div class="flex items-center gap-2">
          <button
            v-if="pendingCount > 1"
            class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            @click="handleNext"
          >
            {{ $t('conflict.resolve.nextConflict') }}
          </button>
          <button
            class="inline-flex h-9 items-center rounded-md bg-blue-600 px-4 text-sm font-medium text-white transition-opacity hover:opacity-90"
            @click="handleKeepLocal"
          >
            {{ $t('conflict.resolve.keepLocal') }}
          </button>
          <button
            class="inline-flex h-9 items-center rounded-md bg-amber-600 px-4 text-sm font-medium text-white transition-opacity hover:opacity-90"
            @click="handleKeepRemote"
          >
            {{ $t('conflict.resolve.keepRemote') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useConflictStore } from '@/stores/conflictStore';
import type { SyncConflict } from '@/types';

defineProps<{
  conflict: SyncConflict | null;
}>();

const emit = defineEmits<{
  resolved: [conflictId: string, outcome: 'kept-local' | 'kept-remote'];
}>();

const conflictStore = useConflictStore();
const { pendingCount } = storeToRefs(conflictStore);

function formatTime(isoString: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(isoString));
  } catch {
    return isoString;
  }
}

function handleKeepLocal() {
  const conflict = conflictStore.activeConflict;
  if (!conflict) return;
  conflictStore.resolveManually(conflict.id, 'kept-local');
  emit('resolved', conflict.id, 'kept-local');
  conflictStore.openNextConflict();
}

function handleKeepRemote() {
  const conflict = conflictStore.activeConflict;
  if (!conflict) return;
  conflictStore.resolveManually(conflict.id, 'kept-remote');
  emit('resolved', conflict.id, 'kept-remote');
  conflictStore.openNextConflict();
}

function handleDismiss() {
  const conflict = conflictStore.activeConflict;
  if (!conflict) return;
  conflictStore.dismissConflict(conflict.id);
  conflictStore.openNextConflict();
}

function handleNext() {
  conflictStore.openNextConflict();
}
</script>
