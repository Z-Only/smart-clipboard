<template>
  <div
    class="rounded-xl border border-border bg-background/80 p-3 shadow-sm transition-colors hover:bg-accent/30"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0 space-y-1">
        <div class="flex items-center gap-2">
          <h4 class="truncate text-sm font-semibold text-foreground">
            {{ displayName }}
          </h4>
          <span
            class="inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium"
            :class="statusClass"
          >
            {{ statusText }}
          </span>
        </div>
        <p class="truncate text-xs text-muted-foreground">
          {{ device.address || device.ip || `ID: ${device.id}` }}
        </p>
        <p class="text-[11px] text-muted-foreground">
          {{ $t('sync.panel.port') }}: {{ device.port ?? '—' }}
        </p>
      </div>

      <div v-if="mode === 'paired'" class="flex items-center gap-2">
        <button
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
          :class="device.syncEnabled ? 'bg-primary' : 'bg-input'"
          @click="$emit('toggle-sync', device, !device.syncEnabled)"
        >
          <span
            class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
            :class="device.syncEnabled ? 'translate-x-4' : 'translate-x-0'"
          />
        </button>
      </div>
    </div>

    <div class="mt-3 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
      <span>{{ subtitle }}</span>
      <button
        v-if="mode === 'discovered'"
        class="inline-flex items-center rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground transition-opacity hover:opacity-90"
        @click="$emit('pair', device)"
      >
        {{ $t('sync.actions.pair') }}
      </button>
      <button
        v-else
        class="inline-flex items-center rounded-md border border-border px-2.5 py-1 text-xs font-medium text-foreground transition-colors hover:bg-accent"
        @click="$emit('unpair', device)"
      >
        {{ $t('sync.actions.unpair') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SyncDevice } from '@/types';

const props = defineProps<{
  device: SyncDevice;
  mode: 'paired' | 'discovered';
}>();

defineEmits<{
  pair: [device: SyncDevice];
  unpair: [device: SyncDevice];
  'toggle-sync': [device: SyncDevice, enabled: boolean];
}>();

const { t } = useI18n();

const displayName = computed(() => props.device.deviceName || props.device.name || props.device.id);

const statusText = computed(() => t(`sync.statusValues.${props.device.status ?? 'unknown'}`));

const statusClass = computed(() => {
  switch (props.device.status) {
    case 'online':
    case 'connected':
      return 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400';
    case 'connecting':
    case 'pairing':
      return 'bg-amber-500/10 text-amber-600 dark:text-amber-400';
    case 'offline':
    case 'disabled':
      return 'bg-muted text-muted-foreground';
    case 'error':
      return 'bg-destructive/10 text-destructive';
    default:
      return 'bg-secondary text-secondary-foreground';
  }
});

const subtitle = computed(() => {
  if (props.mode === 'paired') {
    if (!props.device.syncEnabled || props.device.status === 'disabled') {
      return t('sync.device.syncOff');
    }
    return t('sync.device.syncOn');
  }
  return t('sync.device.availableToPair');
});
</script>
