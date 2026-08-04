import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  check as checkUpdater,
  type DownloadEvent,
  type Update,
} from '@tauri-apps/plugin-updater'

import type {
  ConfigChangeRequest,
  ConfigKind,
  ConfigPreview,
  FrpcConfigSnapshot,
  FrpsConfigSnapshot,
  SaveAndRestartResult,
  ValidationReport,
} from '@/domain/config'
import type {
  ConfigChangedEvent,
  LogEntry,
  ProcessKind,
  ProcessSnapshot,
  StopAllResult,
} from '@/domain/process'
import type { DiagnosticsReport } from '@/domain/diagnostics'
import type {
  ApplyLocalMonitorRequest,
  ApplyLocalMonitorResult,
  FrpcTrafficResult,
} from '@/domain/monitor'
import type { AppSettings, AppSettingsPatch } from '@/domain/settings'
import type {
  AvailableUpdate,
  UpdateDownloadProgress,
} from '@/domain/updater'
import { mapUpdateError } from '@/services/errorMapper'

type ConfigSnapshotFor<K extends ConfigKind> = K extends 'frpc'
  ? FrpcConfigSnapshot
  : FrpsConfigSnapshot

type ConfigChangeResult<R extends ConfigChangeRequest> = R extends {
  kind: 'frpc'
}
  ? FrpcConfigSnapshot
  : FrpsConfigSnapshot

const getConfigSnapshot = <K extends ConfigKind>(
  kind: K,
): Promise<ConfigSnapshotFor<K>> =>
  invoke<ConfigSnapshotFor<K>>('get_config_snapshot', { kind })

const validateConfigSource = (
  kind: ConfigKind,
  raw: string,
): Promise<ValidationReport> =>
  invoke<ValidationReport>('validate_config_source', { kind, raw })

const previewConfigChange = (
  request: ConfigChangeRequest,
): Promise<ConfigPreview> =>
  invoke<ConfigPreview>('preview_config_change', { request })

const applyConfigChange = <R extends ConfigChangeRequest>(
  request: R,
): Promise<ConfigChangeResult<R>> =>
  invoke<ConfigChangeResult<R>>('apply_config_change', { request })

const restoreConfigBackup = <K extends ConfigKind>(
  kind: K,
  expectedRevision: string,
): Promise<ConfigSnapshotFor<K>> =>
  invoke<ConfigSnapshotFor<K>>('restore_config_backup', {
    kind,
    expectedRevision,
  })

const saveConfigAndRestart = (
  request: ConfigChangeRequest,
): Promise<SaveAndRestartResult> =>
  invoke<SaveAndRestartResult>('save_config_and_restart', { request })

const getProcessSnapshot = (kind: ProcessKind): Promise<ProcessSnapshot> =>
  invoke<ProcessSnapshot>('get_process_snapshot', { kind })

const startProcess = (kind: ProcessKind): Promise<ProcessSnapshot> =>
  invoke<ProcessSnapshot>('start_process', { kind })

const stopProcess = (kind: ProcessKind): Promise<ProcessSnapshot> =>
  invoke<ProcessSnapshot>('stop_process', { kind })

const restartProcess = (kind: ProcessKind): Promise<ProcessSnapshot> =>
  invoke<ProcessSnapshot>('restart_process', { kind })

const stopAllProcesses = (): Promise<StopAllResult> =>
  invoke<StopAllResult>('stop_all_processes')

const prepareShutdown = (): Promise<StopAllResult> =>
  invoke<StopAllResult>('prepare_shutdown')

const exportLogs = (path: string): Promise<string> =>
  invoke<string>('export_logs', { path })

/** `null` / omitted deletes both frpc and frps managed log files. */
const deleteDiskLogs = (kind?: ProcessKind | null): Promise<void> =>
  invoke<void>('delete_disk_logs', { kind: kind ?? null })

const exportDeployScript = (
  path: string,
  tomlContent: string,
): Promise<void> =>
  invoke<void>('export_deploy_script', { path, tomlContent })

const getFrpcTraffic = (): Promise<FrpcTrafficResult> =>
  invoke<FrpcTrafficResult>('get_frpc_traffic')

const getAppSettings = (): Promise<AppSettings> =>
  invoke<AppSettings>('get_app_settings')

const updateAppSettings = (patch: AppSettingsPatch): Promise<AppSettings> =>
  invoke<AppSettings>('update_app_settings', { patch })

