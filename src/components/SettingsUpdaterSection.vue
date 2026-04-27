<template>
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
        :class="config.auto_check_enabled ? 'bg-primary' : 'bg-input'"
        @click="toggle('auto_check_enabled')"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="config.auto_check_enabled ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>

    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium">{{ $t('settings.updater.checkFrequency') }}</label>
      <select
        :value="config.check_interval_hours"
        class="h-8 rounded-md border border-input bg-background px-2 text-sm"
        :disabled="!config.auto_check_enabled"
        @change="
          updateField('check_interval_hours', Number(($event.target as HTMLSelectElement).value))
        "
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
        :class="config.auto_download_enabled ? 'bg-primary' : 'bg-input'"
        @click="toggle('auto_download_enabled')"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="config.auto_download_enabled ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>

    <div class="flex items-center justify-between">
      <div>
        <label class="text-sm font-medium">{{ $t('settings.updater.wifiOnly') }}</label>
      </div>
      <button
        class="relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors"
        :disabled="!config.auto_download_enabled"
        :class="config.wifi_only ? 'bg-primary' : 'bg-input'"
        @click="toggle('wifi_only')"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="config.wifi_only ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>

    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium">{{ $t('settings.updater.mirrors') }}</label>
      <textarea
        :value="mirrorsText"
        data-test="updater-mirrors"
        class="flex min-h-[60px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
        rows="3"
        :placeholder="$t('settings.updater.mirrorsPlaceholder')"
        @input="onMirrorsInput(($event.target as HTMLTextAreaElement).value)"
      />
      <span v-if="validationError" class="text-xs text-destructive">{{ $t(validationError) }}</span>
    </div>

    <button
      type="button"
      class="w-full rounded-md border border-input px-3 py-2 text-left text-sm hover:bg-accent"
      data-test="updater-check-now"
      @click="manualCheck"
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
      <div class="text-xs text-muted-foreground">{{ $t(phaseLabel) }}</div>
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
        {{ $t('settings.updater.availableTitle', { version: updater.status.availableVersion }) }}
      </div>
      <div class="text-xs text-muted-foreground">{{ localizedAvailableNotes }}</div>
      <div class="flex gap-2">
        <Button size="sm" @click="downloadAvailable">{{
          $t('settings.updater.downloadInstaller')
        }}</Button>
      </div>
    </div>

    <div v-if="updater.status.pendingUpdate" class="rounded-md border border-input p-3 space-y-2">
      <div class="text-sm font-medium">
        {{ $t('settings.updater.readyTitle', { version: updater.status.pendingUpdate.version }) }}
      </div>
      <div class="text-xs text-muted-foreground">
        {{ localizedPendingNotes }}
      </div>
      <div class="flex gap-2">
        <Button size="sm" @click="installPending">{{
          $t('settings.updater.installAndRestart')
        }}</Button>
        <Button size="sm" variant="outline" @click="discardPending">{{
          $t('settings.updater.discardPending')
        }}</Button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import { useUpdaterStore } from '@/stores/updaterStore';
import { extractLocalizedNotes } from '@/lib/changelog';

interface UpdaterConfig {
  auto_check_enabled: boolean;
  check_interval_hours: number;
  auto_download_enabled: boolean;
  wifi_only: boolean;
  mirrors: string[];
  last_check_at: string | null;
}

const props = defineProps<{ modelValue: UpdaterConfig }>();
const emit = defineEmits<{ 'update:modelValue': [value: UpdaterConfig] }>();

const { locale } = useI18n();
const updater = useUpdaterStore();

const config = computed(() => props.modelValue);

const localizedAvailableNotes = computed(() =>
  extractLocalizedNotes(updater.status.availableNotes, locale.value),
);

const localizedPendingNotes = computed(() =>
  extractLocalizedNotes(updater.status.pendingUpdate?.notes ?? null, locale.value),
);

function updateField<K extends keyof UpdaterConfig>(key: K, value: UpdaterConfig[K]) {
  emit('update:modelValue', { ...props.modelValue, [key]: value });
}

function toggle(key: 'auto_check_enabled' | 'auto_download_enabled' | 'wifi_only') {
  updateField(key, !props.modelValue[key]);
}

const mirrorsText = computed(() => props.modelValue.mirrors.join('\n'));

function onMirrorsInput(value: string) {
  const mirrors = value
    .split('\n')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  emit('update:modelValue', { ...props.modelValue, mirrors });
}

const validationError = ref('');

function validateMirrors(): boolean {
  const invalid = props.modelValue.mirrors.find(
    (mirror) => !mirror.startsWith('https://') || !mirror.includes('{url}'),
  );
  validationError.value = invalid ? 'settings.updater.invalidMirror' : '';
  return !invalid;
}

const phaseLabel = computed(() => `settings.updater.phase.${updater.status.phase}`);

async function manualCheck() {
  await updater.checkNow();
}

async function downloadAvailable() {
  await updater.downloadAvailable();
}

async function installPending() {
  await updater.installPending();
}

async function discardPending() {
  await updater.discardPending();
}

async function quitApp() {
  await invoke('quit_app');
}

defineExpose({ validateMirrors });
</script>
