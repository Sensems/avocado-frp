import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import { APP_VERSION } from '@/appVersion'
import type { CommandError } from '@/domain/errors'
import type {
  AvailableUpdate,
  UpdateDownloadProgress,
  UpdaterPhase,
} from '@/domain/updater'
import { mapUpdateError, normalizeCommandError } from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'

export const useUpdaterStore = defineStore('updater', () => {
  const phase = ref<UpdaterPhase>('idle')
  const available = ref<AvailableUpdate | null>(null)
  const progress = ref<UpdateDownloadProgress | null>(null)
  const lastCheckedAt = ref<string | null>(null)
  const lastError = ref<CommandError | null>(null)
  const launchNotice = ref(false)
  const busy = ref(false)

  const currentVersion = computed(
    () => available.value?.currentVersion ?? APP_VERSION,
  )

  const check = async (options?: { fromLaunch?: boolean }): Promise<AvailableUpdate | null> => {
    busy.value = true
    phase.value = 'checking'
    lastError.value = null
    progress.value = null
    try {
      const result = await tauriClient.checkForUpdates()
      lastCheckedAt.value = new Date().toISOString()
      available.value = result
      if (result) {
        phase.value = 'available'
        if (options?.fromLaunch) {
          launchNotice.value = true
        }
      } else {
        phase.value = 'upToDate'
        if (!options?.fromLaunch) {
          launchNotice.value = false
        }
      }
      return result
    } catch (error) {
      const mapped = mapUpdateError(error)
      lastError.value = mapped
      phase.value = 'error'
      available.value = null
      throw mapped
    } finally {
      busy.value = false
    }
  }

  /**
   * Stops app-owned sidecars then downloads and installs the pending update.
   * Caller must confirm with the user first.
   */
  const installAfterConfirm = async (): Promise<void> => {
    if (!available.value) {
      throw mapUpdateError(
        new Error('No pending update. Check for updates before installing.'),
      )
    }

    busy.value = true
    lastError.value = null
    phase.value = 'downloading'
    progress.value = { downloadedBytes: 0 }

    try {
      await tauriClient.prepareShutdown()
      await tauriClient.downloadAndInstallUpdate((next) => {
        progress.value = next
        if (next.percent !== undefined && next.percent >= 100) {
          phase.value = 'ready'
        }
      })
      phase.value = 'ready'
      progress.value = null
      launchNotice.value = false
    } catch (error) {
      const normalized = normalizeCommandError(error)
      const mapped =
        normalized.code === 'UNKNOWN' ? mapUpdateError(error) : normalized
      lastError.value = mapped
      phase.value = 'error'
      throw mapped
    } finally {
      busy.value = false
    }
  }

  const dismissLaunchNotice = () => {
    launchNotice.value = false
  }

  const clearError = () => {
    lastError.value = null
    if (phase.value === 'error') {
      phase.value = available.value ? 'available' : 'idle'
    }
  }

  return {
    phase,
    available,
    progress,
    lastCheckedAt,
    lastError,
    launchNotice,
    busy,
    currentVersion,
    check,
    installAfterConfirm,
    dismissLaunchNotice,
    clearError,
  }
})
