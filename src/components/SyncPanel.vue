<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 px-4"
    @click.self="close"
  >
    <div class="w-full max-w-4xl overflow-hidden rounded-2xl border border-border bg-card shadow-2xl">
      <div class="flex items-center justify-between border-b border-border px-5 py-4">
        <div>
          <h2 class="text-lg font-semibold">{{ $t('sync.title') }}</h2>
          <p class="text-sm text-muted-foreground">{{ $t('sync.subtitle') }}</p>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            @click="refresh"
          >
            {{ $t('sync.actions.refresh') }}
          </button>
          <button class="text-muted-foreground hover:text-foreground" @click="close">
            <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 6 6 18" /><path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      </div>

      <div class="grid max-h-[80vh] gap-0 overflow-y-auto lg:grid-cols-[320px_minmax(0,1fr)]">
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
                <p class="mt-1 text-sm font-semibold">{{ currentStatusText }}</p>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.pairedDevices') }}</p>
                <p class="mt-1 text-sm font-semibold">{{ pairedDevices.length }}</p>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.discoveredDevices') }}</p>
                <p class="mt-1 text-sm font-semibold">{{ discoveredDevices.length }}</p>
              </div>
              <div class="rounded-xl border border-border bg-background p-3">
                <p class="text-xs text-muted-foreground">{{ $t('sync.panel.onlineDevices') }}</p>
                <p class="mt-1 text-sm font-semibold">{{ onlinePairedCount }}</p>
              </div>
            </div>

            <div v-if="error" class="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
              {{ error }}
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
              <div v-if="!pairedDevices.length" class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
                {{ $t('sync.empty.paired') }}
              </div>
            </div>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-semibold">{{ $t('sync.panel.discoveredDevices') }}</h3>
              <span class="text-xs text-muted-foreground">{{ discoveredDevices.length }}</span>
            </div>
            <div class="space-y-3">
              <DeviceCard
                v-for="device in discoveredDevices"
                :key="`discovered-${device.id}`"
                :device="device"
                mode="discovered"
                @pair="handlePair"
              />
              <div v-if="!discoveredDevices.length" class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
                {{ $t('sync.empty.discovered') }}
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import DeviceCard from "@/components/DeviceCard.vue";
import { useSyncStore } from "@/stores/syncStore";
import type { SyncDevice } from "@/types";

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
  isSaving,
  error,
  onlinePairedCount,
} = storeToRefs(syncStore);

const form = reactive({
  enabled: false,
  deviceName: "",
  port: 8484,
});

const currentStatusText = computed(() => t(`sync.statusValues.${status.value}`));

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

async function handlePair(device: SyncDevice) {
  await syncStore.pairDevice(device.id);
}

async function handleUnpair(device: SyncDevice) {
  await syncStore.unpairDevice(device.id);
}

async function handleToggleSync(device: SyncDevice, nextEnabled: boolean) {
  await syncStore.toggleDeviceSync(device.id, nextEnabled);
}

function close() {
  emit("close");
}
</script>
