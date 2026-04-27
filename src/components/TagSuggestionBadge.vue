<template>
  <div v-if="suggestions.length > 0" class="flex items-center gap-1 mt-1">
    <span class="text-[10px] text-muted-foreground">{{ $t('smart.suggestedTags') }}</span>
    <button
      v-for="suggestion in suggestions"
      :key="suggestion.tag.id"
      class="inline-flex items-center rounded-full bg-primary/10 px-1.5 py-0 text-[10px] text-primary hover:bg-primary/20 transition-colors"
      :title="`${Math.round(suggestion.confidence * 100)}% confidence`"
      @click.stop="$emit('accept', suggestion.tag.id)"
    >
      {{ suggestion.tag.name }}
    </button>
    <button
      class="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
      @click.stop="$emit('dismiss')"
      title="Dismiss suggestions"
    >
      ✕
    </button>
  </div>
</template>

<script setup lang="ts">
import type { TagSuggestion } from '@/types';

defineProps<{
  suggestions: TagSuggestion[];
}>();

defineEmits<{
  accept: [tagId: number];
  dismiss: [];
}>();
</script>
