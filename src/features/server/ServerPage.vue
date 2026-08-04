<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { onBeforeRouteLeave, RouterLink } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { useDialog, useMessage, NButton } from 'naive-ui'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { Download } from 'lucide-vue-next'

import ConfigModeToggle, {
  type ConfigEditorMode,
} from '@/components/editor/ConfigModeToggle.vue'
import ConfigOpsBar from '@/components/editor/ConfigOpsBar.vue'
import SourceApplyPanel from '@/components/editor/SourceApplyPanel.vue'
import TomlSourceEditor from '@/components/editor/TomlSourceEditor.vue'
import {
  attachBeforeUnload,
  unsavedGuard,
  type UnsavedDialogApi,
} from '@/components/feedback/unsavedGuard'
import FrpsConfigForm from '@/components/FrpsConfigForm.vue'
import ProcessPhaseBadge from '@/components/status/ProcessPhaseBadge.vue'
import { buildFrpsPatch, type FrpsFormData } from '@/domain/config'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'
import { useConfigStore } from '@/stores/config'
import { useLogsStore } from '@/stores/logs'
import { useProcessStore } from '@/stores/process'

const RECENT_LOG_LIMIT = 12
const FRP_SAMPLE_VERSION = '0.67.0'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog() as UnsavedDialogApi

const configStore = useConfigStore()
const processStore = useProcessStore()
const logsStore = useLogsStore()

const { frps: frpsSnapshot } = storeToRefs(configStore)
const { frps: frpsProcess } = storeToRefs(processStore)
const { entries } = storeToRefs(logsStore)

const mode = ref<ConfigEditorMode>('form')
const formDirty = ref(false)
const busy = ref(false)

const sourceDraft = ref('')
const sourceBaseline = ref('')

const formRef = ref<{
  getFormData: () => FrpsFormData
  isDirty: () => boolean
} | null>(null)

const sourceApplyRef = ref<{
  apply: () => Promise<boolean>
  applyAndRestart: () => Promise<boolean>
} | null>(null)

const sourceDirty = computed(
  () => sourceDraft.value !== sourceBaseline.value,
)

const pageDirty = computed(() =>
  mode.value === 'form' ? formDirty.value : sourceDirty.value,
)

const leaveDirty = computed(() => formDirty.value || sourceDirty.value)

const recentFrpsLogs = computed(() =>
  [...entries.value]
    .filter((entry) => entry.source === 'frps')
    .slice(-RECENT_LOG_LIMIT)
    .reverse(),
)

const showCommandError = (error: unknown) => {
  message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
}

const handleConflictReload = async (code: string) => {
  if (code !== 'CONFIG_CONFLICT') return
  message.warning(t('errors.CONFIG_CONFLICT'))
  try {
    await configStore.load('frps')
  } catch (error) {
    showCommandError(error)
  }
}

const applyFrpsPatch = async (
  formData: FrpsFormData,
): Promise<boolean> => {
  const current = frpsSnapshot.value
  if (!current) {
    try {
      await configStore.load('frps')
    } catch {
      return false
    }
  }
  const snapshot = frpsSnapshot.value
  if (!snapshot) return false

  busy.value = true
  try {
    const result = await configStore.applyPatch(
      'frps',
      snapshot.revision,
      buildFrpsPatch(formData),
    )
    if (!result.ok) {
      showCommandError(result.error)
      await handleConflictReload(result.error.code)
      return false
    }
    return true
  } finally {
    busy.value = false
  }
}

const onStart = async () => {
  const result = await processStore.start('frps')
  if (result.ok) {
    message.success(t('feedback.startSuccess', { name: 'frps' }))
  } else {
    message.error(
      t('feedback.startFail', {
        name: 'frps',
        error: t(getCommandErrorI18nKey(result.error)),
      }),
    )
  }
}

const onStop = async () => {
  const result = await processStore.stop('frps')
  if (result.ok) {
    message.success(t('feedback.stopSuccess', { name: 'frps' }))
  } else {
    message.error(
      t('feedback.stopFail', {
        name: 'frps',
        error: t(getCommandErrorI18nKey(result.error)),
      }),
    )
  }
}

const resolveFormData = (): FrpsFormData | null => {
  if (formRef.value) return formRef.value.getFormData()
  const known = frpsSnapshot.value?.known
  if (!known) return null
  return {
    bindPort: known.bindPort ?? '',
    vhostHttpPort: known.vhostHTTPPort ?? '',
    vhostHttpsPort: known.vhostHTTPSPort ?? '',
    authMethod: known.auth.method ?? 'token',
    token: known.auth.token ?? '',
    dashboardPort: known.webServer.port ?? '',
    dashboardUser: known.webServer.user ?? '',
    dashboardPwd: known.webServer.password ?? '',
  }
}

const onSaveForm = async (configData: FrpsFormData) => {
  if (await applyFrpsPatch(configData)) {
    message.success(t('feedback.saveSuccess'))
  }
}

