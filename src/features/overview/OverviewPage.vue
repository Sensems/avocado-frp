<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useMessage, NButton } from 'naive-ui'
import { Play, Square, RotateCw } from 'lucide-vue-next'

import PendingRestartBanner from '@/components/status/PendingRestartBanner.vue'
import ProcessPhaseBadge from '@/components/status/ProcessPhaseBadge.vue'
import TrafficChart from '@/components/TrafficChart.vue'
import type { ProcessKind, ProcessSnapshot } from '@/domain/process'
import { getCommandErrorI18nKey } from '@/services/errorMapper'
import { useConfigStore } from '@/stores/config'
import { useLogsStore } from '@/stores/logs'
import { useProcessStore } from '@/stores/process'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const message = useMessage()

const processStore = useProcessStore()
  const configStore = useConfigStore()
  const logsStore = useLogsStore()
  const settingsStore = useSettingsStore()

  const {
    frpc,
    frps,
    pendingRestart,
    frpcLoading,
    frpsLoading,
    frpcRunning,
    frpsRunning,
  } = storeToRefs(processStore)

  const { frpc: frpcConfig, frps: frpsConfig } = storeToRefs(configStore)
  const { entries } = storeToRefs(logsStore)
  const { settings } = storeToRefs(settingsStore)

  void settingsStore.load().catch(() => {
    /* Overview still works; traffic command reports disabled/not_configured */
  })

const formatUptime = (seconds: number | undefined): string => {
  if (seconds == null || seconds <= 0) return t('overview.valueNone')
  const hrs = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)
  if (hrs > 0) return `${hrs}h ${mins}m`
  if (mins > 0) return `${mins}m ${secs}s`
  return `${secs}s`
}

const shortRevision = (revision: string | undefined): string => {
  if (!revision) return t('overview.valueNone')
  return revision.length > 12 ? `${revision.slice(0, 12)}…` : revision
}

const lastErrorText = (snapshot: ProcessSnapshot | null): string => {
  if (!snapshot?.lastError) return t('overview.valueNone')
  return t(getCommandErrorI18nKey(snapshot.lastError))
}

const issueCounts = computed(() => {
  const issues = [
    ...(frpcConfig.value?.issues ?? []),
    ...(frpsConfig.value?.issues ?? []),
  ]
  let errors = 0
  let warnings = 0
  for (const issue of issues) {
    if (issue.severity === 'error') errors += 1
    else if (issue.severity === 'warning') warnings += 1
  }
  return { errors, warnings, total: issues.length }
})

const recentErrors = computed(() =>
  [...entries.value]
    .filter((entry) => entry.type === 'err')
    .slice(-5)
    .reverse(),
)

const frpcDegraded = computed(() => frpc.value?.phase === 'degraded')

  const monitorEnabled = computed(
    () => settings.value?.localMonitor.enabled === true,
  )

const runAction = async (
  kind: ProcessKind,
  action: 'start' | 'stop' | 'restart',
) => {
  const result = await processStore[action](kind)
  if (result.ok) {
    if (action === 'stop') {
      message.success(t('feedback.stopSuccess', { name: kind }))
    } else if (action === 'start') {
      message.success(t('feedback.startSuccess', { name: kind }))
    } else {
      message.success(t('overview.restartSuccess', { name: kind }))
    }
    return
  }

  const errorText = t(getCommandErrorI18nKey(result.error))
  if (action === 'stop') {
    message.error(t('feedback.stopFail', { name: kind, error: errorText }))
  } else if (action === 'start') {
    message.error(t('feedback.startFail', { name: kind, error: errorText }))
  } else {
    message.error(t('overview.restartFail', { name: kind, error: errorText }))
  }
}

const onRestartPending = (kind: ProcessKind) => {
  void runAction(kind, 'restart')
}
</script>

