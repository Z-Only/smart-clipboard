import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UpdaterStatus } from '@/types';

const defaultStatus: UpdaterStatus = {
  phase: 'idle',
  currentVersion: '',
  availableVersion: null,
  availableNotes: null,
  availableReleaseDate: null,
  pendingUpdate: null,
  downloadProgress: null,
  lastError: null,
  lastCheckSilent: false,
};

export const useUpdaterStore = defineStore('updater', () => {
  const status = ref<UpdaterStatus>({ ...defaultStatus });
  const loading = ref(false);
  const bound = ref(false);

  const isChecking = computed(() => loading.value || status.value.phase === 'checking');

  async function loadStatus() {
    status.value = await invoke<UpdaterStatus>('get_updater_status');
  }

  async function checkNow() {
    loading.value = true;
    try {
      status.value = await invoke<UpdaterStatus>('check_for_updates_now');
    } finally {
      loading.value = false;
    }
  }

  async function downloadAvailable() {
    loading.value = true;
    try {
      status.value = await invoke<UpdaterStatus>('download_available_update');
    } finally {
      loading.value = false;
    }
  }

  async function installPending() {
    loading.value = true;
    try {
      status.value = await invoke<UpdaterStatus>('install_pending_update');
    } finally {
      loading.value = false;
    }
  }

  async function discardPending() {
    loading.value = true;
    try {
      status.value = await invoke<UpdaterStatus>('discard_pending_update');
    } finally {
      loading.value = false;
    }
  }

  async function bindEvents() {
    if (bound.value) return;
    await listen<UpdaterStatus>('updater-status-changed', (event) => {
      status.value = event.payload;
    });
    bound.value = true;
  }

  return {
    status,
    loading,
    isChecking,
    loadStatus,
    checkNow,
    downloadAvailable,
    installPending,
    discardPending,
    bindEvents,
  };
});
