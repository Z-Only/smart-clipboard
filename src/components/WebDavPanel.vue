<template>
  <div class="space-y-5">
    <!-- Connection Form -->
    <div v-if="status === 'disconnected' || status === 'error'" class="space-y-4">
      <div class="rounded-xl bg-accent/40 p-4">
        <p class="text-sm font-medium">{{ $t('webdav.title') }}</p>
        <p class="mt-1 text-xs text-muted-foreground">{{ $t('webdav.description') }}</p>
      </div>

      <div class="space-y-3">
        <div class="space-y-1.5">
          <label class="text-sm font-medium">{{ $t('webdav.serverUrl') }}</label>
          <input
            v-model="form.serverUrl"
            class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            :placeholder="$t('webdav.serverUrlPlaceholder')"
          />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <label class="text-sm font-medium">{{ $t('webdav.username') }}</label>
            <input
              v-model="form.username"
              class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </div>
          <div class="space-y-1.5">
            <label class="text-sm font-medium">{{ $t('webdav.password') }}</label>
            <input
              v-model="form.password"
              type="password"
              class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </div>
        </div>
        <div class="space-y-1.5">
          <label class="text-sm font-medium">{{ $t('webdav.syncPassword') }}</label>
          <input
            v-model="form.syncPassword"
            type="password"
            class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
          <p class="text-xs text-muted-foreground">{{ $t('webdav.syncPasswordHint') }}</p>
        </div>
      </div>

      <button
        class="inline-flex h-9 w-full items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="isConnecting || !canConnect"
        @click="handleConnect"
      >
        {{ isConnecting ? $t('webdav.connecting') : $t('webdav.connect') }}
      </button>
    </div>

    <!-- Connected Status -->
    <div v-else class="space-y-4">
      <div class="grid grid-cols-2 gap-3">
        <div class="rounded-xl border border-border bg-background p-3">
          <p class="text-xs text-muted-foreground">{{ $t('webdav.status') }}</p>
          <div class="mt-1 flex items-center gap-2">
            <span class="inline-block h-2 w-2 rounded-full" :class="statusDotClass" />
            <p class="text-sm font-semibold">{{ statusText }}</p>
          </div>
        </div>
        <div class="rounded-xl border border-border bg-background p-3">
          <p class="text-xs text-muted-foreground">{{ $t('webdav.cloudEntries') }}</p>
          <p class="mt-1 text-sm font-semibold">{{ cloudEntryCount }}</p>
        </div>
        <div class="rounded-xl border border-border bg-background p-3">
          <p class="text-xs text-muted-foreground">{{ $t('webdav.lastSync') }}</p>
          <p class="mt-1 text-sm font-semibold">{{ lastSyncDisplay }}</p>
        </div>
        <div class="rounded-xl border border-border bg-background p-3">
          <p class="text-xs text-muted-foreground">{{ $t('webdav.rateLimit') }}</p>
          <p class="mt-1 text-sm font-semibold">
            {{ rateLimitAvailable }} / {{ rateLimitCapacity }}
          </p>
        </div>
      </div>

      <div class="flex gap-2">
        <button
          class="inline-flex h-9 flex-1 items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="isSyncing"
          @click="handleTriggerSync"
        >
          {{ $t('webdav.triggerSync') }}
        </button>
        <button
          class="inline-flex h-9 items-center rounded-md border border-destructive/30 px-3 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
          @click="handleDisconnect"
        >
          {{ $t('webdav.disconnect') }}
        </button>
      </div>

      <!-- Registered Devices -->
      <div class="space-y-2">
        <h3 class="text-sm font-semibold">{{ $t('webdav.registeredDevices') }}</h3>
        <div
          v-if="registeredDevices.length === 0"
          class="rounded-xl border border-dashed border-border p-4 text-center text-sm text-muted-foreground"
        >
          {{ $t('webdav.noDevices') }}
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="device in registeredDevices"
            :key="device.deviceId"
            class="flex items-center justify-between rounded-lg border border-border bg-background p-3"
          >
            <div>
              <p class="text-sm font-medium">{{ device.deviceName }}</p>
              <p class="text-xs text-muted-foreground">
                {{
                  device.lastSyncAt
                    ? formatTime(device.lastSyncAt)
                    : device.registeredAt
                      ? formatTime(device.registeredAt)
                      : ''
                }}
              </p>
            </div>
            <button
              class="text-xs text-destructive hover:underline"
              @click="handleRemoveDevice(device.deviceId, device.deviceName)"
            >
              {{ $t('webdav.removeDevice') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Settings -->
      <details class="rounded-xl border border-border">
        <summary class="cursor-pointer px-4 py-3 text-sm font-medium">
          {{ $t('webdav.settings') }}
        </summary>
        <div class="space-y-3 border-t border-border px-4 py-3">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-medium">{{ $t('webdav.syncImages') }}</p>
            </div>
            <button
              class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="settingsForm.syncImages ? 'bg-primary' : 'bg-input'"
              @click="settingsForm.syncImages = !settingsForm.syncImages"
            >
              <span
                class="pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg transition-transform"
                :class="settingsForm.syncImages ? 'translate-x-5' : 'translate-x-0'"
              />
            </button>
          </div>
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-medium">{{ $t('webdav.syncSensitive') }}</p>
            </div>
            <button
              class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="settingsForm.syncSensitive ? 'bg-primary' : 'bg-input'"
              @click="settingsForm.syncSensitive = !settingsForm.syncSensitive"
            >
              <span
                class="pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg transition-transform"
                :class="settingsForm.syncSensitive ? 'translate-x-5' : 'translate-x-0'"
              />
            </button>
          </div>
          <div class="space-y-1.5">
            <label class="text-sm font-medium">{{ $t('webdav.pollInterval') }}</label>
            <div class="flex items-center gap-2">
              <input
                v-model.number="settingsForm.pollIntervalSecs"
                type="number"
                min="10"
                max="3600"
                class="flex h-9 w-24 rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
              <span class="text-sm text-muted-foreground">{{ $t('webdav.pollIntervalUnit') }}</span>
            </div>
          </div>
          <div class="space-y-1.5">
            <label class="text-sm font-medium">{{ $t('webdav.maxCloudEntries') }}</label>
            <input
              v-model.number="settingsForm.maxCloudEntries"
              type="number"
              min="100"
              max="50000"
              class="flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </div>

          <details class="rounded-lg border border-border">
            <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-muted-foreground">
              {{ $t('webdav.advanced') }}
            </summary>
            <div class="space-y-3 border-t border-border px-3 py-2">
              <div class="space-y-1.5">
                <label class="text-xs font-medium">{{ $t('webdav.remotePath') }}</label>
                <input
                  v-model="settingsForm.remotePath"
                  class="flex h-8 w-full rounded-md border border-input bg-background px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                />
              </div>
              <div class="space-y-1.5">
                <label class="text-xs font-medium">{{ $t('webdav.rateLimitCapacity') }}</label>
                <input
                  v-model.number="settingsForm.rateLimitCapacity"
                  type="number"
                  min="10"
                  max="10000"
                  class="flex h-8 w-full rounded-md border border-input bg-background px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                />
              </div>
              <div class="space-y-1.5">
                <label class="text-xs font-medium">{{ $t('webdav.rateLimitRefillMinutes') }}</label>
                <input
                  v-model.number="settingsForm.rateLimitRefillMinutes"
                  type="number"
                  min="1"
                  max="1440"
                  class="flex h-8 w-full rounded-md border border-input bg-background px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                />
              </div>
            </div>
          </details>

          <button
            class="inline-flex h-9 w-full items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
            @click="handleSaveSettings"
          >
            {{ $t('settings.save') }}
          </button>
        </div>
      </details>
    </div>

    <!-- Error Display -->
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
          <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
          <path d="M12 9v4" />
          <path d="M12 17h.01" />
        </svg>
        <span>{{ error }}</span>
      </div>
      <button
        class="shrink-0 text-destructive/70 hover:text-destructive"
        @click="webdavStore.clearError()"
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
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useI18n } from 'vue-i18n';
import { useWebDavStore } from '@/stores/webdavStore';

const props = defineProps<{ isActive: boolean }>();

const { t } = useI18n();
const webdavStore = useWebDavStore();
const {
  config,
  status,
  lastSyncAt,
  cloudEntryCount,
  registeredDevices,
  rateLimitAvailable,
  rateLimitCapacity,
  error,
  isConnecting,
} = storeToRefs(webdavStore);

const isSyncing = ref(false);

const form = reactive({
  serverUrl: '',
  username: '',
  password: '',
  syncPassword: '',
});

const settingsForm = reactive({
  syncImages: false,
  syncSensitive: false,
  pollIntervalSecs: 30,
  maxCloudEntries: 2000,
  remotePath: '/SmartClipboard',
  rateLimitCapacity: 150,
  rateLimitRefillMinutes: 30,
});

const canConnect = computed(
  () =>
    form.serverUrl.trim() !== '' &&
    form.username.trim() !== '' &&
    form.password.trim() !== '' &&
    form.syncPassword.trim() !== '',
);

const statusText = computed(() => {
  switch (status.value) {
    case 'connected':
      return t('webdav.connected');
    case 'connecting':
      return t('webdav.connecting');
    case 'disconnected':
      return t('webdav.disconnected');
    case 'error':
      return t('webdav.error');
    default:
      return status.value;
  }
});

const statusDotClass = computed(() => {
  if (status.value === 'connected') return 'bg-emerald-500';
  if (status.value === 'connecting') return 'bg-amber-500 animate-pulse';
  if (status.value === 'error') return 'bg-destructive';
  return 'bg-muted-foreground';
});

const lastSyncDisplay = computed(() => {
  if (!lastSyncAt.value) return '—';
  return formatTime(lastSyncAt.value);
});

function formatTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    return date.toLocaleString();
  } catch {
    return isoString;
  }
}

