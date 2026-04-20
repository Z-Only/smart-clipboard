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
        <Badge v-if="entry.is_sensitive" variant="destructive" class="text-[10px] px-1.5 py-0 shrink-0 flex items-center gap-0.5">
          <svg class="h-2.5 w-2.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          {{ t('entry.sensitive') }}
        </Badge>
        <span v-if="expiryText" class="text-[10px] text-destructive/80 shrink-0">
          {{ expiryText }}
        </span>
        <span class="text-xs text-muted-foreground truncate">
          {{ relativeTime }}
        </span>
        <span v-if="entry.source_app" class="text-xs text-muted-foreground truncate">
          {{ entry.source_app }}
        </span>
      </div>
      <!-- Image preview for image entries -->
      <div v-if="isImage" class="flex items-center gap-2">
        <img
          :src="imageAssetUrl"
          alt="Clipboard image"
          class="rounded border border-border object-cover"
          style="max-height: 64px; max-width: 120px;"
          loading="lazy"
        />
        <span class="text-xs text-muted-foreground">
          {{ t('entry.categoryLabels.image') }}
        </span>
      </div>
      <!-- Text preview for non-image entries -->
      <div v-else class="text-sm leading-snug break-all line-clamp-3" :class="isCodeLike ? 'font-mono text-xs' : ''">
        {{ truncatedContent }}
      </div>
      <div v-if="entryTags.length > 0" class="flex flex-wrap gap-1 mt-1.5">
        <span
          v-for="tag in entryTags"
          :key="tag.id"
          class="inline-flex items-center px-1.5 py-0 text-[10px] rounded-full bg-primary/15 text-primary border border-primary/20"
        >
          {{ tag.name }}
        </span>
      </div>
    </div>
    <div class="flex flex-col gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
      <button
        class="p-1 rounded hover:bg-background/80 text-muted-foreground"
        :class="entry.is_favorite ? 'text-yellow-500 !opacity-100' : ''"
        :style="entry.is_favorite ? 'opacity: 1' : ''"
        @click.stop="$emit('toggleFavorite', entry.id)"
        :title="t('entry.toggleFavorite')"
      >
        <svg class="h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
          :fill="entry.is_favorite ? 'currentColor' : 'none'"
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
        </svg>
      </button>
      <TransformMenu v-if="!isImage" :content="entry.content" :category="entry.category" />
      <TagPicker :entry-id="entry.id" @tags-changed="onTagsChanged" />
      <button
        class="p-1 rounded hover:bg-destructive/20 text-muted-foreground hover:text-destructive"
        @click.stop="$emit('delete', entry.id)"
        :title="t('entry.delete')"
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
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { Badge } from "@/components/ui/badge";
import TransformMenu from "@/components/TransformMenu.vue";
import TagPicker from "@/components/TagPicker.vue";
import type { ClipboardEntry, Tag } from "@/types";

const { t } = useI18n();

const props = defineProps<{
  entry: ClipboardEntry;
  isSelected?: boolean;
}>();

defineEmits<{
  select: [id: number];
  toggleFavorite: [id: number];
  delete: [id: number];
}>();

const entryTags = ref<Tag[]>([]);

async function loadEntryTags() {
  try {
    entryTags.value = await invoke<Tag[]>("get_entry_tags", { entryId: props.entry.id });
  } catch (e) {
    // silently ignore - tags are optional display
  }
}

function onTagsChanged(tags: Tag[]) {
  entryTags.value = tags;
}

// Load tags when entry changes
watch(
  () => props.entry.id,
  () => loadEntryTags(),
  { immediate: true }
);

const isImage = computed(() => props.entry.content_type === "image");

const imageAssetUrl = computed(() => {
  if (!isImage.value) return "";
  return convertFileSrc(props.entry.content);
});

const categoryLabel = computed(() => {
  const key = `entry.categoryLabels.${props.entry.category}`;
  const translated = t(key);
  return translated !== key ? translated : props.entry.category;
});

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

const expiryText = computed(() => {
  if (!props.entry.expires_at) return null;
  const now = new Date();
  const expiresAt = new Date(props.entry.expires_at);
  const diffMs = expiresAt.getTime() - now.getTime();
  if (diffMs <= 0) return null;
  const diffMin = Math.ceil(diffMs / 60000);
  return t("entry.expiresIn", { n: diffMin });
});

const relativeTime = computed(() => {
  const now = new Date();
  const created = new Date(props.entry.created_at);
  const diffMs = now.getTime() - created.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return t("entry.justNow");
  if (diffMin < 60) return t("entry.minutesAgo", { n: diffMin });
  if (diffHour < 24) return t("entry.hoursAgo", { n: diffHour });
  if (diffDay < 7) return t("entry.daysAgo", { n: diffDay });
  return created.toLocaleDateString();
});
</script>
