export interface LogPolicy {
  maxFileBytes: number
  maxRotatedFiles: number
}

export interface LocalMonitorPrefs {
  enabled: boolean
  addr: string
  port: number
  /** Optional Basic auth user; defaults to "admin" when password is set. */
  user?: string
  /** Optional; never log plaintext. */
  password?: string
}

export interface AppSettings {
  schemaVersion: number
  logPolicy: LogPolicy
  localMonitor: LocalMonitorPrefs
  logPolicyNoticeShown: boolean
}

export interface LogPolicyPatch {
  maxFileBytes?: number
  maxRotatedFiles?: number
}

export interface LocalMonitorPrefsPatch {
  enabled?: boolean
  addr?: string
  port?: number
  /** `null` clears; omit leaves unchanged. */
  user?: string | null
  /** `null` clears; omit leaves unchanged. */
  password?: string | null
}

export interface AppSettingsPatch {
  logPolicy?: LogPolicyPatch
  localMonitor?: LocalMonitorPrefsPatch
  logPolicyNoticeShown?: boolean
}
