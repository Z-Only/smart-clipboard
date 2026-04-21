<template>
  <div
    class="fixed inset-0 z-[80] flex items-center justify-center bg-background/95 backdrop-blur-sm"
  >
    <div class="w-[360px] rounded-xl border border-border bg-card p-6 shadow-2xl">
      <div class="mb-5 text-center">
        <h2 class="text-lg font-semibold">{{ $t('lock.title') }}</h2>
        <p class="mt-1 text-sm text-muted-foreground">
          {{ reasonText }}
        </p>
      </div>

      <div class="space-y-3">
        <input
          v-model="password"
          type="password"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
          :placeholder="$t('lock.passwordPlaceholder')"
          @keyup.enter="submitPassword"
        />

        <p v-if="security.error" class="text-sm text-destructive">{{ security.error }}</p>

        <button
          class="w-full h-10 rounded-md bg-primary text-primary-foreground text-sm font-medium"
          @click="submitPassword"
        >
          {{ $t('lock.unlockWithPassword') }}
        </button>

        <button
          v-if="security.status.biometric_available && security.status.biometric_enabled"
          class="w-full h-10 rounded-md border border-input bg-background text-sm font-medium"
          @click="unlockWithBiometric"
        >
          {{ $t('lock.unlockWithBiometric') }}
        </button>
      </div>

      <p class="mt-4 text-center text-xs text-muted-foreground">
        {{ $t('lock.failedAttempts', { count: security.status.failed_attempts }) }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useSecurityStore } from '@/stores/securityStore';

const security = useSecurityStore();
const { t } = useI18n();
const password = ref('');

const reasonText = computed(() => {
  switch (security.status.unlock_reason) {
    case 'startup':
      return t('lock.reasonStartup');
    case 'auto_lock':
      return t('lock.reasonAutoLock');
    case 'manual':
      return t('lock.reasonManual');
    case 'tray_menu':
    case 'shortcut':
    case 'focus':
      return t('lock.reasonProtected');
    default:
      return t('lock.reasonDefault');
  }
});

async function submitPassword() {
  try {
    await security.unlock(password.value, false);
    password.value = '';
  } catch {
    password.value = '';
  }
}

async function unlockWithBiometric() {
  try {
    await security.unlock(null, true);
    password.value = '';
  } catch {
    // backend will fall back to password; keep overlay visible
  }
}
</script>
