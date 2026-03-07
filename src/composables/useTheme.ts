import { ref, computed, watchEffect } from 'vue'
import { useOsTheme } from 'naive-ui'

type ThemeMode = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'frp-desktop-theme'

/**
 * 读取持久化的主题偏好
 */
const getStoredTheme = (): ThemeMode => {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored
  }
  return 'system'
}

/** 全局单例状态 — 多个组件共享同一份 */
const themeMode = ref<ThemeMode>(getStoredTheme())
const osTheme = useOsTheme()

/**
 * 主题管理 composable
 * - themeMode: 用户选择（system / light / dark）
 * - isDark: 实际是否暗色
 * - cycleTheme: 循环切换 system → light → dark → system
 */
export const useTheme = () => {
  const isDark = computed(() => {
    if (themeMode.value === 'system') {
      return osTheme.value === 'dark'
    }
    return themeMode.value === 'dark'
  })

  /** 持久化 + 同步 DOM class */
  watchEffect(() => {
    localStorage.setItem(STORAGE_KEY, themeMode.value)
    if (isDark.value) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  })

  /** 三模式循环：system → light → dark → system */
  const cycleTheme = () => {
    const order: ThemeMode[] = ['system', 'light', 'dark']
    const idx = order.indexOf(themeMode.value)
    themeMode.value = order[(idx + 1) % order.length]
  }

  const setTheme = (mode: ThemeMode) => {
    themeMode.value = mode
  }

  return {
    themeMode,
    isDark,
    osTheme,
    cycleTheme,
    setTheme,
  }
}
