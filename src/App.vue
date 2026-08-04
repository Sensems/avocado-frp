<script setup lang="ts">
import { darkTheme, createDiscreteApi } from 'naive-ui'
import type { GlobalThemeOverrides } from 'naive-ui'
import { computed, watch, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useTheme } from '@/composables/useTheme'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { persistLocale } from '@/lib/preferences'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'
import { useConfigStore } from '@/stores/config'
import { useLogsStore } from '@/stores/logs'
import { useProcessStore } from '@/stores/process'
import { useUpdaterStore } from '@/stores/updater'
import { Sun, Moon, Monitor, Languages, Minus, X, Maximize2 } from 'lucide-vue-next'
import AppShell from '@/components/shell/AppShell.vue'

const { isDark, themeMode, cycleTheme } = useTheme()
const processStore = useProcessStore()
const configStore = useConfigStore()
const logsStore = useLogsStore()
const updaterStore = useUpdaterStore()
const { frpcRunning, frpsRunning } = storeToRefs(processStore)
const { locale, t } = useI18n()

const theme = computed(() => (isDark.value ? darkTheme : null))

/** 语言切换时持久化 */
watch(locale, (value) => {
  persistLocale(value)
})

/** 主题切换图标 */
const themeIcon = computed(() => {
  if (themeMode.value === 'dark') return Moon
  if (themeMode.value === 'light') return Sun
  return Monitor
})

/** 主题切换 tooltip */
const themeTooltip = computed(() => {
  const labels: Record<string, string> = {
    system: t('theme.system'),
    light: t('theme.light'),
    dark: t('theme.dark'),
  }
  return labels[themeMode.value] ?? ''
})

const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const dark = isDark.value
  return {
    common: {
      borderRadius: '8px',
      primaryColor: '#2563eb',
      primaryColorHover: '#3b82f6',
      primaryColorPressed: '#1d4ed8',
      primaryColorSuppl: '#2563eb',
      infoColor: '#2563eb',
      infoColorHover: '#3b82f6',
      infoColorPressed: '#1d4ed8',
      successColor: '#15803d',
      warningColor: '#a16207',
      errorColor: '#b91c1c',
      errorColorHover: '#dc2626',
      errorColorPressed: '#991b1b',
      bodyColor: dark ? '#0f1419' : '#f4f6f8',
      cardColor: dark ? '#171d25' : '#ffffff',
      modalColor: dark ? '#171d25' : '#ffffff',
      popoverColor: dark ? '#171d25' : '#ffffff',
      inputColor: dark ? '#171d25' : '#ffffff',
      tableColor: dark ? '#171d25' : '#ffffff',
      hoverColor: dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.04)',
      dividerColor: dark ? '#2a3441' : '#d7dee7',
      borderColor: dark ? '#2a3441' : '#d7dee7',
      textColorBase: dark ? '#e7edf5' : '#142033',
      textColor1: dark ? '#e7edf5' : '#142033',
      textColor2: dark ? '#9aa8b5' : '#5b6b7c',
      textColor3: dark ? '#9aa8b5' : '#5b6b7c',
    },
    Card: {
      borderRadius: '8px',
      color: dark ? '#171d25' : '#ffffff',
      borderColor: dark ? '#2a3441' : '#d7dee7',
    },
    Button: {
      borderRadiusMedium: '8px',
      borderRadiusLarge: '8px',
    },
    Input: {
      borderRadius: '8px',
      color: dark ? '#171d25' : '#ffffff',
      borderHover: '1px solid #2563eb',
      borderFocus: '1px solid #2563eb',
    },
    Tabs: {
      tabBorderRadius: '8px',
    },
    Switch: {
      railColorActive: '#15803d',
    }
  }
})

const silentCheckUpdatesOnLaunch = async () => {
  try {
    const settings = await tauriClient.getAppSettings()
    if (!settings.checkUpdatesOnLaunch) return
    const update = await updaterStore.check({ fromLaunch: true })
    if (!update) return
    createDiscreteApi(['message'], {
      configProviderProps: computed(() => ({
        theme: theme.value,
        themeOverrides: themeOverrides.value,
      })),
    }).message.info(
      t('settings.launchUpdateAvailable', { version: update.version }),
      { duration: 8000 },
    )
  } catch {
    // Launch check is best-effort; Settings remains the primary path.
  }
}

onMounted(() => {
  void Promise.allSettled([
    processStore.init(),
    configStore.init(),
    logsStore.init(),
  ]).then(() => {
    void silentCheckUpdatesOnLaunch()
  })
})

