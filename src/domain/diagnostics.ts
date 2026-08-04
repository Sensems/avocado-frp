export type DiagnosticStatus = 'pass' | 'warning' | 'fail'

export interface DiagnosticResult {
  id: string
  status: DiagnosticStatus
  /** Stable i18n key under `diagnostics.checks.*` when present. */
  titleKey?: string
  detail: string
  /** Stable action code mapped via `diagnostics.actions.*`. */
  suggestedAction: string
}

export interface DiagnosticsReport {
  startedAt: string
  finishedAt: string
  results: DiagnosticResult[]
}
