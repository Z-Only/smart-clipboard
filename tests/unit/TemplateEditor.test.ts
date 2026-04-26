import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import TemplateEditor from '@/components/TemplateEditor.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      templates: {
        create: 'Create Template',
        edit: 'Edit Template',
        name: 'Name',
        namePlaceholder: 'Template name',
        category: 'Category',
        content: 'Content',
        contentPlaceholder: 'Template content',
        placeholderHint: 'Use {name} for placeholders',
        detectedPlaceholders: 'Detected placeholders',
        cancel: 'Cancel',
        save: 'Save',
      },
    },
  },
});

describe('TemplateEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it('renders create mode when template is null', () => {
    const wrapper = mount(TemplateEditor, {
      props: { template: null },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').text()).toBe('Create Template');
  });

  it('renders edit mode when template is provided', () => {
    const wrapper = mount(TemplateEditor, {
      props: {
        template: {
          id: 1,
          name: 'Test',
          content: 'Hello {{name}}',
          category: 'general',
          is_favorite: false,
          use_count: 0,
          created_at: '2026-04-21',
          updated_at: '2026-04-21',
        },
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').text()).toBe('Edit Template');
  });

  it('disables save button when form is invalid', () => {
    const wrapper = mount(TemplateEditor, {
      props: { template: null },
      global: { plugins: [i18n] },
    });
    const saveBtn = wrapper.findAll('button').find((b) => b.text() === 'Save');
    expect(saveBtn!.attributes('disabled')).toBeDefined();
  });

  it('emits close when cancel button is clicked', async () => {
    const wrapper = mount(TemplateEditor, {
      props: { template: null },
      global: { plugins: [i18n] },
    });
    const cancelBtn = wrapper.findAll('button').find((b) => b.text() === 'Cancel');
    await cancelBtn!.trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });
});
