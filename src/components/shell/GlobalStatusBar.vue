<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'

import ProcessPhaseBadge from '@/components/status/ProcessPhaseBadge.vue'
import { getCommandErrorI18nKey } from '@/services/errorMapper'
import { useProcessStore } from '@/stores/process'

const { t } = useI18n()
const processStore = useProcessStore()
const { frpc, frps, lastFault, pendingRestart } = storeToRefs(processStore)

const pendingRestartLabel = computed(() => {
  const parts: string[] = []
  if (pendingRestart.value.frpc) parts.push('frpc')
  if (pendingRestart.value.frps) parts.push('frps')
  if (parts.length === 0) return ''
  return t('status.pendingRestart', { targets: parts.join(' / ') })
})

const faultText = computed(() => {
  if (!lastFault.value) return ''
  return t(getCommandErrorI18nKey(lastFault.value))
})
</script>

<template>
  <div class="global-status-bar" role="status" aria-live="polite">
    <div class="global-status-bar__item">
      <span class="global-status-bar__label">{{ t('status.frpcLabel') }}</span>
      <ProcessPhaseBadge :snapshot="frpc" />
      <span
        v-if="pendingRestart.frpc"
        class="global-status-bar__pending"
      >
        {{ t('status.pendingRestartShort') }}
      </span>
    </div>

    <div class="global-status-bar__divider" aria-hidden="true" />

    <div class="global-status-bar__item">
      <span class="global-status-bar__label">{{ t('status.frpsLabel') }}</span>
      <ProcessPhaseBadge :snapshot="frps" />
      <span
        v-if="pendingRestart.frps"
        class="global-status-bar__pending"
      >
        {{ t('status.pendingRestartShort') }}
      </span>
    </div>

    <div class="global-status-bar__divider" aria-hidden="true" />

    <div class="global-status-bar__item global-status-bar__item--fault">
      <span class="global-status-bar__label">{{ t('status.fault') }}</span>
      <template v-if="lastFault">
        <span class="global-status-bar__fault" :title="faultText">
          {{ faultText }}
        </span>
        <RouterLink
          class="global-status-bar__link"
          to="/logs"
          :aria-label="t('status.viewLogs')"
        >
          {{ t('status.viewLogs') }}
        </RouterLink>
      </template>
      <span v-else class="global-status-bar__value">{{ t('status.noFault') }}</span>
    </div>

    <span
      v-if="pendingRestartLabel"
      class="global-status-bar__pending-banner"
    >
      {{ pendingRestartLabel }}
    </span>
  </div>
</template>

<style scoped>
.global-status-bar {
  height: var(--ops-topbar-height);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 var(--ops-gap);
  border-bottom: 1px solid var(--ops-border);
  background: var(--ops-surface);
  color: var(--ops-muted);
  font-size: 12px;
  min-width: 0;
}

.global-status-bar__item {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.global-status-bar__item--fault {
  flex: 1;
  min-width: 0;
}

.global-status-bar__label {
  font-weight: 500;
  flex-shrink: 0;
}

.global-status-bar__value {
  color: var(--ops-text);
  font-variant-numeric: tabular-nums;
}

.global-status-bar__fault {
  color: var(--ops-danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.global-status-bar__link {
  color: var(--ops-accent);
  text-decoration: none;
  flex-shrink: 0;
}

.global-status-bar__link:hover {
  text-decoration: underline;
}

.global-status-bar__pending {
  color: var(--ops-warn);
  flex-shrink: 0;
}

.global-status-bar__pending-banner {
  margin-left: auto;
  color: var(--ops-warn);
  flex-shrink: 0;
}

.global-status-bar__divider {
  width: 1px;
  height: 14px;
  background: var(--ops-border);
  flex-shrink: 0;
}
</style>
