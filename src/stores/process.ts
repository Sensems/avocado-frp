import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { CommandError } from '@/domain/errors'
import type {
  ProcessKind,
  ProcessPhase,
  ProcessSnapshot,
} from '@/domain/process'
import { normalizeCommandError } from '@/services/errorMapper'
import {
  tauriClient,
  type TauriUnlistenFn,
} from '@/services/tauriClient'

export const isRunningPhase = (phase: ProcessPhase): boolean =>
  phase === 'starting' ||
  phase === 'healthy' ||
  phase === 'degraded' ||
  phase === 'stopping'

export type ProcessActionResult =
  | { ok: true; snapshot: ProcessSnapshot }
  | { ok: false; error: CommandError }

export const useProcessStore = defineStore('process', () => {
  const frpc = ref<ProcessSnapshot | null>(null)
  const frps = ref<ProcessSnapshot | null>(null)
  const lastFault = ref<CommandError | null>(null)
  const pendingRestart = ref<{ frpc: boolean; frps: boolean }>({
    frpc: false,
    frps: false,
  })
  const frpcLoading = ref(false)
  const frpsLoading = ref(false)

  let initialization: Promise<void> | undefined
  let unlistenProcessState: TauriUnlistenFn | undefined

  const frpcRunning = computed(() =>
    isRunningPhase(frpc.value?.phase ?? 'stopped'),
  )
  const frpsRunning = computed(() =>
    isRunningPhase(frps.value?.phase ?? 'stopped'),
  )

  const applySnapshot = (snapshot: ProcessSnapshot) => {
    if (snapshot.kind === 'frpc') {
      frpc.value = snapshot
    } else {
      frps.value = snapshot
    }

    if (
      snapshot.lastError &&
      (snapshot.phase === 'crashed' || snapshot.phase === 'degraded')
    ) {
      lastFault.value = snapshot.lastError
    }
  }

  const initialize = async () => {
    if (!unlistenProcessState) {
      unlistenProcessState =
        await tauriClient.onProcessStateChanged(applySnapshot)
    }

    const [frpcSnapshot, frpsSnapshot] = await Promise.all([
      tauriClient.getProcessSnapshot('frpc'),
      tauriClient.getProcessSnapshot('frps'),
    ])
    applySnapshot(frpcSnapshot)
    applySnapshot(frpsSnapshot)
  }

  const init = (): Promise<void> => {
    if (!initialization) {
      initialization = initialize().catch((error) => {
        initialization = undefined
        throw error
      })
    }
    return initialization
  }

  const runProcessCommand = async (
    kind: ProcessKind,
    command: (kind: ProcessKind) => Promise<ProcessSnapshot>,
  ): Promise<ProcessActionResult> => {
    const loading = kind === 'frpc' ? frpcLoading : frpsLoading
    loading.value = true

    try {
      const snapshot = await command(kind)
      applySnapshot(snapshot)
      if (kind === 'frpc') {
        pendingRestart.value.frpc = false
      } else {
        pendingRestart.value.frps = false
      }
      return { ok: true, snapshot }
    } catch (error) {
      const commandError = normalizeCommandError(error)
      lastFault.value = commandError
      return { ok: false, error: commandError }
    } finally {
      loading.value = false
    }
  }

  const start = (kind: ProcessKind) =>
    runProcessCommand(kind, tauriClient.startProcess)
  const stop = (kind: ProcessKind) =>
    runProcessCommand(kind, tauriClient.stopProcess)
  const restart = (kind: ProcessKind) =>
    runProcessCommand(kind, tauriClient.restartProcess)

  const setPendingRestart = (kind: ProcessKind, value: boolean) => {
    if (kind === 'frpc') {
      pendingRestart.value.frpc = value
    } else {
      pendingRestart.value.frps = value
    }
  }

  const clearPendingRestart = (kind: ProcessKind) => {
    setPendingRestart(kind, false)
  }

  const setLastFault = (error: CommandError | null) => {
    lastFault.value = error
  }

  const cleanup = () => {
    unlistenProcessState?.()
    unlistenProcessState = undefined
    initialization = undefined
  }

  return {
    frpc,
    frps,
    lastFault,
    pendingRestart,
    frpcLoading,
    frpsLoading,
    frpcRunning,
    frpsRunning,
    init,
    start,
    stop,
    restart,
    setPendingRestart,
    clearPendingRestart,
    setLastFault,
    applySnapshot,
    cleanup,
  }
})
