import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type {
  ClipboardEntry,
  ConflictConfig,
  ConflictLogEntry,
  ConflictOutcome,
  ConflictResolutionStrategy,
  SyncConflict,
} from '@/types';

const CONFLICT_LOG_STORAGE_KEY = 'smart-clipboard-conflict-log';
const CONFLICT_CONFIG_STORAGE_KEY = 'smart-clipboard-conflict-config';

function generateConflictId(): string {
  return `conflict-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

function truncateContent(content: string, maxLength = 80): string {
  if (content.length <= maxLength) return content;
  return content.slice(0, maxLength) + '…';
}

function loadLogFromStorage(): ConflictLogEntry[] {
  try {
    const raw = localStorage.getItem(CONFLICT_LOG_STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as ConflictLogEntry[];
  } catch {
    return [];
  }
}

function saveLogToStorage(entries: ConflictLogEntry[]) {
  localStorage.setItem(CONFLICT_LOG_STORAGE_KEY, JSON.stringify(entries));
}

function loadConfigFromStorage(): ConflictConfig {
  const defaults: ConflictConfig = {
    strategy: 'last-write-wins',
    keepConflictLog: true,
    maxLogEntries: 200,
  };
  try {
    const raw = localStorage.getItem(CONFLICT_CONFIG_STORAGE_KEY);
    if (!raw) return defaults;
    const parsed = JSON.parse(raw) as Partial<ConflictConfig>;
    return { ...defaults, ...parsed };
  } catch {
    return defaults;
  }
}

function saveConfigToStorage(config: ConflictConfig) {
  localStorage.setItem(CONFLICT_CONFIG_STORAGE_KEY, JSON.stringify(config));
}

export const useConflictStore = defineStore('conflict', () => {
  // --- State ---
  const config = ref<ConflictConfig>(loadConfigFromStorage());
  const pendingConflicts = ref<SyncConflict[]>([]);
  const conflictLog = ref<ConflictLogEntry[]>(loadLogFromStorage());
  const activeConflict = ref<SyncConflict | null>(null);

  // --- Getters ---
  const pendingCount = computed(() => pendingConflicts.value.length);
  const hasConflicts = computed(() => pendingConflicts.value.length > 0);
  const logCount = computed(() => conflictLog.value.length);

  const sortedLog = computed(() =>
    [...conflictLog.value].sort(
      (a, b) => new Date(b.resolvedAt).getTime() - new Date(a.resolvedAt).getTime(),
    ),
  );

  // --- Conflict Detection ---

  /**
   * Detect whether two versions of the same entry are in conflict.
   * A conflict is detected when the same entry (by id) has different hashes
   * and the remote version was updated after the local version (or vice versa),
   * meaning changes happened on both sides.
   */
  function detectConflict(
    localEntry: ClipboardEntry,
    remoteEntry: ClipboardEntry,
    remoteDeviceId: string,
    remoteDeviceName: string,
  ): SyncConflict | null {
    if (localEntry.id !== remoteEntry.id) return null;
    if (localEntry.hash === remoteEntry.hash) return null;

    return {
      id: generateConflictId(),
      entryId: localEntry.id,
      localVersion: { ...localEntry },
      remoteVersion: { ...remoteEntry },
      remoteDeviceId,
      remoteDeviceName,
      detectedAt: new Date().toISOString(),
      resolved: false,
      resolution: null,
    };
  }

  /**
   * Batch-detect conflicts between a set of local entries and incoming remote entries.
   * Returns the list of new conflicts that need resolution.
   */
  function detectConflicts(
    localEntries: ClipboardEntry[],
    remoteEntries: ClipboardEntry[],
    remoteDeviceId: string,
    remoteDeviceName: string,
  ): SyncConflict[] {
    const localMap = new Map(localEntries.map((entry) => [entry.id, entry]));
    const newConflicts: SyncConflict[] = [];

    for (const remoteEntry of remoteEntries) {
      const localEntry = localMap.get(remoteEntry.id);
      if (!localEntry) continue;

      const conflict = detectConflict(localEntry, remoteEntry, remoteDeviceId, remoteDeviceName);
      if (conflict) {
        const alreadyPending = pendingConflicts.value.some(
          (existing) => existing.entryId === conflict.entryId && !existing.resolved,
        );
        if (!alreadyPending) {
          newConflicts.push(conflict);
        }
      }
    }

    if (newConflicts.length > 0) {
      pendingConflicts.value = [...pendingConflicts.value, ...newConflicts];
    }

    return newConflicts;
  }

  // --- Conflict Resolution ---

  /**
   * Apply the configured auto-resolution strategy to a conflict.
   * Returns `null` if the strategy is 'manual' (requires user intervention).
   */
  function autoResolve(conflict: SyncConflict): ClipboardEntry | null {
    const strategy = config.value.strategy;

    switch (strategy) {
      case 'local-first':
        markResolved(conflict.id, 'kept-local');
        return conflict.localVersion;

      case 'remote-first':
        markResolved(conflict.id, 'kept-remote');
        return conflict.remoteVersion;

      case 'last-write-wins': {
        const localTime = new Date(conflict.localVersion.updated_at).getTime();
        const remoteTime = new Date(conflict.remoteVersion.updated_at).getTime();
        if (localTime >= remoteTime) {
          markResolved(conflict.id, 'auto-resolved');
          return conflict.localVersion;
        }
        markResolved(conflict.id, 'auto-resolved');
        return conflict.remoteVersion;
      }

      case 'manual':
        return null;

      default:
        return null;
    }
  }

  /**
   * Try to auto-resolve all pending conflicts.
   * Returns conflicts that could not be auto-resolved (requiring manual intervention).
   */
  function autoResolveAll(): SyncConflict[] {
    const unresolvable: SyncConflict[] = [];

    for (const conflict of [...pendingConflicts.value]) {
      if (conflict.resolved) continue;
      const result = autoResolve(conflict);
      if (result === null) {
        unresolvable.push(conflict);
      }
    }

    return unresolvable;
  }

  /**
   * Manually resolve a conflict by choosing which version to keep.
   */
  function resolveManually(conflictId: string, outcome: 'kept-local' | 'kept-remote'): void {
    markResolved(conflictId, outcome);
    if (activeConflict.value?.id === conflictId) {
      activeConflict.value = null;
    }
  }

  /**
   * Dismiss a conflict without resolving (e.g., user chose to skip).
   */
  function dismissConflict(conflictId: string): void {
    markResolved(conflictId, 'dismissed');
    if (activeConflict.value?.id === conflictId) {
      activeConflict.value = null;
    }
  }

  function markResolved(conflictId: string, outcome: ConflictOutcome): void {
    const index = pendingConflicts.value.findIndex((conflict) => conflict.id === conflictId);
    if (index === -1) return;

    const conflict = pendingConflicts.value[index];
    pendingConflicts.value[index] = {
      ...conflict,
      resolved: true,
      resolution: outcome,
    };

    if (config.value.keepConflictLog) {
      appendLogEntry(conflict, outcome);
    }

    pendingConflicts.value = pendingConflicts.value.filter((c) => !c.resolved);
  }

  // --- Conflict Log ---

  function appendLogEntry(conflict: SyncConflict, outcome: ConflictOutcome): void {
    const logEntry: ConflictLogEntry = {
      id: conflict.id,
      entryId: conflict.entryId,
      localContentPreview: truncateContent(conflict.localVersion.content),
      remoteContentPreview: truncateContent(conflict.remoteVersion.content),
      remoteDeviceName: conflict.remoteDeviceName,
      strategy: config.value.strategy,
      outcome,
      resolvedAt: new Date().toISOString(),
    };

    conflictLog.value = [logEntry, ...conflictLog.value].slice(0, config.value.maxLogEntries);
    saveLogToStorage(conflictLog.value);
  }

  function clearLog(): void {
    conflictLog.value = [];
    saveLogToStorage([]);
  }

  function removeLogEntry(logEntryId: string): void {
    conflictLog.value = conflictLog.value.filter((entry) => entry.id !== logEntryId);
    saveLogToStorage(conflictLog.value);
  }

  // --- Configuration ---

  function updateConfig(newConfig: Partial<ConflictConfig>): void {
    config.value = { ...config.value, ...newConfig };
    saveConfigToStorage(config.value);
  }

  function updateStrategy(strategy: ConflictResolutionStrategy): void {
    updateConfig({ strategy });
  }

  // --- Dialog Management ---

  function openConflictDialog(conflict: SyncConflict): void {
    activeConflict.value = conflict;
  }

  function openNextConflict(): void {
    const next = pendingConflicts.value.find((c) => !c.resolved);
    activeConflict.value = next ?? null;
  }

  function closeConflictDialog(): void {
    activeConflict.value = null;
  }

  // --- Cleanup ---

  function clearAll(): void {
    pendingConflicts.value = [];
    activeConflict.value = null;
    conflictLog.value = [];
    saveLogToStorage([]);
  }

  return {
    // State
    config,
    pendingConflicts,
    conflictLog,
    activeConflict,

    // Getters
    pendingCount,
    hasConflicts,
    logCount,
    sortedLog,

    // Detection
    detectConflict,
    detectConflicts,

    // Resolution
    autoResolve,
    autoResolveAll,
    resolveManually,
    dismissConflict,

    // Log
    clearLog,
    removeLogEntry,

    // Config
    updateConfig,
    updateStrategy,

    // Dialog
    openConflictDialog,
    openNextConflict,
    closeConflictDialog,

    // Cleanup
    clearAll,
  };
});
