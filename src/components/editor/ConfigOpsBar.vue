<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { NButton } from 'naive-ui'
import { Play, Square, Save, RotateCw } from 'lucide-vue-next'

import PendingRestartBanner from '@/components/status/PendingRestartBanner.vue'
import type { ConfigKind } from '@/domain/config'
import type { ProcessKind } from '@/domain/process'
import { useConfigStore } from '@/stores/config'
import { useProcessStore } from '@/stores/process'
import type { ConfigEditorMode } from '@/components/editor/ConfigModeToggle.vue'

const props = defineProps<{
  kind: ConfigKind
  mode: ConfigEditorMode
  dirty: boolean
  busy: boolean
}>()

const emit = defineEmits<{
  start: []
  stop: []
  save: []
  saveAndRestart: []
}>()

const { t } = useI18n()
const processStore = useProcessStore()
const configStore = useConfigStore()

const {
  pendingRestart,
  frpcLoading,
  frpsLoading,
  frpcRunning,
  frpsRunning,
} = storeToRefs(processStore)

const { frpc: frpcConfig, frps: frpsConfig } = storeToRefs(configStore)

const processLoading = computed(() =>
  props.kind === 'frpc' ? frpcLoading.value : frpsLoading.value,
)

const running = computed(() =>
  props.kind === 'frpc' ? frpcRunning.value : frpsRunning.value,
)

const kindPending = computed(() =>
  props.kind === 'frpc'
    ? pendingRestart.value.frpc
    : pendingRestart.value.frps,
)

const revision = computed(() => {
  const snapshot =
    props.kind === 'frpc' ? frpcConfig.value : frpsConfig.value
  return snapshot?.revision
})

const shortRevision = computed(() => {
  const value = revision.value
  if (!value) return t('overview.valueNone')
  return value.length > 12 ? `${value.slice(0, 12)}…` : value
})

const actionsDisabled = computed(
  () => props.busy || processLoading.value,
)

const startLabel = computed(() =>
  props.kind === 'frpc'
    ? t('dashboard.startFrpc')
    : t('dashboard.startFrps'),
)

const stopLabel = computed(() =>
  props.kind === 'frpc' ? t('dashboard.stopFrpc') : t('dashboard.stopFrps'),
)

const saveAndRestartLabel = computed(() =>
  props.kind === 'frpc'
    ? t('overview.saveAndRestartFrpc')
    : t('overview.saveAndRestartFrps'),
)

const onStart = () => {
  emit('start')
}

const onStop = () => {
  emit('stop')
}

const onSave = () => {
  emit('save')
}

const onSaveAndRestart = () => {
  if (props.dirty) {
    emit('saveAndRestart')
    return
  }
  if (kindPending.value) {
    void processStore.restart(props.kind)
  }
}

const onBannerRestart = (target: ProcessKind) => {
  if (target !== props.kind) return
  void processStore.restart(target)
}
</script>

<template>
  <div class="config-ops-bar">
    <div class="config-ops-bar__row">
      <div class="config-ops-bar__actions">
        <NButton
          type="primary"
          size="small"
          :loading="processLoading && !running"
          :disabled="running || actionsDisabled"
          :aria-label="startLabel"
          @click="onStart"
        >
          <template #icon>
            <Play :size="14" aria-hidden="true" />
          </template>
          {{ startLabel }}
        </NButton>

        <NButton
          type="error"
          ghost
          size="small"
          :loading="processLoading && running"
          :disabled="!running || actionsDisabled"
          :aria-label="stopLabel"
          @click="onStop"
        >
          <template #icon>
            <Square :size="14" aria-hidden="true" />
          </template>
          {{ stopLabel }}
        </NButton>

        <NButton
          size="small"
          :loading="busy"
          :disabled="!dirty || actionsDisabled"
          :aria-label="t('forms.save')"
          @click="onSave"
        >
          <template #icon>
            <Save :size="14" aria-hidden="true" />
          </template>
          {{ t('forms.save') }}
        </NButton>

        <NButton
          type="warning"
          secondary
          size="small"
          :loading="busy"
          :disabled="(!dirty && !kindPending) || actionsDisabled"
          :aria-label="saveAndRestartLabel"
          @click="onSaveAndRestart"
        >
          <template #icon>
            <RotateCw :size="14" aria-hidden="true" />
          </template>
          {{ t('overview.saveAndRestart') }}
        </NButton>

        <span
          v-if="dirty"
          class="config-ops-bar__dirty"
          role="status"
        >
          {{ t('editor.unsavedBadge') }}
        </span>
      </div>

      <div class="config-ops-bar__meta">
        <span class="config-ops-bar__mode" :title="t('editor.modeGroup')">
          {{
            mode === 'form' ? t('editor.formMode') : t('editor.sourceMode')
          }}
        </span>
        <span class="config-ops-bar__revision" :title="revision ?? undefined">
          <span class="config-ops-bar__revision-label">
            {{ t('overview.configRevision') }}
          </span>
          <code class="config-ops-bar__revision-hash">{{ shortRevision }}</code>
        </span>
      </div>
    </div>

    <PendingRestartBanner
      :pending-frpc="kind === 'frpc' && pendingRestart.frpc"
      :pending-frps="kind === 'frps' && pendingRestart.frps"
      :loading-frpc="frpcLoading"
      :loading-frps="frpsLoading"
      @restart="onBannerRestart"
    />
  </div>
</template>

<style scoped>
.config-ops-bar {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px var(--ops-gap);
  border-bottom: 1px solid var(--ops-border);
  background: var(--ops-surface);
  position: sticky;
  top: 0;
  z-index: 2;
}

.config-ops-bar__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.config-ops-bar__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
}

.config-ops-bar__dirty {
  font-size: 12px;
  font-weight: 600;
  color: var(--ops-warn);
}

.config-ops-bar__meta {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  color: var(--ops-muted);
  font-size: 12px;
  margin-left: auto;
}

.config-ops-bar__mode {
  padding: 2px 8px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  color: var(--ops-text);
  background: var(--ops-bg);
}

.config-ops-bar__revision {
  display: inline-flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}

.config-ops-bar__revision-label {
  flex-shrink: 0;
}

.config-ops-bar__revision-hash {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  color: var(--ops-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 16ch;
}
</style>
