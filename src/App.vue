<script setup lang="ts">
import { darkTheme, createDiscreteApi } from 'naive-ui'
import type { GlobalThemeOverrides } from 'naive-ui'
import { computed, watch, onMounted } from 'vue'
import { useTheme } from '@/composables/useTheme'
import { useProcessStatus } from '@/composables/useProcessStatus'
import { useAppLogs } from '@/composables/useAppLogs'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Sun, Moon, Monitor, Languages, Minus, X, Maximize2 } from 'lucide-vue-next'

const { isDark, themeMode, cycleTheme } = useTheme()
const { frpcRunning, frpsRunning } = useProcessStatus()
const { initListeners } = useAppLogs()
const { locale, t } = useI18n()

onMounted(() => {
  initListeners()
})

const theme = computed(() => (isDark.value ? darkTheme : null))

/** 语言切换时持久化 */
watch(locale, (val) => {
  localStorage.setItem('avocado-frp-lang', val)
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
      borderRadius: '12px',
      primaryColor: '#22C55E',
      primaryColorHover: '#4ADE80',
      primaryColorPressed: '#16A34A',
      primaryColorSuppl: '#22C55E',
      infoColor: '#0EA5E9',
      infoColorHover: '#38BDF8',
      infoColorPressed: '#0284C7',
      errorColor: '#EF4444',
      errorColorHover: '#F87171',
      errorColorPressed: '#DC2626',
      bodyColor: dark ? '#020617' : '#F8FAFC',
      cardColor: dark ? '#0F172A' : '#FFFFFF',
      modalColor: dark ? '#0F172A' : '#FFFFFF',
      popoverColor: dark ? '#1E293B' : '#FFFFFF',
      inputColor: dark ? 'rgba(255,255,255,0.04)' : 'rgba(0,0,0,0.02)',
      tableColor: dark ? '#0F172A' : '#FFFFFF',
      hoverColor: dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.04)',
      dividerColor: dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
      borderColor: dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)',
      textColorBase: dark ? '#F8FAFC' : '#0F172A',
      textColor1: dark ? '#F1F5F9' : '#1E293B',
      textColor2: dark ? '#CBD5E1' : '#475569',
      textColor3: dark ? '#94A3B8' : '#64748B',
    },
    Card: {
      borderRadius: '16px',
      color: dark ? 'rgba(15, 23, 42, 0.6)' : 'rgba(255, 255, 255, 0.7)',
      borderColor: dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
    },
    Button: {
      borderRadiusMedium: '10px',
      borderRadiusLarge: '12px',
    },
    Input: {
      borderRadius: '10px',
      color: dark ? 'rgba(255,255,255,0.04)' : 'rgba(0,0,0,0.02)',
      borderHover: '1px solid rgba(34, 197, 94, 0.4)',
      borderFocus: '1px solid rgba(34, 197, 94, 0.6)',
    },
    Tabs: {
      tabBorderRadius: '10px',
    },
    Switch: {
      railColorActive: '#22C55E',
    }
  }
})

const appWindow = getCurrentWindow()

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
    const { dialog } = createDiscreteApi(['dialog'], {
      configProviderProps: computed(() => ({
        theme: theme.value,
        themeOverrides: themeOverrides.value
      }))
    })

    dialog.warning({
      title: t('app.closeTitle'),
      content: t('app.closeRunningContent'),
      positiveText: t('app.minimizeToTray'),
      negativeText: t('app.closeApp'),
      onPositiveClick: () => {
        appWindow.hide()
      },
      onNegativeClick: () => {
        appWindow.close()
      }
    })
  } else {
    appWindow.close()
  }
}
</script>

