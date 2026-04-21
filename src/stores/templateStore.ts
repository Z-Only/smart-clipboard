import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Template } from '@/types';

export const useTemplateStore = defineStore('template', () => {
  const templates = ref<Template[]>([]);
  const categories = ref<string[]>([]);
  const selectedCategory = ref<string | null>(null);
  const isLoading = ref(false);

  const filteredTemplates = computed(() => {
    if (!selectedCategory.value) return templates.value;
    return templates.value.filter((t) => t.category === selectedCategory.value);
  });

  async function fetchTemplates() {
    isLoading.value = true;
    try {
      templates.value = await invoke<Template[]>('get_templates', {
        category: selectedCategory.value,
      });
    } catch (e) {
      console.error('Failed to fetch templates:', e);
    } finally {
      isLoading.value = false;
    }
  }

  async function fetchCategories() {
    try {
      categories.value = await invoke<string[]>('get_template_categories');
    } catch (e) {
      console.error('Failed to fetch template categories:', e);
    }
  }

  async function createTemplate(name: string, content: string, category?: string) {
    try {
      const template = await invoke<Template>('create_template', { name, content, category });
      templates.value.unshift(template);
      await fetchCategories();
      return template;
    } catch (e) {
      console.error('Failed to create template:', e);
      throw e;
    }
  }

  async function updateTemplate(id: number, name: string, content: string, category?: string) {
    try {
      const template = await invoke<Template>('update_template', { id, name, content, category });
      const idx = templates.value.findIndex((t) => t.id === id);
      if (idx !== -1) templates.value[idx] = template;
      await fetchCategories();
      return template;
    } catch (e) {
      console.error('Failed to update template:', e);
      throw e;
    }
  }

  async function deleteTemplate(id: number) {
    try {
      await invoke('delete_template', { id });
      templates.value = templates.value.filter((t) => t.id !== id);
      await fetchCategories();
    } catch (e) {
      console.error('Failed to delete template:', e);
      throw e;
    }
  }

  async function useTemplate(id: number, values: Record<string, string>): Promise<string> {
    try {
      const result = await invoke<string>('use_template', { id, values });
      // Increment local use_count
      const tmpl = templates.value.find((t) => t.id === id);
      if (tmpl) tmpl.use_count++;
      return result;
    } catch (e) {
      console.error('Failed to use template:', e);
      throw e;
    }
  }

  async function getPlaceholders(content: string): Promise<string[]> {
    try {
      return await invoke<string[]>('get_template_placeholders', { content });
    } catch (e) {
      console.error('Failed to get placeholders:', e);
      return [];
    }
  }

  function clearSensitiveState() {
    templates.value = [];
    categories.value = [];
    selectedCategory.value = null;
    isLoading.value = false;
  }

  function setCategory(category: string | null) {
    selectedCategory.value = category;
    fetchTemplates();
  }

  return {
    templates,
    categories,
    selectedCategory,
    isLoading,
    filteredTemplates,
    fetchTemplates,
    fetchCategories,
    createTemplate,
    updateTemplate,
    deleteTemplate,
    useTemplate,
    getPlaceholders,
    setCategory,
    clearSensitiveState,
  };
});
