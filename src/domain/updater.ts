/** UI / store phase for application updates (not a process lifecycle phase). */
export type UpdaterPhase =
  | 'idle'
  | 'checking'
  | 'available'
  | 'upToDate'
  | 'downloading'
  | 'ready'
  | 'error'

/** Public snapshot of an available update (no plugin types). */
export interface AvailableUpdate {
  currentVersion: string
  version: string
  body?: string
  date?: string
}

/** Normalized download progress for Settings UI. */
export interface UpdateDownloadProgress {
  /** Bytes received so far. */
  downloadedBytes: number
  /** Total size when known. */
  contentLength?: number
  /** 0–100 when contentLength is known. */
  percent?: number
}

export interface UpdaterState {
  phase: UpdaterPhase
  currentVersion: string
  available?: AvailableUpdate | null
  progress: UpdateDownloadProgress | null
  lastCheckedAt?: string
  /** True when a silent launch check found an update. */
  launchNotice: boolean
}
