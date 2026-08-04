<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import {
  NButton,
  NCollapse,
  NCollapseItem,
  NInput,
  NInputNumber,
  NProgress,
  NSelect,
  NSwitch,
  useDialog,
  useMessage,
} from 'naive-ui'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { relaunch } from '@tauri-apps/plugin-process'

import type { ConfigKind } from '@/domain/config'
import { persistLocale, type SupportedLocale } from '@/lib/preferences'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { useTheme } from '@/composables/useTheme'
import { useConfigStore } from '@/stores/config'
import { useProcessStore } from '@/stores/process'
import { APP_VERSION } from '@/appVersion'
import { useSettingsStore } from '@/stores/settings'
import { useUpdaterStore } from '@/stores/updater'
const BYTES_PER_MB = 1024 * 1024
const MIN_SIZE_MB = 1
const MAX_SIZE_MB = 100
const MIN_ROTATED_FILES = 1
const MAX_ROTATED_FILES = 30
const DEFAULT_MONITOR_ADDR = '127.0.0.1'
const DEFAULT_MONITOR_PORT = 7400
const MIN_MONITOR_PORT = 1
const MAX_MONITOR_PORT = 65535

const { t, locale } = useI18n()
const message = useMessage()
const dialog = useDialog()
const configStore = useConfigStore()
const processStore = useProcessStore()
const settingsStore = useSettingsStore()
const updaterStore = useUpdaterStore()
const { frpc, frps } = storeToRefs(configStore)
const { settings, loading: settingsLoading, saving: settingsSaving } =
  storeToRefs(settingsStore)
const {
  phase: updaterPhase,
  available: availableUpdate,
  progress: updateProgress,
  busy: updaterBusy,
  currentVersion: updaterCurrentVersion,
} = storeToRefs(updaterStore)
const { themeMode, setTheme } = useTheme()

const autostartEnabled = ref(false)
const autostartBusy = ref(false)
const restoreBusy = ref<ConfigKind | null>(null)
const maxFileSizeMb = ref<number | null>(10)
const maxRotatedFiles = ref<number | null>(7)
const monitorEnabled = ref(false)
const monitorAddr = ref(DEFAULT_MONITOR_ADDR)
const monitorPort = ref<number | null>(DEFAULT_MONITOR_PORT)
const monitorUser = ref('')
const monitorPassword = ref('')
const monitorSaving = ref(false)
const checkUpdatesOnLaunch = ref(true)
const checkOnLaunchBusy = ref(false)

const themeOptions = computed(() => [
  { label: t('theme.system'), value: 'system' },
  { label: t('theme.light'), value: 'light' },
  { label: t('theme.dark'), value: 'dark' },
])

const localeOptions = [
  { label: '简体中文', value: 'zh' },
  { label: 'English', value: 'en' },
]

const checkAutostart = async () => {
  try {
    autostartEnabled.value = await isEnabled()
  } catch (error) {
    console.error('Failed to check autostart:', error)
  }
}

const onAutostartChange = async (value: boolean) => {
  autostartBusy.value = true
  try {
    if (value) await enable()
    else await disable()
    await checkAutostart()
    message.success(t('settings.autostartUpdated'))
  } catch {
    message.error(t('settings.autostartFail'))
    await checkAutostart()
  } finally {
    autostartBusy.value = false
  }
}

const onLocaleChange = (value: string) => {
  locale.value = persistLocale(value) as SupportedLocale
}

const confirmRestore = (kind: ConfigKind) => {
  const snapshot = kind === 'frpc' ? frpc.value : frps.value
  if (!snapshot?.backupAvailable) {
    message.warning(t('settings.backupUnavailable', { kind }))
    return
  }

  dialog.warning({
    title: t('settings.restoreConfirmTitle', { kind }),
    content: t('settings.restoreConfirmContent', {
      kind,
      revision: snapshot.revision.slice(0, 12),
    }),
    positiveText: t('settings.restoreConfirm'),
    negativeText: t('forms.cancel'),
    onPositiveClick: () => void runRestore(kind, snapshot.revision),
  })
}

const runRestore = async (kind: ConfigKind, expectedRevision: string) => {
  restoreBusy.value = kind
  try {
    const result = await configStore.restoreBackup(kind, expectedRevision)
    if (result.ok) {
      message.success(t('settings.restoreSuccess', { kind }))
    } else {
      message.error(t(getCommandErrorI18nKey(result.error)))
      if (result.error.code === 'CONFIG_CONFLICT') {
        await configStore.load(kind)
      }
    }
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  } finally {
    restoreBusy.value = null
  }
}