const onSourceApplied = (raw: string) => {
  sourceDraft.value = raw
  sourceBaseline.value = raw
}

const onSourceConflict = () => {
  const raw = frpsSnapshot.value?.raw ?? ''
  sourceDraft.value = raw
  sourceBaseline.value = raw
}

const onSave = async () => {
  if (mode.value === 'source') {
    if (!sourceApplyRef.value) {
      message.info(t('editor.opsBarSourceHint'))
      return
    }
    busy.value = true
    try {
      await sourceApplyRef.value.apply()
    } finally {
      busy.value = false
    }
    return
  }

  const formData = resolveFormData()
  if (!formData) return
  await onSaveForm(formData)
}

const reportSaveAndRestartFailure = (
  error: Parameters<typeof getCommandErrorI18nKey>[0],
  recovery?: {
    configRestored: boolean
    processRestored: boolean
  },
) => {
  message.error(
    t('feedback.saveAndRestartFail', {
      error: t(getCommandErrorI18nKey(error)),
    }),
  )
  if (recovery) {
    message.warning(
      t('feedback.recoveryHint', {
        config: recovery.configRestored
          ? t('feedback.recovered')
          : t('feedback.notRecovered'),
        process: recovery.processRestored
          ? t('feedback.recovered')
          : t('feedback.notRecovered'),
      }),
    )
  }
}

const onSaveAndRestart = async () => {
  const current = frpsSnapshot.value
  if (!current) return

  busy.value = true
  try {
    if (mode.value === 'source') {
      if (!sourceApplyRef.value) {
        message.info(t('editor.opsBarSourceHint'))
        return
      }
      await sourceApplyRef.value.applyAndRestart()
      return
    }

    const formData = resolveFormData()
    if (!formData) return

    const result = await configStore.saveAndRestart({
      kind: 'frps',
      expectedRevision: current.revision,
      change: {
        mode: 'patch',
        patch: buildFrpsPatch(formData),
      },
    })
    if (!result.ok) {
      reportSaveAndRestartFailure(result.error, result.recovery)
      await handleConflictReload(result.error.code)
      return
    }
    message.success(t('feedback.saveAndRestartSuccess'))
  } finally {
    busy.value = false
  }
}

const onEnterSource = () => {
  // Form is torn down via v-if; clear so leave-guard does not false-positive.
  formDirty.value = false
  const raw = frpsSnapshot.value?.raw ?? ''
  sourceDraft.value = raw
  sourceBaseline.value = raw
}

const onDiscardSource = () => {
  sourceDraft.value = sourceBaseline.value
}

const confirmExportDeploy = (): Promise<boolean> =>
  new Promise((resolve) => {
    dialog.warning({
      title: t('server.exportTitle'),
      content: t('server.exportNote', { version: FRP_SAMPLE_VERSION }),
      positiveText: t('server.exportContinue'),
      negativeText: t('forms.cancel'),
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
    })
  })

const onExportDeploy = async () => {
  if (!frpsSnapshot.value) {
    message.warning(t('server.exportNeedConfig'))
    return
  }

  const confirmed = await confirmExportDeploy()
  if (!confirmed) return

  busy.value = true
  try {
    const selectedDir = await openDialog({
      directory: true,
      multiple: false,
      title: t('server.exportPickDir'),
    })
    if (typeof selectedDir !== 'string') return

    await tauriClient.exportDeployScript(
      selectedDir,
      frpsSnapshot.value.raw,
    )
    message.success(t('feedback.exportSuccess'))
  } catch (error) {
    const commandError = normalizeCommandError(error)
    message.error(
      t('feedback.exportFail', {
        error: t(getCommandErrorI18nKey(commandError)),
      }),
    )
  } finally {
    busy.value = false
  }
}

onBeforeRouteLeave(unsavedGuard(() => leaveDirty.value, dialog, t))

let detachBeforeUnload: (() => void) | undefined

onMounted(() => {
  detachBeforeUnload = attachBeforeUnload(() => leaveDirty.value)
  if (!frpsSnapshot.value) {
    void configStore.load('frps').catch(() => {
      /* App shell already surfaces load failures */
    })
  }
  void logsStore.init().catch(() => {
    /* App shell may already own the log listener */
  })
})

onUnmounted(() => {
  detachBeforeUnload?.()
})
</script>

