export type ErrorCode =
  | 'CONFIG_INVALID'
  | 'CONFIG_CONFLICT'
  | 'CONFIG_IO'
  | 'SIDECAR_MISSING'
  | 'SIDECAR_INCOMPATIBLE'
  | 'PORT_CONFLICT'
  | 'PROCESS_ALREADY_RUNNING'
  | 'PROCESS_NOT_RUNNING'
  | 'SPAWN_FAILED'
  | 'HEALTHCHECK_FAILED'
  | 'STOP_TIMEOUT'
  | 'PERMISSION_DENIED'
  | 'NETWORK_UNREACHABLE'
  | 'UPDATE_FAILED'
  | 'UNKNOWN'

export interface CommandError {
  code: ErrorCode
  message: string
  detail?: string
  recoverable: boolean
  suggestedAction?: string
}
