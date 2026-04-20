<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" @click.self="$emit('close')">
    <div class="bg-background rounded-lg shadow-xl w-[600px] max-h-[80vh] flex flex-col border border-border">
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 class="text-base font-semibold">{{ $t('templates.title') }}</h2>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1.5 text-xs rounded-md bg-primary text-primary-foreground hover:bg-primary/90"
            @click="showEditor = true; editingTemplate = null"
          >
            {{ $t('templates.create') }}
          </button>
          <button class="p-1 rounded hover:bg-accent" @click="$emit('close')">
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Category filter -->
      <div v-if="store.categories.length > 0" class="flex items-center gap-1 px-4 py-2 border-b border-border overflow-x-auto">
        <button
          class="px-2 py-1 text-xs rounded-md whitespace-nowrap"
          :class="store.selectedCategory === null ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-accent'"
          @click="store.setCategory(null)"
        >{{ $t('categories.all') }}</button>
        <button
          v-for="cat in store.categories"
          :key="cat"
          class="px-2 py-1 text-xs rounded-md whitespace-nowrap"
          :class="store.selectedCategory === cat ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-accent'"
          @click="store.setCategory(cat)"
        >{{ cat }}</button>
      </div>

      <!-- Template list -->
      <div class="flex-1 overflow-y-auto p-4">
        <div v-if="store.templates.length === 0" class="text-center text-muted-foreground py-8">
          {{ $t('templates.noTemplates') }}
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="tmpl in store.templates"
            :key="tmpl.id ?? tmpl.name"
            class="p-3 rounded-md border border-border hover:border-primary/50 transition-colors group"
          >
            <div class="flex items-center justify-between mb-1">
              <span class="font-medium text-sm">{{ tmpl.name }}</span>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  class="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground"
                  :title="$t('templates.use')"
                  @click="handleUseTemplate(tmpl)"
                >
                  <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
                    <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
                  </svg>
                </button>
                <button
                  class="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground"
                  :title="$t('templates.edit')"
                  @click="editingTemplate = tmpl; showEditor = true"
                >
                  <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                  </svg>
                </button>
                <button
                  class="p-1 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive"
                  :title="$t('templates.delete')"
                  @click="handleDelete(tmpl.id!)"
                >
                  <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  </svg>
                </button>
              </div>
            </div>
            <div class="text-xs text-muted-foreground line-clamp-2 font-mono">{{ tmpl.content }}</div>
            <div class="flex items-center gap-2 mt-2 text-[10px] text-muted-foreground">
              <span class="px-1.5 py-0.5 rounded bg-muted">{{ tmpl.category }}</span>
              <span v-if="tmpl.use_count > 0">{{ $t('statistics.times', { n: tmpl.use_count }) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Editor dialog -->
    <TemplateEditor
      v-if="showEditor"
      :template="editingTemplate"
      @close="showEditor = false"
      @saved="handleSaved"
    />

    <!-- Fill dialog -->
    <TemplateFillDialog
      v-if="showFillDialog"
      :template="fillingTemplate!"
      :placeholders="fillPlaceholders"
      @close="showFillDialog = false"
      @submit="handleFillSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useTemplateStore } from "@/stores/templateStore";
import TemplateEditor from "./TemplateEditor.vue";
import TemplateFillDialog from "./TemplateFillDialog.vue";
import type { Template } from "@/types";

defineProps<{ isOpen: boolean }>();
defineEmits<{ close: [] }>();

const store = useTemplateStore();
const showEditor = ref(false);
const editingTemplate = ref<Template | null>(null);
const showFillDialog = ref(false);
const fillingTemplate = ref<Template | null>(null);
const fillPlaceholders = ref<string[]>([]);

onMounted(() => {
  store.fetchTemplates();
  store.fetchCategories();
});

async function handleUseTemplate(tmpl: Template) {
  const placeholders = await store.getPlaceholders(tmpl.content);
  if (placeholders.length === 0) {
    // No placeholders, just copy directly
    await store.useTemplate(tmpl.id!, {});
  } else {
    fillingTemplate.value = tmpl;
    fillPlaceholders.value = placeholders;
    showFillDialog.value = true;
  }
}

async function handleFillSubmit(values: Record<string, string>) {
  if (fillingTemplate.value) {
    await store.useTemplate(fillingTemplate.value.id!, values);
  }
  showFillDialog.value = false;
}

function handleSaved() {
  showEditor.value = false;
  store.fetchTemplates();
  store.fetchCategories();
}

async function handleDelete(id: number) {
  await store.deleteTemplate(id);
}
</script>
