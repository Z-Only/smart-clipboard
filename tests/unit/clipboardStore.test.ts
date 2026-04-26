import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { Tag } from '@/types';
import type { ClipboardEntry } from '@/types';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

// clipboardStore 依赖 i18n，需要 mock
vi.mock('@/i18n', () => ({
  default: {
    global: {
      t: (key: string) => key,
      locale: { value: 'en' },
    },
  },
}));

const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

function makeEntry(overrides: Partial<ClipboardEntry> = {}): ClipboardEntry {
  return {
    id: 1,
    content: 'test content',
    content_type: 'text',
    category: 'text',
    hash: 'abc123',
    source_app: null,
    is_favorite: false,
    is_sensitive: false,
    use_count: 1,
    created_at: '2026-04-26 12:00:00',
    updated_at: '2026-04-26 12:00:00',
    expires_at: null,
    ...overrides,
  };
}

describe('useClipboardStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    errorSpy.mockClear();
  });

  describe('虚拟滚动高度管理', () => {
    it('setVirtualItemHeight 存储高度值，getVirtualItemHeight 能取回', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.setVirtualItemHeight('item-1', 100);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);
    });

    it('setVirtualItemHeight 忽略无效值（非有限数）', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.setVirtualItemHeight('item-1', 100);
      store.setVirtualItemHeight('item-1', NaN);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);

      store.setVirtualItemHeight('item-1', Infinity);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);

      store.setVirtualItemHeight('item-1', -Infinity);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);
    });

    it('setVirtualItemHeight 忽略无效值（<=0）', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.setVirtualItemHeight('item-1', 100);
      store.setVirtualItemHeight('item-1', 0);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);

      store.setVirtualItemHeight('item-1', -10);
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);
    });

    it('setVirtualItemHeight 相同值不触发更新', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.setVirtualItemHeight('item-1', 100);
      // 设置相同值，不应该触发更新
      store.setVirtualItemHeight('item-1', 100);
      // 验证值保持不变
      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);
    });

    it('getVirtualItemHeight 对未设置的 key 返回 fallback', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      expect(store.getVirtualItemHeight('non-existent', 50)).toBe(50);
      expect(store.getVirtualItemHeight('another-item', 100)).toBe(100);
    });

    it('clearVirtualItemHeights 清空所有高度', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.setVirtualItemHeight('item-1', 100);
      store.setVirtualItemHeight('item-2', 200);
      store.setVirtualItemHeight('item-3', 150);

      expect(store.getVirtualItemHeight('item-1', 50)).toBe(100);
      expect(store.getVirtualItemHeight('item-2', 50)).toBe(200);
      expect(store.getVirtualItemHeight('item-3', 50)).toBe(150);

      store.clearVirtualItemHeights();

      expect(store.getVirtualItemHeight('item-1', 50)).toBe(50);
      expect(store.getVirtualItemHeight('item-2', 50)).toBe(50);
      expect(store.getVirtualItemHeight('item-3', 50)).toBe(50);
    });
  });

  describe('标签缓存', () => {
    it('setEntryTags / getEntryTags 基本读写', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const tags: Tag[] = [
        { id: 1, name: 'work' },
        { id: 2, name: 'important' },
      ];

      store.setEntryTags(1, tags);
      expect(store.getEntryTags(1)).toEqual(tags);
    });

    it('getEntryTags 对未缓存 ID 返回空数组', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      expect(store.getEntryTags(999)).toEqual([]);
      expect(store.getEntryTags(0)).toEqual([]);
    });

    it('clearEntryTagsCache 清空标签缓存', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const tags1: Tag[] = [{ id: 1, name: 'work' }];
      const tags2: Tag[] = [{ id: 2, name: 'personal' }];

      store.setEntryTags(1, tags1);
      store.setEntryTags(2, tags2);

      expect(store.getEntryTags(1)).toEqual(tags1);
      expect(store.getEntryTags(2)).toEqual(tags2);

      store.clearEntryTagsCache();

      expect(store.getEntryTags(1)).toEqual([]);
      expect(store.getEntryTags(2)).toEqual([]);
    });

    it('batchLoadEntryTags 只加载未缓存的 ID，跳过已缓存的', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const cachedTags: Tag[] = [{ id: 1, name: 'cached' }];
      const uncachedTags: Tag[] = [{ id: 2, name: 'uncached' }];

      // 先缓存 ID 1 的标签
      store.setEntryTags(1, cachedTags);

      // mock invoke 返回不同 ID 的标签
      invoke.mockImplementation((cmd: string, { entryId }: { entryId: number }) => {
        if (entryId === 2) return Promise.resolve(uncachedTags);
        return Promise.resolve([]);
      });

      // 批量加载，ID 1 已缓存，ID 2 未缓存
      await store.batchLoadEntryTags([1, 2]);

      // invoke 应该只被调用一次（针对 ID 2）
      expect(invoke).toHaveBeenCalledTimes(1);
      expect(invoke).toHaveBeenCalledWith('get_entry_tags', { entryId: 2 });

      // 验证缓存结果
      expect(store.getEntryTags(1)).toEqual(cachedTags);
      expect(store.getEntryTags(2)).toEqual(uncachedTags);
    });

    it('batchLoadEntryTags 全部已缓存时不调用 invoke', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const tags1: Tag[] = [{ id: 1, name: 'tag1' }];
      const tags2: Tag[] = [{ id: 2, name: 'tag2' }];

      // 先缓存所有标签
      store.setEntryTags(1, tags1);
      store.setEntryTags(2, tags2);

      // 批量加载已缓存的 ID
      await store.batchLoadEntryTags([1, 2]);

      // invoke 不应该被调用
      expect(invoke).not.toHaveBeenCalled();

      // 验证缓存保持不变
      expect(store.getEntryTags(1)).toEqual(tags1);
      expect(store.getEntryTags(2)).toEqual(tags2);
    });
  });

  describe('fetchEntries 基本获取测试', () => {
    it('成功获取 entries 并更新 entries、totalCount、activeEntryId', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const mockEntries = [
        makeEntry({ id: 1, content: 'entry 1' }),
        makeEntry({ id: 2, content: 'entry 2' }),
      ];

      invoke.mockResolvedValue({
        entries: mockEntries,
        total_count: 10,
      });

      await store.fetchEntries();

      expect(store.entries).toEqual(mockEntries);
      expect(store.totalCount).toBe(10);
      expect(store.activeEntryId).toBe(1);
      expect(store.isLoading).toBe(false);
      expect(invoke).toHaveBeenCalledWith('get_entries', {
        limit: expect.any(Number),
        offset: 0,
        category: null,
        isFavorite: null,
      });
    });

    it('获取失败时 isLoading 恢复 false 并输出 console.error', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      const testError = new Error('Network error');
      invoke.mockRejectedValue(testError);

      await store.fetchEntries();

      expect(store.isLoading).toBe(false);
      expect(errorSpy).toHaveBeenCalledWith('Failed to fetch entries:', testError);
    });
  });

  describe('fetchEntries 搜索与过滤测试', () => {
    it('searchKeyword 非空时调用 search_entries', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.searchKeyword = 'test query';
      const mockEntries = [makeEntry({ id: 1 })];

      invoke.mockResolvedValue({
        entries: mockEntries,
        total_count: 5,
      });

      await store.fetchEntries();

      expect(invoke).toHaveBeenCalledWith('search_entries', {
        keyword: 'test query',
        category: null,
        isFavorite: null,
        limit: expect.any(Number),
        offset: 0,
      });
    });

    it('selectedCategory 为具体类型时传递 category 参数', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.selectedCategory = 'image';
      const mockEntries = [makeEntry({ id: 1, category: 'image' })];

      invoke.mockResolvedValue({
        entries: mockEntries,
        total_count: 3,
      });

      await store.fetchEntries();

      expect(invoke).toHaveBeenCalledWith('get_entries', {
        limit: expect.any(Number),
        offset: 0,
        category: 'image',
        isFavorite: null,
      });
    });

    it('selectedCategory 为 favorites 时传递 isFavorite=true', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.selectedCategory = 'favorites';
      const mockEntries = [makeEntry({ id: 1, is_favorite: true })];

      invoke.mockResolvedValue({
        entries: mockEntries,
        total_count: 2,
      });

      await store.fetchEntries();

      expect(invoke).toHaveBeenCalledWith('get_entries', {
        limit: expect.any(Number),
        offset: 0,
        category: null,
        isFavorite: true,
      });
    });

    it('selectedCategory 为 tags 且 selectedTagId 非空时调用 get_entries_by_tag', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.selectedCategory = 'tags';
      store.selectedTagId = 123;
      const mockEntries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];

      invoke.mockResolvedValue(mockEntries);

      await store.fetchEntries();

      expect(invoke).toHaveBeenCalledWith('get_entries_by_tag', {
        tagId: 123,
      });
      expect(store.entries).toEqual(mockEntries);
      expect(store.totalCount).toBe(2);
      expect(store.activeEntryId).toBe(1);
    });
  });

  describe('loadMore 分页测试', () => {
    it('hasMore 为 true 时能加载下一页', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      // 初始加载第一页
      invoke.mockResolvedValue({
        entries: [makeEntry({ id: 1 })],
        total_count: 10,
      });
      await store.fetchEntries();

      expect(store.hasMore).toBe(true);
      expect(store.entries).toHaveLength(1);

      // 加载第二页
      const secondPageEntries = [makeEntry({ id: 2 })];
      invoke.mockResolvedValue({
        entries: secondPageEntries,
        total_count: 10,
      });

      await store.loadMore();

      expect(store.entries).toHaveLength(2);
      expect(invoke).toHaveBeenCalledWith('get_entries', {
        limit: expect.any(Number),
        offset: expect.any(Number),
        category: null,
        isFavorite: null,
      });
    });

    it('hasMore 为 false 时不发起请求', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      // 只有一页数据
      invoke.mockResolvedValue({
        entries: [makeEntry({ id: 1 })],
        total_count: 1,
      });
      await store.fetchEntries();

      expect(store.hasMore).toBe(false);

      await store.loadMore();

      expect(invoke).toHaveBeenCalledTimes(1); // 只调用了一次 fetchEntries
    });

    it('正在加载时不发起重复请求', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      invoke.mockResolvedValue({
        entries: [makeEntry({ id: 1 })],
        total_count: 10,
      });
      await store.fetchEntries();

      store.isLoading = true;

      await store.loadMore();

      expect(invoke).toHaveBeenCalledTimes(1); // 只调用了一次 fetchEntries
    });
  });

  describe('groupedEntryItems 分组逻辑测试', () => {
    it('同一天的 entries 归入同一组', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.entries = [
        makeEntry({ id: 1, created_at: '2026-04-26 10:00:00' }),
        makeEntry({ id: 2, created_at: '2026-04-26 12:00:00' }),
        makeEntry({ id: 3, created_at: '2026-04-26 15:00:00' }),
      ];

      const grouped = store.groupedEntryItems;

      // 应该只有 1 个 group + 3 个 entry
      expect(grouped).toHaveLength(4);
      expect(grouped[0].type).toBe('group');
      expect(grouped[0].group?.dateKey).toBe('2026-04-26');
      expect(grouped[1].type).toBe('entry');
      expect(grouped[2].type).toBe('entry');
      expect(grouped[3].type).toBe('entry');
    });

    it('不同天的 entries 产生不同的 group header', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.entries = [
        makeEntry({ id: 1, created_at: '2026-04-26 10:00:00' }),
        makeEntry({ id: 2, created_at: '2026-04-26 12:00:00' }),
        makeEntry({ id: 3, created_at: '2026-04-25 10:00:00' }),
        makeEntry({ id: 4, created_at: '2026-04-24 15:00:00' }),
      ];

      const grouped = store.groupedEntryItems;

      // 应该有 3 个 group + 4 个 entry
      expect(grouped).toHaveLength(7);

      // 第一个 group (2026-04-26)
      expect(grouped[0].type).toBe('group');
      expect(grouped[0].group?.dateKey).toBe('2026-04-26');

      // 第二个 group (2026-04-25)
      expect(grouped[3].type).toBe('group');
      expect(grouped[3].group?.dateKey).toBe('2026-04-25');

      // 第三个 group (2026-04-24)
      expect(grouped[5].type).toBe('group');
      expect(grouped[5].group?.dateKey).toBe('2026-04-24');
    });
  });

  describe('多选操作', () => {
    it('enterMultiSelectMode 进入多选模式并选中初始条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];

      store.enterMultiSelectMode(1);

      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectionAnchorId).toBe(1);
      expect(store.selectedEntryIds).toEqual([1]);
    });

    it('exitMultiSelectMode 退出多选并清空选择', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.isMultiSelectMode = true;
      store.selectionAnchorId = 1;
      store.selectedEntryIds = [1, 2];

      store.exitMultiSelectMode();

      expect(store.isMultiSelectMode).toBe(false);
      expect(store.selectionAnchorId).toBe(null);
      expect(store.selectedEntryIds).toEqual([]);
    });

    it('toggleEntrySelection 切换条目选中状态', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];

      store.toggleEntrySelection(1);
      expect(store.selectedEntryIds).toEqual([1]);

      store.toggleEntrySelection(2);
      expect(store.selectedEntryIds).toEqual([1, 2]);

      store.toggleEntrySelection(1);
      expect(store.selectedEntryIds).toEqual([2]);
    });

    it('selectAllLoadedEntries 全选所有已加载条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.activeEntryId = 2;

      store.selectAllLoadedEntries();

      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectedEntryIds).toEqual([1, 2, 3]);
      expect(store.selectionAnchorId).toBe(2);
    });

    it('invertLoadedSelection 反选', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.selectedEntryIds = [1, 3];

      store.invertLoadedSelection();

      expect(store.selectedEntryIds).toEqual([2]);
    });

    it('selectRangeTo 范围选择', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1 }),
        makeEntry({ id: 2 }),
        makeEntry({ id: 3 }),
        makeEntry({ id: 4 }),
      ];
      store.selectionAnchorId = 1;

      store.selectRangeTo(3);

      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectedEntryIds).toEqual([1, 2, 3]);
    });
  });

  describe('收藏与删除', () => {
    it('toggleFavorite 成功切换收藏状态', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1, is_favorite: false })];
      invoke.mockResolvedValue(true);

      await store.toggleFavorite(1);

      expect(invoke).toHaveBeenCalledWith('toggle_favorite', { id: 1 });
      expect(store.entries[0].is_favorite).toBe(true);
    });

    it('toggleFavorite 在收藏过滤下取消收藏时移除条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1, is_favorite: true })];
      store.selectedCategory = 'favorites';
      store.totalCount = 1;
      invoke.mockResolvedValue(false);

      await store.toggleFavorite(1);

      expect(store.entries.length).toBe(0);
      expect(store.totalCount).toBe(0);
    });

    it('deleteEntry 成功删除并更新列表', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.totalCount = 2;
      invoke.mockResolvedValue(undefined);

      await store.deleteEntry(1);

      expect(invoke).toHaveBeenCalledWith('delete_entry', { id: 1 });
      expect(store.entries.length).toBe(1);
      expect(store.entries[0].id).toBe(2);
      expect(store.totalCount).toBe(1);
    });

    it('deleteEntry 失败时输出错误但不崩溃', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      const error = new Error('Delete failed');
      invoke.mockRejectedValue(error);

      await store.deleteEntry(1);

      expect(errorSpy).toHaveBeenCalledWith('Failed to delete entry:', error);
      expect(store.entries.length).toBe(1);
    });

    it('deleteSelectedEntries 批量删除并退出多选', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;
      store.totalCount = 3;
      invoke.mockResolvedValue(2);

      await store.deleteSelectedEntries();

      expect(invoke).toHaveBeenCalledWith('delete_entries', { ids: [1, 2] });
      expect(store.entries.length).toBe(1);
      expect(store.entries[0].id).toBe(3);
      expect(store.totalCount).toBe(1);
      expect(store.isMultiSelectMode).toBe(false);
      expect(store.selectedEntryIds).toEqual([]);
    });
  });

  describe('其他操作', () => {
    it('onClipboardChanged 去重并在头部插入新条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      const existingEntry = makeEntry({ id: 1, hash: 'abc123' });
      const newEntry = makeEntry({ id: 2, hash: 'xyz789' });
      store.entries = [existingEntry];
      store.totalCount = 1;

      store.onClipboardChanged(newEntry);

      expect(store.entries.length).toBe(2);
      expect(store.entries[0].id).toBe(2);
      expect(store.totalCount).toBe(2);
    });

    it('pasteEntry 成功时递增 use_count', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      const entry = makeEntry({ id: 1, use_count: 1 });
      store.entries = [entry];
      invoke.mockResolvedValue(undefined);

      await store.pasteEntry(1);

      expect(invoke).toHaveBeenCalledWith('paste_entry', { id: 1 });
      expect(store.entries[0].use_count).toBe(2);
    });

    it('clearSensitiveViewState 重置所有状态', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.totalCount = 5;
      store.allTags = [{ id: 1, name: 'tag1' } as Tag];
      store.selectedTagId = 1;
      store.selectedCategory = 'favorites';
      store.searchKeyword = 'test';
      store.isLoading = true;

      store.clearSensitiveViewState();

      expect(store.entries).toEqual([]);
      expect(store.totalCount).toBe(0);
      expect(store.allTags).toEqual([]);
      expect(store.selectedTagId).toBe(null);
      expect(store.selectedCategory).toBe('all');
      expect(store.searchKeyword).toBe('');
      expect(store.isLoading).toBe(false);
    });
  });
});
