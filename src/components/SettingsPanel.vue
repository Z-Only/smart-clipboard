<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    @click.self="close"
  >
    <div class="bg-card border border-border rounded-lg shadow-lg w-[360px] max-h-[80vh] overflow-y-auto p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-base font-semibold">{{ $t('settings.title') }}</h2>
        <button class="text-muted-foreground hover:text-foreground" @click="close">
          <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18" /><path d="m6 6 12 12" />
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
          <Input v-model="form.monitor_interval_ms" type="number" min="200" max="5000" step="100" class="h-8" />
          <span class="text-xs text-muted-foreground">{{ $t('settings.monitorIntervalHint') }}</span>
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
              :class="appearance === mode
                ? 'border-primary bg-primary text-primary-foreground'
                : 'border-input bg-background text-foreground hover:bg-accent'"
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
                :class="themeColor === color.id
                  ? 'border-foreground scale-110'
                  : 'border-transparent hover:border-muted-foreground/50'"
                :style="{ backgroundColor: color.swatch }"
              />
              <span
                class="text-[10px]"
                :class="themeColor === color.id ? 'text-foreground font-medium' : 'text-muted-foreground'"
              >
                {{ $t(`settings.theme${color.id.charAt(0).toUpperCase() + color.id.slice(1)}`) }}
              </span>
            </button>
          </div>
        </div>

        <Separator />

        <!-- Action buttons -->
        <div class="flex justify-end gap-2">
          <Button variant="outline" size="sm" @click="resetDefaults">{{ $t('settings.resetDefaults') }}</Button>
          <Button size="sm" @click="save">{{ $t('settings.save') }}</Button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { setLocale, getLocale } from "@/i18n";
import { useTheme, type AppearanceMode, type ThemeColor } from "@/composables/useTheme";

interface AppConfig {
  max_entries: number;
  retention_days: number;
  excluded_apps: string[];
  monitor_interval_ms: number;
  autostart_enabled: boolean;
}

const props = defineProps<{ isOpen: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { locale } = useI18n();
const currentLocale = computed(() => locale.value);

function changeLanguage(lang: string) {
  setLocale(lang);
}

const { appearance, themeColor, setAppearance, setThemeColor } = useTheme();
const appearanceModes: AppearanceMode[] = ["system", "light", "dark"];
const themeColors: { id: ThemeColor; swatch: string }[] = [
  { id: "zinc", swatch: "#71717a" },
  { id: "blue", swatch: "#3b82f6" },
  { id: "green", swatch: "#22c55e" },
  { id: "rose", swatch: "#f43f5e" },
  { id: "orange", swatch: "#f97316" },
  { id: "violet", swatch: "#8b5cf6" },
];

const form = reactive<AppConfig>({
  max_entries: 5000,
  retention_days: 30,
  excluded_apps: [],
  monitor_interval_ms: 500,
  autostart_enabled: false,
});

const autostart = ref(false);

const excludedAppsText = computed({
  get: () => form.excluded_apps.join("\n"),
  set: (val: string) => {
    form.excluded_apps = val
      .split("\n")
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
  }
);

async function loadConfig() {
  try {
    const config = await invoke<AppConfig>("get_config");
    Object.assign(form, config);
    autostart.value = await invoke<boolean>("get_autostart_enabled");
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

async function save() {
  try {
    await invoke("update_config", { newConfig: { ...form } });
    close();
  } catch (e) {
    console.error("Failed to save config:", e);
  }
}

function resetDefaults() {
  form.max_entries = 5000;
  form.retention_days = 30;
  form.excluded_apps = [];
  form.monitor_interval_ms = 500;
  form.autostart_enabled = false;
}

async function toggleAutostart() {
  try {
    autostart.value = !autostart.value;
    await invoke("set_autostart_enabled", { enabled: autostart.value });
  } catch (e) {
    console.error("Failed to toggle autostart:", e);
    autostart.value = !autostart.value;
  }
}

function close() {
  emit("close");
}
</script>
