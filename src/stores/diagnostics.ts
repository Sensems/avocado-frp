import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { DiagnosticsReport } from '@/domain/diagnostics'
import type { CommandError } from '@/domain/errors'
import { normalizeCommandError } from '@/services/errorMapper'
import { tauriClient } from '@/services/tauriClient'

export const useDiagnosticsStore = defineStore('diagnostics', () => {
  const report = ref<DiagnosticsReport | null>(null)
  const running = ref(false)
  const exporting = ref(false)
  const lastError = ref<CommandError | null>(null)

  const hasReport = computed(() => report.value !== null)
  const finishedAt = computed(() => report.value?.finishedAt ?? null)

  const run = async (): Promise<DiagnosticsReport> => {
    running.value = true
    lastError.value = null
    try {
      const next = await tauriClient.runDiagnostics()
      report.value = next
      return next
    } catch (error) {
      const normalized = normalizeCommandError(error)
      lastError.value = normalized
      throw normalized
    } finally {
      running.value = false
    }
  }

  const exportPack = async (path: string): Promise<string> => {
    exporting.value = true
    lastError.value = null
    try {
      return await tauriClient.exportDiagnosticsPack(path)
    } catch (error) {
      const normalized = normalizeCommandError(error)
      lastError.value = normalized
      throw normalized
    } finally {
      exporting.value = false
    }
  }

  return {
    report,
    running,
    exporting,
    lastError,
    hasReport,
    finishedAt,
    run,
    exportPack,
  }
})
