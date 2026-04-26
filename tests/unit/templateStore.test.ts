import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const sampleTemplate = {
  id: 1,
  name: 'Greeting',
  content: 'Hello {{name}}',
  category: 'general',
  is_favorite: false,
  use_count: 0,
  created_at: '2026-04-21 00:00:00',
  updated_at: '2026-04-21 00:00:00',
};

const sampleTemplate2 = {
  id: 2,
  name: 'Farewell',
  content: 'Goodbye {{name}}',
  category: 'work',
  is_favorite: false,
  use_count: 1,
  created_at: '2026-04-21 00:00:00',
  updated_at: '2026-04-21 00:00:00',
};

describe('useTemplateStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    errorSpy.mockClear();
  });

  // --- fetchTemplates ---

  describe('fetchTemplates', () => {
    it('fetches templates and sets isLoading', async () => {
      invoke.mockResolvedValue([sampleTemplate]);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      expect(store.isLoading).toBe(false);
      await store.fetchTemplates();

      expect(invoke).toHaveBeenCalledWith('get_templates', { category: null });
      expect(store.templates).toEqual([sampleTemplate]);
      expect(store.isLoading).toBe(false);
    });

    it('passes selectedCategory to invoke', async () => {
      invoke.mockResolvedValue([]);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.selectedCategory = 'work';

      await store.fetchTemplates();

      expect(invoke).toHaveBeenCalledWith('get_templates', { category: 'work' });
    });

    it('handles fetchTemplates error gracefully', async () => {
      invoke.mockRejectedValue(new Error('network error'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await store.fetchTemplates();

      expect(errorSpy).toHaveBeenCalled();
      expect(store.isLoading).toBe(false);
    });
  });

  // --- fetchCategories ---

  describe('fetchCategories', () => {
    it('fetches categories', async () => {
      invoke.mockResolvedValue(['general', 'work']);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await store.fetchCategories();

      expect(invoke).toHaveBeenCalledWith('get_template_categories');
      expect(store.categories).toEqual(['general', 'work']);
    });

    it('handles fetchCategories error gracefully', async () => {
      invoke.mockRejectedValue(new Error('fetch failed'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await store.fetchCategories();

      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- createTemplate ---

  describe('createTemplate', () => {
    it('creates template and refreshes categories', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'create_template') return sampleTemplate;
        if (cmd === 'get_template_categories') return ['general'];
        return [];
      });

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      const result = await store.createTemplate('Greeting', 'Hello {{name}}', 'general');

      expect(result).toEqual(sampleTemplate);
      expect(store.templates[0]).toEqual(sampleTemplate);
      expect(store.categories).toEqual(['general']);
    });

    it('throws and logs error on create failure', async () => {
      invoke.mockRejectedValue(new Error('create failed'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await expect(store.createTemplate('T', 'C')).rejects.toThrow('create failed');
      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- updateTemplate ---

  describe('updateTemplate', () => {
    it('updates existing template in-place and refreshes categories', async () => {
      const updated = { ...sampleTemplate, name: 'Updated Greeting' };

      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'update_template') return updated;
        if (cmd === 'get_template_categories') return ['general'];
        return [];
      });

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }];

      const result = await store.updateTemplate(1, 'Updated Greeting', 'Hello {{name}}', 'general');

      expect(result).toEqual(updated);
      expect(store.templates[0].name).toBe('Updated Greeting');
      expect(store.categories).toEqual(['general']);
    });

    it('does not crash when updating a template not in the local list', async () => {
      const updated = { ...sampleTemplate, id: 99, name: 'Ghost' };

      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'update_template') return updated;
        if (cmd === 'get_template_categories') return [];
        return [];
      });

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }];

      const result = await store.updateTemplate(99, 'Ghost', 'content');

      expect(result).toEqual(updated);
      // Original template unchanged since id 99 is not found
      expect(store.templates[0].id).toBe(1);
    });

    it('throws and logs error on update failure', async () => {
      invoke.mockRejectedValue(new Error('update failed'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await expect(store.updateTemplate(1, 'N', 'C')).rejects.toThrow('update failed');
      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- deleteTemplate ---

  describe('deleteTemplate', () => {
    it('deletes template from local list and refreshes categories', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'delete_template') return undefined;
        if (cmd === 'get_template_categories') return [];
        return [];
      });

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }, { ...sampleTemplate2 }];

      await store.deleteTemplate(1);

      expect(store.templates).toHaveLength(1);
      expect(store.templates[0].id).toBe(2);
    });

    it('throws and logs error on delete failure', async () => {
      invoke.mockRejectedValue(new Error('delete failed'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await expect(store.deleteTemplate(1)).rejects.toThrow('delete failed');
      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- useTemplate ---

  describe('useTemplate', () => {
    it('increments local use count after using template', async () => {
      invoke.mockResolvedValue('Hello Codex');

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate2 }];

      const output = await store.useTemplate(2, { name: 'Codex' });

      expect(output).toBe('Hello Codex');
      expect(store.templates[0].use_count).toBe(2);
    });

    it('does not increment use_count for non-existent template', async () => {
      invoke.mockResolvedValue('result');

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }];

      const output = await store.useTemplate(999, { name: 'X' });

      expect(output).toBe('result');
      expect(store.templates[0].use_count).toBe(0);
    });

    it('throws and logs error on use failure', async () => {
      invoke.mockRejectedValue(new Error('use failed'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await expect(store.useTemplate(1, {})).rejects.toThrow('use failed');
      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- getPlaceholders ---

  describe('getPlaceholders', () => {
    it('returns placeholders from invoke', async () => {
      invoke.mockResolvedValue(['name', 'age']);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      const result = await store.getPlaceholders('Hello {{name}}, age {{age}}');

      expect(result).toEqual(['name', 'age']);
    });

    it('returns empty array on invoke failure', async () => {
      invoke.mockRejectedValue(new Error('placeholder failure'));

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      await expect(store.getPlaceholders('Hello {{name}}')).resolves.toEqual([]);
      expect(errorSpy).toHaveBeenCalled();
    });
  });

  // --- setCategory ---

  describe('setCategory', () => {
    it('sets selectedCategory and triggers fetchTemplates', async () => {
      invoke.mockResolvedValue([sampleTemplate2]);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      store.setCategory('work');

      expect(store.selectedCategory).toBe('work');
      // Wait for the fire-and-forget fetchTemplates
      await vi.waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_templates', { category: 'work' });
      });
    });

    it('sets null category to show all templates', async () => {
      invoke.mockResolvedValue([sampleTemplate, sampleTemplate2]);

      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.selectedCategory = 'work';

      store.setCategory(null);

      expect(store.selectedCategory).toBeNull();
      await vi.waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_templates', { category: null });
      });
    });
  });

  // --- filteredTemplates ---

  describe('filteredTemplates', () => {
    it('returns all templates when no category is selected', async () => {
      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }, { ...sampleTemplate2 }];

      expect(store.filteredTemplates).toHaveLength(2);
    });

    it('filters templates by selected category', async () => {
      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }, { ...sampleTemplate2 }];
      store.selectedCategory = 'work';

      expect(store.filteredTemplates).toHaveLength(1);
      expect(store.filteredTemplates[0].category).toBe('work');
    });

    it('returns empty array when no templates match category', async () => {
      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();
      store.templates = [{ ...sampleTemplate }];
      store.selectedCategory = 'nonexistent';

      expect(store.filteredTemplates).toHaveLength(0);
    });
  });

  // --- clearSensitiveState ---

  describe('clearSensitiveState', () => {
    it('resets all state to defaults', async () => {
      const { useTemplateStore } = await import('@/stores/templateStore');
      const store = useTemplateStore();

      store.templates = [{ ...sampleTemplate }];
      store.categories = ['general'];
      store.selectedCategory = 'general';
      store.isLoading = true;

      store.clearSensitiveState();

      expect(store.templates).toEqual([]);
      expect(store.categories).toEqual([]);
      expect(store.selectedCategory).toBeNull();
      expect(store.isLoading).toBe(false);
    });
  });
});
