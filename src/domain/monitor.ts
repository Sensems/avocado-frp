import type { AppSettings } from './settings'

/** Structured local-monitor / traffic status for Overview empty states. */
export type MonitorStatus =
  | 'disabled'
  | 'process_stopped'
  | 'port_conflict'
  | 'auth_failed'
  | 'timeout'
  | 'ok'
  | 'not_configured'

export interface FrpcTrafficResult {
  status: MonitorStatus
  body?: string
}

export interface ApplyLocalMonitorRequest {
  enabled: boolean
  addr: string
  port: number
  user?: string | null
  password?: string | null
}

export interface ApplyLocalMonitorResult {
  settings: AppSettings
  configPatched: boolean
  pendingRestart: boolean
}
