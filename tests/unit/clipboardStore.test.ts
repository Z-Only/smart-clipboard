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

  describe('多选模式生命周期（补充）', () => {
    it('enterMultiSelectMode 无参数时只设置模式标志', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      store.enterMultiSelectMode();

      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectedEntryIds).toEqual([]);
      expect(store.selectionAnchorId).toBe(null);
    });

    it('clearSelection 只清除选中 ID 不改变模式标志', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.isMultiSelectMode = true;
      store.selectionAnchorId = 1;
      store.selectedEntryIds = [1, 2];

      store.clearSelection();

      expect(store.selectedEntryIds).toEqual([]);
      // 模式标志和锚点不受 clearSelection 影响
      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectionAnchorId).toBe(1);
    });
  });

  describe('选择操作（补充）', () => {
    it('toggleEntrySelection 使用 force=true 强制选中', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];

      store.toggleEntrySelection(1, true);
      expect(store.selectedEntryIds).toContain(1);

      // 再次 force=true 不会取消
      store.toggleEntrySelection(1, true);
      expect(store.selectedEntryIds).toContain(1);
    });

    it('toggleEntrySelection 使用 force=false 强制取消选中', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1, 2];

      store.toggleEntrySelection(1, false);
      expect(store.selectedEntryIds).not.toContain(1);
      expect(store.selectedEntryIds).toContain(2);
    });

    it('取消选中最后一个条目时自动退出多选模式', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1];
      store.selectionAnchorId = 1;

      store.toggleEntrySelection(1);

      expect(store.selectedEntryIds).toEqual([]);
      expect(store.isMultiSelectMode).toBe(false);
      expect(store.selectionAnchorId).toBe(null);
    });

    it('toggleEntrySelection 首次选中时设置锚点', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];

      expect(store.selectionAnchorId).toBe(null);
      store.toggleEntrySelection(1);
      expect(store.selectionAnchorId).toBe(1);
    });

    it('selectRangeTo 反向范围选择（anchor > target）', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1 }),
        makeEntry({ id: 2 }),
        makeEntry({ id: 3 }),
        makeEntry({ id: 4 }),
      ];
      store.selectionAnchorId = 3;

      store.selectRangeTo(1);

      expect(store.isMultiSelectMode).toBe(true);
      expect(store.selectedEntryIds).toEqual([1, 2, 3]);
    });

    it('selectRangeTo 无锚点时使用 activeEntryId', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.selectionAnchorId = null;
      store.activeEntryId = 1;

      store.selectRangeTo(3);

      expect(store.selectedEntryIds).toEqual([1, 2, 3]);
    });

    it('invertLoadedSelection 全部已选时结果为空并退出多选', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1, 2];

      store.invertLoadedSelection();

      expect(store.selectedEntryIds).toEqual([]);
      // 锚点清除
      expect(store.selectionAnchorId).toBe(null);
    });

    it('invertLoadedSelection 无选中时选中全部并进入多选模式', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.selectedEntryIds = [];
      store.isMultiSelectMode = false;

      store.invertLoadedSelection();

      expect(store.selectedEntryIds).toEqual([1, 2]);
      expect(store.isMultiSelectMode).toBe(true);
    });
  });

  describe('computed 属性（多选相关）', () => {
    it('selectedEntryIdSet 是 Set 且与 selectedEntryIds 同步', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.selectedEntryIds = [1, 3, 5];

      expect(store.selectedEntryIdSet).toBeInstanceOf(Set);
      expect(store.selectedEntryIdSet.has(1)).toBe(true);
      expect(store.selectedEntryIdSet.has(2)).toBe(false);
      expect(store.selectedEntryIdSet.has(3)).toBe(true);
    });

    it('selectedCount 返回选中条目数', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.selectedEntryIds = [];
      expect(store.selectedCount).toBe(0);

      store.selectedEntryIds = [1, 2, 3];
      expect(store.selectedCount).toBe(3);
    });

    it('selectedEntries 返回选中的实体条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, content: 'a' }),
        makeEntry({ id: 2, content: 'b' }),
        makeEntry({ id: 3, content: 'c' }),
      ];
      store.selectedEntryIds = [1, 3];

      expect(store.selectedEntries).toHaveLength(2);
      expect(store.selectedEntries[0].id).toBe(1);
      expect(store.selectedEntries[1].id).toBe(3);
    });

    it('canBatchCopy 当选中条目包含非图片时为 true', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, content_type: 'text' }),
        makeEntry({ id: 2, content_type: 'image' }),
      ];
      store.selectedEntryIds = [1, 2];

      expect(store.canBatchCopy).toBe(true);
    });

    it('canBatchCopy 当选中条目全是图片时为 false', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, content_type: 'image' }),
        makeEntry({ id: 2, content_type: 'image' }),
      ];
      store.selectedEntryIds = [1, 2];

      expect(store.canBatchCopy).toBe(false);
    });
  });

  describe('批量动作（补充）', () => {
    it('deleteSelectedEntries 失败时打印错误不崩溃', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.selectedEntryIds = [1];
      store.isMultiSelectMode = true;
      const error = new Error('Batch delete failed');
      invoke.mockRejectedValue(error);

      await store.deleteSelectedEntries();

      expect(errorSpy).toHaveBeenCalledWith('Failed to delete selected entries:', error);
    });

    it('deleteSelectedEntries 无选中时不调用后端', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [];

      await store.deleteSelectedEntries();

      expect(invoke).not.toHaveBeenCalled();
    });

    it('copySelectedEntries 调用后端 copy_entries', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, content: 'hello' }),
        makeEntry({ id: 2, content: 'world' }),
      ];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;
      invoke.mockResolvedValue('hello\n\nworld');

      await store.copySelectedEntries();

      expect(invoke).toHaveBeenCalledWith('copy_entries', { ids: [1, 2] });
    });

    it('copySelectedEntries 无选中时不调用后端', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [];

      await store.copySelectedEntries();

      expect(invoke).not.toHaveBeenCalled();
    });

    it('copySelectedEntries 失败时打印错误', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1, content: 'hello' })];
      store.selectedEntryIds = [1];
      store.isMultiSelectMode = true;
      const error = new Error('Copy failed');
      invoke.mockRejectedValue(error);

      await store.copySelectedEntries();

      expect(errorSpy).toHaveBeenCalledWith('Failed to copy selected entries:', error);
    });

    it('favoriteSelectedEntries(true) 批量设置收藏', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, is_favorite: false }),
        makeEntry({ id: 2, is_favorite: false }),
        makeEntry({ id: 3, is_favorite: true }),
      ];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;
      invoke.mockResolvedValue(2);

      await store.favoriteSelectedEntries(true);

      expect(invoke).toHaveBeenCalledWith('set_favorite_state_for_entries', {
        ids: [1, 2],
        favorite: true,
      });
      expect(store.entries[0].is_favorite).toBe(true);
      expect(store.entries[1].is_favorite).toBe(true);
      // 未选中的条目不受影响
      expect(store.entries[2].is_favorite).toBe(true);
    });

    it('favoriteSelectedEntries(false) 批量取消收藏', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, is_favorite: true }),
        makeEntry({ id: 2, is_favorite: true }),
      ];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;
      invoke.mockResolvedValue(2);

      await store.favoriteSelectedEntries(false);

      expect(invoke).toHaveBeenCalledWith('set_favorite_state_for_entries', {
        ids: [1, 2],
        favorite: false,
      });
      expect(store.entries[0].is_favorite).toBe(false);
      expect(store.entries[1].is_favorite).toBe(false);
    });

    it('favoriteSelectedEntries(false) 在收藏分类下过滤已取消收藏的条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [
        makeEntry({ id: 1, is_favorite: true }),
        makeEntry({ id: 2, is_favorite: true }),
        makeEntry({ id: 3, is_favorite: true }),
      ];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;
      store.selectedCategory = 'favorites';
      store.totalCount = 3;
      invoke.mockResolvedValue(2);

      await store.favoriteSelectedEntries(false);

      // 取消收藏后，在收藏视图下应被过滤
      expect(store.entries.length).toBe(1);
      expect(store.entries[0].id).toBe(3);
      expect(store.totalCount).toBe(1);
    });

    it('favoriteSelectedEntries 失败时打印错误', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [1];
      store.isMultiSelectMode = true;
      const error = new Error('Favorite failed');
      invoke.mockRejectedValue(error);

      await store.favoriteSelectedEntries(true);

      expect(errorSpy).toHaveBeenCalledWith(
        'Failed to update favorite state for selected entries:',
        error,
      );
    });

    it('favoriteSelectedEntries 无选中时不调用后端', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [];

      await store.favoriteSelectedEntries(true);

      expect(invoke).not.toHaveBeenCalled();
    });

    it('applyTagsToSelectedEntries 调用后端并刷新标签缓存', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1, content: 'a' }), makeEntry({ id: 2, content: 'b' })];
      store.selectedEntryIds = [1, 2];
      store.isMultiSelectMode = true;

      const updatedTags: Tag[] = [{ id: 10, name: 'work' }];
      invoke.mockImplementation((cmd: string) => {
        if (cmd === 'set_tags_for_entries') return Promise.resolve();
        if (cmd === 'get_entry_tags') return Promise.resolve(updatedTags);
        return Promise.resolve();
      });

      await store.applyTagsToSelectedEntries([10], 'replace');

      expect(invoke).toHaveBeenCalledWith('set_tags_for_entries', {
        ids: [1, 2],
        tagIds: [10],
        mode: 'replace',
      });
      // 标签缓存应被刷新
      expect(store.getEntryTags(1)).toEqual(updatedTags);
      expect(store.getEntryTags(2)).toEqual(updatedTags);
    });

    it('applyTagsToSelectedEntries 无选中时不调用后端', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [];

      await store.applyTagsToSelectedEntries([10], 'append');

      expect(invoke).not.toHaveBeenCalled();
    });

    it('applyTagsToSelectedEntries 失败时打印错误', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.selectedEntryIds = [1];
      store.isMultiSelectMode = true;
      const error = new Error('Tags failed');
      invoke.mockRejectedValue(error);

      await store.applyTagsToSelectedEntries([10]);

      expect(errorSpy).toHaveBeenCalledWith('Failed to set tags for selected entries:', error);
    });
  });

  describe('handleEntryPrimaryAction 多选模式行为', () => {
    it('多选模式下切换选中而非粘贴', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [];

      store.handleEntryPrimaryAction(1);

      // 应该切换选中状态而不是调用 paste_entry
      expect(store.selectedEntryIds).toContain(1);
      expect(store.activeEntryId).toBe(1);
      expect(invoke).not.toHaveBeenCalled();
    });

    it('多选模式下 + range 调用 selectRangeTo', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.isMultiSelectMode = true;
      store.selectionAnchorId = 1;

      store.handleEntryPrimaryAction(3, { range: true });

      expect(store.selectedEntryIds).toEqual([1, 2, 3]);
      expect(store.activeEntryId).toBe(3);
    });

    it('非多选模式下调用 pasteEntry', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1, use_count: 0 })];
      store.isMultiSelectMode = false;
      invoke.mockResolvedValue(undefined);

      store.handleEntryPrimaryAction(1);

      expect(store.activeEntryId).toBe(1);
      expect(invoke).toHaveBeenCalledWith('paste_entry', { id: 1 });
    });

    it('多选模式下再次点击已选中条目取消选中', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1, 2];

      store.handleEntryPrimaryAction(1);

      expect(store.selectedEntryIds).not.toContain(1);
      expect(store.selectedEntryIds).toContain(2);
    });
  });

  describe('reconcileSelection 选区调和', () => {
    it('移除不再存在于 entries 中的选中 ID', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1, 2, 3];
      store.activeEntryId = 1;

      // 通过 deleteEntry 触发 reconcileSelection
      invoke.mockResolvedValue(undefined);
      await store.deleteEntry(1);

      // ID 1 已被删除，应该从选中集合中移除
      // deleteEntry 会直接从 entries 移除，但 selectedEntryIds 中的 1 由 reconcileSelection 清理
      expect(store.selectedEntryIds).not.toContain(1);
    });

    it('选中数归零时自动退出多选模式', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1];
      store.activeEntryId = 1;
      store.totalCount = 1;

      invoke.mockResolvedValue(undefined);
      await store.deleteEntry(1);

      expect(store.selectedEntryIds).toEqual([]);
      expect(store.isMultiSelectMode).toBe(false);
    });

    it('activeEntryId 不存在时更新为第一个条目', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
      store.activeEntryId = 1;
      store.totalCount = 2;

      invoke.mockResolvedValue(undefined);
      await store.deleteEntry(1);

      // activeEntryId 应更新为剩余的第一个条目
      expect(store.activeEntryId).toBe(2);
    });

    it('selectionAnchorId 不存在时更新为第一个选中的 ID', async () => {
      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();
      store.entries = [makeEntry({ id: 2 }), makeEntry({ id: 3 })];
      store.isMultiSelectMode = true;
      store.selectedEntryIds = [1, 2, 3];
      store.selectionAnchorId = 1;
      store.activeEntryId = 2;
      store.totalCount = 3;

      // 手动触发：模拟删除后 entries 不再包含 ID 1
      invoke.mockResolvedValue(undefined);
      await store.deleteEntry(1);

      // 锚点应更新为剩余选中列表的第一个
      expect(store.selectionAnchorId).not.toBe(1);
    });
  });

  describe('fetchRecentEntries', () => {
    it('invokes get_recent_entries and stores result', async () => {
      const entry1 = makeEntry({ id: 1, content: 'first' });
      const entry2 = makeEntry({ id: 2, content: 'second' });
      invoke.mockResolvedValueOnce([entry1, entry2]);

      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      await store.fetchRecentEntries(9);
      expect(invoke).toHaveBeenCalledWith('get_recent_entries', { limit: 9 });
      expect(store.recentEntries).toEqual([entry1, entry2]);
    });

    it('sets recentEntries to empty array on error', async () => {
      invoke.mockRejectedValueOnce(new Error('locked'));

      const { useClipboardStore } = await import('@/stores/clipboardStore');
      const store = useClipboardStore();

      await store.fetchRecentEntries(9);
      expect(store.recentEntries).toEqual([]);
    });
  });
});
