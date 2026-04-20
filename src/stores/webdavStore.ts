import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  WebDavConfig,
  WebDavSyncStatus,
  RegisteredDevice,
} from "@/types";

export const useWebDavStore = defineStore("webdav", () => {
  const config = ref<WebDavConfig>({
    enabled: false,
    serverUrl: "",
    username: "",
    password: "",
    syncPassword: "",
    pollIntervalSecs: 30,
    syncImages: false,
    syncSensitive: false,
    rateLimitCapacity: 150,
    rateLimitRefillMinutes: 30,
    remotePath: "/SmartClipboard",
    maxCloudEntries: 2000,
  });

  const status = ref<string>("disconnected");
  const lastSyncAt = ref<string | null>(null);
  const cloudEntryCount = ref(0);
  const registeredDevices = ref<RegisteredDevice[]>([]);
  const rateLimitAvailable = ref(0);
  const rateLimitCapacity = ref(0);
  const error = ref<string | null>(null);
  const isLoading = ref(false);
  const isConnecting = ref(false);

  async function loadConfig() {
    try {
      const result = (await invoke("webdav_get_config")) as WebDavConfig;
      config.value = result;
    } catch (e) {
      console.error("Failed to load WebDAV config:", e);
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function saveConfig(newConfig: WebDavConfig) {
    try {
      await invoke("webdav_update_config", { newConfig });
      config.value = newConfig;
    } catch (e) {
      console.error("Failed to save WebDAV config:", e);
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function refreshStatus() {
    try {
      const result = (await invoke("webdav_get_status")) as WebDavSyncStatus;
      status.value = result.status;
      lastSyncAt.value = result.lastSyncAt;
      cloudEntryCount.value = result.cloudEntryCount;
      registeredDevices.value = result.registeredDevices;
      rateLimitAvailable.value = result.rateLimitAvailable;
      rateLimitCapacity.value = result.rateLimitCapacity;
      if (result.error) {
        error.value = result.error;
      }
    } catch (e) {
      console.error("Failed to refresh WebDAV status:", e);
    }
  }

  async function refreshAll() {
    isLoading.value = true;
    error.value = null;
    try {
      await Promise.all([loadConfig(), refreshStatus()]);
    } catch (e) {
      console.error("Failed to refresh WebDAV data:", e);
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function connect(
    serverUrl: string,
    username: string,
    password: string,
    syncPassword: string,
  ) {
    isConnecting.value = true;
    error.value = null;
    try {
      await invoke("webdav_connect", {
        serverUrl,
        username,
        password,
        syncPassword,
      });
      await refreshStatus();
    } catch (e) {
      console.error("WebDAV connect failed:", e);
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      isConnecting.value = false;
    }
  }

  async function disconnect() {
    try {
      await invoke("webdav_disconnect");
      status.value = "disconnected";
      lastSyncAt.value = null;
      cloudEntryCount.value = 0;
      registeredDevices.value = [];
      rateLimitAvailable.value = 0;
      rateLimitCapacity.value = 0;
      error.value = null;
    } catch (e) {
      console.error("WebDAV disconnect failed:", e);
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function triggerSync(): Promise<number> {
    try {
      const count = (await invoke("webdav_trigger_sync")) as number;
      await refreshStatus();
      return count;
    } catch (e) {
      console.error("WebDAV sync failed:", e);
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function removeDevice(deviceId: string) {
    try {
      await invoke("webdav_remove_device", { deviceId });
      await refreshStatus();
    } catch (e) {
      console.error("Failed to remove device:", e);
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    config,
    status,
    lastSyncAt,
    cloudEntryCount,
    registeredDevices,
    rateLimitAvailable,
    rateLimitCapacity,
    error,
    isLoading,
    isConnecting,
    loadConfig,
    saveConfig,
    refreshStatus,
    refreshAll,
    connect,
    disconnect,
    triggerSync,
    removeDevice,
    clearError,
  };
});
