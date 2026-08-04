<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import {
  NButton,
  NCheckbox,
  NInput,
  NSelect,
  NTabPane,
  NTabs,
  useDialog,
  useMessage,
} from 'naive-ui'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { Copy, Download, Eraser, Trash2 } from 'lucide-vue-next'

import DiagnosticsPanel from '@/features/diagnostics/DiagnosticsPanel.vue'
import type { ProcessKind } from '@/domain/process'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'
import { useLogsStore } from '@/stores/logs'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()
const logsStore = useLogsStore()
const { filters, filteredEntries, pauseScroll } = storeToRefs(logsStore)

const activeTab = ref<'logs' | 'diagnostics'>('logs')
const listRef = ref<HTMLElement | null>(null)
const exporting = ref(false)
const deletingDisk = ref(false)

const sourceOptions = computed(() => [
  { label: t('logs.sourceAll'), value: 'all' },
  { label: 'frpc', value: 'frpc' },
  { label: 'frps', value: 'frps' },
])

const levelOptions = computed(() => [
  { label: t('logs.levelAll'), value: 'all' },
  { label: t('logs.levelOut'), value: 'out' },
  { label: t('logs.levelErr'), value: 'err' },
])

/** Match current source filter; `null` deletes both kinds. */
const deleteDiskKind = computed((): ProcessKind | null => {
  const source = filters.value.source
  return source === 'all' ? null : source
})

const scrollToBottom = () => {
  if (pauseScroll.value) return
  nextTick(() => {
    const el = listRef.value
    if (el) el.scrollTop = el.scrollHeight
  })
}

watch(
  () => filteredEntries.value.length,
  () => {
    if (activeTab.value === 'logs') scrollToBottom()
  },
)

const onClear = () => {
  logsStore.clearUiBuffer()
  message.success(t('logs.clearDone'))
}

const runDeleteDisk = async () => {
  deletingDisk.value = true
  try {
    await tauriClient.deleteDiskLogs(deleteDiskKind.value)
    message.success(t('logs.deleteDiskDone'))
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  } finally {
    deletingDisk.value = false
  }
}

const onDeleteDisk = () => {
  dialog.warning({
    title: t('logs.deleteDiskTitle'),
    content: t('logs.deleteDiskContent'),
    positiveText: t('logs.deleteDiskConfirm'),
    negativeText: t('forms.cancel'),
    onPositiveClick: () => void runDeleteDisk(),
  })
}

const onCopy = async () => {
  const lines = filteredEntries.value.map(
    (entry) => `[${entry.time}] [${entry.source}/${entry.type}] ${entry.text}`,
  )
  if (lines.length === 0) {
    message.warning(t('logs.copyEmpty'))
    return
  }
  try {
    await navigator.clipboard.writeText(lines.join('\n'))
    message.success(t('logs.copyDone'))
  } catch {
    message.error(t('logs.copyFail'))
  }
}

const onExport = async () => {
  exporting.value = true
  try {
    const selectedDir = await openDialog({
      directory: true,
      multiple: false,
      title: t('logs.exportTitle'),
    })
    if (typeof selectedDir === 'string') {
      const result = await tauriClient.exportLogs(selectedDir)
      message.success(result || t('feedback.exportSuccess'))
    }
  } catch (error) {
    message.error(
      t('feedback.exportFail', {
        error: t(getCommandErrorI18nKey(normalizeCommandError(error))),
      }),
    )
  } finally {
    exporting.value = false
  }
}

void logsStore.init().catch(() => {
  /* App shell may already own the log listener */
})
</script>

