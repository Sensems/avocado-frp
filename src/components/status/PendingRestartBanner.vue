<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { NButton } from 'naive-ui'
import { RotateCw } from 'lucide-vue-next'

import type { ProcessKind } from '@/domain/process'

const props = defineProps<{
  pendingFrpc?: boolean
  pendingFrps?: boolean
  loadingFrpc?: boolean
  loadingFrps?: boolean
}>()

const emit = defineEmits<{
  restart: [kind: ProcessKind]
}>()

const { t } = useI18n()

const visible = computed(() => Boolean(props.pendingFrpc || props.pendingFrps))

const targetsLabel = computed(() => {
  const parts: string[] = []
  if (props.pendingFrpc) parts.push('frpc')
  if (props.pendingFrps) parts.push('frps')
  return parts.join(' / ')
})
</script>

<template>
  <div
    v-if="visible"
    class="pending-restart-banner"
    role="status"
    aria-live="polite"
  >
    <div class="pending-restart-banner__text">
      <span class="pending-restart-banner__title">
        {{ t('status.pendingRestart', { targets: targetsLabel }) }}
      </span>
      <span class="pending-restart-banner__hint">
        {{ t('overview.pendingRestartHint') }}
      </span>
    </div>

    <div class="pending-restart-banner__actions">
      <NButton
        v-if="pendingFrpc"
        size="small"
        type="warning"
        secondary
        :loading="loadingFrpc"
        :disabled="loadingFrpc"
        :aria-label="t('overview.restartFrpc')"
        @click="emit('restart', 'frpc')"
      >
        <template #icon>
          <RotateCw :size="14" aria-hidden="true" />
        </template>
        {{ t('overview.restartFrpc') }}
      </NButton>

      <NButton
        v-if="pendingFrps"
        size="small"
        type="warning"
        secondary
        :loading="loadingFrps"
        :disabled="loadingFrps"
        :aria-label="t('overview.restartFrps')"
        @click="emit('restart', 'frps')"
      >
        <template #icon>
          <RotateCw :size="14" aria-hidden="true" />
        </template>
        {{ t('overview.restartFrps') }}
      </NButton>

      <RouterLink
        v-if="pendingFrpc"
        class="pending-restart-banner__link"
        to="/client"
        :aria-label="t('overview.openClient')"
      >
        {{ t('overview.openClient') }}
      </RouterLink>
      <RouterLink
        v-if="pendingFrps"
        class="pending-restart-banner__link"
        to="/server"
        :aria-label="t('overview.openServer')"
      >
        {{ t('overview.openServer') }}
      </RouterLink>
    </div>
  </div>
</template>

<style scoped>
.pending-restart-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--ops-warn) 35%, var(--ops-border));
  border-radius: var(--ops-radius);
  background: color-mix(in srgb, var(--ops-warn) 10%, var(--ops-surface));
  color: var(--ops-text);
}

.pending-restart-banner__text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.pending-restart-banner__title {
  font-size: 13px;
  font-weight: 600;
  color: var(--ops-warn);
}

.pending-restart-banner__hint {
  font-size: 12px;
  color: var(--ops-muted);
}

.pending-restart-banner__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.pending-restart-banner__link {
  font-size: 12px;
  color: var(--ops-accent);
  text-decoration: none;
}

.pending-restart-banner__link:hover {
  text-decoration: underline;
}
</style>
