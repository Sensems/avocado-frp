<script setup lang="ts">
import { ref, computed } from 'vue'
import { GitBranch, Save, X } from 'lucide-vue-next'

const props = defineProps<{
    initialData?: any
    editMode?: boolean
    editIndex?: number
}>()

const form = ref({
    name: props.initialData?.name || '',
    type: props.initialData?.type || 'tcp',
    localIp: props.initialData?.localIp || '127.0.0.1',
    localPort: props.initialData?.localPort || '',
    remotePort: props.initialData?.remotePort || '',
    customDomains: props.initialData?.customDomains || ''
})

const emit = defineEmits(['save', 'cancel'])

const protocolTypes = ['tcp', 'udp', 'http', 'https', 'stcp', 'xtcp']

const isHttp = computed(() => ['http', 'https'].includes(form.value.type))

const handleSave = () => {
    if (!form.value.name || !form.value.localPort) return
    emit('save', {
        ...form.value,
        editMode: props.editMode,
        editIndex: props.editIndex
    })
}
</script>

<template>
    <n-card class="w-full max-w-lg rounded-2xl border transition-all duration-300 backdrop-blur-2xl" :class="[
        'bg-white/90 dark:bg-[#0F172A]/90',
        'border-slate-200/60 dark:border-white/[0.06]',
        'shadow-2xl dark:shadow-[0_25px_60px_rgba(0,0,0,0.5)]'
    ]" :bordered="false" size="huge" role="dialog" aria-modal="true">
        <!-- Top Accent Bar -->
        <template #header>
            <div class="flex items-center gap-3">
                <div
                    class="w-9 h-9 rounded-lg flex items-center justify-center bg-emerald-500/10 border border-emerald-500/20">
                    <GitBranch :size="17" class="text-emerald-400" />
                </div>
                <div>
                    <h3 class="text-base font-bold tracking-tight leading-tight">
                        {{ editMode ? $t('forms.editRuleTitle', '编辑规则') : $t('forms.ruleTitle') }}
                    </h3>
                    <p class="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
                        {{ editMode ? '修改已有的本地端口到公网服务器的映射。' : '创建新的本地端口到公网服务器的映射。' }}
                    </p>
                </div>
            </div>
        </template>

        <!-- Accent top line -->
        <div
            class="absolute top-0 left-6 right-6 h-[2px] bg-gradient-to-r from-transparent via-emerald-500/50 to-transparent rounded-full">
        </div>

        <n-form :model="form" size="large" label-placement="top">
            <n-form-item :label="$t('forms.ruleName')" path="name">
                <n-input v-model:value="form.name" :placeholder="$t('forms.ruleNamePlace')" />
            </n-form-item>

            <n-form-item :label="$t('forms.protocolType') + ' (Type)'" path="type">
                <n-select v-model:value="form.type"
                    :options="protocolTypes.map(t => ({ label: t.toUpperCase(), value: t }))" />
            </n-form-item>

            <div class="grid grid-cols-2 gap-5">
                <n-form-item :label="$t('forms.localIp')" path="localIp">
                    <n-input v-model:value="form.localIp" :placeholder="$t('forms.localIpPlace')" />
                </n-form-item>
                <n-form-item :label="$t('forms.localPort')" path="localPort">
                    <n-input v-model:value="form.localPort" placeholder="8080" />
                </n-form-item>
            </div>

            <n-form-item v-if="!isHttp" :label="$t('forms.remotePort') + ' (Remote Port)'" path="remotePort">
                <n-input v-model:value="form.remotePort" placeholder="6000" />
            </n-form-item>

            <n-form-item v-if="isHttp" :label="$t('forms.customDomains') + ' (Custom Domains)'" path="customDomains">
                <n-input v-model:value="form.customDomains" :placeholder="$t('forms.customDomainsPlace')" />
                <template #feedback>
                    多个域名使用逗号分隔
                </template>
            </n-form-item>
        </n-form>

        <div class="flex items-center gap-3 justify-end mt-5">
            <n-button @click="emit('cancel')" size="large"
                class="px-5 transition-all duration-200 active:scale-[0.97] cursor-pointer">
                <template #icon>
                    <X :size="15" />
                </template>
                {{ $t('forms.cancel') }}
            </n-button>
            <n-button type="primary" @click="handleSave" size="large"
                class="px-5 transition-all duration-200 active:scale-[0.97] cursor-pointer glow-green">
                <template #icon>
                    <Save :size="15" />
                </template>
                {{ $t('forms.save') }}
            </n-button>
        </div>
    </n-card>
</template>