<template>
  <div class="server-page">
    <ConfigOpsBar
      kind="frps"
      :mode="mode"
      :dirty="pageDirty"
      :busy="busy"
      @start="onStart"
      @stop="onStop"
      @save="onSave"
      @save-and-restart="onSaveAndRestart"
    />

    <div class="server-page__body">
      <header class="server-page__header">
        <div>
          <h2 class="ops-page__title">{{ t('nav.server') }}</h2>
          <p class="server-page__subtitle">{{ t('server.subtitle') }}</p>
        </div>
        <div class="server-page__header-actions">
          <NButton
            size="small"
            secondary
            :disabled="busy || !frpsSnapshot"
            :aria-label="t('dashboard.exportDeploy')"
            @click="onExportDeploy"
          >
            <template #icon>
              <Download
                :size="14"
                aria-hidden="true"
              />
            </template>
            {{ t('dashboard.exportDeploy') }}
          </NButton>
          <ConfigModeToggle
            v-model="mode"
            :source-dirty="sourceDirty"
            :form-dirty="formDirty"
            :disabled="busy"
            @enter-source="onEnterSource"
            @discard-source="onDiscardSource"
          />
        </div>
      </header>

      <div class="server-page__layout">
        <div class="server-page__main">
          <section
            v-if="mode === 'form'"
            class="ops-card"
          >
            <div class="ops-card__header">
              <h3 class="ops-card__title">{{ t('server.serverConfig') }}</h3>
            </div>
            <FrpsConfigForm
              ref="formRef"
              :initial-data="frpsSnapshot?.known"
              hide-save
              @update:dirty="formDirty = $event"
              @save="onSaveForm"
            />
          </section>

          <section
            v-else
            class="ops-card server-page__source"
          >
            <div class="ops-card__header">
              <h3 class="ops-card__title">{{ t('editor.sourceMode') }}</h3>
            </div>
            <p class="server-page__source-hint">
              {{ t('server.sourceHint') }}
            </p>
            <TomlSourceEditor
              v-model="sourceDraft"
              :disabled="busy"
              :aria-label="t('editor.sourceMode')"
            />
            <SourceApplyPanel
              ref="sourceApplyRef"
              kind="frps"
              :draft="sourceDraft"
              :expected-revision="frpsSnapshot?.revision ?? ''"
              :disabled="busy || !frpsSnapshot"
              @applied="onSourceApplied"
              @conflict="onSourceConflict"
            />
          </section>
        </div>

        <aside class="server-page__aside">
          <article class="ops-card">
            <div class="ops-card__header">
              <h3 class="ops-card__title">{{ t('server.runtimeStatus') }}</h3>
              <ProcessPhaseBadge :snapshot="frpsProcess" />
            </div>
            <dl class="ops-meta">
              <div class="ops-meta__row">
                <dt>{{ t('overview.lastExitCode') }}</dt>
                <dd>
                  {{
                    frpsProcess?.lastExitCode ?? t('overview.valueNone')
                  }}
                </dd>
              </div>
            </dl>
          </article>

          <article class="ops-card server-page__logs">
            <div class="ops-card__header">
              <h3 class="ops-card__title">{{ t('server.recentLogs') }}</h3>
              <RouterLink
                class="ops-card__link"
                to="/logs"
                :aria-label="t('server.openLogs')"
              >
                {{ t('server.openLogs') }}
              </RouterLink>
            </div>
            <ul
              v-if="recentFrpsLogs.length > 0"
              class="log-slice"
            >
              <li
                v-for="(entry, index) in recentFrpsLogs"
                :key="`${entry.timestamp}-${index}`"
                class="log-slice__item"
                :class="{
                  'log-slice__item--err': entry.type === 'err',
                }"
              >
                <span class="log-slice__meta">{{ entry.time }}</span>
                <span
                  class="log-slice__text"
                  :title="entry.text"
                >{{ entry.text }}</span>
              </li>
            </ul>
            <p
              v-else
              class="ops-card__empty"
            >
              {{ t('server.noRecentLogs') }}
            </p>
          </article>
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.server-page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.server-page__body {
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
  padding: var(--ops-gap);
}

.server-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.server-page__header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.ops-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--ops-text);
}

.server-page__subtitle {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.server-page__layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(240px, 320px);
  gap: var(--ops-gap);
  align-items: start;
}

.server-page__main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
}

.server-page__aside {
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
  min-width: 0;
}

.ops-card {
  background: var(--ops-surface);
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  padding: 14px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ops-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.ops-card__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--ops-text);
}

.ops-card__link {
  font-size: 12px;
  color: var(--ops-accent, #2563eb);
  text-decoration: none;
}

.ops-card__link:hover {
  text-decoration: underline;
}

.ops-card__empty {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
  padding: 12px 0;
  text-align: center;
}

.ops-meta {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ops-meta__row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
}

.ops-meta__row dt {
  color: var(--ops-muted);
  margin: 0;
}

.ops-meta__row dd {
  margin: 0;
  color: var(--ops-text);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.server-page__source-hint {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.server-page__logs {
  max-height: 420px;
}

.log-slice {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow: auto;
  max-height: 340px;
}

.log-slice__item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
  border: 1px solid var(--ops-border);
  font-size: 12px;
}

.log-slice__item--err {
  border-color: color-mix(in srgb, var(--ops-danger) 40%, var(--ops-border));
}

.log-slice__meta {
  color: var(--ops-muted);
  font-variant-numeric: tabular-nums;
}

.log-slice__text {
  color: var(--ops-text);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (max-width: 960px) {
  .server-page__layout {
    grid-template-columns: 1fr;
  }
}
</style>
