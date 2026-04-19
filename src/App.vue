<template>
  <div class="flex flex-col h-screen bg-background select-none">
    <!-- Top bar -->
    <div class="flex items-center gap-2 px-3 pt-3 pb-2" data-tauri-drag-region>
      <div class="flex-1">
        <SearchBar ref="searchBarRef" />
      </div>
      <button
        class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
        :title="$t('app.settings')"
        @click="showSettings = true"
      >
        <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
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

    <!-- Settings panel -->
    <SettingsPanel :is-open="showSettings" @close="showSettings = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { listen } from "@tauri-apps/api/event";
import { Separator } from "@/components/ui/separator";
import SearchBar from "@/components/SearchBar.vue";
import CategoryFilter from "@/components/CategoryFilter.vue";
import ClipboardList from "@/components/ClipboardList.vue";
import SettingsPanel from "@/components/SettingsPanel.vue";
import { useClipboardStore } from "@/stores/clipboardStore";
import { useClipboard } from "@/composables/useClipboard";

const store = useClipboardStore();
const { totalCount } = storeToRefs(store);

const searchBarRef = ref<InstanceType<typeof SearchBar> | null>(null);
const showSettings = ref(false);

// Listen for clipboard changes from backend
useClipboard();

onMounted(async () => {
  await store.fetchEntries();

  // Focus search bar when window is shown
  await listen("window-shown", () => {
    searchBarRef.value?.focus();
  });

  // Open settings from tray menu
  await listen("open-settings", () => {
    showSettings.value = true;
  });
});
</script>
