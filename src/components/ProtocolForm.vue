<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { GitBranch, Save, X } from 'lucide-vue-next'
import {
  PROTOCOL_TYPES,
  toProxyRuleForm,
  validateProxyRuleForm,
  type ProxyRuleErrors,
  type ProxyRuleField,
  type ProxyRuleSavePayload,
  type ProxyRuleSource,
} from '@/domain/proxyRule'

const props = defineProps<{
  initialData?: ProxyRuleSource
  editMode?: boolean
  editIndex?: number
}>()

const emit = defineEmits<{
  (event: 'save', payload: ProxyRuleSavePayload): void
  (event: 'cancel'): void
}>()

const { t } = useI18n()
const form = ref(toProxyRuleForm(props.initialData))
const errors = ref<ProxyRuleErrors>({})
const protocolTypes = PROTOCOL_TYPES
const isHttp = computed(
  () => form.value.type === 'http' || form.value.type === 'https',
)

watch(
  () => props.initialData,
  (value) => {
    form.value = toProxyRuleForm(value)
    errors.value = {}
  },
  { deep: true },
)

const validationStatus = (field: ProxyRuleField) =>
  errors.value[field] ? 'error' : undefined

const validationFeedback = (field: ProxyRuleField) => {
  const code = errors.value[field]
  return code ? t(`forms.validation.${code}`) : undefined
}

const handleSave = () => {
  errors.value = validateProxyRuleForm(form.value)
  if (Object.keys(errors.value).length > 0) return

  emit('save', {
    ...form.value,
    editMode: props.editMode,
    editIndex: props.editIndex,
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
                        {{ editMode ? $t('forms.editRuleTitle') : $t('forms.ruleTitle') }}
                    </h3>
                    <p class="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
                        {{ editMode ? $t('forms.ruleEditDesc') : $t('forms.ruleCreateDesc') }}
                    </p>
                </div>
            </div>
        </template>

        <!-- Accent top line -->
        <div
            class="absolute top-0 left-6 right-6 h-[2px] bg-gradient-to-r from-transparent via-emerald-500/50 to-transparent rounded-full">
        </div>

        <n-form :model="form" size="large" label-placement="top">
            <n-form-item
                :label="$t('forms.ruleName')"
                path="name"
                :validation-status="validationStatus('name')"
                :feedback="validationFeedback('name')"
            >
                <n-input v-model:value="form.name" :placeholder="$t('forms.ruleNamePlace')" />
            </n-form-item>

            <n-form-item :label="$t('forms.protocolType') + ' (Type)'" path="type">
                <n-select v-model:value="form.type"
                    :options="protocolTypes.map(t => ({ label: t.toUpperCase(), value: t }))" />
            </n-form-item>

            <div class="grid grid-cols-2 gap-5">
                <n-form-item
                    :label="$t('forms.localIp')"
                    path="localIp"
                    :validation-status="validationStatus('localIp')"
                    :feedback="validationFeedback('localIp')"
                >
                    <n-input v-model:value="form.localIp" :placeholder="$t('forms.localIpPlace')" />
                </n-form-item>
                <n-form-item
                    :label="$t('forms.localPort')"
                    path="localPort"
                    :validation-status="validationStatus('localPort')"
                    :feedback="validationFeedback('localPort')"
                >
                    <n-input v-model:value="form.localPort" placeholder="8080" />
                </n-form-item>
            </div>

            <n-form-item
                v-if="!isHttp"
                :label="$t('forms.remotePort') + ' (Remote Port)'"
                path="remotePort"
                :validation-status="validationStatus('remotePort')"
                :feedback="validationFeedback('remotePort')"
            >
                <n-input v-model:value="form.remotePort" placeholder="6000" />
            </n-form-item>

            <n-form-item
                v-if="isHttp"
                :label="$t('forms.customDomains') + ' (Custom Domains)'"
                path="customDomains"
                :validation-status="validationStatus('customDomains')"
                :feedback="validationFeedback('customDomains') || $t('forms.customDomainsHint')"
            >
                <n-input v-model:value="form.customDomains" :placeholder="$t('forms.customDomainsPlace')" />
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
                class="px-5 transition-all duration-200 active:scale-[0.97] cursor-pointer">
                <template #icon>
                    <Save :size="15" />
                </template>
                {{ $t('forms.save') }}
            </n-button>
        </div>
    </n-card>
</template>
