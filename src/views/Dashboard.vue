<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { parse, stringify } from 'smol-toml'
import ProtocolForm from '@/components/ProtocolForm.vue'
import FrpsConfigForm from '@/components/FrpsConfigForm.vue'
import ConsoleLogger from '@/components/ConsoleLogger.vue'
import TrafficChart from '@/components/TrafficChart.vue'
import HelpGuide from '@/components/HelpGuide.vue'
import { useProcessStatus } from '@/composables/useProcessStatus'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'
import { open } from '@tauri-apps/plugin-dialog'
import {
    Play,
    Square,
    Plus,
    Server,
    Monitor,
    Settings,
    Activity,
    FileText,
    Terminal,
    Upload,
    Power,
    BookOpen,
    Download,
    Edit2,
    Trash2
} from 'lucide-vue-next'

const { t } = useI18n()
const message = useMessage()

const activeTab = ref('frpc')
const showAddForm = ref(false)
const editMode = ref(false)
const editIndex = ref(-1)
const editingData = ref<any>(null)

const frpcConfigContent = ref('')
const parsedFrpcConfig = ref<any>(null)
const frpsConfigContent = ref('')
const isAutostartEnabled = ref(false)

const {
    frpcRunning,
    frpsRunning,
    frpcLoading,
    frpsLoading,
    init: initProcessStatus,
    startFrpc,
    stopFrpc,
    startFrps,
    stopFrps,
} = useProcessStatus()

const loadFrpcConfig = async () => {
    try {
        const content: string = await invoke('read_frpc_config')
        frpcConfigContent.value = content

        try {
            parsedFrpcConfig.value = parse(content)
        } catch (e) {
            console.error('Failed to parse frpc config:', e)
            parsedFrpcConfig.value = null
        }
    } catch (error) {
        console.error('Failed to load frpc config:', error)
    }
}

const loadFrpsConfig = async () => {
    try {
        const content: string = await invoke('read_frps_config')
        frpsConfigContent.value = content
    } catch (error) {
        console.error('Failed to load frps config:', error)
    }
}

const checkAutostart = async () => {
    try {
        isAutostartEnabled.value = await isEnabled()
    } catch (error) {
        console.error('Failed to check autostart:', error)
    }
}

const toggleAutostart = async (val: boolean) => {
    try {
        if (val) {
            await enable()
        } else {
            await disable()
        }
        await checkAutostart()
    } catch (error) {
        console.error('Failed to toggle autostart:', error)
    }
}

/** 启动 frpc — 带消息反馈 */
const handleStartFrpc = async () => {
    message.loading(t('feedback.starting', { name: 'frpc' }), { duration: 1500 })
    const { ok, msg } = await startFrpc()
    if (ok) {
        message.success(t('feedback.startSuccess', { name: 'frpc' }))
    } else {
        message.error(t('feedback.startFail', { name: 'frpc', error: msg }))
    }
}

/** 停止 frpc — 带消息反馈 */
const handleStopFrpc = async () => {
    message.loading(t('feedback.stopping', { name: 'frpc' }), { duration: 1500 })
    const { ok, msg } = await stopFrpc()
    if (ok) {
        message.success(t('feedback.stopSuccess', { name: 'frpc' }))
    } else {
        message.error(t('feedback.stopFail', { name: 'frpc', error: msg }))
    }
}

/** 启动 frps — 带消息反馈 */
const handleStartFrps = async () => {
    message.loading(t('feedback.starting', { name: 'frps' }), { duration: 1500 })
    const { ok, msg } = await startFrps()
    if (ok) {
        message.success(t('feedback.startSuccess', { name: 'frps' }))
    } else {
        message.error(t('feedback.startFail', { name: 'frps', error: msg }))
    }
}

/** 停止 frps — 带消息反馈 */
const handleStopFrps = async () => {
    message.loading(t('feedback.stopping', { name: 'frps' }), { duration: 1500 })
    const { ok, msg } = await stopFrps()
    if (ok) {
        message.success(t('feedback.stopSuccess', { name: 'frps' }))
    } else {
        message.error(t('feedback.stopFail', { name: 'frps', error: msg }))
    }
}

