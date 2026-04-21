<template>
  <div
    v-if="isOpen && device"
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 px-4"
    @click.self="handleCancel"
  >
    <div
      class="w-full max-w-md overflow-hidden rounded-2xl border border-border bg-card shadow-2xl"
    >
      <div class="px-5 py-4">
        <h3 class="text-lg font-semibold">{{ $t('sync.pairDialog.title') }}</h3>
        <p class="mt-1 text-sm text-muted-foreground">{{ $t('sync.pairDialog.message') }}</p>
      </div>

      <div class="px-5 pb-4">
        <div class="rounded-xl border border-border bg-accent/30 p-4 space-y-3">
          <div>
            <p class="text-xs text-muted-foreground">{{ $t('sync.panel.deviceName') }}</p>
            <p class="text-sm font-medium">{{ device.deviceName }}</p>
          </div>
          <div v-if="device.address || device.ip">
            <p class="text-xs text-muted-foreground">Address</p>
            <p class="text-sm font-medium">{{ device.address || device.ip }}</p>
          </div>
          <div v-if="device.fingerprint">
            <p class="text-xs text-muted-foreground">{{ $t('sync.pairDialog.fingerprint') }}</p>
            <p class="text-xs font-mono text-muted-foreground break-all">
              {{ device.fingerprint }}
            </p>
          </div>
        </div>

        <div
          class="mt-3 flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2"
        >
          <svg
            class="mt-0.5 h-4 w-4 shrink-0 text-amber-500"
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
            <path d="M12 9v4" />
            <path d="M12 17h.01" />
          </svg>
          <p class="text-xs text-amber-600 dark:text-amber-400">
            {{ $t('sync.pairDialog.warning') }}
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2 border-t border-border px-5 py-4">
        <button
          class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
          @click="handleCancel"
        >
          {{ $t('sync.pairDialog.cancel') }}
        </button>
        <button
          class="inline-flex h-9 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
          @click="handleConfirm"
        >
          {{ $t('sync.pairDialog.confirm') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SyncDevice } from '@/types';

defineProps<{
  isOpen: boolean;
  device: SyncDevice | null;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

function handleConfirm() {
  emit('confirm');
}

function handleCancel() {
  emit('cancel');
}
</script>
