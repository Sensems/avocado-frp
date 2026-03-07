<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Trash2 } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface LogEntry {
    type: 'out' | 'err'
    source: 'frpc' | 'frps'
    text: string
    time: string
}

const logs = ref<LogEntry[]>([])
const containerRef = ref<HTMLElement | null>(null)

const unlisteners: UnlistenFn[] = []

const scrollToBottom = () => {
    nextTick(() => {
        if (containerRef.value) {
            containerRef.value.scrollTop = containerRef.value.scrollHeight
        }
    })
}

const addLog = (type: 'out' | 'err', source: 'frpc' | 'frps', text: string) => {
    logs.value.push({
        type,
        source,
        text,
        time: new Date().toLocaleTimeString()
    })
    if (logs.value.length > 500) {
        logs.value.shift()
    }
    scrollToBottom()
}

onMounted(async () => {
    // frpc 事件
    unlisteners.push(
        await listen<string>('frpc-stdout', (event) => {
            addLog('out', 'frpc', event.payload)
        }),
        await listen<string>('frpc-stderr', (event) => {
            addLog('err', 'frpc', event.payload)
        }),
        await listen<number>('frpc-terminated', (event) => {
            addLog('err', 'frpc', `进程已退出，退出码: ${event.payload}`)
        }),
        // frps 事件
        await listen<string>('frps-stdout', (event) => {
            addLog('out', 'frps', event.payload)
        }),
        await listen<string>('frps-stderr', (event) => {
            addLog('err', 'frps', event.payload)
        }),
        await listen<number>('frps-terminated', (event) => {
            addLog('err', 'frps', `进程已退出，退出码: ${event.payload}`)
        }),
    )
})

onUnmounted(() => {
    unlisteners.forEach((fn) => fn())
})

const clearLogs = () => {
    logs.value = []
}

/** 根据日志内容返回左侧竖条颜色类名 */
const getLogBorderClass = (log: LogEntry) => {
    if (log.type === 'err' || log.text.toLowerCase().includes('error')) return 'border-l-red-500/70'
    if (log.text.toLowerCase().includes('warn')) return 'border-l-amber-500/70'
    return 'border-l-transparent'
}

const getLogTextClass = (log: LogEntry) => {
    if (log.type === 'err' || log.text.toLowerCase().includes('error')) return 'text-red-400'
    if (log.text.toLowerCase().includes('warn')) return 'text-amber-400'
    return 'text-slate-300'
}

const getSourceBadgeClass = (source: string) => {
    return source === 'frpc'
        ? 'text-emerald-400 bg-emerald-500/10'
        : 'text-violet-400 bg-violet-500/10'
}
</script>

<template>
    <div
        class="flex flex-col h-full rounded-xl overflow-hidden border transition-all duration-300 bg-[#020617] dark:bg-[#020617] border-white/[0.06] dark:border-white/[0.06]">
        <!-- Terminal Title Bar -->
        <div class="h-9 border-b border-white/[0.06] bg-[#0F172A] flex items-center justify-between px-4">
            <div class="flex items-center gap-2.5">
                <!-- Traffic light dots -->
                <div class="flex items-center gap-1.5">
                    <div class="w-2.5 h-2.5 rounded-full bg-red-500/70 transition-colors hover:bg-red-500"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-yellow-500/70 transition-colors hover:bg-yellow-500"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-green-500/70 transition-colors hover:bg-green-500"></div>
                </div>
                <span class="text-[11px] font-medium text-slate-500 tracking-wider uppercase ml-1">system.log</span>
                <!-- Log count badge -->
                <span v-if="logs.length > 0"
                    class="text-[10px] font-semibold bg-white/[0.06] text-slate-400 px-1.5 py-0.5 rounded-md">
                    {{ logs.length }}
                </span>
            </div>
            <button @click="clearLogs"
                class="flex items-center gap-1.5 text-[11px] font-medium text-slate-500 hover:text-slate-300 transition-colors bg-white/[0.04] hover:bg-white/[0.08] px-2.5 py-1 rounded-md cursor-pointer">
                <Trash2 :size="11" />
                {{ t('forms.cancel') === '取消' ? '清除' : 'Clear' }}
            </button>
        </div>

        <!-- Log Content -->
        <div ref="containerRef" class="flex-1 overflow-auto p-3 font-mono text-[11px] leading-[1.7] custom-scrollbar">
            <!-- Empty State -->
            <div v-if="logs.length === 0" class="flex flex-col items-center justify-center h-full gap-2 select-none">
                <svg xmlns="http://www.w3.org/2000/svg" class="w-8 h-8 text-slate-700" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="4 17 10 11 4 5" />
                    <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
                <span class="text-slate-600 text-xs">{{ t('forms.cancel') === '取消' ? '等待输出...' : 'Waiting for output...'
                    }}</span>
            </div>

            <!-- Log Lines -->
            <div v-for="(log, idx) in logs" :key="idx"
                :class="['py-[2px] break-all border-l-2 pl-3 transition-colors flex items-start gap-1.5', getLogBorderClass(log), getLogTextClass(log)]">
                <span class="text-slate-600 select-none mr-1 text-[10px] shrink-0">{{ String(idx + 1).padStart(3, ' ')
                    }}</span>
                <span class="text-slate-700 select-none mr-1 shrink-0">[{{ log.time }}]</span>
                <span
                    :class="['text-[9px] font-semibold px-1 py-[1px] rounded shrink-0', getSourceBadgeClass(log.source)]">{{
                    log.source }}</span>
                <span>{{ log.text }}</span>
            </div>
        </div>
    </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
    width: 5px;
    height: 5px;
}

.custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.06);
    border-radius: 3px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.12);
}
</style>