<template>
  <n-config-provider :theme="theme" :theme-overrides="themeOverrides"
    class="h-screen w-full flex flex-col overflow-hidden transition-colors duration-300" :style="{
      background: isDark
        ? 'linear-gradient(145deg, #020617 0%, #0F172A 50%, #020617 100%)'
        : 'linear-gradient(145deg, #F8FAFC 0%, #F1F5F9 50%, #E2E8F0 100%)'
    }">
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <!-- Header -->
        <header data-tauri-drag-region @mousedown="appWindow.startDragging()"
          class="h-12 border-b flex items-center justify-between px-5 shrink-0 select-none z-50 transition-all duration-300 relative"
          :class="[
            isDark
              ? 'border-white/[0.06] bg-[#0F172A]/80 backdrop-blur-xl'
              : 'border-black/[0.06] bg-white/80 backdrop-blur-xl'
          ]">
          <!-- Left: Logo + App Name -->
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-300" :class="[
              isDark
                ? 'bg-emerald-500/10 border border-emerald-500/20'
                : 'bg-emerald-50 border border-emerald-200'
            ]">
              <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-emerald-500" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2L2 7l10 5 10-5-10-5z" />
                <path d="M2 17l10 5 10-5" />
                <path d="M2 12l10 5 10-5" />
              </svg>
            </div>
            <h1 class="text-[13px] font-bold tracking-wide" :class="isDark ? 'text-slate-200' : 'text-slate-800'">
              Avocado FRP
            </h1>
          </div>

          <!-- Right: Status + Language + Theme -->
          <div class="flex items-center gap-3">
            <!-- Process Status Indicators -->
            <div class="flex items-center gap-2.5 text-[11px] font-medium"
              :class="isDark ? 'text-slate-400' : 'text-slate-500'">
              <!-- frpc status -->
              <n-tooltip trigger="hover">
                <template #trigger>
                  <div class="flex items-center gap-1.5 cursor-default">
                    <span class="w-2 h-2 rounded-full transition-all duration-500" :class="frpcRunning
                      ? 'bg-emerald-500 dot-pulse shadow-[0_0_8px_rgba(34,197,94,0.6)]'
                      : isDark ? 'bg-slate-600' : 'bg-slate-300'
                      " />
                    <span class="hidden sm:inline">frpc</span>
                  </div>
                </template>
                {{ t('status.frpcLabel') }}: {{ frpcRunning ? t('status.running') : t('status.stopped') }}
              </n-tooltip>

              <!-- frps status -->
              <n-tooltip trigger="hover">
                <template #trigger>
                  <div class="flex items-center gap-1.5 cursor-default">
                    <span class="w-2 h-2 rounded-full transition-all duration-500" :class="frpsRunning
                      ? 'bg-violet-500 dot-pulse shadow-[0_0_8px_rgba(139,92,246,0.6)]'
                      : isDark ? 'bg-slate-600' : 'bg-slate-300'
                      " />
                    <span class="hidden sm:inline">frps</span>
                  </div>
                </template>
                {{ t('status.frpsLabel') }}: {{ frpsRunning ? t('status.running') : t('status.stopped') }}
              </n-tooltip>
            </div>

            <!-- Separator -->
            <div class="w-px h-4" :class="isDark ? 'bg-white/8' : 'bg-black/8'" />

            <!-- Language Switcher -->
            <n-popselect v-model:value="locale"
              :options="[{ label: '简体中文', value: 'zh' }, { label: 'English', value: 'en' }]" trigger="click"
              size="small">
              <button @mousedown.stop
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-200 cursor-pointer pointer-events-auto"
                :class="isDark
                  ? 'hover:bg-white/6 text-slate-400 hover:text-slate-200'
                  : 'hover:bg-black/4 text-slate-500 hover:text-slate-700'
                  ">
                <Languages :size="15" />
              </button>
            </n-popselect>

            <!-- Theme Toggle -->
            <n-tooltip trigger="hover">
              <template #trigger>
                <button @click="cycleTheme" @mousedown.stop
                  class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-200 cursor-pointer pointer-events-auto"
                  :class="isDark
                    ? 'hover:bg-white/6 text-slate-400 hover:text-slate-200'
                    : 'hover:bg-black/4 text-slate-500 hover:text-slate-700'
                    ">
                  <component :is="themeIcon" :size="15" />
                </button>
              </template>
              {{ themeTooltip }}
            </n-tooltip>

            <!-- Separator -->
            <div class="w-px h-4" :class="isDark ? 'bg-white/[0.08]' : 'bg-black/[0.08]'" />

            <!-- Window Controls -->
            <div class="flex items-center justify-end w-[100px] gap-1 z-50 relative pointer-events-auto"
              @mousedown.stop>
              <button @click.stop="handleMinimize"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-200 cursor-pointer pointer-events-auto"
                :class="isDark ? 'hover:bg-white/6 text-slate-400 hover:text-slate-200' : 'hover:bg-black/4 text-slate-500 hover:text-slate-700'">
                <Minus :size="15" />
              </button>
              <button @click.stop="handleMaximize"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-200 cursor-pointer pointer-events-auto"
                :class="isDark ? 'hover:bg-white/6 text-slate-400 hover:text-slate-200' : 'hover:bg-black/4 text-slate-500 hover:text-slate-700'">
                <Maximize2 :size="14" />
              </button>
              <button @click.stop="handleClose"
                class="w-7 h-7 rounded-lg flex items-center justify-center transition-all duration-200 cursor-pointer hover:bg-red-500 hover:text-white pointer-events-auto"
                :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                <X :size="16" />
              </button>
            </div>
          </div>
        </header>

        <!-- Main Content Area -->
        <main class="flex-1 overflow-auto relative selection:bg-emerald-500/30 grid-bg">
          <router-view />
        </main>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>