<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex justify-end" @click.self="$emit('close')">
    <div
      class="w-80 bg-background border-l border-border shadow-xl flex flex-col h-full animate-in slide-in-from-right"
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-border">
        <h2 class="text-sm font-semibold">{{ $t('smart.groups') }}</h2>
        <div class="flex items-center gap-1">
          <button
            class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
            :disabled="isReclustering"
            :title="$t('smart.refresh')"
            @click="handleRefresh"
          >
            <svg
              class="h-4 w-4"
              :class="{ 'animate-spin': isReclustering }"
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
          </button>
          <button
            class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
            @click="$emit('close')"
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
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-3">
        <p v-if="isReclustering" class="text-sm text-muted-foreground text-center py-8">
          {{ $t('smart.reclustering') }}
        </p>
        <p v-else-if="clusters.length === 0" class="text-sm text-muted-foreground text-center py-8">
          {{ $t('smart.noGroups') }}
        </p>
        <div v-else class="space-y-1">
          <button
            v-for="cluster in clusters"
            :key="cluster.id"
            class="w-full flex items-center justify-between p-2.5 rounded-md hover:bg-accent transition-colors text-left"
            @click="handleClusterClick(cluster.id)"
          >
            <div class="flex items-center gap-2 min-w-0">
              <span class="text-base">📁</span>
              <span class="text-sm truncate">{{ cluster.label }}</span>
            </div>
            <span class="text-xs text-muted-foreground shrink-0 ml-2">
              {{ $t('smart.entries', { count: cluster.entry_count }) }}
            </span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useSmartStore } from '@/stores/smartStore';

const props = defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const smartStore = useSmartStore();
const { clusters, isReclustering } = storeToRefs(smartStore);

async function handleRefresh() {
  await smartStore.triggerRecluster();
}

function handleClusterClick(clusterId: number) {
  // For now, just log. In a future iteration, this would navigate to filtered view.
  console.log('Cluster clicked:', clusterId);
}

watch(
  () => props.isOpen,
  async (open) => {
    if (open) {
      await smartStore.fetchClusters();
    }
  },
);
</script>
