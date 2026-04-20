<template>
  <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50" @click.self="$emit('close')">
    <div class="bg-background rounded-lg shadow-xl w-[500px] max-h-[70vh] flex flex-col border border-border">
      <div class="flex items-center justify-between px-4 py-3 border-b border-border">
        <h3 class="text-sm font-semibold">
          {{ template ? $t('templates.edit') : $t('templates.create') }}
        </h3>
        <button class="p-1 rounded hover:bg-accent" @click="$emit('close')">
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <!-- Name -->
        <div>
          <label class="block text-xs font-medium text-muted-foreground mb-1">{{ $t('templates.name') }}</label>
          <input
            v-model="form.name"
            type="text"
            class="w-full px-3 py-2 text-sm rounded-md border border-input bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            :placeholder="$t('templates.namePlaceholder')"
          />
        </div>

        <!-- Category -->
        <div>
          <label class="block text-xs font-medium text-muted-foreground mb-1">{{ $t('templates.category') }}</label>
          <input
            v-model="form.category"
            type="text"
            class="w-full px-3 py-2 text-sm rounded-md border border-input bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            placeholder="general"
          />
        </div>

        <!-- Content -->
        <div>
          <label class="block text-xs font-medium text-muted-foreground mb-1">
            {{ $t('templates.content') }}
            <span class="text-[10px] ml-1 opacity-70">{{ $t('templates.placeholderHint') }}</span>
          </label>
          <textarea
            v-model="form.content"
            rows="6"
            class="w-full px-3 py-2 text-sm rounded-md border border-input bg-background font-mono focus:outline-none focus:ring-1 focus:ring-ring resize-none"
            :placeholder="$t('templates.contentPlaceholder')"
          />
        </div>

        <!-- Placeholders preview -->
        <div v-if="placeholders.length > 0">
          <label class="block text-xs font-medium text-muted-foreground mb-1">{{ $t('templates.detectedPlaceholders') }}</label>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="ph in placeholders"
              :key="ph"
              class="px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary"
            >{{ ph }}</span>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex justify-end gap-2 px-4 py-3 border-t border-border">
        <button
          class="px-3 py-1.5 text-xs rounded-md border border-input hover:bg-accent"
          @click="$emit('close')"
        >{{ $t('templates.cancel') }}</button>
        <button
          class="px-3 py-1.5 text-xs rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          :disabled="!isValid"
          @click="handleSave"
        >{{ $t('templates.save') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useTemplateStore } from "@/stores/templateStore";
import type { Template } from "@/types";

const props = defineProps<{ template: Template | null }>();
const emit = defineEmits<{ close: []; saved: [] }>();

const store = useTemplateStore();

const form = ref({
  name: "",
  content: "",
  category: "general",
});

const placeholders = ref<string[]>([]);

const isValid = computed(() => form.value.name.trim() !== "" && form.value.content.trim() !== "");

// Initialize form if editing
onMounted(() => {
  if (props.template) {
    form.value = {
      name: props.template.name,
      content: props.template.content,
      category: props.template.category,
    };
  }
});

// Extract placeholders as user types
watch(() => form.value.content, async (content) => {
  if (content.trim()) {
    placeholders.value = await store.getPlaceholders(content);
  } else {
    placeholders.value = [];
  }
}, { immediate: true });

async function handleSave() {
  try {
    if (props.template?.id) {
      await store.updateTemplate(
        props.template.id,
        form.value.name.trim(),
        form.value.content,
        form.value.category.trim() || undefined
      );
    } else {
      await store.createTemplate(
        form.value.name.trim(),
        form.value.content,
        form.value.category.trim() || undefined
      );
    }
    emit("saved");
  } catch (e) {
    console.error("Failed to save template:", e);
  }
}
</script>
