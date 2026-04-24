import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

describe('useUpdaterStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    listen.mockReset();
    listen.mockResolvedValue(() => {});
  });

  it('loads status and performs manual check', async () => {
    const initial = {
      phase: 'idle',
      currentVersion: '2.1.0',
      availableVersion: null,
      pendingUpdate: null,
      availableNotes: null,
      availableReleaseDate: null,
      availableNotes: null,
      availableReleaseDate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };
    const checked = { ...initial, phase: 'up_to_date' };
    invoke.mockResolvedValueOnce(initial).mockResolvedValueOnce(checked);

    const { useUpdaterStore } = await import('@/stores/updaterStore');
    const store = useUpdaterStore();

    await store.loadStatus();
    await store.checkNow();

    expect(invoke).toHaveBeenNthCalledWith(1, 'get_updater_status');
    expect(invoke).toHaveBeenNthCalledWith(2, 'check_for_updates_now');
    expect(store.status.phase).toBe('up_to_date');
  });

  it('updates progress from updater-status-changed events', async () => {
    let eventHandler: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation(
      async (_event: string, handler: (event: { payload: unknown }) => void) => {
        eventHandler = handler;
        return () => {};
      },
    );

    const { useUpdaterStore } = await import('@/stores/updaterStore');
    const store = useUpdaterStore();
    await store.bindEvents();

    eventHandler?.({
      payload: {
        phase: 'downloading',
        currentVersion: '2.1.0',
        availableVersion: '2.2.0',
        availableNotes: 'Release notes',
        availableReleaseDate: '2026-04-23T10:30:00Z',
        pendingUpdate: null,
        downloadProgress: 0.42,
        lastError: null,
        lastCheckSilent: false,
      },
    });

    expect(store.status.phase).toBe('downloading');
    expect(store.status.downloadProgress).toBe(0.42);
  });

  it('downloads available update and stores ready-to-install status', async () => {
    const ready = {
      phase: 'ready_to_install',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'Bug fixes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: {
        version: '2.2.0',
        releaseDate: '2026-04-23T10:30:00Z',
        currentVersion: '2.1.0',
        notes: 'Bug fixes',
        artifactPath: '/tmp/app.tar.gz',
        signaturePath: '/tmp/app.tar.gz.sig',
        canonicalAssetUrl: 'https://github.com/x',
        sourceAssetUrl: 'https://mirror/x',
        downloadedAt: '2026-04-23T10:35:00Z',
      },
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };
    invoke.mockResolvedValueOnce(ready);

    const { useUpdaterStore } = await import('@/stores/updaterStore');
    const store = useUpdaterStore();

    await store.downloadAvailable();

    expect(invoke).toHaveBeenCalledWith('download_available_update');
    expect(store.status.phase).toBe('ready_to_install');
    expect(store.status.pendingUpdate?.version).toBe('2.2.0');
  });

  it('discards pending update and refreshes state', async () => {
    const pending = {
      phase: 'ready_to_install',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      pendingUpdate: {
        version: '2.2.0',
        releaseDate: null,
        currentVersion: '2.1.0',
        notes: null,
        artifactPath: '/tmp/app',
        signaturePath: '/tmp/app.sig',
        canonicalAssetUrl: 'https://github.com/x',
        sourceAssetUrl: 'https://mirror/x',
        downloadedAt: '2026-04-23T10:35:00Z',
      },
      availableNotes: null,
      availableReleaseDate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };
    const cleared = { ...pending, phase: 'idle', pendingUpdate: null, availableVersion: null };
    invoke.mockResolvedValueOnce(cleared);

    const { useUpdaterStore } = await import('@/stores/updaterStore');
    const store = useUpdaterStore();
    store.status = pending as never;

    await store.discardPending();

    expect(invoke).toHaveBeenCalledWith('discard_pending_update');
    expect(store.status.phase).toBe('idle');
    expect(store.status.pendingUpdate).toBeNull();
  });
});
