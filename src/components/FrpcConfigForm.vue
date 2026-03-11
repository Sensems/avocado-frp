<script setup lang="ts">
import { ref, watch } from 'vue'
import { Save } from 'lucide-vue-next'

const props = defineProps<{
    initialData?: any
}>()

const form = ref({
    serverAddr: props.initialData?.serverAddr || '',
    serverPort: props.initialData?.serverPort || '',
    authMethod: props.initialData?.auth?.method || null,
    token: props.initialData?.auth?.token || '',
})

watch(() => props.initialData, (newVal) => {
    if (newVal) {
        form.value.serverAddr = newVal.serverAddr || ''
        form.value.serverPort = newVal.serverPort || ''
        form.value.authMethod = newVal.auth?.method || null
        form.value.token = newVal.auth?.token || ''
    }
}, { deep: true, immediate: true })

const emit = defineEmits(['save'])

const handleSave = () => {
    emit('save', { ...form.value })
}
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

        <div class="flex justify-end mt-5">
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

<style scoped>
.glow-blue {
    box-shadow: 0 4px 14px 0 rgba(14, 165, 233, 0.39);
}
</style>
