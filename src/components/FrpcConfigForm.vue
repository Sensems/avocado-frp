<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Save } from 'lucide-vue-next'
import type {
    FrpcGlobalFormData,
    FrpcKnownConfig,
} from '@/domain/config'

const props = withDefaults(
    defineProps<{
        initialData?: FrpcKnownConfig
        /** When true, omit the inline Save button (parent OpsBar handles save). */
        hideSave?: boolean
    }>(),
    {
        hideSave: false,
    },
)

interface FrpcFormState {
    serverAddr: string
    serverPort: number | null
    authMethod: string | null
    token: string
}

const toFrpcForm = (value?: FrpcKnownConfig): FrpcFormState => ({
    serverAddr: value?.serverAddr ?? '',
    serverPort: value?.serverPort ?? null,
    authMethod: value?.auth.method ?? null,
    token: value?.auth.token ?? '',
})

	const form = ref<FrpcFormState>(toFrpcForm(props.initialData))

	const emit = defineEmits<{
	    save: [value: FrpcGlobalFormData]
	    'update:dirty': [dirty: boolean]
	}>()

	const isDirty = computed(() => {
	    const baseline = toFrpcForm(props.initialData)
	    const current = form.value
	    return (
	        baseline.serverAddr !== current.serverAddr ||
	        baseline.serverPort !== current.serverPort ||
	        baseline.authMethod !== current.authMethod ||
	        baseline.token !== current.token
	    )
	})

	// Watch only global baseline fields — proxy list churn must not reset in-progress edits.
	watch(
	    () =>
	        [
	            props.initialData?.serverAddr ?? '',
	            props.initialData?.serverPort ?? null,
	            props.initialData?.auth.method ?? null,
	            props.initialData?.auth.token ?? '',
	        ] as const,
	    (_next, prev) => {
	        // On subsequent updates, keep local dirty edits intact.
	        if (prev !== undefined && isDirty.value) return
	        form.value = toFrpcForm(props.initialData)
	    },
	    { immediate: true },
	)

	watch(
	    isDirty,
	    (dirty) => {
	        emit('update:dirty', dirty)
	    },
	    { immediate: true },
	)

const handleSave = () => {
    emit('save', { ...form.value })
}

defineExpose({
    getFormData: (): FrpcGlobalFormData => ({ ...form.value }),
    isDirty: () => isDirty.value,
})
</script>

<template>
    <div>
        <n-form :model="form" size="large" label-placement="top">
            <div class="grid grid-cols-2 gap-5">
                <n-form-item :label="$t('forms.serverAddr', '服务器 IP/域名')" path="serverAddr">
                    <n-input v-model:value="form.serverAddr" placeholder="127.0.0.1" />
                </n-form-item>
                <n-form-item :label="$t('forms.serverPort', '服务器端口')" path="serverPort">
                    <n-input-number v-model:value="form.serverPort" :show-button="false" placeholder="7000"
                        class="w-full" />
                </n-form-item>
            </div>

            <div class="grid grid-cols-2 gap-5">
                <n-form-item :label="$t('forms.authMethod', '认证方式')" path="authMethod">
                    <n-select v-model:value="form.authMethod" placeholder="请选择认证方式"
                        :options="[{ label: 'Token', value: 'token' }]" />
                </n-form-item>
                <n-form-item :label="$t('forms.authToken', '认证 Token')" path="token">
                    <n-input v-model:value="form.token" type="password" show-password-on="click"
                        :placeholder="$t('forms.authTokenPlace', '请输入 Token')" />
                </n-form-item>
            </div>
        </n-form>

        <div v-if="!hideSave" class="flex justify-end mt-5">
            <n-button type="primary" size="large" @click="handleSave"
                class="px-6 transition-all duration-200 active:scale-[0.97] cursor-pointer">
                <template #icon>
                    <Save :size="15" />
                </template>
                {{ $t('forms.save', '保存设置') }}
            </n-button>
        </div>
    </div>
</template>
