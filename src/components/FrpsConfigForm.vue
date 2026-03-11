<script setup lang="ts">
import { ref } from 'vue'
import { Save } from 'lucide-vue-next'

const props = defineProps<{
    initialData?: any
}>()

const form = ref({
    bindPort: props.initialData?.bindPort || '',
    vhostHttpPort: props.initialData?.vhostHttpPort || '',
    vhostHttpsPort: props.initialData?.vhostHttpsPort || '',
    authMethod: props.initialData?.authMethod || '',
    token: props.initialData?.token || '',
    dashboardPort: props.initialData?.dashboardPort || null,
    dashboardUser: props.initialData?.dashboardUser || '',
    dashboardPwd: props.initialData?.dashboardPwd || '',
})

const emit = defineEmits(['save'])

const handleSave = () => {
    emit('save', { ...form.value })
}
</script>

<template>
    <div>
        <n-form :model="form" size="large" label-placement="top">
            <div class="grid grid-cols-2 gap-5">
                <n-form-item :label="$t('forms.bindPort')" path="bindPort">
                    <n-input v-model:value="form.bindPort" placeholder="7000" />
                </n-form-item>
                <n-form-item :label="$t('forms.authToken')" path="token">
                    <n-input v-model:value="form.token" type="password" show-password-on="click"
                        :placeholder="$t('forms.authTokenPlace')" />
                </n-form-item>
            </div>

            <div class="grid grid-cols-2 gap-5">
                <n-form-item :label="$t('forms.vhostHttpPort')" path="vhostHttpPort">
                    <n-input v-model:value="form.vhostHttpPort" placeholder="80" />
                </n-form-item>
                <n-form-item :label="$t('forms.vhostHttpsPort')" path="vhostHttpsPort">
                    <n-input v-model:value="form.vhostHttpsPort" placeholder="443" />
                </n-form-item>
            </div>

            <!-- Dashboard Config Section -->
            <div class="flex items-center gap-3 my-5">
                <div class="flex-1 h-px bg-gradient-to-r from-transparent via-slate-500/15 to-transparent"></div>
                <span
                    class="text-xs font-semibold text-slate-400 dark:text-slate-500 tracking-wide uppercase">Dashboard</span>
                <div class="flex-1 h-px bg-gradient-to-r from-transparent via-slate-500/15 to-transparent"></div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
                <n-form-item :label="$t('forms.dashboardPort')" path="dashboardPort">
                    <n-input v-model:value="form.dashboardPort" placeholder="7500" />
                </n-form-item>
                <n-form-item :label="$t('forms.dashboardUser')" path="dashboardUser">
                    <n-input v-model:value="form.dashboardUser" />
                </n-form-item>
                <n-form-item :label="$t('forms.dashboardPwd')" path="dashboardPwd">
                    <n-input v-model:value="form.dashboardPwd" type="password" show-password-on="click" />
                </n-form-item>
            </div>
        </n-form>

        <div class="flex justify-end mt-5">
            <n-button type="primary" size="large" @click="handleSave"
                class="px-6 transition-all duration-200 active:scale-[0.97] cursor-pointer glow-green">
                <template #icon>
                    <Save :size="15" />
                </template>
                {{ $t('forms.save') }} frps.toml
            </n-button>
        </div>
    </div>
</template>
