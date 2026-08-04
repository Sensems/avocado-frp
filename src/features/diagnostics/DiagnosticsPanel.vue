<script setup lang="ts">
import { computed, type Component } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { NButton, useMessage } from 'naive-ui'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import {
  AlertTriangle,
  CheckCircle2,
  Download,
  Play,
  XCircle,
} from 'lucide-vue-next'

import type { DiagnosticResult, DiagnosticStatus } from '@/domain/diagnostics'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { useDiagnosticsStore } from '@/stores/diagnostics'

const { t, te } = useI18n()
const message = useMessage()
const diagnosticsStore = useDiagnosticsStore()
const { report, running, exporting, finishedAt } = storeToRefs(diagnosticsStore)

const lastRunLabel = computed(() => {
  if (!finishedAt.value) return null
  const parsed = new Date(finishedAt.value)
  const time = Number.isNaN(parsed.getTime())
    ? finishedAt.value
    : parsed.toLocaleString()
  return t('diagnostics.lastRun', { time })
})

const statusMeta = (
  status: DiagnosticStatus,
): { icon: Component; labelKey: string; tone: string } => {
  switch (status) {
    case 'pass':
      return {
        icon: CheckCircle2,
        labelKey: 'diagnostics.statusPass',
        tone: 'diag-status--pass',
      }
    case 'warning':
      return {
        icon: AlertTriangle,
        labelKey: 'diagnostics.statusWarning',
        tone: 'diag-status--warning',
      }
    case 'fail':
      return {
        icon: XCircle,
        labelKey: 'diagnostics.statusFail',
        tone: 'diag-status--fail',
      }
  }
}

const checkTitle = (result: DiagnosticResult): string => {
  if (result.titleKey) {
    const key = `diagnostics.checks.${result.titleKey}`
    if (te(key)) return t(key)
  }
  return result.id
}

const actionLabel = (code: string): string => {
  const key = `diagnostics.actions.${code}`
  return te(key) ? t(key) : code
}

const onRun = async () => {
  try {
    await diagnosticsStore.run()
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  }
}

const onExport = async () => {
  try {
    const path = await saveDialog({
      title: t('diagnostics.exportTitle'),
      defaultPath: 'avocado-diagnostics.zip',
      filters: [{ name: 'Zip', extensions: ['zip'] }],
    })
    if (!path) return
    await diagnosticsStore.exportPack(path)
    message.success(t('diagnostics.exportSuccess'))
  } catch (error) {
    message.error(
      t('diagnostics.exportFail', {
        error: t(getCommandErrorI18nKey(normalizeCommandError(error))),
      }),
    )
  }
}
</script>

<template>
  <div class="diagnostics-panel">
    <div class="diagnostics-panel__toolbar">
      <div class="diagnostics-panel__meta">
        <p v-if="lastRunLabel" class="diagnostics-panel__last-run">
          {{ lastRunLabel }}
        </p>
        <p v-else class="diagnostics-panel__last-run diagnostics-panel__last-run--muted">
          {{ t('diagnostics.empty') }}
        </p>
      </div>
      <div class="diagnostics-panel__actions">
        <NButton
          size="small"
          type="primary"
          :loading="running"
          :aria-label="t('diagnostics.run')"
          @click="onRun"
        >
          <template #icon>
            <Play :size="14" aria-hidden="true" />
          </template>
          {{ t('diagnostics.run') }}
        </NButton>
        <NButton
          size="small"
          secondary
          :loading="exporting"
          :aria-label="t('diagnostics.export')"
          @click="onExport"
        >
          <template #icon>
            <Download :size="14" aria-hidden="true" />
          </template>
          {{ t('diagnostics.export') }}
        </NButton>
      </div>
    </div>

    <ul
      v-if="report?.results.length"
      class="diagnostics-panel__list"
      :aria-label="t('diagnostics.title')"
    >
      <li
        v-for="result in report.results"
        :key="result.id"
        class="diagnostics-panel__item ops-card"
      >
        <div
          class="diag-status"
          :class="statusMeta(result.status).tone"
        >
          <component
            :is="statusMeta(result.status).icon"
            :size="16"
            aria-hidden="true"
          />
          <span class="diag-status__text">
            {{ t(statusMeta(result.status).labelKey) }}
          </span>
        </div>
        <div class="diagnostics-panel__body">
          <h4 class="diagnostics-panel__title">{{ checkTitle(result) }}</h4>
          <p class="diagnostics-panel__detail">{{ result.detail }}</p>
          <p class="diagnostics-panel__action">
            <span class="diagnostics-panel__action-label">
              {{ t('diagnostics.suggestedAction') }}:
            </span>
            {{ actionLabel(result.suggestedAction) }}
          </p>
        </div>
      </li>
    </ul>

    <div
      v-else
      class="ops-card diagnostics-panel__empty"
    >
      {{ t('diagnostics.empty') }}
    </div>
  </div>
</template>

<style scoped>
.diagnostics-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.diagnostics-panel__toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.diagnostics-panel__last-run {
  margin: 0;
  font-size: 13px;
  color: var(--ops-text);
}

.diagnostics-panel__last-run--muted {
  color: var(--ops-muted);
}

.diagnostics-panel__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.diagnostics-panel__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  overflow: auto;
  max-height: calc(100vh - 260px);
}

.ops-card {
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
}

.diagnostics-panel__item {
  display: flex;
  gap: 14px;
  padding: 12px 14px;
  align-items: flex-start;
}

.diag-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  min-width: 92px;
  font-size: 12px;
  font-weight: 600;
}

.diag-status--pass {
  color: var(--ops-ok);
}

.diag-status--warning {
  color: var(--ops-warn);
}

.diag-status--fail {
  color: var(--ops-danger);
}

.diagnostics-panel__body {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.diagnostics-panel__title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--ops-text);
}

.diagnostics-panel__detail {
  margin: 0;
  font-size: 12px;
  color: var(--ops-muted);
  line-height: 1.5;
  word-break: break-word;
}

.diagnostics-panel__action {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--ops-text);
  line-height: 1.45;
}

.diagnostics-panel__action-label {
  color: var(--ops-muted);
  font-weight: 500;
}

.diagnostics-panel__empty {
  padding: 32px 16px;
  text-align: center;
  color: var(--ops-muted);
  font-size: 13px;
}
</style>