const applyLocalMonitor = (
  request: ApplyLocalMonitorRequest,
): Promise<ApplyLocalMonitorResult> =>
  invoke<ApplyLocalMonitorResult>('apply_local_monitor', { request })

const runDiagnostics = (): Promise<DiagnosticsReport> =>
  invoke<DiagnosticsReport>('run_diagnostics')

/** Save a redacted diagnostics zip at the given file path. */
const exportDiagnosticsPack = (path: string): Promise<string> =>
  invoke<string>('export_diagnostics_pack', { path })

const onProcessStateChanged = (
  listener: (snapshot: ProcessSnapshot) => void,
): Promise<UnlistenFn> =>
  listen<ProcessSnapshot>('process://state-changed', (event) => {
    listener(event.payload)
  })

const onConfigChanged = (
  listener: (event: ConfigChangedEvent) => void,
): Promise<UnlistenFn> =>
  listen<ConfigChangedEvent>('config://changed', (event) => {
    listener(event.payload)
  })

const onLogEntry = (
  listener: (entry: LogEntry) => void,
): Promise<UnlistenFn> =>
  listen<LogEntry>('log://entry', (event) => {
    listener(event.payload)
  })

/** Held after a successful check so install does not re-expose plugin types. */
let pendingUpdate: Update | null = null

type UpdaterStateListener = (progress: UpdateDownloadProgress | null) => void
const updaterProgressListeners = new Set<UpdaterStateListener>()

const toAvailableUpdate = (update: Update): AvailableUpdate => ({
  currentVersion: update.currentVersion,
  version: update.version,
  body: update.body,
  date: update.date,
})

const notifyUpdaterProgress = (progress: UpdateDownloadProgress | null) => {
  for (const listener of updaterProgressListeners) {
    listener(progress)
  }
}

/** Check for updates. Returns null when already up to date. */
const checkForUpdates = async (): Promise<AvailableUpdate | null> => {
  try {
    if (pendingUpdate) {
      await pendingUpdate.close().catch(() => undefined)
      pendingUpdate = null
    }
    const update = await checkUpdater()
    pendingUpdate = update
    return update ? toAvailableUpdate(update) : null
  } catch (error) {
    pendingUpdate = null
    throw mapUpdateError(error)
  }
}

/**
 * Download and install the update from the last successful `checkForUpdates`.
 * Call `prepareShutdown` / stop sidecars before this when the user confirms.
 */
const downloadAndInstallUpdate = async (
  onProgress?: (progress: UpdateDownloadProgress) => void,
): Promise<void> => {
  if (!pendingUpdate) {
    throw mapUpdateError(
      new Error('No pending update. Check for updates before installing.'),
    )
  }

  let downloadedBytes = 0
  let contentLength: number | undefined

  const handleEvent = (event: DownloadEvent) => {
    if (event.event === 'Started') {
      downloadedBytes = 0
      contentLength = event.data.contentLength
    } else if (event.event === 'Progress') {
      downloadedBytes += event.data.chunkLength
    }

    const progress: UpdateDownloadProgress = {
      downloadedBytes,
      contentLength,
      percent:
        contentLength && contentLength > 0
          ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
          : undefined,
    }
    onProgress?.(progress)
    notifyUpdaterProgress(progress)
  }

  try {
    await pendingUpdate.downloadAndInstall(handleEvent)
    notifyUpdaterProgress(null)
  } catch (error) {
    throw mapUpdateError(error)
  }
}

/** Progress listener for download/install (in-process; not a Tauri event). */
const onUpdaterStateChanged = (
  listener: UpdaterStateListener,
): (() => void) => {
  updaterProgressListeners.add(listener)
  return () => {
    updaterProgressListeners.delete(listener)
  }
}

export const tauriClient = {
  getConfigSnapshot,
  validateConfigSource,
  previewConfigChange,
  applyConfigChange,
  restoreConfigBackup,
  saveConfigAndRestart,
  getProcessSnapshot,
  startProcess,
  stopProcess,
  restartProcess,
  stopAllProcesses,
  prepareShutdown,
  exportLogs,
  deleteDiskLogs,
  exportDeployScript,
  getFrpcTraffic,
  getAppSettings,
  updateAppSettings,
  applyLocalMonitor,
  runDiagnostics,
  exportDiagnosticsPack,
  checkForUpdates,
  downloadAndInstallUpdate,
  onUpdaterStateChanged,
  onProcessStateChanged,
  onConfigChanged,
  onLogEntry,
}

export type TauriUnlistenFn = UnlistenFn
