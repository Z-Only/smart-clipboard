<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 px-4"
    @click.self="close"
  >
    <div
      class="w-full max-w-4xl overflow-hidden rounded-2xl border border-border bg-card shadow-2xl"
    >
      <div class="flex items-center justify-between border-b border-border px-5 py-4">
        <div>
          <h2 class="text-lg font-semibold">{{ $t('sync.title') }}</h2>
          <p class="text-sm text-muted-foreground">{{ $t('sync.subtitle') }}</p>
          <div class="mt-2 flex gap-1 rounded-lg bg-muted p-1">
            <button
              class="rounded-md px-3 py-1 text-sm font-medium transition-colors"
              :class="
                activeTab === 'lan'
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="activeTab = 'lan'"
            >
              {{ $t('sync.tab.lan') }}
            </button>
            <button
              class="rounded-md px-3 py-1 text-sm font-medium transition-colors"
              :class="
                activeTab === 'webdav'
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="activeTab = 'webdav'"
            >
              {{ $t('sync.tab.webdav') }}
            </button>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="inline-flex h-9 items-center gap-2 rounded-md border border-border px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="isLoading"
            @click="refresh"
          >
            <svg
              v-if="isLoading"
              class="h-4 w-4 animate-spin"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 12a9 9 0 1 1-6.219-8.56" />
            </svg>
            {{ isLoading ? $t('sync.actions.refreshing') : $t('sync.actions.refresh') }}
          </button>
          <button class="text-muted-foreground hover:text-foreground" @click="close">
            <svg
              class="h-4 w-4"
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

      <!-- WebDAV Tab -->
      <div v-if="activeTab === 'webdav'" class="max-h-[80vh] overflow-y-auto p-5">
        <WebDavPanel :is-active="activeTab === 'webdav' && isOpen" />
      </div>

      <!-- LAN Tab -->
      <div
        v-else
        class="grid max-h-[80vh] gap-0 overflow-y-auto lg:grid-cols-[320px_minmax(0,1fr)]"
      >
        <section class="border-b border-border p-5 lg:border-b-0 lg:border-r">
          <div class="space-y-5">
            <div class="rounded-xl bg-accent/40 p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <p class="text-sm font-medium">{{ $t('sync.panel.enabled') }}</p>
                  <p class="text-xs text-muted-foreground">{{ $t('sync.hints.enabled') }}</p>
                </div>
                <button
                  class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
                  :class="form.enabled ? 'bg-primary' : 'bg-input'"
                  @click="form.enabled = !form.enabled"
                >
                  <span
                    class="pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg transition-transform"
                    :class="form.enabled ? 'translate-x-5' : 'translate-x-0'"
                  />
                </button>
              </div>
            </div>

            <div class="space-y-3">
              <div class="space-y-1.5">
                <label class="text-sm font-medium">{{ $t('sync.panel.deviceName') }}</label>
                <input
                  v-model="form.deviceName"
                  class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  :placeholder="$t('sync.placeholders.deviceName')"
                />
              </div>

              <div class="space-y-1.5">
                <label class="text-sm font-medium">{{ $t('sync.panel.port') }}</label>
                <input
                  v-model.number="form.port"
                  type="number"
                  min="1"
                  max="65535"
                  class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.status') }}</p>
                <div class="mt-1 flex items-center gap-2">
                  <span class="inline-block h-2 w-2 rounded-full" :class="statusDotClass" />
                  <p class="text-sm font-semibold">{{ currentStatusText }}</p>
                </div>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.pairedDevices') }}</p>
                <p class="mt-1 text-sm font-semibold">{{ pairedDevices.length }}</p>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">
                  {{ $t('sync.panel.discoveredDevices') }}
                </p>
                <p class="mt-1 text-sm font-semibold">{{ discoveredDevices.length }}</p>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.activeDevices') }}</p>
                <p class="mt-1 text-sm font-semibold">{{ activePairedCount }}</p>
              </div>
            </div>

            <div
              v-if="error"
              class="flex items-start justify-between gap-3 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
            >
              <div class="flex items-start gap-2">
                <svg
                  class="mt-0.5 h-4 w-4 shrink-0"
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path
                    d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"
                  />
                  <path d="M12 9v4" />
                  <path d="M12 17h.01" />
                </svg>
                <span>{{ error }}</span>
              </div>
              <button
                class="shrink-0 text-destructive/70 hover:text-destructive"
                @click="clearError"
              >
                <svg
                  class="h-4 w-4"
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

            <div class="flex justify-end gap-2">
              <button
                class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
                @click="close"
              >
                {{ $t('sync.actions.close') }}
              </button>
              <button
                class="inline-flex h-9 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="isSaving"
                @click="save"
              >
                {{ isSaving ? $t('sync.actions.saving') : $t('sync.actions.save') }}
              </button>
            </div>
          </div>
        </section>

        <section class="grid gap-5 p-5 lg:grid-cols-2">
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-semibold">{{ $t('sync.panel.pairedDevices') }}</h3>
              <span class="text-xs text-muted-foreground">{{ pairedDevices.length }}</span>
            </div>
            <div class="space-y-3">
              <DeviceCard
                v-for="device in pairedDevices"
                :key="`paired-${device.id}`"
                :device="device"
                mode="paired"
                @toggle-sync="handleToggleSync"
                @unpair="handleUnpair"
              />
              <div
                v-if="!pairedDevices.length && !isLoading"
                class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground"
              >
                {{ $t('sync.empty.paired') }}
              </div>
            </div>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-semibold">{{ $t('sync.panel.discoveredDevices') }}</h3>
              <span class="text-xs text-muted-foreground">{{ discoveredDevices.length }}</span>
            </div>
            <div v-if="isLoading" class="flex flex-col items-center justify-center gap-3 py-12">
              <svg
                class="h-8 w-8 animate-spin text-muted-foreground"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M21 12a9 9 0 1 1-6.219-8.56" />
              </svg>
              <p class="text-sm text-muted-foreground">{{ $t('sync.loading') }}</p>
            </div>
            <div v-else class="space-y-3">
              <DeviceCard
                v-for="device in discoveredDevices"
                :key="`discovered-${device.id}`"
                :device="device"
                mode="discovered"
                @pair="handlePair"
              />
              <div
                v-if="!discoveredDevices.length"
                class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground"
              >
                {{ $t('sync.empty.discovered') }}
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>

    <PairConfirmDialog
      :is-open="!!pairingDevice"
      :device="pairingDevice"
      @confirm="confirmPair"
      @cancel="cancelPair"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useI18n } from 'vue-i18n';
