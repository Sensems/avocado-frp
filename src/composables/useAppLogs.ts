import { ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface LogEntry {
  type: 'out' | 'err'
  source: 'frpc' | 'frps'
  text: string
  time: string
}

// Global state
const logs = ref<LogEntry[]>([])
let isListening = false
const unlisteners: UnlistenFn[] = []

export const useAppLogs = () => {
  const addLog = (type: 'out' | 'err', source: 'frpc' | 'frps', text: string) => {
    logs.value.push({
      type,
      source,
      text,
      time: new Date().toLocaleTimeString()
    })
    // Keep max 500 lines
    if (logs.value.length > 500) {
      logs.value.shift()
    }
  }

  const clearLogs = () => {
    logs.value = []
  }

  const initListeners = async () => {
    if (isListening) return
    isListening = true

    // frpc events
    unlisteners.push(
      await listen<string>('frpc-stdout', (event) => {
        addLog('out', 'frpc', event.payload)
      }),
      await listen<string>('frpc-stderr', (event) => {
        addLog('err', 'frpc', event.payload)
      }),
      await listen<number>('frpc-terminated', (event) => {
        addLog('err', 'frpc', `进程已退出，退出码: ${event.payload}`)
      })
    )

    // frps events
    unlisteners.push(
      await listen<string>('frps-stdout', (event) => {
        addLog('out', 'frps', event.payload)
      }),
      await listen<string>('frps-stderr', (event) => {
        addLog('err', 'frps', event.payload)
      }),
      await listen<number>('frps-terminated', (event) => {
        addLog('err', 'frps', `进程已退出，退出码: ${event.payload}`)
      })
    )
  }

  const cleanupListeners = () => {
    unlisteners.forEach((fn) => fn())
    unlisteners.length = 0
    isListening = false
  }

  return {
    logs,
    clearLogs,
    initListeners,
    cleanupListeners
  }
}
