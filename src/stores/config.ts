import { ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ConfigChangeRequest,
  ConfigKind,
  ConfigSnapshot,
  FrpcConfigPatch,
  FrpcConfigSnapshot,
  FrpsConfigPatch,
  FrpsConfigSnapshot,
  SaveAndRestartResult,
} from '@/domain/config'
import type { CommandError } from '@/domain/errors'
import { normalizeCommandError } from '@/services/errorMapper'
import {
  tauriClient,
  type TauriUnlistenFn,
} from '@/services/tauriClient'
import { isRunningPhase, useProcessStore } from '@/stores/process'

export type ConfigActionResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: CommandError; recovery?: SaveAndRestartResult['recovery'] }

export const useConfigStore = defineStore('config', () => {
  const frpc = ref<FrpcConfigSnapshot | null>(null)
  const frps = ref<FrpsConfigSnapshot | null>(null)
  const lastError = ref<CommandError | null>(null)

  let initialization: Promise<void> | undefined
  let unlistenConfigChanged: TauriUnlistenFn | undefined

  const applySnapshot = (snapshot: ConfigSnapshot) => {
    if (snapshot.kind === 'frpc') {
      frpc.value = snapshot
    } else {
      frps.value = snapshot
    }
  }

  const load = async (kind: ConfigKind): Promise<ConfigSnapshot> => {
    const snapshot = await tauriClient.getConfigSnapshot(kind)
    applySnapshot(snapshot)
    return snapshot
  }

  const ensureListeners = async () => {
    if (unlistenConfigChanged) return

    unlistenConfigChanged = await tauriClient.onConfigChanged(async (event) => {
      try {
        await load(event.kind)
      } catch (error) {
        lastError.value = normalizeCommandError(error)
      }
    })
  }

  const loadAll = async (): Promise<void> => {
    await ensureListeners()
    const [frpcSnapshot, frpsSnapshot] = await Promise.all([
      tauriClient.getConfigSnapshot('frpc'),
      tauriClient.getConfigSnapshot('frps'),
    ])
    applySnapshot(frpcSnapshot)
    applySnapshot(frpsSnapshot)
  }

  const init = (): Promise<void> => {
    if (!initialization) {
      initialization = loadAll().catch((error) => {
        initialization = undefined
        throw error
      })
    }
    return initialization
  }

  const markPendingIfRunning = (kind: ConfigKind) => {
    const processStore = useProcessStore()
    const snapshot = kind === 'frpc' ? processStore.frpc : processStore.frps
    if (snapshot && isRunningPhase(snapshot.phase)) {
      processStore.setPendingRestart(kind, true)
    }
  }

  const applyChange = async <R extends ConfigChangeRequest>(
    request: R,
  ): Promise<ConfigActionResult<ConfigSnapshot>> => {
    try {
      const snapshot = await tauriClient.applyConfigChange(request)
      applySnapshot(snapshot)
      markPendingIfRunning(request.kind)
      return { ok: true, data: snapshot }
    } catch (error) {
      const commandError = normalizeCommandError(error)
      lastError.value = commandError
      return { ok: false, error: commandError }
    }
  }

  function applyPatch(
    kind: 'frpc',
    expectedRevision: string,
    patch: FrpcConfigPatch,
  ): Promise<ConfigActionResult<ConfigSnapshot>>
  function applyPatch(
    kind: 'frps',
    expectedRevision: string,
    patch: FrpsConfigPatch,
  ): Promise<ConfigActionResult<ConfigSnapshot>>
  function applyPatch(
    kind: ConfigKind,
    expectedRevision: string,
    patch: FrpcConfigPatch | FrpsConfigPatch,
  ): Promise<ConfigActionResult<ConfigSnapshot>> {
    if (kind === 'frpc') {
      return applyChange({
        kind: 'frpc',
        expectedRevision,
        change: { mode: 'patch', patch: patch as FrpcConfigPatch },
      })
    }
    return applyChange({
      kind: 'frps',
      expectedRevision,
      change: { mode: 'patch', patch: patch as FrpsConfigPatch },
    })
  }

  const applySource = (
    kind: ConfigKind,
    expectedRevision: string,
    raw: string,
  ): Promise<ConfigActionResult<ConfigSnapshot>> => {
    if (kind === 'frpc') {
      return applyChange({
        kind: 'frpc',
        expectedRevision,
        change: { mode: 'source', raw },
      })
    }
    return applyChange({
      kind: 'frps',
      expectedRevision,
      change: { mode: 'source', raw },
    })
  }

  const saveAndRestart = async (
    request: ConfigChangeRequest,
  ): Promise<ConfigActionResult<SaveAndRestartResult>> => {
    const processStore = useProcessStore()
    try {
      const result = await tauriClient.saveConfigAndRestart(request)
      applySnapshot(result.config)
      processStore.applySnapshot(result.process)
      processStore.clearPendingRestart(request.kind)

      if (result.failure) {
        lastError.value = result.failure
        processStore.setLastFault(result.failure)
        return {
          ok: false,
          error: result.failure,
          recovery: result.recovery,
        }
      }

      return { ok: true, data: result }
    } catch (error) {
      const commandError = normalizeCommandError(error)
      lastError.value = commandError
      processStore.setLastFault(commandError)
      return { ok: false, error: commandError }
    }
  }

  const restoreBackup = async (
    kind: ConfigKind,
    expectedRevision: string,
  ): Promise<ConfigActionResult<ConfigSnapshot>> => {
    try {
      const snapshot = await tauriClient.restoreConfigBackup(
        kind,
        expectedRevision,
      )
      applySnapshot(snapshot)
      markPendingIfRunning(kind)
      return { ok: true, data: snapshot }
    } catch (error) {
      const commandError = normalizeCommandError(error)
      lastError.value = commandError
      return { ok: false, error: commandError }
    }
  }

  const cleanup = () => {
    unlistenConfigChanged?.()
    unlistenConfigChanged = undefined
    initialization = undefined
  }

  return {
    frpc,
    frps,
    lastError,
    init,
    loadAll,
    load,
    applyChange,
    applyPatch,
    applySource,
    saveAndRestart,
    restoreBackup,
    cleanup,
  }
})
