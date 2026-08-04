<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDialog, useMessage, NButton } from 'naive-ui'

import type {
  ConfigKind,
  ConfigPreview,
  ValidationIssue,
} from '@/domain/config'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'
import { useConfigStore } from '@/stores/config'

const props = withDefaults(
  defineProps<{
    kind: ConfigKind
    draft: string
    expectedRevision: string
    disabled?: boolean
  }>(),
  {
    disabled: false,
  },
)

const emit = defineEmits<{
  /** New snapshot.raw after successful apply; parent resets draft + baseline. */
  applied: [raw: string]
  /** Revision conflict: store reloaded; parent should reset draft from snapshot. */
  conflict: []
}>()

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()
const configStore = useConfigStore()

const issues = ref<ValidationIssue[]>([])
const validatedOk = ref(false)
const preview = ref<ConfigPreview | null>(null)
const busyAction = ref<'validate' | 'preview' | 'apply' | 'restart' | null>(
  null,
)

const busy = computed(() => busyAction.value !== null)
const actionsDisabled = computed(() => props.disabled || busy.value)

const previewReady = computed(
  () =>
    validatedOk.value &&
    preview.value != null &&
    !preview.value.issues.some((issue) => issue.severity === 'error'),
)

const invalidatePipeline = () => {
  validatedOk.value = false
  preview.value = null
  issues.value = []
}

watch(
  () => [props.draft, props.expectedRevision, props.kind] as const,
  () => {
    invalidatePipeline()
  },
)

const showCommandError = (error: unknown) => {
  message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
}

const formatIssueLocation = (issue: ValidationIssue) => {
  if (issue.line == null && issue.column == null) return ''
  const line = issue.line ?? '?'
  const column = issue.column ?? '?'
  return `${line}:${column}`
}

const onValidate = async (): Promise<boolean> => {
  busyAction.value = 'validate'
  preview.value = null
  try {
    const report = await tauriClient.validateConfigSource(
      props.kind,
      props.draft,
    )
    issues.value = report.issues
    validatedOk.value = !report.issues.some(
      (issue) => issue.severity === 'error',
    )
    if (validatedOk.value) {
      message.success(t('editor.validateOk'))
    } else {
      message.error(t('editor.validateFailed'))
    }
    return validatedOk.value
  } catch (error) {
    validatedOk.value = false
    issues.value = []
    showCommandError(error)
    return false
  } finally {
    busyAction.value = null
  }
}

const onPreview = async (): Promise<boolean> => {
  if (!validatedOk.value) {
    message.warning(t('editor.previewNeedValidate'))
    return false
  }

  busyAction.value = 'preview'
  try {
    const result = await tauriClient.previewConfigChange(
      props.kind === 'frpc'
        ? {
            kind: 'frpc',
            expectedRevision: props.expectedRevision,
            change: { mode: 'source', raw: props.draft },
          }
        : {
            kind: 'frps',
            expectedRevision: props.expectedRevision,
            change: { mode: 'source', raw: props.draft },
          },
    )
    preview.value = result
    if (result.issues.length > 0) {
      issues.value = result.issues
    }
    if (result.issues.some((issue) => issue.severity === 'error')) {
      message.error(t('editor.previewFailed'))
      return false
    }
    message.success(t('editor.previewOk'))
    return true
  } catch (error) {
    preview.value = null
    const commandError = normalizeCommandError(error)
    if (commandError.code === 'CONFIG_CONFLICT') {
      message.warning(t('errors.CONFIG_CONFLICT'))
      try {
        await configStore.load(props.kind)
      } catch (loadError) {
        showCommandError(loadError)
      }
      emit('conflict')
      return false
    }
    showCommandError(error)
    return false
  } finally {
    busyAction.value = null
  }
}

const confirmApply = (andRestart: boolean) =>
  new Promise<boolean>((resolve) => {
    dialog.warning({
      title: andRestart
        ? t('editor.applyRestartConfirmTitle')
        : t('editor.applyConfirmTitle'),
      content: andRestart
        ? t('editor.applyRestartConfirmContent')
        : t('editor.applyConfirmContent'),
      positiveText: andRestart
        ? t('editor.applyAndRestart')
        : t('editor.apply'),
      negativeText: t('forms.cancel'),
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
    })
  })

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

