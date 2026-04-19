<template>
  <div
    class="group flex items-start gap-3 p-3 rounded-lg cursor-pointer transition-colors"
    :class="[
      isSelected
        ? 'bg-accent'
        : 'hover:bg-accent/50',
    ]"
    @click="$emit('select', entry.id)"
  >
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2 mb-1">
        <Badge variant="secondary" class="text-[10px] px-1.5 py-0 shrink-0">
          {{ categoryLabel }}
        </Badge>
        <span class="text-xs text-muted-foreground truncate">
          {{ relativeTime }}
        </span>
        <span v-if="entry.source_app" class="text-xs text-muted-foreground truncate">
          {{ entry.source_app }}
        </span>
      </div>
      <div class="text-sm leading-snug break-all line-clamp-3" :class="isCodeLike ? 'font-mono text-xs' : ''">
        {{ truncatedContent }}
      </div>
    </div>
    <div class="flex flex-col gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
      <button
        class="p-1 rounded hover:bg-background/80 text-muted-foreground"
        :class="entry.is_favorite ? 'text-yellow-500 !opacity-100' : ''"
        :style="entry.is_favorite ? 'opacity: 1' : ''"
        @click.stop="$emit('toggleFavorite', entry.id)"
        title="Toggle favorite"
      >
        <svg class="h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
          :fill="entry.is_favorite ? 'currentColor' : 'none'"
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
        </svg>
      </button>
      <button
        class="p-1 rounded hover:bg-destructive/20 text-muted-foreground hover:text-destructive"
        @click.stop="$emit('delete', entry.id)"
        title="Delete"
      >
        <svg class="h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18" /><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
          <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Badge } from "@/components/ui/badge";
import type { ClipboardEntry } from "@/types";

const props = defineProps<{
  entry: ClipboardEntry;
  isSelected?: boolean;
}>();

defineEmits<{
  select: [id: number];
  toggleFavorite: [id: number];
  delete: [id: number];
}>();

const CATEGORY_LABELS: Record<string, string> = {
  url: "URL",
  email: "Email",
  color: "Color",
  filepath: "File",
  json: "JSON",
  xml: "XML",
  code: "Code",
  phone: "Phone",
  address: "Addr",
  text: "Text",
};

const categoryLabel = computed(
  () => CATEGORY_LABELS[props.entry.category] || props.entry.category
);

const isCodeLike = computed(() =>
  ["code", "json", "xml"].includes(props.entry.category)
);

const truncatedContent = computed(() => {
  const content = props.entry.content;
  if (content.length > 200) {
    return content.slice(0, 200) + "...";
  }
  return content;
});

const relativeTime = computed(() => {
  const now = new Date();
  const created = new Date(props.entry.created_at);
  const diffMs = now.getTime() - created.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return created.toLocaleDateString();
});
</script>