const applySettingsToForm = () => {
  const policy = settings.value?.logPolicy
  if (policy) {
    maxFileSizeMb.value = Math.round(policy.maxFileBytes / BYTES_PER_MB)
    maxRotatedFiles.value = policy.maxRotatedFiles
  }
  const monitor = settings.value?.localMonitor
  if (monitor) {
    monitorEnabled.value = monitor.enabled
    monitorAddr.value = monitor.addr || DEFAULT_MONITOR_ADDR
    monitorPort.value = monitor.port || DEFAULT_MONITOR_PORT
    monitorUser.value = monitor.user ?? ''
    monitorPassword.value = monitor.password ?? ''
  }
  if (settings.value) {
    checkUpdatesOnLaunch.value = settings.value.checkUpdatesOnLaunch
  }
}

const updateStatusText = computed(() => {
  switch (updaterPhase.value) {
    case 'checking':
      return t('settings.updateChecking')
    case 'available':
      return t('settings.updateAvailable', {
        version: availableUpdate.value?.version ?? '',
      })
    case 'upToDate':
      return t('settings.updateUpToDate')
    case 'downloading':
      return t('settings.updateDownloading')
    case 'ready':
      return t('settings.updateReady')
    case 'error':
      return t('settings.updateError')
    default:
      return t('settings.checkUpdatesDesc')
  }
})

const onCheckUpdates = async () => {
  try {
    const result = await updaterStore.check()
    if (result) {
      message.success(
        t('settings.updateAvailable', { version: result.version }),
      )
    } else {
      message.success(t('settings.updateUpToDate'))
    }
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  }
}

const confirmInstallUpdate = () => {
  const update = availableUpdate.value
  if (!update) {
    message.warning(t('settings.updateNonePending'))
    return
  }

  dialog.warning({
    title: t('settings.updateInstallConfirmTitle'),
    content: t('settings.updateInstallConfirmContent', {
      version: update.version,
    }),
    positiveText: t('settings.updateInstallConfirm'),
    negativeText: t('forms.cancel'),
    onPositiveClick: () => void runInstallUpdate(),
  })
}

const runInstallUpdate = async () => {
  try {
    await updaterStore.installAfterConfirm()
    dialog.success({
      title: t('settings.updateInstalledTitle'),
      content: t('settings.updateInstalledContent'),
      positiveText: t('settings.updateRestartNow'),
      negativeText: t('forms.cancel'),
      onPositiveClick: () => {
        void relaunch()
      },
    })
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  }
}

const onCheckUpdatesOnLaunchChange = async (value: boolean) => {
  checkOnLaunchBusy.value = true
  checkUpdatesOnLaunch.value = value
  try {
    await settingsStore.update({ checkUpdatesOnLaunch: value })
    message.success(t('settings.checkUpdatesOnLaunchSaved'))
  } catch (error) {
    checkUpdatesOnLaunch.value = settings.value?.checkUpdatesOnLaunch ?? true
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  } finally {
    checkOnLaunchBusy.value = false
  }
}

const showLogPolicyNoticeIfNeeded = () => {
  if (settings.value?.logPolicyNoticeShown) return

  dialog.info({
    title: t('settings.logPolicyNoticeTitle'),
    content: t('settings.logPolicyNoticeContent'),
    positiveText: t('settings.logPolicyNoticeConfirm'),
    onPositiveClick: () => {
      void settingsStore
        .update({ logPolicyNoticeShown: true })
        .catch((error) => {
          message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
        })
    },
  })
}

const onSaveLogPolicy = async () => {
  const sizeMb = Math.min(
    MAX_SIZE_MB,
    Math.max(MIN_SIZE_MB, Math.round(maxFileSizeMb.value || MIN_SIZE_MB)),
  )
  const files = Math.min(
    MAX_ROTATED_FILES,
    Math.max(
      MIN_ROTATED_FILES,
      Math.round(maxRotatedFiles.value || MIN_ROTATED_FILES),
    ),
  )
  maxFileSizeMb.value = sizeMb
  maxRotatedFiles.value = files

  try {
    await settingsStore.update({
      logPolicy: {
        maxFileBytes: sizeMb * BYTES_PER_MB,
        maxRotatedFiles: files,
      },
    })
    message.success(t('settings.logPolicySaved'))
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  }
}

