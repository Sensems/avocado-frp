import type { CommandError } from './errors'
import type { ConfigKind } from './config'

export type ProcessKind = 'frpc' | 'frps'

export type ProcessPhase =
  | 'stopped'
  | 'starting'
  | 'healthy'
  | 'degraded'
  | 'stopping'
  | 'crashed'

export interface ProcessSnapshot {
  kind: ProcessKind
  phase: ProcessPhase
  pid?: number
  startedAt?: string
  uptimeSeconds: number
  configRevision?: string
  lastExitCode?: number
  lastError?: CommandError
}

export interface StopAllResult {
  frpc: ProcessSnapshot
  frps: ProcessSnapshot
  errors: CommandError[]
}

/** Event payload for `process://state-changed` — full process snapshot. */
export type ProcessStateChangedEvent = ProcessSnapshot

/** Event payload for `config://changed` — kind and revision only. */
export interface ConfigChangedEvent {
  kind: ConfigKind
  revision: string
}

export type LogStream = 'stdout' | 'stderr'

/** Minimal payload for temporary `log://entry` events (RFC 3339 timestamp). */
export interface LogEntry {
  kind: ProcessKind
  stream: LogStream
  text: string
  timestamp: string
}