const handleExportDeploy = async () => {
    try {
        const selectedDir = await open({
            directory: true,
            multiple: false,
            title: '选择导出部署脚本存储目录'
        })
        if (selectedDir) {
            await invoke('export_deploy_script', { path: selectedDir, tomlContent: frpsConfigContent.value })
            message.success(t('feedback.exportSuccess'))
        }
    } catch (error) {
        console.error('Failed to export:', error)
        message.error(t('feedback.exportFail', { error: String(error) }))
    }
}

/** 导出日志 */
const handleExportLogs = async () => {
    try {
        const selectedDir = await open({
            directory: true,
            multiple: false,
            title: '选择日志导出目录'
        })
        if (selectedDir) {
            const result = await invoke<string>('export_logs', { path: selectedDir })
            message.success(result)
        }
    } catch (error) {
        message.error(String(error))
    }
}

const handleAddRule = () => {
    editMode.value = false
    editIndex.value = -1
    editingData.value = null
    showAddForm.value = true
}

const handleEditRule = (proxy: any, index: number) => {
    editMode.value = true
    editIndex.value = index
    editingData.value = proxy
    showAddForm.value = true
}

const handleDeleteRule = async (index: number) => {
    try {
        let configObj: any = parsedFrpcConfig.value
        if (!configObj || !configObj.proxies || configObj.proxies.length <= index) return

        configObj.proxies.splice(index, 1)

        const updatedConfigStr = stringify(configObj)
        await invoke('save_frpc_config', { configContent: updatedConfigStr })
        await loadFrpcConfig()
        message.success(t('feedback.exportSuccess', '操作成功'))
    } catch (e) {
        console.error('删除失败:', e)
        message.error(String(e))
    }
}

const handleSaveRule = async (payload: any) => {
    try {
        let configObj: any = parsedFrpcConfig.value || { proxies: [] }
        if (!configObj.proxies) {
            configObj.proxies = []
        }

        const newRule: any = {
            name: payload.name,
            type: payload.type,
            localIp: payload.localIp || '127.0.0.1',
            localPort: Number(payload.localPort)
        }

        if (['http', 'https'].includes(String(payload.type))) {
            newRule.customDomains = payload.customDomains?.split(',').map((d: string) => d.trim()) || []
        } else {
            newRule.remotePort = Number(payload.remotePort)
        }

        if (payload.editMode && typeof payload.editIndex === 'number' && payload.editIndex >= 0) {
            configObj.proxies[payload.editIndex] = newRule
        } else {
            configObj.proxies.push(newRule)
        }

        const updatedConfigStr = stringify(configObj)
        await invoke('save_frpc_config', { configContent: updatedConfigStr })
        await loadFrpcConfig()
        showAddForm.value = false
        message.success(t('feedback.exportSuccess', '保存成功'))
    } catch (e) {
        console.error('保存失败:', e)
        message.error(String(e))
    }
}

const handleSaveFrpsForm = async (configData: Record<string, string | number>) => {
    const tomlContent = `
bindPort = ${configData.bindPort}
auth.method = "token"
auth.token = "${configData.token}"
vhostHTTPPort = ${configData.vhostHttpPort || 80}
vhostHTTPSPort = ${configData.vhostHttpsPort || 443}

webServer.port = ${configData.dashboardPort}
webServer.user = "${configData.dashboardUser}"
webServer.password = "${configData.dashboardPwd}"
`
    try {
        await invoke('save_frps_config', { configContent: tomlContent })
        await loadFrpsConfig()
        message.success(t('feedback.exportSuccess'))
    } catch (e) {
        console.error('保存Frps失败:', e)
        message.error(String(e))
    }
}

onMounted(() => {
    loadFrpcConfig()
    loadFrpsConfig()
    checkAutostart()
    initProcessStatus()
})
</script>

