<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    @click.self="close"
  >
    <div
      class="bg-card border border-border rounded-lg shadow-lg w-[360px] max-h-[80vh] overflow-y-auto p-5"
    >
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-base font-semibold">{{ $t('settings.title') }}</h2>
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

      <div class="flex flex-col gap-4">
        <!-- Max entries -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.maxEntries') }}</label>
          <Input v-model="form.max_entries" type="number" min="100" max="50000" class="h-8" />
          <span class="text-xs text-muted-foreground">{{ $t('settings.maxEntriesHint') }}</span>
        </div>

        <!-- Retention days -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.retentionDays') }}</label>
          <Input v-model="form.retention_days" type="number" min="1" max="365" class="h-8" />
          <span class="text-xs text-muted-foreground">{{ $t('settings.retentionDaysHint') }}</span>
        </div>

        <!-- Monitor interval -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.monitorInterval') }}</label>
          <Input
            v-model="form.monitor_interval_ms"
            type="number"
            min="200"
            max="5000"
            step="100"
            class="h-8"
          />
          <span class="text-xs text-muted-foreground">{{
            $t('settings.monitorIntervalHint')
          }}</span>
        </div>

        <!-- Sensitive expiry -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.sensitiveExpiry') }}</label>
          <Input
            v-model="form.sensitive_expiry_minutes"
            type="number"
            min="0"
            max="1440"
            class="h-8"
          />
          <span class="text-xs text-muted-foreground">{{
            $t('settings.sensitiveExpiryHint')
          }}</span>
        </div>

        <!-- Excluded apps -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.excludedApps') }}</label>
          <textarea
            v-model="excludedAppsText"
            class="flex min-h-[60px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            :placeholder="$t('settings.excludedAppsPlaceholder')"
            rows="3"
          />
          <span class="text-xs text-muted-foreground">{{ $t('settings.excludedAppsHint') }}</span>
        </div>

        <!-- Auto-start -->
        <div class="flex items-center justify-between">
          <div>
            <label class="text-sm font-medium">{{ $t('settings.autoStart') }}</label>
            <p class="text-xs text-muted-foreground">{{ $t('settings.autoStartHint') }}</p>
          </div>
          <button
            class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            :class="autostart ? 'bg-primary' : 'bg-input'"
            @click="toggleAutostart"
          >
            <span
              class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg ring-0 transition-transform"
              :class="autostart ? 'translate-x-4' : 'translate-x-0'"
            />
          </button>
        </div>

        <!-- Language -->
        <div class="flex items-center justify-between">
          <div>
            <label class="text-sm font-medium">{{ $t('settings.language') }}</label>
            <p class="text-xs text-muted-foreground">{{ $t('settings.languageHint') }}</p>
          </div>
          <select
            :value="currentLocale"
            class="h-8 rounded-md border border-input bg-background px-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            @change="changeLanguage(($event.target as HTMLSelectElement).value)"
          >
            <option value="en">English</option>
            <option value="zh-CN">中文</option>
          </select>
        </div>

        <!-- Appearance Mode -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.appearance') }}</label>
          <p class="text-xs text-muted-foreground">{{ $t('settings.appearanceHint') }}</p>
          <div class="flex gap-1.5">
            <button
              v-for="mode in appearanceModes"
              :key="mode"
              class="flex-1 h-8 rounded-md border text-xs font-medium transition-colors"
              :class="
                appearance === mode
                  ? 'border-primary bg-primary text-primary-foreground'
                  : 'border-input bg-background text-foreground hover:bg-accent'
              "
              @click="setAppearance(mode)"
            >
              {{ $t(`settings.appearance${mode.charAt(0).toUpperCase() + mode.slice(1)}`) }}
            </button>
          </div>
        </div>

        <!-- Theme Color -->
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium">{{ $t('settings.themeColor') }}</label>
          <p class="text-xs text-muted-foreground">{{ $t('settings.themeColorHint') }}</p>
          <div class="flex gap-2 flex-wrap">
            <button
              v-for="color in themeColors"
              :key="color.id"
              class="flex flex-col items-center gap-1 group"
              @click="setThemeColor(color.id)"
            >
              <span
                class="w-7 h-7 rounded-full border-2 transition-all"
                :class="
                  themeColor === color.id
                    ? 'border-foreground scale-110'
                    : 'border-transparent hover:border-muted-foreground/50'
                "
                :style="{ backgroundColor: color.swatch }"
              />
              <span
                class="text-[10px]"
                :class="
                  themeColor === color.id ? 'text-foreground font-medium' : 'text-muted-foreground'
                "
              >
                {{ $t(`settings.theme${color.id.charAt(0).toUpperCase() + color.id.slice(1)}`) }}
              </span>
            </button>
          </div>
        </div>

        <Separator />

        <div class="space-y-3">
          <div class="space-y-1">
            <label class="text-sm font-medium">{{ $t('settings.updater.title') }}</label>
            <p class="text-xs text-muted-foreground">{{ $t('settings.updater.hint') }}</p>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('settings.updater.autoCheck') }}</label>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="form.updater.auto_check_enabled ? 'bg-primary' : 'bg-input'"
              @click="form.updater.auto_check_enabled = !form.updater.auto_check_enabled"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="form.updater.auto_check_enabled ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-sm font-medium">{{ $t('settings.updater.checkFrequency') }}</label>
            <select
              v-model.number="form.updater.check_interval_hours"
              class="h-8 rounded-md border border-input bg-background px-2 text-sm"
              :disabled="!form.updater.auto_check_enabled"
            >
              <option :value="6">{{ $t('settings.updater.every6Hours') }}</option>
              <option :value="12">{{ $t('settings.updater.every12Hours') }}</option>
              <option :value="24">{{ $t('settings.updater.daily') }}</option>
              <option :value="168">{{ $t('settings.updater.weekly') }}</option>
            </select>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('settings.updater.autoDownload') }}</label>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="form.updater.auto_download_enabled ? 'bg-primary' : 'bg-input'"
              @click="form.updater.auto_download_enabled = !form.updater.auto_download_enabled"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="form.updater.auto_download_enabled ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('settings.updater.wifiOnly') }}</label>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors"
              :disabled="!form.updater.auto_download_enabled"
              :class="form.updater.wifi_only ? 'bg-primary' : 'bg-input'"
              @click="form.updater.wifi_only = !form.updater.wifi_only"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="form.updater.wifi_only ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-sm font-medium">{{ $t('settings.updater.mirrors') }}</label>
            <textarea
              v-model="updaterMirrorsText"
              data-test="updater-mirrors"
              class="flex min-h-[60px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              rows="3"
              :placeholder="$t('settings.updater.mirrorsPlaceholder')"
            />
            <span v-if="updaterValidationError" class="text-xs text-destructive">{{
              $t(updaterValidationError)
            }}</span>
          </div>

          <button
            type="button"
            class="w-full rounded-md border border-input px-3 py-2 text-left text-sm hover:bg-accent"
            data-test="updater-check-now"
            @click="manualUpdaterCheck"
          >
            <div>
              {{
                $t('settings.updater.currentVersion', {
                  version: updater.status.currentVersion || 'unknown',
                })
              }}
            </div>
            <div class="text-xs text-muted-foreground">
              {{ $t('settings.updater.clickToCheck') }}
            </div>
            <div class="text-xs text-muted-foreground">{{ $t(updaterPhaseLabel()) }}</div>
          </button>

          <div
            v-if="updater.status.phase === 'installing'"
            class="rounded-md border border-input p-3 space-y-2"
          >
            <div class="text-sm font-medium">{{ $t('settings.updater.installingTitle') }}</div>
            <div class="text-xs text-muted-foreground">
              {{ $t('settings.updater.installingHint') }}
            </div>
            <div class="flex gap-2">
              <Button size="sm" variant="destructive" @click="quitApp">{{
                $t('settings.updater.quitNow')
              }}</Button>
            </div>
          </div>

          <div
            v-if="!updater.status.pendingUpdate && updater.status.phase === 'downloading'"
            class="rounded-md border border-input p-3 space-y-2"
          >
            <div class="text-sm font-medium">{{ $t('settings.updater.phase.downloading') }}</div>
            <div class="text-xs text-muted-foreground">
              {{ Math.round((updater.status.downloadProgress ?? 0) * 100) }}%
            </div>
          </div>

          <div
            v-if="!updater.status.pendingUpdate && updater.status.phase === 'update_available'"
            class="rounded-md border border-input p-3 space-y-2"
          >
            <div class="text-sm font-medium">
              {{
                $t('settings.updater.availableTitle', { version: updater.status.availableVersion })
              }}
            </div>
            <div class="text-xs text-muted-foreground">{{ updater.status.availableNotes }}</div>
            <div class="flex gap-2">
              <Button size="sm" @click="downloadAvailableUpdate">{{
                $t('settings.updater.downloadInstaller')
              }}</Button>
            </div>
          </div>

          <div
            v-if="updater.status.pendingUpdate"
            class="rounded-md border border-input p-3 space-y-2"
          >
            <div class="text-sm font-medium">
              {{
                $t('settings.updater.readyTitle', { version: updater.status.pendingUpdate.version })
              }}
            </div>
            <div class="text-xs text-muted-foreground">
              {{ updater.status.pendingUpdate.notes }}
            </div>
            <div class="flex gap-2">
              <Button size="sm" @click="installPendingUpdate">{{
                $t('settings.updater.installAndRestart')
              }}</Button>
              <Button size="sm" variant="outline" @click="discardPendingUpdate">{{
                $t('settings.updater.discardPending')
              }}</Button>
            </div>
          </div>
        </div>

        <Separator />

        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('lock.settingsTitle') }}</label>
              <p class="text-xs text-muted-foreground">{{ $t('lock.settingsHint') }}</p>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="form.app_lock.enabled ? 'bg-primary' : 'bg-input'"
              @click="form.app_lock.enabled = !form.app_lock.enabled"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="form.app_lock.enabled ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-sm font-medium">{{ $t('lock.autoLock') }}</label>
            <Input
              v-model="form.app_lock.auto_lock_seconds"
              type="number"
              min="0"
              max="86400"
              class="h-8"
            />
            <span class="text-xs text-muted-foreground">{{ $t('lock.autoLockHint') }}</span>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('lock.biometric') }}</label>
              <p class="text-xs text-muted-foreground">{{ $t('lock.biometricHint') }}</p>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :disabled="!security.status.biometric_available"
              :class="
                form.app_lock.biometric_enabled ? 'bg-primary' : 'bg-input disabled:opacity-50'
              "
              @click="form.app_lock.biometric_enabled = !form.app_lock.biometric_enabled"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="form.app_lock.biometric_enabled ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>

          <div class="grid gap-2">
            <Input
              v-model="currentPassword"
              type="password"
              class="h-8"
              :placeholder="$t('lock.currentPasswordPlaceholder')"
            />
            <Input
              v-model="newPassword"
              type="password"
              class="h-8"
              :placeholder="$t('lock.newPasswordPlaceholder')"
            />
            <Button variant="outline" size="sm" @click="savePassword">{{
              $t('lock.setPassword')
            }}</Button>
          </div>

          <Button variant="outline" size="sm" @click="manualLock">{{ $t('lock.lockNow') }}</Button>
        </div>

        <Separator />

        <!-- Database Encryption -->
        <div class="space-y-3">
          <div class="space-y-1">
            <label class="text-sm font-medium">{{ $t('encryption.settingsTitle') }}</label>
            <p class="text-xs text-muted-foreground">{{ $t('encryption.settingsHint') }}</p>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <label class="text-sm font-medium">{{ $t('encryption.enabled') }}</label>
              <p v-if="security.encryption.migrating" class="text-xs text-yellow-600">
                {{ $t('encryption.migrating') }}
              </p>
              <p v-else-if="security.encryption.enabled" class="text-xs text-muted-foreground">
                {{
                  $t('encryption.statusEncrypted', { count: security.encryption.encrypted_count })
                }}
              </p>
              <p
                v-else-if="security.encryption.plaintext_count > 0"
                class="text-xs text-muted-foreground"
              >
                {{
                  $t('encryption.statusPlaintext', { count: security.encryption.plaintext_count })
                }}
              </p>
            </div>
            <button
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
              :class="security.encryption.enabled ? 'bg-primary' : 'bg-input'"
              :disabled="security.encryption.migrating || security.loading"
              @click="toggleEncryption"
            >
              <span
                class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
                :class="security.encryption.enabled ? 'translate-x-4' : 'translate-x-0'"
              />
            </button>
          </div>
        </div>

        <!-- Action buttons -->
        <div class="flex justify-end gap-2">
          <Button variant="outline" size="sm" @click="resetDefaults">{{
            $t('settings.resetDefaults')
          }}</Button>
          <Button data-test="settings-save" size="sm" @click="save">{{
            $t('settings.save')
          }}</Button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { setLocale } from '@/i18n';
