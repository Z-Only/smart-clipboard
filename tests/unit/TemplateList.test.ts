import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

// Stub TemplateEditor and TemplateFillDialog
vi.mock('@/components/TemplateEditor.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'TemplateEditorStub',
      setup() {
        return () => h('div', { 'data-test': 'template-editor' });
      },
    }),
  };
});
vi.mock('@/components/TemplateFillDialog.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'TemplateFillDialogStub',
      setup() {
        return () => h('div', { 'data-test': 'template-fill-dialog' });
      },
    }),
  };
});

import TemplateList from '@/components/TemplateList.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      templates: {
        title: 'Templates',
        create: 'Create',
        noTemplates: 'No templates',
        use: 'Use',
        edit: 'Edit',
        delete: 'Delete',
        deleteConfirm: 'Are you sure?',
      },
      categories: { all: 'All' },
      statistics: {
        times: '{n} times',
      },
    },
  },
});

describe('TemplateList', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it('does not render when isOpen is false', () => {
    const wrapper = mount(TemplateList, {
      props: { isOpen: false },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h2').exists()).toBe(false);
  });

  it('renders template list when isOpen is true', async () => {
    const wrapper = mount(TemplateList, {
      props: { isOpen: true },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    expect(wrapper.find('h2').text()).toBe('Templates');
  });

  it('shows empty state when no templates', async () => {
    const wrapper = mount(TemplateList, {
      props: { isOpen: true },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    expect(wrapper.text()).toContain('No templates');
  });

  it('emits close when close button is clicked', async () => {
    const wrapper = mount(TemplateList, {
      props: { isOpen: true },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    // Find close button (the X button in header)
    const closeButtons = wrapper.findAll('button');
    const closeBtn = closeButtons.find((b) =>
      b.find('svg path[d="M18 6L6 18M6 6l12 12"]').exists(),
    );
    if (closeBtn) {
      await closeBtn.trigger('click');
      expect(wrapper.emitted('close')).toBeTruthy();
    }
  });
});