<template>
  <div class="ops-page overview-page">
    <header class="overview-page__header">
      <h2 class="ops-page__title">{{ t('nav.overview') }}</h2>
      <p class="overview-page__subtitle">{{ t('overview.subtitle') }}</p>
    </header>

    <PendingRestartBanner
      :pending-frpc="pendingRestart.frpc"
      :pending-frps="pendingRestart.frps"
      :loading-frpc="frpcLoading"
      :loading-frps="frpsLoading"
      @restart="onRestartPending"
    />

    <section class="overview-grid overview-grid--process">
      <article class="ops-card">
        <div class="ops-card__header">
          <h3 class="ops-card__title">{{ t('status.frpcLabel') }} (frpc)</h3>
          <ProcessPhaseBadge :snapshot="frpc" />
        </div>
        <dl class="ops-meta">
          <div class="ops-meta__row">
            <dt>{{ t('overview.uptime') }}</dt>
            <dd>{{ formatUptime(frpc?.uptimeSeconds) }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.configRevision') }}</dt>
            <dd class="ops-meta__mono">{{ shortRevision(frpc?.configRevision) }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.lastExitCode') }}</dt>
            <dd>{{ frpc?.lastExitCode ?? t('overview.valueNone') }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.lastError') }}</dt>
            <dd
              class="ops-meta__error"
              :title="lastErrorText(frpc)"
            >
              {{ lastErrorText(frpc) }}
            </dd>
          </div>
        </dl>
        <p
          v-if="frpcDegraded"
          class="ops-card__hint ops-card__hint--warn"
          role="status"
        >
          {{
            monitorEnabled
              ? t('overview.degradedMonitorHint')
              : t('overview.degradedNoMonitorHint')
          }}
        </p>
        <div class="ops-card__actions">
          <NButton
            type="primary"
            size="small"
            :loading="frpcLoading"
            :disabled="frpcRunning || frpcLoading"
            :aria-label="t('dashboard.startFrpc')"
            @click="runAction('frpc', 'start')"
          >
            <template #icon>
              <Play :size="14" aria-hidden="true" />
            </template>
            {{ t('dashboard.startFrpc') }}
          </NButton>
          <NButton
            type="error"
            ghost
            size="small"
            :loading="frpcLoading"
            :disabled="!frpcRunning || frpcLoading"
            :aria-label="t('dashboard.stopFrpc')"
            @click="runAction('frpc', 'stop')"
          >
            <template #icon>
              <Square :size="14" aria-hidden="true" />
            </template>
            {{ t('dashboard.stopFrpc') }}
          </NButton>
          <NButton
            v-if="pendingRestart.frpc"
            type="warning"
            secondary
            size="small"
            :loading="frpcLoading"
            :disabled="frpcLoading"
            :aria-label="t('overview.saveAndRestartFrpc')"
            @click="runAction('frpc', 'restart')"
          >
            <template #icon>
              <RotateCw :size="14" aria-hidden="true" />
            </template>
            {{ t('overview.saveAndRestart') }}
          </NButton>
        </div>
      </article>

      <article class="ops-card">
        <div class="ops-card__header">
          <h3 class="ops-card__title">{{ t('status.frpsLabel') }} (frps)</h3>
          <ProcessPhaseBadge :snapshot="frps" />
        </div>
        <dl class="ops-meta">
          <div class="ops-meta__row">
            <dt>{{ t('overview.uptime') }}</dt>
            <dd>{{ formatUptime(frps?.uptimeSeconds) }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.configRevision') }}</dt>
            <dd class="ops-meta__mono">{{ shortRevision(frps?.configRevision) }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.lastExitCode') }}</dt>
            <dd>{{ frps?.lastExitCode ?? t('overview.valueNone') }}</dd>
          </div>
          <div class="ops-meta__row">
            <dt>{{ t('overview.lastError') }}</dt>
            <dd
              class="ops-meta__error"
              :title="lastErrorText(frps)"
            >
              {{ lastErrorText(frps) }}
            </dd>
          </div>
        </dl>
        <div class="ops-card__actions">
          <NButton
            type="primary"
            size="small"
            :loading="frpsLoading"
            :disabled="frpsRunning || frpsLoading"
            :aria-label="t('dashboard.startFrps')"
            @click="runAction('frps', 'start')"
          >
            <template #icon>
              <Play :size="14" aria-hidden="true" />
            </template>
            {{ t('dashboard.startFrps') }}
          </NButton>
          <NButton
            type="error"
            ghost
            size="small"
            :loading="frpsLoading"
            :disabled="!frpsRunning || frpsLoading"
            :aria-label="t('dashboard.stopFrps')"
            @click="runAction('frps', 'stop')"
          >
            <template #icon>
              <Square :size="14" aria-hidden="true" />
            </template>
            {{ t('dashboard.stopFrps') }}
          </NButton>
          <NButton
            v-if="pendingRestart.frps"
            type="warning"
            secondary
            size="small"
            :loading="frpsLoading"
            :disabled="frpsLoading"
            :aria-label="t('overview.saveAndRestartFrps')"
            @click="runAction('frps', 'restart')"
          >
            <template #icon>
              <RotateCw :size="14" aria-hidden="true" />
            </template>
            {{ t('overview.saveAndRestart') }}
          </NButton>
        </div>
      </article>
    </section>

    <section class="overview-grid overview-grid--secondary">
      <article class="ops-card">
        <div class="ops-card__header">
          <h3 class="ops-card__title">{{ t('overview.configHealth') }}</h3>
        </div>
        <div class="health-counts">
          <div class="health-counts__item health-counts__item--danger">
            <span class="health-counts__value">{{ issueCounts.errors }}</span>
            <span class="health-counts__label">{{ t('overview.errors') }}</span>
          </div>
          <div class="health-counts__item health-counts__item--warn">
            <span class="health-counts__value">{{ issueCounts.warnings }}</span>
            <span class="health-counts__label">{{ t('overview.warnings') }}</span>
          </div>
          <div class="health-counts__item">
            <span class="health-counts__value">{{ issueCounts.total }}</span>
            <span class="health-counts__label">{{ t('overview.totalIssues') }}</span>
          </div>
        </div>
        <p
          v-if="issueCounts.total === 0"
          class="ops-card__empty"
        >
          {{ t('overview.configHealthy') }}
        </p>
        <div class="ops-card__footer-links">
          <RouterLink
            to="/client"
            :aria-label="t('overview.openClient')"
          >
            {{ t('overview.openClient') }}
          </RouterLink>
          <RouterLink
            to="/server"
            :aria-label="t('overview.openServer')"
          >
            {{ t('overview.openServer') }}
          </RouterLink>
        </div>
      </article>

      <article class="ops-card">
        <div class="ops-card__header">
          <h3 class="ops-card__title">{{ t('overview.recentFaults') }}</h3>
          <RouterLink
            class="ops-card__link"
            to="/logs"
            :aria-label="t('status.viewLogs')"
          >
            {{ t('status.viewLogs') }}
          </RouterLink>
        </div>
        <ul
          v-if="recentErrors.length > 0"
          class="fault-list"
        >
          <li
            v-for="(entry, index) in recentErrors"
            :key="`${entry.timestamp}-${index}`"
            class="fault-list__item"
          >
            <span class="fault-list__meta">
              {{ entry.time }} · {{ entry.source }}
            </span>
            <span
              class="fault-list__text"
              :title="entry.text"
            >{{ entry.text }}</span>
          </li>
        </ul>
        <p
          v-else
          class="ops-card__empty"
        >
          {{ t('overview.noRecentFaults') }}
        </p>
      </article>
    </section>

    <section class="ops-card ops-card--traffic">
      <div class="ops-card__header">
        <h3 class="ops-card__title">{{ t('overview.trafficSummary') }}</h3>
        <RouterLink
          v-if="!monitorEnabled"
          class="ops-card__link"
          to="/settings"
          :aria-label="t('overview.openMonitorSettings')"
        >
          {{ t('overview.openMonitorSettings') }}
        </RouterLink>
      </div>
      <div class="overview-page__chart">
        <!-- Always poll; get_frpc_traffic returns structured MonitorStatus empty states. -->
        <TrafficChart />
      </div>
    </section>
  </div>
</template>

<style scoped>
.ops-page {
  padding: var(--ops-gap);
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
  min-width: 0;
}

.ops-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--ops-text);
}

.overview-page__header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.overview-page__subtitle {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.overview-grid {
  display: grid;
  gap: var(--ops-gap);
  min-width: 0;
}

.overview-grid--process,
.overview-grid--secondary {
  grid-template-columns: repeat(2, minmax(0, 1fr));
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

.ops-card--traffic {
  min-height: 240px;
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

.ops-card__link,
.ops-card__footer-links a {
  font-size: 12px;
  color: var(--ops-accent);
  text-decoration: none;
}

.ops-card__link:hover,
.ops-card__footer-links a:hover {
  text-decoration: underline;
}

.ops-card__footer-links {
  display: flex;
  gap: 12px;
}

.ops-card__empty {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.ops-card__hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--ops-muted);
}

.ops-card__hint--warn {
  color: var(--ops-warn);
}

.ops-card__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.ops-meta {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ops-meta__row {
  display: grid;
  grid-template-columns: 120px minmax(0, 1fr);
  gap: 8px;
  font-size: 12px;
}

.ops-meta__row dt {
  margin: 0;
  color: var(--ops-muted);
}

.ops-meta__row dd {
  margin: 0;
  color: var(--ops-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ops-meta__mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.ops-meta__error {
  color: var(--ops-danger);
}

.health-counts {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.health-counts__item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
}

.health-counts__value {
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--ops-text);
}

.health-counts__label {
  font-size: 11px;
  color: var(--ops-muted);
}

.health-counts__item--danger .health-counts__value {
  color: var(--ops-danger);
}

.health-counts__item--warn .health-counts__value {
  color: var(--ops-warn);
}

.fault-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.fault-list__item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.fault-list__meta {
  font-size: 11px;
  color: var(--ops-muted);
}

.fault-list__text {
  font-size: 12px;
  color: var(--ops-danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overview-page__chart {
  height: 200px;
  min-height: 180px;
}

@media (max-width: 960px) {
  .overview-grid--process,
  .overview-grid--secondary {
    grid-template-columns: 1fr;
  }
}
</style>
