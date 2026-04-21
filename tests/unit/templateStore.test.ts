import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('useTemplateStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
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
});
