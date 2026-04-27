<template>
  <div class="flex flex-col h-screen bg-background select-none">
    <!-- Top bar -->
    <div class="flex items-center gap-2 px-3 pt-3 pb-2" data-tauri-drag-region>
      <div class="flex-1">
        <SearchBar ref="searchBarRef" />
      </div>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('templates.title')"
        @click="showTemplates = true"
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
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <line x1="16" y1="13" x2="8" y2="13" />
          <line x1="16" y1="17" x2="8" y2="17" />
          <polyline points="10 9 9 9 8 9" />
        </svg>
      </button>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('app.statistics')"
        @click="showStatistics = true"
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
          <line x1="18" y1="20" x2="18" y2="10" />
          <line x1="12" y1="20" x2="12" y2="4" />
          <line x1="6" y1="20" x2="6" y2="14" />
        </svg>
      </button>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('sync.title')"
        @click="showSync = true"
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
          <path d="M8 17l-4-4 4-4" />
          <path d="M16 7l4 4-4 4" />
          <path d="M14 4l-4 16" />
        </svg>
      </button>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('smart.groups')"
        @click="showSmartGroups = true"
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
          <circle cx="18" cy="5" r="3" />
          <circle cx="6" cy="12" r="3" />
          <circle cx="18" cy="19" r="3" />
          <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
          <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
        </svg>
      </button>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('app.settings')"
        @click="showSettings = true"
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
          <path
            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
          />
          <circle cx="12" cy="12" r="3" />
        </svg>
      </button>
    </div>

    <Separator />

    <!-- Main content -->
    <div class="flex flex-1 min-h-0">
      <!-- Sidebar -->
      <div class="w-28 shrink-0 border-r border-border overflow-y-auto">
        <CategoryFilter />
      </div>

      <!-- Clipboard list -->
      <div class="flex-1 min-w-0">
        <ClipboardList />
      </div>
    </div>

    <Separator />

    <!-- Status bar -->
    <div class="flex items-center justify-between px-3 py-1.5 text-xs text-muted-foreground">
      <span>{{ $t('app.entries', { count: totalCount }) }}</span>
      <span class="opacity-60">Cmd+Shift+V</span>
    </div>

    <LockScreen v-if="showLockOverlay" />

    <!-- Sync panel -->
    <SyncPanel :is-open="showSync" @close="showSync = false" />

    <!-- Smart Groups panel -->
    <SmartGroupsPanel :is-open="showSmartGroups" @close="showSmartGroups = false" />

    <!-- Settings panel -->
    <SettingsPanel :is-open="showSettings" @close="showSettings = false" />

    <!-- Statistics panel -->
    <StatisticsPanel :is-open="showStatistics" @close="showStatistics = false" />

    <!-- Templates panel -->
    <TemplateList :is-open="showTemplates" @close="showTemplates = false" />

    <!-- Conflict resolve dialog -->
    <ConflictResolveDialog :conflict="activeConflict" />

    <!-- Quick paste overlay -->
    <QuickPasteOverlay
      :entries="recentEntries"
      :is-active="quickPasteActive"
      @paste="handleQuickPaste"
      @dismiss="handleQuickPasteDismiss"
      @search="handleQuickPasteSearch"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import { Separator } from '@/components/ui/separator';
import SearchBar from '@/components/SearchBar.vue';
import CategoryFilter from '@/components/CategoryFilter.vue';
import ClipboardList from '@/components/ClipboardList.vue';
import SettingsPanel from '@/components/SettingsPanel.vue';
import StatisticsPanel from '@/components/StatisticsPanel.vue';
import TemplateList from '@/components/TemplateList.vue';
import SyncPanel from '@/components/SyncPanel.vue';
import SmartGroupsPanel from '@/components/SmartGroupsPanel.vue';
import LockScreen from '@/components/LockScreen.vue';
import ConflictResolveDialog from '@/components/ConflictResolveDialog.vue';
import QuickPasteOverlay from '@/components/QuickPasteOverlay.vue';
import { useSecurityStore } from '@/stores/securityStore';
import { useClipboardStore } from '@/stores/clipboardStore';
import { useSyncStore } from '@/stores/syncStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useWebDavStore } from '@/stores/webdavStore';
import { useConflictStore } from '@/stores/conflictStore';
import { useSmartStore } from '@/stores/smartStore';
import { useClipboard } from '@/composables/useClipboard';
import { useTheme } from '@/composables/useTheme';

const store = useClipboardStore();
const security = useSecurityStore();
const syncStore = useSyncStore();
const templateStore = useTemplateStore();
const webdavStore = useWebDavStore();
const conflictStore = useConflictStore();
const smartStore = useSmartStore();
useTheme();

const { totalCount } = storeToRefs(store);
const { activeConflict } = storeToRefs(conflictStore);

const searchBarRef = ref<InstanceType<typeof SearchBar> | null>(null);
const showSettings = ref(false);
const showStatistics = ref(false);
const showTemplates = ref(false);
const showSync = ref(false);
const showSmartGroups = ref(false);
const showLockOverlay = computed(() => security.status.enabled && security.status.locked);

const quickPasteActive = ref(false);
const { recentEntries } = storeToRefs(store);

// Listen for clipboard changes from backend
useClipboard();

watch(
  () => security.status.locked,
  async (locked, prev) => {
    if (locked) {
      store.clearSensitiveViewState();
      syncStore.clearSensitiveState();
      templateStore.clearSensitiveState();
      webdavStore.clearSensitiveState();
      smartStore.clearSensitiveState();
      showSettings.value = false;
      showStatistics.value = false;
      showTemplates.value = false;
      showSync.value = false;
      showSmartGroups.value = false;
      quickPasteActive.value = false;
      return;
    }
    if (prev && !locked) {
      await Promise.allSettled([
        store.fetchEntries(true),
        store.fetchAllTags(),
        syncStore.refreshAll(),
        templateStore.fetchTemplates(),
        templateStore.fetchCategories(),
        webdavStore.refreshAll(),
        smartStore.fetchClusters(),
      ]);
      searchBarRef.value?.focus();
    }
  },
);

async function activateQuickPaste() {
  if (security.status.locked) return;
  await store.fetchRecentEntries(9);
  quickPasteActive.value = true;
}

async function handleQuickPaste(entryId: number) {
  quickPasteActive.value = false;
  await store.pasteEntry(entryId);
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().hide();
}

function handleQuickPasteDismiss() {
  quickPasteActive.value = false;
}

function handleQuickPasteSearch(text: string) {
  quickPasteActive.value = false;
  searchBarRef.value?.focus();
  store.setSearch(text);
}

onMounted(async () => {
  await security.init();
  if (!security.status.locked) {
    await store.fetchEntries();
  }

  // Focus search bar when window is shown
  await listen('window-shown', async () => {
    if (!security.status.locked) {
      searchBarRef.value?.focus();
      await store.fetchEntries();
    }
  });

  // Open settings from tray menu
  await listen('open-settings', () => {
    showSettings.value = true;
  });

  // Quick paste activation
  await listen('quick-paste-activated', () => {
    activateQuickPaste();
  });
});
</script>