const onSaveLocalMonitor = async () => {
  const addr = (monitorAddr.value || DEFAULT_MONITOR_ADDR).trim()
  const port = Math.min(
    MAX_MONITOR_PORT,
    Math.max(
      MIN_MONITOR_PORT,
      Math.round(monitorPort.value || DEFAULT_MONITOR_PORT),
    ),
  )
  monitorAddr.value = addr
  monitorPort.value = port

  monitorSaving.value = true
  try {
    if (!frpc.value) {
      await configStore.load('frpc')
    }
    const result = await settingsStore.applyLocalMonitor({
      enabled: monitorEnabled.value,
      addr,
      port,
      user: monitorUser.value.trim() || null,
      password: monitorPassword.value.trim() || null,
    })
    applySettingsToForm()
    if (result.configPatched) {
      await configStore.load('frpc')
    }
    if (result.pendingRestart) {
      processStore.setPendingRestart('frpc', true)
      message.success(t('settings.localMonitorSavedRestart'))
    } else if (result.configPatched) {
      message.success(t('settings.localMonitorSavedPatched'))
    } else {
      message.success(t('settings.localMonitorSaved'))
    }
  } catch (error) {
    message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
  } finally {
    monitorSaving.value = false
  }
}

onMounted(() => {
  void checkAutostart()
  void settingsStore
    .load()
    .then(() => {
      applySettingsToForm()
      showLogPolicyNoticeIfNeeded()
    })
    .catch((error) => {
      message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
    })
})
</script>