import DeviceCard from '@/components/DeviceCard.vue';
import PairConfirmDialog from '@/components/PairConfirmDialog.vue';
import WebDavPanel from '@/components/WebDavPanel.vue';
import { useSyncStore } from '@/stores/syncStore';
import type { SyncDevice, SyncStatus } from '@/types';

const props = defineProps<{ isOpen: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const syncStore = useSyncStore();
const {
  enabled,
  deviceName,
  port,
  status,
  pairedDevices,
  discoveredDevices,
  isLoading,
  isSaving,
  error,
  activePairedCount,
} = storeToRefs(syncStore);

const form = reactive({
  enabled: false,
  deviceName: '',
  port: 8484,
});

const activeTab = ref<'lan' | 'webdav'>('lan');
const pairingDevice = ref<SyncDevice | null>(null);

function getStatusText(value: SyncStatus) {
  return t(`sync.statusValues.${value}`);
}

const currentStatusText = computed(() => getStatusText(status.value));

const statusDotClass = computed(() => {
  const s = status.value;
  if (s === 'online' || s === 'connected') {
    return 'bg-emerald-500';
  }
  if (s === 'discovering' || s === 'connecting' || s === 'pairing') {
    return 'bg-amber-500 animate-pulse';
  }
  if (s === 'error') {
    return 'bg-destructive';
  }
  return 'bg-muted-foreground';
});

watch(
  () => props.isOpen,
  async (open) => {
    if (!open) return;
    await syncStore.refreshAll();
    syncFormFromStore();
  },
);

watch([enabled, deviceName, port], () => {
  if (!props.isOpen) return;
  syncFormFromStore();
});

function syncFormFromStore() {
  form.enabled = enabled.value;
  form.deviceName = deviceName.value;
  form.port = port.value;
}

async function refresh() {
  await syncStore.refreshAll();
}

async function save() {
  await syncStore.saveConfig({
    enabled: form.enabled,
    deviceName: form.deviceName.trim(),
    port: Number(form.port),
  });
  syncFormFromStore();
}

function handlePair(device: SyncDevice) {
  pairingDevice.value = device;
}

async function confirmPair() {
  if (!pairingDevice.value) return;
  await syncStore.pairDevice(pairingDevice.value.id);
  pairingDevice.value = null;
}

function cancelPair() {
  pairingDevice.value = null;
}

async function handleUnpair(device: SyncDevice) {
  await syncStore.unpairDevice(device.id);
}

async function handleToggleSync(device: SyncDevice, nextEnabled: boolean) {
  await syncStore.toggleDeviceSync(device.id, nextEnabled);
}

function clearError() {
  syncStore.clearError();
}

function close() {
  emit('close');
}
</script>
