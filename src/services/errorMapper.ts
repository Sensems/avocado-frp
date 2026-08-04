import type { CommandError, ErrorCode } from '@/domain/errors'

const ERROR_CODES: ReadonlySet<ErrorCode> = new Set([
  'CONFIG_INVALID',
  'CONFIG_CONFLICT',
  'CONFIG_IO',
  'SIDECAR_MISSING',
  'SIDECAR_INCOMPATIBLE',
  'PORT_CONFLICT',
  'PROCESS_ALREADY_RUNNING',
  'PROCESS_NOT_RUNNING',
  'SPAWN_FAILED',
  'HEALTHCHECK_FAILED',
  'STOP_TIMEOUT',
  'PERMISSION_DENIED',
  'NETWORK_UNREACHABLE',
  'UPDATE_FAILED',
  'UNKNOWN',
])

const isOptionalString = (value: unknown): value is string | undefined =>
  value === undefined || typeof value === 'string'

const isCommandError = (value: unknown): value is CommandError => {
  if (typeof value !== 'object' || value === null) return false

  const candidate = value as Partial<CommandError>
  return (
    typeof candidate.code === 'string' &&
    ERROR_CODES.has(candidate.code as ErrorCode) &&
    typeof candidate.message === 'string' &&
    typeof candidate.recoverable === 'boolean' &&
    isOptionalString(candidate.detail) &&
    isOptionalString(candidate.suggestedAction)
  )
}

const errorText = (error: unknown): string => {
  if (typeof error === 'string') return error
  if (error instanceof Error && error.message) return error.message
  if (typeof error === 'object' && error !== null) {
    const candidate = error as { message?: unknown }
    if (typeof candidate.message === 'string') return candidate.message
  }
  return 'Unknown command error'
}

/** Map updater / network failures to the stable `UPDATE_FAILED` code. */
export const mapUpdateError = (error: unknown): CommandError => {
  if (isCommandError(error)) {
    return error.code === 'UNKNOWN'
      ? { ...error, code: 'UPDATE_FAILED', recoverable: true }
      : error
  }

  const message = errorText(error)
  return {
    code: 'UPDATE_FAILED',
    message: message || 'The update operation failed.',
    recoverable: true,
    detail: message,
  }
}

export const normalizeCommandError = (error: unknown): CommandError => {
  if (isCommandError(error)) return error

  return {
    code: 'UNKNOWN',
    message: errorText(error),
    recoverable: false,
  }
}

export type CommandErrorI18nKey = `errors.${ErrorCode}`

export const getCommandErrorI18nKey = (
  error: CommandError | ErrorCode,
): CommandErrorI18nKey =>
  `errors.${typeof error === 'string' ? error : error.code}`

export const commandErrorI18nKey = getCommandErrorI18nKey
