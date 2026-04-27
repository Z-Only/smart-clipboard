<template>
  <div v-if="entries.length > 0" class="border-t border-border pt-2 mt-2">
    <button
      class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors w-full"
      @click="expanded = !expanded"
    >
      <svg
        class="h-3 w-3 transition-transform"
        :class="{ 'rotate-90': expanded }"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <polyline points="9 18 15 12 9 6" />
      </svg>
      {{ $t('smart.related', { count: entries.length }) }}
    </button>
    <div v-if="expanded" class="mt-1.5 space-y-1">
      <div
        v-for="related in entries"
        :key="related.entry.id"
        class="flex items-center gap-2 p-1.5 rounded text-xs hover:bg-accent/50 cursor-pointer transition-colors"
        @click="$emit('select', related.entry.id)"
      >
        <span class="text-muted-foreground shrink-0 w-8 text-right font-mono">
          {{ Math.round(related.score * 100) }}%
        </span>
        <span class="truncate">{{ related.entry.content }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import type { RelatedEntry } from '@/types';

defineProps<{
  entries: RelatedEntry[];
}>();

defineEmits<{
  select: [entryId: number];
}>();

const expanded = ref(false);
</script>