const appWindow = getCurrentWindow()

const discreteDialog = () =>
  createDiscreteApi(['dialog'], {
    configProviderProps: computed(() => ({
      theme: theme.value,
      themeOverrides: themeOverrides.value,
    })),
  }).dialog

const quitApplication = async () => {
  try {
    await tauriClient.prepareShutdown()
    await appWindow.close()
  } catch (error) {
    const commandError = normalizeCommandError(error)
    discreteDialog().error({
      title: t('app.closeFailedTitle'),
      content: t('app.closeFailedContent', {
        error: t(getCommandErrorI18nKey(commandError)),
      }),
      positiveText: t('forms.confirm'),
    })
  }
}

const handleMinimize = () => {
  appWindow.minimize()
}

const handleMaximize = async () => {
  const isMaximized = await appWindow.isMaximized()
  if (isMaximized) {
    appWindow.unmaximize()
  } else {
    appWindow.maximize()
  }
}

const handleClose = () => {
  if (frpcRunning.value || frpsRunning.value) {
    discreteDialog().warning({
      title: t('app.closeTitle'),
      content: t('app.closeRunningContent'),
      positiveText: t('app.minimizeToTray'),
      negativeText: t('app.closeApp'),
      onPositiveClick: () => {
        appWindow.hide()
      },
      onNegativeClick: () => {
        void quitApplication()
      },
    })
  } else {
    void quitApplication()
  }
}
</script>

<template>
  <n-config-provider
    :theme="theme"
    :theme-overrides="themeOverrides"
    class="h-screen w-full flex flex-col overflow-hidden"
    :style="{ background: 'var(--ops-bg)', color: 'var(--ops-text)' }"
  >
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <!-- Window chrome -->
        <header
          data-tauri-drag-region
          @mousedown="appWindow.startDragging()"
          class="h-12 border-b flex items-center justify-between px-5 shrink-0 select-none z-50 relative"
          :style="{
            borderColor: 'var(--ops-border)',
            background: 'var(--ops-surface)',
          }"
        >
          <div class="flex items-center gap-2.5">
            <img src="@/assets/logo.png" alt="Logo" class="w-7 h-7" />
            <h1
              class="text-[14px] font-bold tracking-wide leading-1"
              :style="{ color: 'var(--ops-text)' }"
            >
              Avocado FRP
            </h1>
          </div>

          <div class="flex items-center gap-3">
            <n-popselect
              v-model:value="locale"
              :options="[{ label: '简体中文', value: 'zh' }, { label: 'English', value: 'en' }]"
              trigger="click"
              size="small"
            >
              <button
                @mousedown.stop
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-colors duration-150 cursor-pointer pointer-events-auto"
                :aria-label="t('dashboard.language')"
                :style="{ color: 'var(--ops-muted)' }"
              >
                <Languages :size="15" />
              </button>
            </n-popselect>

            <n-tooltip trigger="hover">
              <template #trigger>
                <button
                  @click="cycleTheme"
                  @mousedown.stop
                  class="w-7 h-7 rounded-lg flex items-center justify-center transition-colors duration-150 cursor-pointer pointer-events-auto"
                  :aria-label="themeTooltip"
                  :style="{ color: 'var(--ops-muted)' }"
                >
                  <component :is="themeIcon" :size="15" />
                </button>
              </template>
              {{ themeTooltip }}
            </n-tooltip>

            <div class="w-px h-4" :style="{ background: 'var(--ops-border)' }" />

            <div
              class="flex items-center justify-end w-[100px] gap-1 z-50 relative pointer-events-auto"
              @mousedown.stop
            >
              <button
                @click.stop="handleMinimize"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-colors duration-150 cursor-pointer pointer-events-auto"
                :aria-label="t('app.minimize')"
                :style="{ color: 'var(--ops-muted)' }"
              >
                <Minus :size="15" />
              </button>
              <button
                @click.stop="handleMaximize"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-colors duration-150 cursor-pointer pointer-events-auto"
                :aria-label="t('app.maximize')"
                :style="{ color: 'var(--ops-muted)' }"
              >
                <Maximize2 :size="14" />
              </button>
              <button
                @click.stop="handleClose"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-colors duration-150 cursor-pointer hover:bg-red-600 hover:text-white pointer-events-auto"
                :aria-label="t('app.closeApp')"
                :style="{ color: 'var(--ops-muted)' }"
              >
                <X :size="16" />
              </button>
            </div>
          </div>
        </header>

        <AppShell />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