<template>
    <div class="px-5 py-6 md:px-6 flex flex-col gap-5 relative z-10 w-full max-w-7xl mx-auto">

        <!-- Tab Navigation -->
        <n-tabs v-model:value="activeTab" type="segment" animated class="transition-all duration-300 rounded-xl"
            pane-style="padding-top: 1.25rem;">

            <!-- Tab: Client (frpc) -->
            <n-tab-pane name="frpc">
                <template #tab>
                    <div class="flex items-center gap-2 py-0.5">
                        <Monitor :size="15" :stroke-width="2" />
                        <span class="text-[13px] font-semibold">{{ $t('dashboard.client') }}</span>
                    </div>
                </template>

                <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
                    <!-- Left Column -->
                    <div class="col-span-1 lg:col-span-2 flex flex-col gap-5">
                        <!-- Connection Profile -->
                        <n-card class="glass-card" size="medium">
                            <template #header>
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-sky-500/10 border border-sky-500/20">
                                        <FileText :size="16" class="text-sky-400" />
                                    </div>
                                    <span class="text-sm font-semibold">{{ $t('dashboard.connectionStatus') }}</span>
                                </div>
                            </template>
                            <div class="h-[236px]">
                                <n-scrollbar>
                                    <div v-if="parsedFrpcConfig?.proxies?.length"
                                        class="flex flex-col gap-2 p-1.5 pr-3">
                                        <div v-for="(proxy, idx) in parsedFrpcConfig.proxies" :key="idx"
                                            class="flex items-center justify-between p-3 rounded-xl border border-slate-200/60 dark:border-white/6 bg-slate-50/50 dark:bg-slate-900/30 hover:bg-slate-100 dark:hover:bg-slate-800/60 transition-colors group">
                                            <div class="flex items-center gap-4">
                                                <div class="flex flex-col min-w-[70px]">
                                                    <span class="text-sm font-semibold truncate max-w-[150px]">{{
                                                        proxy.name }}</span>
                                                    <span
                                                        class="text-[11px] text-slate-500 font-bold tracking-wider mt-0.5 uppercase">{{
                                                            proxy.type }}</span>
                                                </div>
                                                <div class="h-8 w-px bg-slate-200 dark:bg-white/10 shrink-0"></div>
                                                <div
                                                    class="flex flex-col text-[12px] font-mono leading-relaxed text-slate-600 dark:text-slate-400">
                                                    <div>
                                                        <span class="text-slate-400/80 mr-1">Local:</span>
                                                        <span class="text-slate-700 dark:text-slate-200">{{
                                                            proxy.localIP ||
                                                            proxy.localIp }}:{{ proxy.localPort }}</span>
                                                    </div>
                                                    <div>
                                                        <span class="text-slate-400/80 mr-1"
                                                            v-if="['http', 'https'].includes(proxy.type)">Domain:</span>
                                                        <span class="text-slate-400/80 mr-1" v-else>Remote:</span>
                                                        <span class="text-slate-700 dark:text-slate-200">{{ ['http',
                                                            'https'].includes(proxy.type) ?
                                                            (proxy.customDomains?.join(', ') || '-') :
                                                            proxy.remotePort }}</span>
                                                    </div>
                                                </div>
                                            </div>
                                            <div
                                                class="flex items-center gap-1 opacity-100 lg:opacity-0 group-hover:opacity-100 transition-opacity">
                                                <n-button circle size="small" tertiary type="info"
                                                    @click="handleEditRule(proxy, Number(idx))" class="cursor-pointer">
                                                    <template #icon>
                                                        <Edit2 :size="14" />
                                                    </template>
                                                </n-button>
                                                <n-popconfirm @positive-click="handleDeleteRule(Number(idx))"
                                                    placement="top">
                                                    <template #trigger>
                                                        <n-button circle size="small" tertiary type="error"
                                                            class="cursor-pointer">
                                                            <template #icon>
                                                                <Trash2 :size="14" />
                                                            </template>
                                                        </n-button>
                                                    </template>
                                                    确定删除该规则吗？
                                                </n-popconfirm>
                                            </div>
                                        </div>
                                    </div>
                                    <div v-else
                                        class="h-full flex items-center justify-center text-slate-400 text-sm py-16">
                                        {{ $t('dashboard.ruleEmpty', '暂无配置规则') }}
                                    </div>
                                </n-scrollbar>
                            </div>
                        </n-card>

                        <!-- Traffic Chart -->
                        <n-card class="glass-card h-[320px]" size="medium">
                            <template #header>
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-emerald-500/10 border border-emerald-500/20">
                                        <Activity :size="16" class="text-emerald-400" />
                                    </div>
                                    <span class="text-sm font-semibold">{{ $t('dashboard.realtimeTraffic') }}</span>
                                </div>
                            </template>
                            <div class="w-full h-full relative -mt-3">
                                <TrafficChart />
                            </div>
                        </n-card>

                        <!-- Console Logs -->
                        <n-card class="glass-card" size="medium">
                            <template #header>
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-amber-500/10 border border-amber-500/20">
                                        <Terminal :size="16" class="text-amber-400" />
                                    </div>
                                    <span class="text-sm font-semibold">{{ $t('dashboard.logs') }}</span>
                                </div>
                            </template>
                            <div class="h-[220px]">
                                <ConsoleLogger class="w-full h-full rounded-xl" />
                            </div>
                        </n-card>
                    </div>

                    <!-- Right Column: Actions -->
                    <div class="col-span-1">
                        <div class="glass-card p-5 sticky top-4 flex flex-col gap-4">
                            <div class="flex items-center gap-2.5 mb-1">
                                <div
                                    class="w-8 h-8 rounded-lg flex items-center justify-center bg-emerald-500/10 border border-emerald-500/20">
                                    <Monitor :size="16" class="text-emerald-400" />
                                </div>
                                <span class="text-sm font-semibold">{{ $t('dashboard.clientActions') }}</span>
                            </div>

                            <!-- 运行状态标签 -->
                            <div class="flex items-center gap-2 px-3 py-2 rounded-lg transition-colors" :class="frpcRunning
                                ? 'bg-emerald-500/10 border border-emerald-500/20'
                                : 'bg-slate-500/5 border border-slate-500/10'
                                ">
                                <span class="w-2 h-2 rounded-full transition-all duration-500" :class="frpcRunning
                                    ? 'bg-emerald-500 dot-pulse shadow-[0_0_6px_rgba(34,197,94,0.5)]'
                                    : 'bg-slate-400 dark:bg-slate-600'
                                    " />
                                <span class="text-xs font-medium"
                                    :class="frpcRunning ? 'text-emerald-500' : 'text-slate-400 dark:text-slate-500'">
                                    {{ frpcRunning ? $t('status.running') : $t('status.stopped') }}
                                </span>
                            </div>

                            <n-button type="primary" block size="large" :disabled="frpcRunning || frpcLoading"
                                :loading="frpcLoading" @click="handleStartFrpc"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Play :size="16" />
                                </template>
                                {{ $t('dashboard.startFrpc') }}
                            </n-button>

                            <n-button type="error" ghost block size="large" :disabled="!frpcRunning || frpcLoading"
                                :loading="frpcLoading" @click="handleStopFrpc"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Square :size="14" />
                                </template>
                                {{ $t('dashboard.stopFrpc') }}
                            </n-button>

                            <div
                                class="w-full h-px bg-gradient-to-r from-transparent via-slate-500/20 to-transparent my-1">
                            </div>

                            <n-button block size="large" @click="handleAddRule"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Plus :size="16" />
                                </template>
                                {{ $t('dashboard.addRule') }}
                            </n-button>
                        </div>
                    </div>
                </div>
            </n-tab-pane>

            <!-- Tab: Server (frps) -->
            <n-tab-pane name="frps">
                <template #tab>
                    <div class="flex items-center gap-2 py-0.5">
                        <Server :size="15" :stroke-width="2" />
                        <span class="text-[13px] font-semibold">{{ $t('dashboard.server') }}</span>
                    </div>
                </template>

                <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
                    <div class="col-span-1 lg:col-span-2 flex flex-col gap-5">
                        <!-- Server Config Preview -->
                        <n-card class="glass-card" size="medium">
                            <template #header>
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-violet-500/10 border border-violet-500/20">
                                        <FileText :size="16" class="text-violet-400" />
                                    </div>
                                    <span class="text-sm font-semibold">{{ $t('dashboard.serverConfigPreview') }}</span>
                                </div>
                            </template>
                            <n-log :log="frpsConfigContent || $t('dashboard.frpsEmpty')" :rows="12"
                                class="config-log-viewer rounded-xl p-2 font-mono text-xs leading-relaxed transition-colors duration-300 bg-slate-50 dark:bg-slate-950 border border-slate-200/60 dark:border-white/[0.04]" />
                        </n-card>

                        <!-- Parameter Settings -->
                        <n-card class="glass-card" size="medium">
                            <template #header>
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-sky-500/10 border border-sky-500/20">
                                        <Settings :size="16" class="text-sky-400" />
                                    </div>
                                    <span class="text-sm font-semibold">{{ $t('dashboard.parameterSettings') }}</span>
                                </div>
                            </template>
                            <FrpsConfigForm @save="handleSaveFrpsForm" />
                        </n-card>
                    </div>

                    <!-- Server Actions -->
                    <div class="col-span-1">
                        <div class="glass-card p-5 sticky top-4 flex flex-col gap-4">
                            <div class="flex items-center gap-2.5 mb-1">
                                <div
                                    class="w-8 h-8 rounded-lg flex items-center justify-center bg-violet-500/10 border border-violet-500/20">
                                    <Server :size="16" class="text-violet-400" />
                                </div>
                                <span class="text-sm font-semibold">{{ $t('dashboard.serverActions') }}</span>
                            </div>

                            <!-- 运行状态标签 -->
                            <div class="flex items-center gap-2 px-3 py-2 rounded-lg transition-colors" :class="frpsRunning
                                ? 'bg-violet-500/10 border border-violet-500/20'
                                : 'bg-slate-500/5 border border-slate-500/10'
                                ">
                                <span class="w-2 h-2 rounded-full transition-all duration-500" :class="frpsRunning
                                    ? 'bg-violet-500 dot-pulse shadow-[0_0_6px_rgba(139,92,246,0.5)]'
                                    : 'bg-slate-400 dark:bg-slate-600'
                                    " />
                                <span class="text-xs font-medium"
                                    :class="frpsRunning ? 'text-violet-500' : 'text-slate-400 dark:text-slate-500'">
                                    {{ frpsRunning ? $t('status.running') : $t('status.stopped') }}
                                </span>
                            </div>

                            <n-button type="primary" block size="large" :disabled="frpsRunning || frpsLoading"
                                :loading="frpsLoading" @click="handleStartFrps"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Play :size="16" />
                                </template>
                                {{ $t('dashboard.startFrps') }}
                            </n-button>

                            <n-button type="error" ghost block size="large" :disabled="!frpsRunning || frpsLoading"
                                :loading="frpsLoading" @click="handleStopFrps"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Square :size="14" />
                                </template>
                                {{ $t('dashboard.stopFrps') }}
                            </n-button>

                            <div
                                class="w-full h-px bg-gradient-to-r from-transparent via-slate-500/20 to-transparent my-1">
                            </div>

                            <n-button type="info" secondary block size="large" @click="handleExportDeploy"
                                class="transition-all duration-200 active:scale-[0.97] cursor-pointer">
                                <template #icon>
                                    <Upload :size="16" />
                                </template>
                                {{ $t('dashboard.exportDeploy') }}
                            </n-button>
                        </div>
                    </div>
                </div>
            </n-tab-pane>

            <!-- Tab: Settings -->
            <n-tab-pane name="settings">
                <template #tab>
                    <div class="flex items-center gap-2 py-0.5">
                        <Settings :size="15" :stroke-width="2" />
                        <span class="text-[13px] font-semibold">{{ $t('dashboard.settings') }}</span>
                    </div>
                </template>

                <div class="max-w-3xl mx-auto">
                    <div class="glass-card p-6 flex flex-col gap-1">
                        <div class="flex items-center gap-2.5 mb-4">
                            <div
                                class="w-8 h-8 rounded-lg flex items-center justify-center bg-slate-500/10 border border-slate-500/20">
                                <Settings :size="16" class="text-slate-400" />
                            </div>
                            <span class="text-sm font-semibold">{{ $t('dashboard.generalSettings') }}</span>
                        </div>

                        <!-- Autostart Setting -->
                        <div
                            class="flex items-center justify-between px-4 py-4 rounded-xl transition-colors duration-200 hover:bg-white/[0.03] dark:hover:bg-white/[0.03] cursor-default group">
                            <div class="flex items-center gap-3.5">
                                <div
                                    class="w-9 h-9 rounded-lg flex items-center justify-center bg-emerald-500/10 border border-emerald-500/15 transition-colors group-hover:bg-emerald-500/15">
                                    <Power :size="16" class="text-emerald-400" />
                                </div>
                                <div>
                                    <p class="text-sm font-semibold leading-tight">{{ $t('dashboard.autostart') }}</p>
                                    <p class="text-xs text-slate-400 dark:text-slate-500 mt-1">{{
                                        $t('dashboard.autostartDesc') }}
                                    </p>
                                </div>
                            </div>
                            <n-switch :value="isAutostartEnabled" @update:value="toggleAutostart" size="large"
                                class="cursor-pointer" />
                        </div>

                        <div class="w-full h-px bg-gradient-to-r from-transparent via-slate-500/10 to-transparent">
                        </div>

                        <!-- Log Export -->
                        <div
                            class="flex items-center justify-between px-4 py-4 rounded-xl transition-colors duration-200 hover:bg-white/[0.03] dark:hover:bg-white/[0.03] cursor-default group">
                            <div class="flex items-center gap-3.5">
                                <div
                                    class="w-9 h-9 rounded-lg flex items-center justify-center bg-amber-500/10 border border-amber-500/15 transition-colors group-hover:bg-amber-500/15">
                                    <Download :size="16" class="text-amber-400" />
                                </div>
                                <div>
                                    <p class="text-sm font-semibold leading-tight">{{ $t('dashboard.logExport') }}</p>
                                    <p class="text-xs text-slate-400 dark:text-slate-500 mt-1">{{
                                        $t('dashboard.logExportDesc') }}
                                    </p>
                                </div>
                            </div>
                            <n-button size="small" @click="handleExportLogs" class="cursor-pointer">
                                {{ $t('dashboard.logExportBtn') }}
                            </n-button>
                        </div>
                    </div>
                </div>
            </n-tab-pane>

            <!-- Tab: Help Guide -->
            <n-tab-pane name="help">
                <template #tab>
                    <div class="flex items-center gap-2 py-0.5">
                        <BookOpen :size="15" :stroke-width="2" />
                        <span class="text-[13px] font-semibold">{{ $t('dashboard.help') }}</span>
                    </div>
                </template>

                <HelpGuide />
            </n-tab-pane>
        </n-tabs>

        <!-- Rule Form Modal -->
        <n-modal v-model:show="showAddForm" transform-origin="center">
            <ProtocolForm v-if="showAddForm" :edit-mode="editMode" :edit-index="editIndex" :initial-data="editingData"
                @save="handleSaveRule" @cancel="showAddForm = false" />
        </n-modal>
    </div>
</template>

<style scoped>
@reference "@/assets/main.css";

/* 修复 n-log 组件在亮色模式下文字颜色过浅 */
.config-log-viewer :deep(.n-log),
.config-log-viewer :deep(.n-code) {
    @apply text-slate-800 dark:text-slate-300 transition-colors duration-300;
}
</style>
