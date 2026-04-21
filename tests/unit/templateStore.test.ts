import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('useTemplateStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    errorSpy.mockClear();
  });

  it('creates template and refreshes categories', async () => {
    const created = {
      id: 1,
      name: 'Greeting',
      content: 'Hello {{name}}',
      category: 'general',
      is_favorite: false,
      use_count: 0,
      created_at: '2026-04-21 00:00:00',
      updated_at: '2026-04-21 00:00:00',
    };

    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'create_template') return created;
      if (cmd === 'get_template_categories') return ['general'];
      return [];
    });

    const { useTemplateStore } = await import('@/stores/templateStore');
    const store = useTemplateStore();

    const result = await store.createTemplate('Greeting', 'Hello {{name}}', 'general');

    expect(result).toEqual(created);
    expect(store.templates[0]).toEqual(created);
    expect(store.categories).toEqual(['general']);
  });

  it('increments local use count after using template', async () => {
    invoke.mockResolvedValue('Hello Codex');

    const { useTemplateStore } = await import('@/stores/templateStore');
    const store = useTemplateStore();
    store.templates = [
      {
        id: 2,
        name: 'Greeting',
        content: 'Hello {{name}}',
        category: 'general',
        is_favorite: false,
        use_count: 1,
        created_at: '2026-04-21 00:00:00',
        updated_at: '2026-04-21 00:00:00',
      },
    ];

    const output = await store.useTemplate(2, { name: 'Codex' });

    expect(output).toBe('Hello Codex');
    expect(store.templates[0].use_count).toBe(2);
  });

  it('returns empty placeholders on invoke failure', async () => {
    invoke.mockRejectedValue(new Error('placeholder failure'));

    const { useTemplateStore } = await import('@/stores/templateStore');
    const store = useTemplateStore();

    await expect(store.getPlaceholders('Hello {{name}}')).resolves.toEqual([]);
    expect(errorSpy).toHaveBeenCalled();
  });

  it('clears sensitive state', async () => {
    const { useTemplateStore } = await import('@/stores/templateStore');
    const store = useTemplateStore();

    store.templates = [
      {
        id: 1,
        name: 'T',
        content: 'C',
        category: 'general',
        is_favorite: false,
        use_count: 0,
        created_at: '2026-04-21 00:00:00',
        updated_at: '2026-04-21 00:00:00',
      },
    ];
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