import { useTheme, type AppearanceMode, type ThemeColor } from '@/composables/useTheme';
import { useSecurityStore } from '@/stores/securityStore';
import { useUpdaterStore } from '@/stores/updaterStore';

interface AppLockConfig {
  enabled: boolean;
  auto_lock_seconds: number;
  biometric_enabled: boolean;
}

interface UpdaterConfig {
  auto_check_enabled: boolean;
  check_interval_hours: number;
  auto_download_enabled: boolean;
  wifi_only: boolean;
  mirrors: string[];
  last_check_at: string | null;
}

interface AppConfig {
  max_entries: number;
  retention_days: number;
  excluded_apps: string[];
  monitor_interval_ms: number;
  autostart_enabled: boolean;
  sensitive_expiry_minutes: number;
  app_lock: AppLockConfig;
  updater: UpdaterConfig;
}

const props = defineProps<{ isOpen: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { locale } = useI18n();
const currentLocale = computed(() => locale.value);

function changeLanguage(lang: string) {
  setLocale(lang);
}

const { appearance, themeColor, setAppearance, setThemeColor } = useTheme();
const security = useSecurityStore();
const updater = useUpdaterStore();
const appearanceModes: AppearanceMode[] = ['system', 'light', 'dark'];
const themeColors: { id: ThemeColor; swatch: string }[] = [
  { id: 'zinc', swatch: '#71717a' },
  { id: 'blue', swatch: '#3b82f6' },
  { id: 'green', swatch: '#22c55e' },
  { id: 'rose', swatch: '#f43f5e' },
  { id: 'orange', swatch: '#f97316' },
  { id: 'violet', swatch: '#8b5cf6' },
];

const form = reactive<AppConfig>({
  max_entries: 5000,
  retention_days: 30,
  excluded_apps: [],
  monitor_interval_ms: 500,
  autostart_enabled: false,
  sensitive_expiry_minutes: 5,
  app_lock: {
    enabled: false,
    auto_lock_seconds: 0,
    biometric_enabled: false,
  },
  updater: {
    auto_check_enabled: true,
    check_interval_hours: 24,
    auto_download_enabled: false,
    wifi_only: true,
    mirrors: [],
    last_check_at: null,
  },
});

const autostart = ref(false);

const updaterMirrorsText = computed({
  get: () => form.updater.mirrors.join('\n'),
  set: (val: string) => {
    form.updater.mirrors = val
      .split('\n')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  },
});

const updaterValidationError = ref('');

function validateUpdaterMirrors() {
  const invalid = form.updater.mirrors.find(
    (mirror) => !mirror.startsWith('https://') || !mirror.includes('{url}'),
  );
  updaterValidationError.value = invalid ? 'settings.updater.invalidMirror' : '';
  return !invalid;
}

function updaterPhaseLabel() {
  return `settings.updater.phase.${updater.status.phase}`;
}

const excludedAppsText = computed({
  get: () => form.excluded_apps.join('\n'),
  set: (val: string) => {
    form.excluded_apps = val
      .split('\n')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  },
});

watch(
  () => props.isOpen,
  async (open) => {
    if (open) {
      await loadConfig();
    }
  },
);

async function loadConfig() {
  try {
    const config = await invoke<AppConfig>('get_config');
    Object.assign(form, config);
    autostart.value = await invoke<boolean>('get_autostart_enabled');
    await security.refresh();
  } catch (e) {
    console.error('Failed to load config:', e);
  }
}

async function save() {
  try {
    if (!validateUpdaterMirrors()) return;
    await invoke('update_config', { newConfig: { ...form } });
    await security.updateSettings(form.app_lock);
    close();
  } catch (e) {
    console.error('Failed to save config:', e);
  }
}

function resetDefaults() {
  form.max_entries = 5000;
  form.retention_days = 30;
  form.excluded_apps = [];
  form.monitor_interval_ms = 500;
  form.autostart_enabled = false;
  form.sensitive_expiry_minutes = 5;
  form.app_lock = { enabled: false, auto_lock_seconds: 0, biometric_enabled: false };
  form.updater = {
    auto_check_enabled: true,
    check_interval_hours: 24,
    auto_download_enabled: false,
    wifi_only: true,
    mirrors: [],
    last_check_at: null,
  };
}

async function manualUpdaterCheck() {
  await updater.checkNow();
}

async function downloadAvailableUpdate() {
  await updater.downloadAvailable();
}

async function installPendingUpdate() {
  await updater.installPending();
}

async function discardPendingUpdate() {
  await updater.discardPending();
}

async function quitApp() {
  await invoke('quit_app');
}

const currentPassword = ref('');
const newPassword = ref('');

async function savePassword() {
  if (!newPassword.value) return;
  await security.setPassword(currentPassword.value || null, newPassword.value);
  currentPassword.value = '';
  newPassword.value = '';
  form.app_lock.enabled = true;
}

async function manualLock() {
  await security.lock();
  close();
}

async function toggleEncryption() {
  const { t } = useI18n();
  if (security.encryption.enabled) {
    if (!confirm(t('encryption.disableConfirm'))) return;
    try {
      await security.disableEncryption();
    } catch (error) {
      console.error('Failed to disable encryption:', error);
    }
  } else {
    if (!confirm(t('encryption.enableConfirm'))) return;
    try {
      await security.enableEncryption();
    } catch (error) {
      console.error('Failed to enable encryption:', error);
    }
  }
}

async function toggleAutostart() {
  try {
    autostart.value = !autostart.value;
    await invoke('set_autostart_enabled', { enabled: autostart.value });
  } catch (e) {
    console.error('Failed to toggle autostart:', e);
    autostart.value = !autostart.value;
  }
}

function close() {
  emit('close');
}
</script>
