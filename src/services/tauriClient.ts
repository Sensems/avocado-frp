import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

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
  onProcessStateChanged,
  onConfigChanged,
  onLogEntry,
}

export type TauriUnlistenFn = UnlistenFn
