<template>
  <div class="fixed inset-0 z-[70] flex items-center justify-center bg-black/50" @click.self="$emit('close')">
    <div class="bg-background rounded-lg shadow-xl w-[450px] max-h-[70vh] flex flex-col border border-border">
      <div class="flex items-center justify-between px-4 py-3 border-b border-border">
        <h3 class="text-sm font-semibold">{{ $t('templates.fillPlaceholders') }}</h3>
        <button class="p-1 rounded hover:bg-accent" @click="$emit('close')">
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-4 space-y-3">
        <!-- Placeholder inputs -->
        <div v-for="ph in placeholders" :key="ph">
          <label class="block text-xs font-medium text-muted-foreground mb-1">{{ ph }}</label>
          <input
            v-model="values[ph]"
            type="text"
            class="w-full px-3 py-2 text-sm rounded-md border border-input bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            :placeholder="`Enter ${ph}...`"
            @keydown.enter="handleSubmit"
          />
        </div>

        <!-- Preview -->
        <div class="mt-4">
          <label class="block text-xs font-medium text-muted-foreground mb-1">{{ $t('templates.preview') }}</label>
          <div class="p-3 rounded-md bg-muted text-sm font-mono whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
            {{ renderedPreview }}
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
          class="px-3 py-1.5 text-xs rounded-md bg-primary text-primary-foreground hover:bg-primary/90"
          @click="handleSubmit"
        >{{ $t('templates.copyToClipboard') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import type { Template } from "@/types";

const props = defineProps<{
  template: Template;
  placeholders: string[];
}>();

const emit = defineEmits<{
  close: [];
  submit: [values: Record<string, string>];
}>();

const values = ref<Record<string, string>>({});

// Initialize empty values for each placeholder
props.placeholders.forEach(ph => {
  values.value[ph] = "";
});

const renderedPreview = computed(() => {
  let result = props.template.content;
  for (const [key, val] of Object.entries(values.value)) {
    const regex = new RegExp(`\\{\\{${key}\\}\\}`, "g");
    result = result.replace(regex, val || `{{${key}}}`);
  }
  return result;
});

function handleSubmit() {
  emit("submit", { ...values.value });
}
</script>
