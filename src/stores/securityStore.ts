import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppLockStatus, EncryptionStatus } from '@/types/security';

const defaultEncryptionStatus: EncryptionStatus = {
  enabled: false,
  key_exists: false,
  encrypted_count: 0,
  plaintext_count: 0,
  migrating: false,
};

interface State {
  status: AppLockStatus;
  encryption: EncryptionStatus;
  loading: boolean;
  error: string | null;
  initialized: boolean;
}

const defaultStatus: AppLockStatus = {
  enabled: false,
  configured: false,
  locked: false,
  biometric_available: false,
  biometric_enabled: false,
  auto_lock_seconds: 0,
  unlock_reason: null,
  failed_attempts: 0,
};

export const useSecurityStore = defineStore('security', {
  state: (): State => ({
    status: defaultStatus,
    encryption: defaultEncryptionStatus,
    loading: false,
    error: null,
    initialized: false,
  }),
  actions: {
    async init() {
      if (!this.initialized) {
        await listen<{ status: AppLockStatus }>('app-lock-status', (event) => {
          this.status = event.payload.status;
        });
        this.initialized = true;
      }
      await this.refresh();
    },
    async refresh() {
      this.status = await invoke<AppLockStatus>('get_app_lock_status');
      try {
        this.encryption = await invoke<EncryptionStatus>('get_encryption_status');
      } catch {
        // Encryption commands may not be available if app is locked
      }
    },
    async setPassword(currentPassword: string | null, newPassword: string) {
      this.loading = true;
      this.error = null;
      try {
        this.status = await invoke<AppLockStatus>('set_app_lock_password', {
          payload: { current_password: currentPassword, new_password: newPassword },
        });
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async updateSettings(payload: {
      enabled: boolean;
      auto_lock_seconds: number;
      biometric_enabled: boolean;
    }) {
      this.loading = true;
      this.error = null;
      try {
        this.status = await invoke<AppLockStatus>('update_app_lock_settings', { payload });
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async lock() {
      this.status = await invoke<AppLockStatus>('lock_app');
    },
    async unlock(password: string | null, preferBiometric = false) {
      this.loading = true;
      this.error = null;
      try {
        this.status = await invoke<AppLockStatus>('unlock_app', {
          payload: { password, prefer_biometric: preferBiometric },
        });
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async enableEncryption() {
      this.loading = true;
      this.error = null;
      try {
        this.encryption = await invoke<EncryptionStatus>('enable_encryption');
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async disableEncryption() {
      this.loading = true;
      this.error = null;
      try {
        this.encryption = await invoke<EncryptionStatus>('disable_encryption');
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async refreshEncryption() {
      try {
        this.encryption = await invoke<EncryptionStatus>('get_encryption_status');
      } catch {
        // silently ignore if locked
      }
    },
  },
});