watch(
  () => props.isActive,
  async (active) => {
    if (!active) return;
    await webdavStore.refreshAll();
    syncSettingsFromStore();
    syncFormFromStore();
  },
);

function syncFormFromStore() {
  form.serverUrl = config.value.serverUrl;
  form.username = config.value.username;
  form.password = config.value.password;
  form.syncPassword = config.value.syncPassword;
}

function syncSettingsFromStore() {
  settingsForm.syncImages = config.value.syncImages;
  settingsForm.syncSensitive = config.value.syncSensitive;
  settingsForm.pollIntervalSecs = config.value.pollIntervalSecs;
  settingsForm.maxCloudEntries = config.value.maxCloudEntries;
  settingsForm.remotePath = config.value.remotePath;
  settingsForm.rateLimitCapacity = config.value.rateLimitCapacity;
  settingsForm.rateLimitRefillMinutes = config.value.rateLimitRefillMinutes;
}

async function handleConnect() {
  try {
    await webdavStore.connect(
      form.serverUrl.trim(),
      form.username.trim(),
      form.password.trim(),
      form.syncPassword.trim(),
    );
    // Save credentials to config
    await webdavStore.saveConfig({
      ...config.value,
      enabled: true,
      serverUrl: form.serverUrl.trim(),
      username: form.username.trim(),
      password: form.password.trim(),
      syncPassword: form.syncPassword.trim(),
    });
  } catch {
    // Error is handled in store
  }
}

async function handleDisconnect() {
  await webdavStore.disconnect();
  await webdavStore.saveConfig({
    ...config.value,
    enabled: false,
  });
}

async function handleTriggerSync() {
  isSyncing.value = true;
  try {
    await webdavStore.triggerSync();
  } catch {
    // Error handled in store
  } finally {
    isSyncing.value = false;
  }
}

async function handleSaveSettings() {
  await webdavStore.saveConfig({
    ...config.value,
    syncImages: settingsForm.syncImages,
    syncSensitive: settingsForm.syncSensitive,
    pollIntervalSecs: settingsForm.pollIntervalSecs,
    maxCloudEntries: settingsForm.maxCloudEntries,
    remotePath: settingsForm.remotePath,
    rateLimitCapacity: settingsForm.rateLimitCapacity,
    rateLimitRefillMinutes: settingsForm.rateLimitRefillMinutes,
  });
}

function handleRemoveDevice(deviceId: string, deviceName: string) {
  const confirmed = window.confirm(t('webdav.removeDeviceConfirm', { name: deviceName }));
  if (confirmed) {
    webdavStore.removeDevice(deviceId);
  }
}
</script>
