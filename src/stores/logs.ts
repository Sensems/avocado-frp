import { computed, reactive, ref } from 'vue'
import { defineStore } from 'pinia'

import type { LogEntry as DomainLogEntry, ProcessKind } from '@/domain/process'
import {
  tauriClient,
  type TauriUnlistenFn,
} from '@/services/tauriClient'

export type LogLevel = 'out' | 'err'

/** UI-facing log row derived from `log://entry`. */
export interface UiLogEntry {
  type: LogLevel
  source: ProcessKind
  text: string
  time: string
  timestamp: string
}

export interface LogFilters {
  source: 'all' | ProcessKind
  level: 'all' | LogLevel
  query: string
}

const MAX_ENTRIES_PER_PROCESS = 1000

const displayTime = (timestamp: string): string => {
  const parsed = new Date(timestamp)
  return Number.isNaN(parsed.getTime())
    ? timestamp
    : parsed.toLocaleTimeString()
}

const toUiEntry = (entry: DomainLogEntry): UiLogEntry => ({
  type: entry.stream === 'stderr' ? 'err' : 'out',
  source: entry.kind,
  text: entry.text,
  time: displayTime(entry.timestamp),
  timestamp: entry.timestamp,
})

export const useLogsStore = defineStore('logs', () => {
  const entries = ref<UiLogEntry[]>([])
  const filters = reactive<LogFilters>({
    source: 'all',
    level: 'all',
    query: '',
  })
  const pauseScroll = ref(false)

  let initialization: Promise<void> | undefined
  let unlistenLogEntry: TauriUnlistenFn | undefined

  const filteredEntries = computed(() => {
    const query = filters.query.trim().toLowerCase()
    return entries.value.filter((entry) => {
      if (filters.source !== 'all' && entry.source !== filters.source) {
        return false
      }
      if (filters.level !== 'all' && entry.type !== filters.level) {
        return false
      }
      if (query && !entry.text.toLowerCase().includes(query)) {
        return false
      }
      return true
    })
  })

  const trimBuffer = (source: ProcessKind) => {
    let count = 0
    for (let i = entries.value.length - 1; i >= 0; i -= 1) {
      if (entries.value[i]?.source !== source) continue
      count += 1
      if (count > MAX_ENTRIES_PER_PROCESS) {
        entries.value.splice(i, 1)
      }
    }
  }

  const pushEntry = (entry: DomainLogEntry) => {
    const uiEntry = toUiEntry(entry)
    entries.value.push(uiEntry)
    trimBuffer(uiEntry.source)
  }

  const initialize = async () => {
    if (!unlistenLogEntry) {
      unlistenLogEntry = await tauriClient.onLogEntry(pushEntry)
    }
  }

  const init = (): Promise<void> => {
    if (!initialization) {
      initialization = initialize().catch((error) => {
        initialization = undefined
        throw error
      })
    }
    return initialization
  }

  const clearUiBuffer = () => {
    entries.value = []
  }

  const setFilters = (next: Partial<LogFilters>) => {
    if (next.source !== undefined) filters.source = next.source
    if (next.level !== undefined) filters.level = next.level
    if (next.query !== undefined) filters.query = next.query
  }

  const cleanup = () => {
    unlistenLogEntry?.()
    unlistenLogEntry = undefined
    initialization = undefined
  }

  return {
    entries,
    filters,
    filteredEntries,
    pauseScroll,
    init,
    clearUiBuffer,
    setFilters,
    cleanup,
  }
})
