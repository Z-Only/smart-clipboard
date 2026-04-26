<template>
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <div>
        <label class="text-sm font-medium">{{ $t('lock.settingsTitle') }}</label>
        <p class="text-xs text-muted-foreground">{{ $t('lock.settingsHint') }}</p>
      </div>
      <button
        class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
        :class="config.enabled ? 'bg-primary' : 'bg-input'"
        @click="toggle('enabled')"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="config.enabled ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>

    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium">{{ $t('lock.autoLock') }}</label>
      <Input
        :model-value="config.auto_lock_seconds"
        type="number"
        min="0"
        max="86400"
        class="h-8"
        @update:model-value="updateField('auto_lock_seconds', Number($event))"
      />
      <span class="text-xs text-muted-foreground">{{ $t('lock.autoLockHint') }}</span>
    </div>

    <div class="flex items-center justify-between">
      <div>
        <label class="text-sm font-medium">{{ $t('lock.biometric') }}</label>
        <p class="text-xs text-muted-foreground">{{ $t('lock.biometricHint') }}</p>
      </div>
      <button
        class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
        :disabled="!security.status.biometric_available"
        :class="config.biometric_enabled ? 'bg-primary' : 'bg-input disabled:opacity-50'"
        @click="toggle('biometric_enabled')"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="config.biometric_enabled ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>

    <div class="grid gap-2">
      <Input
        v-model="currentPassword"
        type="password"
        class="h-8"
        :placeholder="$t('lock.currentPasswordPlaceholder')"
      />
      <Input
        v-model="newPassword"
        type="password"
        class="h-8"
        :placeholder="$t('lock.newPasswordPlaceholder')"
      />
      <Button variant="outline" size="sm" @click="savePassword">{{
        $t('lock.setPassword')
      }}</Button>
    </div>

    <Button variant="outline" size="sm" @click="manualLock">{{ $t('lock.lockNow') }}</Button>
  </div>

  <Separator />

  <!-- Database Encryption -->
  <div class="space-y-3">
    <div class="space-y-1">
      <label class="text-sm font-medium">{{ $t('encryption.settingsTitle') }}</label>
      <p class="text-xs text-muted-foreground">{{ $t('encryption.settingsHint') }}</p>
    </div>

    <div class="flex items-center justify-between">
      <div>
        <label class="text-sm font-medium">{{ $t('encryption.enabled') }}</label>
        <p v-if="security.encryption.migrating" class="text-xs text-yellow-600">
          {{ $t('encryption.migrating') }}
        </p>
        <p v-else-if="security.encryption.enabled" class="text-xs text-muted-foreground">
          {{ $t('encryption.statusEncrypted', { count: security.encryption.encrypted_count }) }}
        </p>
        <p
          v-else-if="security.encryption.plaintext_count > 0"
          class="text-xs text-muted-foreground"
        >
          {{ $t('encryption.statusPlaintext', { count: security.encryption.plaintext_count }) }}
        </p>
      </div>
      <button
        class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
        :class="security.encryption.enabled ? 'bg-primary' : 'bg-input'"
        :disabled="security.encryption.migrating || security.loading"
        @click="toggleEncryption"
      >
        <span
          class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
          :class="security.encryption.enabled ? 'translate-x-4' : 'translate-x-0'"
        />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { useSecurityStore } from '@/stores/securityStore';

interface AppLockConfig {
  enabled: boolean;
  auto_lock_seconds: number;
  biometric_enabled: boolean;
}

const props = defineProps<{ modelValue: AppLockConfig }>();
const emit = defineEmits<{ 'update:modelValue': [value: AppLockConfig]; close: [] }>();

const security = useSecurityStore();

const config = computed(() => props.modelValue);

function updateField<K extends keyof AppLockConfig>(key: K, value: AppLockConfig[K]) {
  emit('update:modelValue', { ...props.modelValue, [key]: value });
}

function toggle(key: 'enabled' | 'biometric_enabled') {
  updateField(key, !props.modelValue[key]);
}

const currentPassword = ref('');
const newPassword = ref('');

async function savePassword() {
  if (!newPassword.value) return;
  await security.setPassword(currentPassword.value || null, newPassword.value);
  currentPassword.value = '';
  newPassword.value = '';
  emit('update:modelValue', { ...props.modelValue, enabled: true });
}

async function manualLock() {
  await security.lock();
  emit('close');
}

async function toggleEncryption() {
  const { t } = useI18n();
  if (security.encryption.enabled) {
    if (!confirm(t('encryption.disableConfirm'))) return;
    try {
      await security.disableEncryption();
    } catch (error) {
      console.error('Failed to disable encryption:', error);
    }
  } else {
    if (!confirm(t('encryption.enableConfirm'))) return;
    try {
      await security.enableEncryption();
    } catch (error) {
      console.error('Failed to enable encryption:', error);
    }
  }
}
</script>