<template>
  <div class="ops-page logs-page">
    <header class="logs-page__header">
      <div>
        <h2 class="ops-page__title">{{ t('nav.logs') }}</h2>
        <p class="logs-page__subtitle">{{ t('logs.subtitle') }}</p>
      </div>
      <div v-if="activeTab === 'logs'" class="logs-page__actions">
        <NButton
          size="small"
          secondary
          :aria-label="t('logs.copy')"
          @click="onCopy"
        >
          <template #icon>
            <Copy :size="14" aria-hidden="true" />
          </template>
          {{ t('logs.copy') }}
        </NButton>
        <NButton
          size="small"
          secondary
          :loading="exporting"
          :aria-label="t('logs.export')"
          @click="onExport"
        >
          <template #icon>
            <Download :size="14" aria-hidden="true" />
          </template>
          {{ t('logs.export') }}
        </NButton>
        <NButton
          size="small"
          tertiary
          :aria-label="t('logs.clear')"
          @click="onClear"
        >
          <template #icon>
            <Eraser :size="14" aria-hidden="true" />
          </template>
          {{ t('logs.clear') }}
        </NButton>
        <NButton
          size="small"
          type="error"
          :loading="deletingDisk"
          :aria-label="t('logs.deleteDisk')"
          @click="onDeleteDisk"
        >
          <template #icon>
            <Trash2 :size="14" aria-hidden="true" />
          </template>
          {{ t('logs.deleteDisk') }}
        </NButton>
      </div>
    </header>

    <NTabs
      v-model:value="activeTab"
      type="line"
      size="small"
      animated
      class="logs-page__tabs"
    >
      <NTabPane name="logs" :tab="t('logs.tabLogs')">
        <div class="logs-page__pane">
          <p class="logs-page__hint" role="note">
            {{ t('logs.clearUiOnlyHint') }}
          </p>

          <div class="logs-page__toolbar" role="search">
            <NSelect
              :value="filters.source"
              :options="sourceOptions"
              size="small"
              class="logs-page__select"
              :aria-label="t('logs.filterSource')"
              @update:value="(v) => logsStore.setFilters({ source: v })"
            />
            <NSelect
              :value="filters.level"
              :options="levelOptions"
              size="small"
              class="logs-page__select"
              :aria-label="t('logs.filterLevel')"
              @update:value="(v) => logsStore.setFilters({ level: v })"
            />
            <NInput
              :value="filters.query"
              size="small"
              clearable
              class="logs-page__query"
              :placeholder="t('logs.filterKeyword')"
              :aria-label="t('logs.filterKeyword')"
              @update:value="(v) => logsStore.setFilters({ query: v })"
            />
            <NCheckbox
              :checked="pauseScroll"
              :aria-label="t('logs.pauseScroll')"
              @update:checked="(v) => (pauseScroll = v)"
            >
              {{ t('logs.pauseScroll') }}
            </NCheckbox>
          </div>

          <section
            ref="listRef"
            class="ops-card logs-page__list"
            :aria-label="t('logs.tabLogs')"
            tabindex="0"
          >
            <div
              v-if="filteredEntries.length === 0"
              class="logs-page__empty"
            >
              {{ t('logs.empty') }}
            </div>
            <div
              v-for="(entry, idx) in filteredEntries"
              :key="`${entry.timestamp}-${idx}`"
              class="logs-page__row"
              :class="{ 'logs-page__row--err': entry.type === 'err' }"
            >
              <span class="logs-page__time">{{ entry.time }}</span>
              <span
                class="logs-page__source"
                :class="`logs-page__source--${entry.source}`"
              >{{ entry.source }}</span>
              <span class="logs-page__level">{{ entry.type }}</span>
              <span class="logs-page__text">{{ entry.text }}</span>
            </div>
          </section>
        </div>
      </NTabPane>

      <NTabPane name="diagnostics" :tab="t('logs.tabDiagnostics')">
        <DiagnosticsPanel />
      </NTabPane>
    </NTabs>
  </div>
</template>

<style scoped>
.ops-page {
  padding: var(--ops-gap);
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 100%;
  box-sizing: border-box;
}

.ops-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--ops-text);
}

.logs-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.logs-page__subtitle {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.logs-page__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.logs-page__tabs {
  min-height: 0;
}

.logs-page__pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 4px;
}

.logs-page__hint {
  margin: 0;
  font-size: 12px;
  color: var(--ops-muted);
  padding: 8px 12px;
  border: 1px dashed var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
}

.logs-page__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}

.logs-page__select {
  width: 140px;
}

.logs-page__query {
  min-width: 200px;
  flex: 1;
  max-width: 360px;
}

.logs-page__list {
  flex: 1;
  min-height: 280px;
  max-height: calc(100vh - 320px);
  overflow: auto;
  padding: 10px 12px;
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 12px;
  line-height: 1.6;
}

.ops-card {
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
}

.logs-page__empty {
  color: var(--ops-muted);
  padding: 32px 8px;
  text-align: center;
}

.logs-page__row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 2px 0;
  color: var(--ops-text);
  word-break: break-word;
}

.logs-page__row--err {
  color: var(--ops-danger);
}

.logs-page__time {
  color: var(--ops-muted);
  flex-shrink: 0;
}

.logs-page__source {
  flex-shrink: 0;
  font-weight: 600;
  font-size: 10px;
  text-transform: uppercase;
  padding: 0 6px;
  border-radius: 4px;
}

.logs-page__source--frpc {
  color: var(--ops-accent);
  background: color-mix(in srgb, var(--ops-accent) 12%, transparent);
}

.logs-page__source--frps {
  color: var(--ops-warn, #d97706);
  background: color-mix(in srgb, var(--ops-warn, #d97706) 12%, transparent);
}

.logs-page__level {
  color: var(--ops-muted);
  flex-shrink: 0;
  min-width: 28px;
}

.logs-page__text {
  flex: 1;
  min-width: 0;
}
</style>
