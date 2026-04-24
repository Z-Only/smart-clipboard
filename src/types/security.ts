export interface AppLockConfig {
  enabled: boolean;
  auto_lock_seconds: number;
  biometric_enabled: boolean;
}

export interface AppLockStatus {
  enabled: boolean;
  configured: boolean;
  locked: boolean;
  biometric_available: boolean;
  biometric_enabled: boolean;
  auto_lock_seconds: number;
  unlock_reason: string | null;
  failed_attempts: number;
}

export interface EncryptionStatus {
  enabled: boolean;
  key_exists: boolean;
  encrypted_count: number;
  plaintext_count: number;
  migrating: boolean;
}