<template>
  <div class="ops-page settings-page">
    <header class="settings-page__header">
      <div>
        <h2 class="ops-page__title">{{ t('nav.settings') }}</h2>
        <p class="settings-page__subtitle">{{ t('settings.subtitle') }}</p>
      </div>
      <span class="settings-page__version">
        {{ t('settings.appVersion', { version: APP_VERSION }) }}
      </span>
    </header>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('settings.general') }}</h3>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('dashboard.autostart') }}</div>
          <p class="settings-row__desc">{{ t('dashboard.autostartDesc') }}</p>
        </div>
        <NSwitch
          :value="autostartEnabled"
          :loading="autostartBusy"
          :aria-label="t('dashboard.autostart')"
          @update:value="onAutostartChange"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.theme') }}</div>
          <p class="settings-row__desc">{{ t('settings.themeDesc') }}</p>
        </div>
        <NSelect
          :value="themeMode"
          :options="themeOptions"
          size="small"
          class="settings-row__control"
          :aria-label="t('settings.theme')"
          @update:value="setTheme"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('dashboard.language') }}</div>
          <p class="settings-row__desc">{{ t('dashboard.languageDesc') }}</p>
        </div>
        <NSelect
          :value="locale"
          :options="localeOptions"
          size="small"
          class="settings-row__control"
          :aria-label="t('dashboard.language')"
          @update:value="onLocaleChange"
        />
      </div>
    </section>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('settings.backup') }}</h3>
      <p class="settings-section__hint">{{ t('settings.backupHint') }}</p>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">frpc.toml.bak</div>
          <p class="settings-row__desc">
            {{
              frpc?.backupAvailable
                ? t('settings.backupReady')
                : t('settings.backupMissing')
            }}
          </p>
        </div>
        <NButton
          size="small"
          secondary
          :disabled="!frpc?.backupAvailable"
          :loading="restoreBusy === 'frpc'"
          :aria-label="t('settings.restoreFrpc')"
          @click="confirmRestore('frpc')"
        >
          {{ t('settings.restoreFrpc') }}
        </NButton>
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">frps.toml.bak</div>
          <p class="settings-row__desc">
            {{
              frps?.backupAvailable
                ? t('settings.backupReady')
                : t('settings.backupMissing')
            }}
          </p>
        </div>
        <NButton
          size="small"
          secondary
          :disabled="!frps?.backupAvailable"
          :loading="restoreBusy === 'frps'"
          :aria-label="t('settings.restoreFrps')"
          @click="confirmRestore('frps')"
        >
          {{ t('settings.restoreFrps') }}
        </NButton>
      </div>
    </section>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('settings.logRetention') }}</h3>
      <p class="settings-section__hint">{{ t('settings.logRetentionDesc') }}</p>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.maxFileSizeMb') }}</div>
          <p class="settings-row__desc">{{ t('settings.maxFileSizeMbDesc') }}</p>
        </div>
        <NInputNumber
          v-model:value="maxFileSizeMb"
          :min="MIN_SIZE_MB"
          :max="MAX_SIZE_MB"
          :step="1"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading"
          :aria-label="t('settings.maxFileSizeMb')"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.maxRotatedFiles') }}</div>
          <p class="settings-row__desc">{{ t('settings.maxRotatedFilesDesc') }}</p>
        </div>
        <NInputNumber
          v-model:value="maxRotatedFiles"
          :min="MIN_ROTATED_FILES"
          :max="MAX_ROTATED_FILES"
          :step="1"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading"
          :aria-label="t('settings.maxRotatedFiles')"
        />
      </div>
      <div class="settings-row settings-row--end">
        <NButton
          size="small"
          type="primary"
          :loading="settingsSaving"
          :disabled="settingsLoading"
          :aria-label="t('settings.saveLogPolicy')"
          @click="onSaveLogPolicy"
        >
          {{ t('settings.saveLogPolicy') }}
        </NButton>
      </div>
    </section>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('settings.localMonitor') }}</h3>
      <p class="settings-section__hint">{{ t('settings.localMonitorDesc') }}</p>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.localMonitorEnable') }}</div>
          <p class="settings-row__desc">{{ t('settings.localMonitorEnableDesc') }}</p>
        </div>
        <NSwitch
          v-model:value="monitorEnabled"
          :disabled="settingsLoading || monitorSaving"
          :aria-label="t('settings.localMonitorEnable')"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.localMonitorAddr') }}</div>
          <p class="settings-row__desc">{{ t('settings.localMonitorAddrDesc') }}</p>
        </div>
        <NInput
          v-model:value="monitorAddr"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading || monitorSaving || !monitorEnabled"
          :placeholder="DEFAULT_MONITOR_ADDR"
          :aria-label="t('settings.localMonitorAddr')"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.localMonitorPort') }}</div>
          <p class="settings-row__desc">{{ t('settings.localMonitorPortDesc') }}</p>
        </div>
        <NInputNumber
          v-model:value="monitorPort"
          :min="MIN_MONITOR_PORT"
          :max="MAX_MONITOR_PORT"
          :step="1"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading || monitorSaving || !monitorEnabled"
          :aria-label="t('settings.localMonitorPort')"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.localMonitorUser') }}</div>
          <p class="settings-row__desc">{{ t('settings.localMonitorUserDesc') }}</p>
        </div>
        <NInput
          v-model:value="monitorUser"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading || monitorSaving || !monitorEnabled"
          :placeholder="t('settings.localMonitorUserPlaceholder')"
          :aria-label="t('settings.localMonitorUser')"
        />
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.localMonitorPassword') }}</div>
          <p class="settings-row__desc">{{ t('settings.localMonitorPasswordDesc') }}</p>
        </div>
        <NInput
          v-model:value="monitorPassword"
          type="password"
          show-password-on="click"
          size="small"
          class="settings-row__control"
          :disabled="settingsLoading || monitorSaving || !monitorEnabled"
          :placeholder="t('settings.localMonitorPasswordPlaceholder')"
          :aria-label="t('settings.localMonitorPassword')"
        />
      </div>
      <div class="settings-row settings-row--end">
        <NButton
          size="small"
          type="primary"
          :loading="monitorSaving"
          :disabled="settingsLoading"
          :aria-label="t('settings.saveLocalMonitor')"
          @click="onSaveLocalMonitor"
        >
          {{ t('settings.saveLocalMonitor') }}
        </NButton>
      </div>
    </section>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('settings.updates') }}</h3>
      <p class="settings-section__hint">{{ t('settings.updatesHint') }}</p>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.currentVersion') }}</div>
          <p class="settings-row__desc">
            {{
              t('settings.appVersion', {
                version: updaterCurrentVersion || APP_VERSION,
              })
            }}
          </p>
        </div>
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">{{ t('settings.checkUpdates') }}</div>
          <p class="settings-row__desc">{{ updateStatusText }}</p>
          <p
            v-if="availableUpdate?.body"
            class="settings-row__notes"
          >
            {{ availableUpdate.body }}
          </p>
        </div>
        <div class="settings-row__actions">
          <NButton
            size="small"
            secondary
            :loading="updaterBusy && updaterPhase === 'checking'"
            :disabled="updaterBusy && updaterPhase !== 'checking'"
            :aria-label="t('settings.checkUpdates')"
            @click="onCheckUpdates"
          >
            {{ t('settings.checkUpdates') }}
          </NButton>
          <NButton
            size="small"
            type="primary"
            :disabled="!availableUpdate || updaterBusy"
            :loading="
              updaterBusy &&
              (updaterPhase === 'downloading' || updaterPhase === 'ready')
            "
            :aria-label="t('settings.installUpdate')"
            @click="confirmInstallUpdate"
          >
            {{ t('settings.installUpdate') }}
          </NButton>
        </div>
      </div>
      <div
        v-if="updateProgress && updaterPhase === 'downloading'"
        class="settings-update-progress"
      >
        <NProgress
          type="line"
          :percentage="updateProgress.percent ?? 0"
          :indicator-placement="'inside'"
          :aria-label="t('settings.updateDownloading')"
        />
        <p class="settings-row__desc">
          {{
            updateProgress.percent !== undefined
              ? t('settings.updateProgressPercent', {
                  percent: updateProgress.percent,
                })
              : t('settings.updateProgressBytes', {
                  bytes: updateProgress.downloadedBytes,
                })
          }}
        </p>
      </div>
      <div class="settings-row">
        <div class="settings-row__text">
          <div class="settings-row__label">
            {{ t('settings.checkUpdatesOnLaunch') }}
          </div>
          <p class="settings-row__desc">
            {{ t('settings.checkUpdatesOnLaunchDesc') }}
          </p>
        </div>
        <NSwitch
          :value="checkUpdatesOnLaunch"
          :loading="checkOnLaunchBusy"
          :disabled="settingsLoading"
          :aria-label="t('settings.checkUpdatesOnLaunch')"
          @update:value="onCheckUpdatesOnLaunchChange"
        />
      </div>
    </section>

    <section class="ops-card">
      <h3 class="ops-card__title">{{ t('guide.title') }}</h3>
      <NCollapse>
        <NCollapseItem
          :title="t('guide.quickStart')"
          name="quickstart"
        >
          <ol class="settings-help-list">
            <li>
              <strong>{{ t('guide.step1Title') }}</strong>
              — {{ t('guide.step1Desc') }}
            </li>
            <li>
              <strong>{{ t('guide.step2Title') }}</strong>
              — {{ t('guide.step2Desc') }}
            </li>
            <li>
              <strong>{{ t('guide.step3Title') }}</strong>
              — {{ t('guide.step3Desc') }}
            </li>
          </ol>
        </NCollapseItem>
        <NCollapseItem
          :title="t('guide.clientServer')"
          name="roles"
        >
          <p>{{ t('guide.clientDesc') }}</p>
          <p>{{ t('guide.serverDesc') }}</p>
        </NCollapseItem>
        <NCollapseItem
          :title="t('guide.faq')"
          name="faq"
        >
          <p><strong>{{ t('guide.faq1q') }}</strong> {{ t('guide.faq1a') }}</p>
          <p><strong>{{ t('guide.faq3q') }}</strong> {{ t('guide.faq3a') }}</p>
          <p><strong>{{ t('guide.faq4q') }}</strong> {{ t('guide.faq4a') }}</p>
        </NCollapseItem>
      </NCollapse>
    </section>
  </div>
</template>

<style scoped>
.ops-page {
  padding: var(--ops-gap);
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 880px;
}

.ops-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--ops-text);
}

.settings-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.settings-page__subtitle {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.settings-page__version {
  font-size: 12px;
  color: var(--ops-muted);
  font-variant-numeric: tabular-nums;
  padding: 4px 10px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
}

.ops-card {
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ops-card__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--ops-text);
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-section__hint {
  margin: 0;
  font-size: 12px;
  color: var(--ops-muted);
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: var(--ops-control-height);
  flex-wrap: wrap;
}

.settings-row--end {
  justify-content: flex-end;
}

.settings-row__text {
  min-width: 0;
  flex: 1;
}

.settings-row__label {
  font-size: 13px;
  font-weight: 500;
  color: var(--ops-text);
}

.settings-row__desc {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--ops-muted);
}

.settings-row__control {
  width: 160px;
}

.settings-row__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.settings-row__notes {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--ops-text);
  white-space: pre-wrap;
  max-height: 120px;
  overflow: auto;
}

.settings-update-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.settings-help-list {
  margin: 0;
  padding-left: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
  color: var(--ops-text);
  line-height: 1.5;
}
</style>
