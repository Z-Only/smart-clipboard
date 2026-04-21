<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    @click.self="close"
  >
    <div
      class="bg-card border border-border rounded-lg shadow-lg w-[380px] max-h-[80vh] overflow-y-auto p-5"
    >
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-base font-semibold">{{ $t('statistics.title') }}</h2>
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

      <!-- Loading state -->
      <div v-if="loading" class="text-center text-muted-foreground py-8 text-sm">
        {{ $t('list.loading') }}
      </div>

      <div v-else-if="stats" class="flex flex-col gap-4">
        <!-- Summary cards -->
        <div class="grid grid-cols-3 gap-2">
          <div class="bg-muted/50 rounded-md p-2.5 text-center">
            <div class="text-lg font-bold text-foreground">{{ stats.total_entries }}</div>
            <div class="text-[10px] text-muted-foreground mt-0.5">
              {{ $t('statistics.totalEntries') }}
            </div>
          </div>
          <div class="bg-muted/50 rounded-md p-2.5 text-center">
            <div class="text-lg font-bold text-foreground">{{ stats.total_favorites }}</div>
            <div class="text-[10px] text-muted-foreground mt-0.5">
              {{ $t('statistics.totalFavorites') }}
            </div>
          </div>
          <div class="bg-muted/50 rounded-md p-2.5 text-center">
            <div class="text-lg font-bold text-foreground">
              {{ formatBytes(stats.storage_size_bytes) }}
            </div>
            <div class="text-[10px] text-muted-foreground mt-0.5">
              {{ $t('statistics.storageSize') }}
            </div>
          </div>
        </div>

        <Separator />

        <!-- Category breakdown -->
        <div>
          <h3 class="text-sm font-medium mb-2">{{ $t('statistics.categoryBreakdown') }}</h3>
          <div v-if="stats.entries_by_category.length === 0" class="text-xs text-muted-foreground">
            {{ $t('statistics.noData') }}
          </div>
          <div v-else class="flex flex-col gap-1.5">
            <div
              v-for="cat in stats.entries_by_category"
              :key="cat.category"
              class="flex items-center gap-2"
            >
              <span class="w-12 text-xs text-right text-muted-foreground truncate">{{
                cat.category
              }}</span>
              <div class="flex-1 h-4 bg-muted rounded overflow-hidden">
                <div
                  class="h-full bg-primary rounded transition-all"
                  :style="{ width: categoryBarWidth(cat.count) }"
                />
              </div>
              <span class="w-8 text-xs text-right text-muted-foreground tabular-nums">{{
                cat.count
              }}</span>
            </div>
          </div>
        </div>

        <Separator />

        <!-- Daily activity -->
        <div>
          <h3 class="text-sm font-medium mb-2">{{ $t('statistics.dailyActivity') }}</h3>
          <div v-if="stats.entries_by_day.length === 0" class="text-xs text-muted-foreground">
            {{ $t('statistics.noData') }}
          </div>
          <div v-else class="flex items-end gap-px h-20">
            <div
              v-for="day in reversedDays"
              :key="day.date"
              class="flex-1 bg-primary/70 rounded-t min-w-[3px] transition-all hover:bg-primary"
              :style="{ height: dayBarHeight(day.count) }"
              :title="`${day.date}: ${day.count}`"
            />
          </div>
          <div v-if="stats.entries_by_day.length > 0" class="flex justify-between mt-1">
            <span class="text-[10px] text-muted-foreground">{{ reversedDays[0]?.date }}</span>
            <span class="text-[10px] text-muted-foreground">{{
              reversedDays[reversedDays.length - 1]?.date
            }}</span>
          </div>
        </div>

        <Separator />

        <!-- Most used entries -->
        <div>
          <h3 class="text-sm font-medium mb-2">{{ $t('statistics.mostUsed') }}</h3>
          <div v-if="stats.most_used.length === 0" class="text-xs text-muted-foreground">
            {{ $t('statistics.noData') }}
          </div>
          <div v-else class="flex flex-col gap-1">
            <div
              v-for="(entry, idx) in stats.most_used"
              :key="entry.id"
              class="flex items-center gap-2 px-2 py-1 rounded hover:bg-muted/50"
            >
              <span class="text-xs text-muted-foreground w-4 text-right shrink-0"
                >{{ idx + 1 }}.</span
              >
              <span class="text-xs truncate flex-1">
                {{ entry.content_type === 'image' ? '[Image]' : truncateContent(entry.content) }}
              </span>
              <span class="text-[10px] text-muted-foreground shrink-0 tabular-nums">
                {{ $t('statistics.times', { n: entry.use_count }) }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Separator } from '@/components/ui/separator';

interface CategoryCount {
  category: string;
  count: number;
}

interface DayCount {
  date: string;
  count: number;
}

interface ClipboardEntryBrief {
  id: number;
  content: string;
  content_type: string;
  use_count: number;
}

interface Statistics {
  total_entries: number;
  total_favorites: number;
  entries_by_category: CategoryCount[];
  entries_by_day: DayCount[];
  most_used: ClipboardEntryBrief[];
  storage_size_bytes: number;
}

const props = defineProps<{ isOpen: boolean }>();
const emit = defineEmits<{ close: [] }>();

const stats = ref<Statistics | null>(null);
const loading = ref(false);

watch(
  () => props.isOpen,
  async (open) => {
    if (open) {
      await loadStatistics();
    } else {
      stats.value = null;
      loading.value = false;
    }
  },
);

async function loadStatistics() {
  loading.value = true;
  try {
    stats.value = await invoke<Statistics>('get_statistics');
  } catch (e) {
    console.error('Failed to load statistics:', e);
  } finally {
    loading.value = false;
  }
}

const maxCategoryCount = computed(() => {
  if (!stats.value || stats.value.entries_by_category.length === 0) return 1;
  return Math.max(...stats.value.entries_by_category.map((c) => c.count));
});

function categoryBarWidth(count: number): string {
  const pct = (count / maxCategoryCount.value) * 100;
  return `${Math.max(pct, 2)}%`;
}

const reversedDays = computed(() => {
  if (!stats.value) return [];
  return [...stats.value.entries_by_day].reverse();
});

const maxDayCount = computed(() => {
  if (!stats.value || stats.value.entries_by_day.length === 0) return 1;
  return Math.max(...stats.value.entries_by_day.map((d) => d.count));
});

function dayBarHeight(count: number): string {
  const pct = (count / maxDayCount.value) * 100;
  return `${Math.max(pct, 3)}%`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / 1048576).toFixed(1) + ' MB';
}

function truncateContent(content: string): string {
  const single = content.replace(/\s+/g, ' ').trim();
  return single.length > 50 ? single.slice(0, 50) + '...' : single;
}

function close() {
  emit('close');
}
</script>
