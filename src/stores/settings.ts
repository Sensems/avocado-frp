import { ref } from 'vue'
import { defineStore } from 'pinia'

import type { CommandError } from '@/domain/errors'
import type {
  ApplyLocalMonitorRequest,
  ApplyLocalMonitorResult,
} from '@/domain/monitor'
import type { AppSettings, AppSettingsPatch } from '@/domain/settings'
import { normalizeCommandError } from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const lastError = ref<CommandError | null>(null)

  const load = async (): Promise<AppSettings> => {
    loading.value = true
    lastError.value = null
    try {
      const next = await tauriClient.getAppSettings()
      settings.value = next
      return next
    } catch (error) {
      const normalized = normalizeCommandError(error)
      lastError.value = normalized
      throw normalized
    } finally {
      loading.value = false
    }
  }

  const update = async (patch: AppSettingsPatch): Promise<AppSettings> => {
    saving.value = true
    lastError.value = null
    try {
      const next = await tauriClient.updateAppSettings(patch)
      settings.value = next
      return next
    } catch (error) {
      const normalized = normalizeCommandError(error)
      lastError.value = normalized
      throw normalized
    } finally {
      saving.value = false
    }
  }

  const applyLocalMonitor = async (
    request: ApplyLocalMonitorRequest,
  ): Promise<ApplyLocalMonitorResult> => {
    saving.value = true
    lastError.value = null
    try {
      const result = await tauriClient.applyLocalMonitor(request)
      settings.value = result.settings
      return result
    } catch (error) {
      const normalized = normalizeCommandError(error)
      lastError.value = normalized
      throw normalized
    } finally {
      saving.value = false
    }
  }

  return {
    settings,
    loading,
    saving,
    lastError,
    load,
    update,
    applyLocalMonitor,
  }
})