const runApply = async (andRestart: boolean): Promise<boolean> => {
  if (!previewReady.value) {
    message.warning(t('editor.applyNeedPreview'))
    return false
  }

  const confirmed = await confirmApply(andRestart)
  if (!confirmed) return false

  busyAction.value = andRestart ? 'restart' : 'apply'
  try {
    if (andRestart) {
      const result = await configStore.saveAndRestart(
        props.kind === 'frpc'
          ? {
              kind: 'frpc',
              expectedRevision: props.expectedRevision,
              change: { mode: 'source', raw: props.draft },
            }
          : {
              kind: 'frps',
              expectedRevision: props.expectedRevision,
              change: { mode: 'source', raw: props.draft },
            },
      )
      if (!result.ok) {
        reportSaveAndRestartFailure(result.error, result.recovery)
        if (result.error.code === 'CONFIG_CONFLICT') {
          try {
            await configStore.load(props.kind)
          } catch (loadError) {
            showCommandError(loadError)
          }
          emit('conflict')
        }
        return false
      }
      emit('applied', result.data.config.raw)
      message.success(t('feedback.saveAndRestartSuccess'))
      invalidatePipeline()
      return true
    }

    const result = await configStore.applySource(
      props.kind,
      props.expectedRevision,
      props.draft,
    )
    if (!result.ok) {
      showCommandError(result.error)
      if (result.error.code === 'CONFIG_CONFLICT') {
        try {
          await configStore.load(props.kind)
        } catch (loadError) {
          showCommandError(loadError)
        }
        emit('conflict')
      }
      return false
    }
    emit('applied', result.data.raw)
    message.success(t('feedback.saveSuccess'))
    invalidatePipeline()
    return true
  } finally {
    busyAction.value = null
  }
}

const apply = () => runApply(false)
const applyAndRestart = () => runApply(true)

defineExpose({
  apply,
  applyAndRestart,
  validate: onValidate,
  preview: onPreview,
})
</script>

<template>
  <div class="source-apply-panel">
    <div class="source-apply-panel__actions">
      <NButton
        size="small"
        :disabled="actionsDisabled"
        :loading="busyAction === 'validate'"
        :aria-label="t('editor.validate')"
        @click="onValidate"
      >
        {{ t('editor.validate') }}
      </NButton>
      <NButton
        size="small"
        :disabled="actionsDisabled || !validatedOk"
        :loading="busyAction === 'preview'"
        :aria-label="t('editor.preview')"
        @click="onPreview"
      >
        {{ t('editor.preview') }}
      </NButton>
      <NButton
        type="primary"
        size="small"
        :disabled="actionsDisabled || !previewReady"
        :loading="busyAction === 'apply'"
        :aria-label="t('editor.apply')"
        @click="apply"
      >
        {{ t('editor.apply') }}
      </NButton>
      <NButton
        type="warning"
        size="small"
        secondary
        :disabled="actionsDisabled || !previewReady"
        :loading="busyAction === 'restart'"
        :aria-label="t('editor.applyAndRestart')"
        @click="applyAndRestart"
      >
        {{ t('editor.applyAndRestart') }}
      </NButton>
    </div>

    <p
      v-if="!validatedOk && issues.length === 0 && !preview"
      class="source-apply-panel__hint"
    >
      {{ t('editor.applyPipelineHint') }}
    </p>

    <div
      v-if="issues.length > 0"
      class="source-apply-panel__issues"
      role="list"
      :aria-label="t('editor.issuesLabel')"
    >
      <div
        v-for="(issue, index) in issues"
        :key="`${issue.code}-${index}`"
        class="source-apply-panel__issue"
        :class="`source-apply-panel__issue--${issue.severity}`"
        role="listitem"
      >
        <span class="source-apply-panel__issue-sev">
          {{ issue.severity }}
        </span>
        <span
          v-if="formatIssueLocation(issue)"
          class="source-apply-panel__issue-loc"
        >
          {{ formatIssueLocation(issue) }}
        </span>
        <span class="source-apply-panel__issue-msg">{{ issue.message }}</span>
      </div>
    </div>

    <div
      v-if="preview"
      class="source-apply-panel__preview"
    >
      <div class="source-apply-panel__preview-head">
        <h4 class="source-apply-panel__preview-title">
          {{ t('editor.previewDiff') }}
        </h4>
        <span
          v-if="preview.diff.changedPaths.length"
          class="source-apply-panel__paths"
        >
          {{ preview.diff.changedPaths.join(', ') }}
        </span>
      </div>
      <pre class="source-apply-panel__diff">{{ preview.diff.unified }}</pre>
    </div>
  </div>
</template>

<style scoped>
.source-apply-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.source-apply-panel__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.source-apply-panel__hint {
  margin: 0;
  font-size: 12px;
  color: var(--ops-muted);
}

.source-apply-panel__issues {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 160px;
  overflow: auto;
  padding: 8px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
}

.source-apply-panel__issue {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: baseline;
  font-size: 12px;
  color: var(--ops-text);
}

.source-apply-panel__issue-sev {
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  font-size: 11px;
}

.source-apply-panel__issue--error .source-apply-panel__issue-sev {
  color: var(--ops-danger);
}

.source-apply-panel__issue--warning .source-apply-panel__issue-sev {
  color: var(--ops-warn);
}

.source-apply-panel__issue-loc {
  font-family: var(--font-mono, ui-monospace, monospace);
  color: var(--ops-muted);
}

.source-apply-panel__issue-msg {
  flex: 1 1 180px;
  min-width: 0;
}

.source-apply-panel__preview {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.source-apply-panel__preview-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.source-apply-panel__preview-title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--ops-text);
}

.source-apply-panel__paths {
  font-size: 11px;
  color: var(--ops-muted);
}

.source-apply-panel__diff {
  margin: 0;
  max-height: 220px;
  overflow: auto;
  padding: 10px 12px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
  color: var(--ops-text);
  font-family: var(--font-mono, 'Fira Code', ui-monospace, monospace);
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
