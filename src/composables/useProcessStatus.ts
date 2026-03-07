import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** 全局单例状态 — 供多组件共享 */
const frpcRunning = ref(false)
const frpsRunning = ref(false)
const frpcLoading = ref(false)
const frpsLoading = ref(false)

/** 已初始化标记，确保事件监听只注册一次 */
let initialized = false
const unlisteners: UnlistenFn[] = []

/**
 * 进程状态管理 composable
 * - 封装 start/stop invoke 调用
 * - 通过 Tauri 事件 (terminated) 自动同步运行状态
 * - 返回 loading/running 状态供 UI 绑定
 */
export const useProcessStatus = () => {
  /**
   * 初始化事件监听 + 从后端查询一次当前进程状态
   * 仅在首次调用时执行
   */
  const init = async () => {
    if (initialized) return
    initialized = true

    try {
      frpcRunning.value = await invoke<boolean>('get_frpc_status')
    } catch {
      frpcRunning.value = false
    }
    try {
      frpsRunning.value = await invoke<boolean>('get_frps_status')
    } catch {
      frpsRunning.value = false
    }

    // 监听 terminated 事件自动重置状态
    unlisteners.push(
      await listen('frpc-terminated', () => {
        frpcRunning.value = false
        frpcLoading.value = false
      }),
      await listen('frps-terminated', () => {
        frpsRunning.value = false
        frpsLoading.value = false
      }),
    )
  }

  /** 启动 frpc */
  const startFrpc = async (): Promise<{ ok: boolean; msg: string }> => {
    frpcLoading.value = true
    try {
      const msg = await invoke<string>('start_frpc')
      frpcRunning.value = true
      return { ok: true, msg }
    } catch (e) {
      return { ok: false, msg: String(e) }
    } finally {
      frpcLoading.value = false
    }
  }

  /** 停止 frpc */
  const stopFrpc = async (): Promise<{ ok: boolean; msg: string }> => {
    frpcLoading.value = true
    try {
      const msg = await invoke<string>('stop_frpc')
      frpcRunning.value = false
      return { ok: true, msg }
    } catch (e) {
      return { ok: false, msg: String(e) }
    } finally {
      frpcLoading.value = false
    }
  }

  /** 启动 frps */
  const startFrps = async (): Promise<{ ok: boolean; msg: string }> => {
    frpsLoading.value = true
    try {
      const msg = await invoke<string>('start_frps')
      frpsRunning.value = true
      return { ok: true, msg }
    } catch (e) {
      return { ok: false, msg: String(e) }
    } finally {
      frpsLoading.value = false
    }
  }

  /** 停止 frps */
  const stopFrps = async (): Promise<{ ok: boolean; msg: string }> => {
    frpsLoading.value = true
    try {
      const msg = await invoke<string>('stop_frps')
      frpsRunning.value = false
      return { ok: true, msg }
    } catch (e) {
      return { ok: false, msg: String(e) }
    } finally {
      frpsLoading.value = false
    }
  }

  return {
    frpcRunning,
    frpsRunning,
    frpcLoading,
    frpsLoading,
    init,
    startFrpc,
    stopFrpc,
    startFrps,
    stopFrps,
  }
}
