import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { ClipboardEntry } from '@/types';

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

describe('useConflictStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  describe('冲突检测', () => {
    it('detectConflict 检测到同 ID 不同 hash 的条目时返回冲突对象', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const local = makeEntry({ id: 1, hash: 'local-hash' });
      const remote = makeEntry({ id: 1, hash: 'remote-hash' });

      const conflict = store.detectConflict(local, remote, 'dev-1', 'Phone');

      expect(conflict).not.toBeNull();
      expect(conflict!.entryId).toBe(1);
      expect(conflict!.localVersion).toEqual(local);
      expect(conflict!.remoteVersion).toEqual(remote);
      expect(conflict!.remoteDeviceId).toBe('dev-1');
      expect(conflict!.remoteDeviceName).toBe('Phone');
      expect(conflict!.resolved).toBe(false);
      expect(conflict!.resolution).toBeNull();
    });

    it('detectConflict 同 hash 条目返回 null', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const local = makeEntry({ id: 1, hash: 'same-hash' });
      const remote = makeEntry({ id: 1, hash: 'same-hash' });

      expect(store.detectConflict(local, remote, 'dev-1', 'Phone')).toBeNull();
    });

    it('detectConflict 不同 ID 条目返回 null', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const local = makeEntry({ id: 1, hash: 'hash-a' });
      const remote = makeEntry({ id: 2, hash: 'hash-b' });

      expect(store.detectConflict(local, remote, 'dev-1', 'Phone')).toBeNull();
    });

    it('detectConflicts 批量检测多个冲突并添加到 pendingConflicts', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-1' }), makeEntry({ id: 2, hash: 'local-2' })];
      const remotes = [
        makeEntry({ id: 1, hash: 'remote-1' }),
        makeEntry({ id: 2, hash: 'remote-2' }),
      ];

      const detected = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      expect(detected).toHaveLength(2);
      expect(store.pendingConflicts).toHaveLength(2);
    });

    it('detectConflicts 不重复添加同一 entryId 的未解决冲突', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-1' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-1' })];

      store.detectConflicts(locals, remotes, 'dev-1', 'Phone');
      expect(store.pendingConflicts).toHaveLength(1);

      const secondBatch = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');
      expect(secondBatch).toHaveLength(0);
      expect(store.pendingConflicts).toHaveLength(1);
    });
  });

  describe('自动解决策略', () => {
    it('autoResolve local-first 策略返回本地版本', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();
      store.updateStrategy('local-first');

      const locals = [makeEntry({ id: 1, hash: 'local-hash', content: 'local' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash', content: 'remote' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      const result = store.autoResolve(conflict);

      expect(result).toEqual(locals[0]);
      expect(store.pendingConflicts).toHaveLength(0);
    });

    it('autoResolve remote-first 策略返回远程版本', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();
      store.updateStrategy('remote-first');

      const locals = [makeEntry({ id: 1, hash: 'local-hash', content: 'local' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash', content: 'remote' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      const result = store.autoResolve(conflict);

      expect(result).toEqual(remotes[0]);
      expect(store.pendingConflicts).toHaveLength(0);
    });

    it('autoResolve last-write-wins 策略返回较新的版本', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();
      store.updateStrategy('last-write-wins');

      const locals = [makeEntry({ id: 1, hash: 'local-hash', updated_at: '2026-04-26 10:00:00' })];
      const remotes = [
        makeEntry({ id: 1, hash: 'remote-hash', updated_at: '2026-04-26 12:00:00' }),
      ];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      const result = store.autoResolve(conflict);

      expect(result).toEqual(remotes[0]);
    });

    it('autoResolve manual 策略返回 null', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();
      store.updateStrategy('manual');

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      const result = store.autoResolve(conflict);

      expect(result).toBeNull();
      expect(store.pendingConflicts).toHaveLength(1);
    });

    it('autoResolveAll 解决所有可自动解决的冲突并返回需要手动解决的', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();
      store.updateStrategy('manual');

      const locals = [makeEntry({ id: 1, hash: 'local-1' }), makeEntry({ id: 2, hash: 'local-2' })];
      const remotes = [
        makeEntry({ id: 1, hash: 'remote-1' }),
        makeEntry({ id: 2, hash: 'remote-2' }),
      ];
      store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      const unresolvable = store.autoResolveAll();

      expect(unresolvable).toHaveLength(2);
      expect(store.pendingConflicts).toHaveLength(2);
    });
  });

  describe('手动解决与日志', () => {
    it('resolveManually 标记冲突为已解决并添加日志', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      store.resolveManually(conflict.id, 'kept-local');

      expect(store.pendingConflicts).toHaveLength(0);
      expect(store.conflictLog).toHaveLength(1);
      expect(store.conflictLog[0].outcome).toBe('kept-local');
    });

    it('dismissConflict 以 dismissed 结果标记冲突', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      store.dismissConflict(conflict.id);

      expect(store.pendingConflicts).toHaveLength(0);
      expect(store.conflictLog).toHaveLength(1);
      expect(store.conflictLog[0].outcome).toBe('dismissed');
    });

    it('解决冲突后 activeConflict 被清除', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      store.openConflictDialog(conflict);
      expect(store.activeConflict).not.toBeNull();

      store.resolveManually(conflict.id, 'kept-remote');
      expect(store.activeConflict).toBeNull();
    });

    it('clearLog 清空冲突日志', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');
      store.resolveManually(conflict.id, 'kept-local');

      expect(store.conflictLog).toHaveLength(1);
      store.clearLog();
      expect(store.conflictLog).toHaveLength(0);
    });

    it('removeLogEntry 删除特定日志条目', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-1' }), makeEntry({ id: 2, hash: 'local-2' })];
      const remotes = [
        makeEntry({ id: 1, hash: 'remote-1' }),
        makeEntry({ id: 2, hash: 'remote-2' }),
      ];
      const conflicts = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');
      store.resolveManually(conflicts[0].id, 'kept-local');
      store.resolveManually(conflicts[1].id, 'kept-remote');

      expect(store.conflictLog).toHaveLength(2);

      store.removeLogEntry(conflicts[0].id);
      expect(store.conflictLog).toHaveLength(1);
      expect(store.conflictLog[0].id).toBe(conflicts[1].id);
    });
  });

  describe('配置与对话框管理', () => {
    it('updateConfig 更新配置并持久化到 localStorage', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      store.updateConfig({ strategy: 'remote-first', maxLogEntries: 100 });

      expect(store.config.strategy).toBe('remote-first');
      expect(store.config.maxLogEntries).toBe(100);

      const stored = JSON.parse(localStorage.getItem('smart-clipboard-conflict-config') ?? '{}');
      expect(stored.strategy).toBe('remote-first');
    });

    it('updateStrategy 更新解决策略', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      store.updateStrategy('local-first');
      expect(store.config.strategy).toBe('local-first');
    });

    it('openConflictDialog 设置 activeConflict', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      store.openConflictDialog(conflict);
      expect(store.activeConflict).toEqual(conflict);
    });

    it('openNextConflict 打开下一个未解决冲突', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-1' }), makeEntry({ id: 2, hash: 'local-2' })];
      const remotes = [
        makeEntry({ id: 1, hash: 'remote-1' }),
        makeEntry({ id: 2, hash: 'remote-2' }),
      ];
      store.detectConflicts(locals, remotes, 'dev-1', 'Phone');

      store.openNextConflict();
      expect(store.activeConflict).not.toBeNull();
      expect(store.activeConflict!.entryId).toBe(1);
    });

    it('openNextConflict 无冲突时 activeConflict 为 null', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      store.openNextConflict();
      expect(store.activeConflict).toBeNull();
    });

    it('clearAll 清空所有状态', async () => {
      const { useConflictStore } = await import('@/stores/conflictStore');
      const store = useConflictStore();

      const locals = [makeEntry({ id: 1, hash: 'local-hash' })];
      const remotes = [makeEntry({ id: 1, hash: 'remote-hash' })];
      const [conflict] = store.detectConflicts(locals, remotes, 'dev-1', 'Phone');
      store.openConflictDialog(conflict);
      store.resolveManually(conflict.id, 'kept-local');

      store.clearAll();

      expect(store.pendingConflicts).toHaveLength(0);
      expect(store.conflictLog).toHaveLength(0);
      expect(store.activeConflict).toBeNull();
    });
  });
});
