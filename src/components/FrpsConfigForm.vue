<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Save } from 'lucide-vue-next'
import type { FrpsFormData, FrpsKnownConfig } from '@/domain/config'

const props = withDefaults(
  defineProps<{
    initialData?: FrpsKnownConfig
    /** When true, omit the inline Save button (parent OpsBar handles save). */
    hideSave?: boolean
  }>(),
  {
    hideSave: false,
  },
)

interface FrpsFormState {
  bindPort: string
  vhostHttpPort: string
  vhostHttpsPort: string
  authMethod: string | null
  token: string
  dashboardPort: string
  dashboardUser: string
  dashboardPwd: string
}

const inputString = (value?: string | number | null): string =>
  value === undefined || value === null ? '' : String(value)

const toFrpsForm = (value?: FrpsKnownConfig): FrpsFormState => ({
  bindPort: inputString(value?.bindPort),
  vhostHttpPort: inputString(value?.vhostHTTPPort),
  vhostHttpsPort: inputString(value?.vhostHTTPSPort),
  authMethod: value?.auth.method ?? 'token',
  token: value?.auth.token ?? '',
  dashboardPort: inputString(value?.webServer.port),
  dashboardUser: value?.webServer.user ?? '',
  dashboardPwd: value?.webServer.password ?? '',
})

const form = ref<FrpsFormState>(toFrpsForm(props.initialData))

const emit = defineEmits<{
  save: [value: FrpsFormData]
  'update:dirty': [dirty: boolean]
}>()

const isDirty = computed(() => {
  const baseline = toFrpsForm(props.initialData)
  const current = form.value
  return (
    baseline.bindPort !== current.bindPort ||
    baseline.vhostHttpPort !== current.vhostHttpPort ||
    baseline.vhostHttpsPort !== current.vhostHttpsPort ||
    baseline.authMethod !== current.authMethod ||
    baseline.token !== current.token ||
    baseline.dashboardPort !== current.dashboardPort ||
    baseline.dashboardUser !== current.dashboardUser ||
    baseline.dashboardPwd !== current.dashboardPwd
  )
})

// Watch only form baseline fields — unrelated snapshot churn must not reset dirty edits.
watch(
  () =>
    [
      props.initialData?.bindPort ?? null,
      props.initialData?.vhostHTTPPort ?? null,
      props.initialData?.vhostHTTPSPort ?? null,
      props.initialData?.auth.method ?? null,
      props.initialData?.auth.token ?? '',
      props.initialData?.webServer.port ?? null,
      props.initialData?.webServer.user ?? '',
      props.initialData?.webServer.password ?? '',
    ] as const,
  (_next, prev) => {
    if (prev !== undefined && isDirty.value) return
    form.value = toFrpsForm(props.initialData)
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
  getFormData: (): FrpsFormData => ({ ...form.value }),
  isDirty: () => isDirty.value,
})
</script>

<template>
  <div>
    <n-form
      :model="form"
      size="large"
      label-placement="top"
    >
      <div class="grid grid-cols-2 gap-5">
        <n-form-item
          :label="$t('forms.bindPort')"
          path="bindPort"
        >
          <n-input
            v-model:value="form.bindPort"
            placeholder="7000"
          />
        </n-form-item>
        <n-form-item
          :label="$t('forms.authToken')"
          path="token"
        >
          <n-input
            v-model:value="form.token"
            type="password"
            show-password-on="click"
            :placeholder="$t('forms.authTokenPlace')"
          />
        </n-form-item>
      </div>

      <div class="grid grid-cols-2 gap-5">
        <n-form-item
          :label="$t('forms.vhostHttpPort')"
          path="vhostHttpPort"
        >
          <n-input
            v-model:value="form.vhostHttpPort"
            placeholder="80"
          />
        </n-form-item>
        <n-form-item
          :label="$t('forms.vhostHttpsPort')"
          path="vhostHttpsPort"
        >
          <n-input
            v-model:value="form.vhostHttpsPort"
            placeholder="443"
          />
        </n-form-item>
      </div>

      <div class="flex items-center gap-3 my-5">
        <div class="flex-1 h-px bg-gradient-to-r from-transparent via-slate-500/15 to-transparent" />
        <span
          class="text-xs font-semibold text-slate-400 dark:text-slate-500 tracking-wide uppercase"
        >{{ $t('forms.dashboardSection') }}</span>
        <div class="flex-1 h-px bg-gradient-to-r from-transparent via-slate-500/15 to-transparent" />
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
        <n-form-item
          :label="$t('forms.dashboardPort')"
          path="dashboardPort"
        >
          <n-input
            v-model:value="form.dashboardPort"
            placeholder="7500"
          />
        </n-form-item>
        <n-form-item
          :label="$t('forms.dashboardUser')"
          path="dashboardUser"
        >
          <n-input v-model:value="form.dashboardUser" />
        </n-form-item>
        <n-form-item
          :label="$t('forms.dashboardPwd')"
          path="dashboardPwd"
        >
          <n-input
            v-model:value="form.dashboardPwd"
            type="password"
            show-password-on="click"
          />
        </n-form-item>
      </div>
    </n-form>

    <div
      v-if="!hideSave"
      class="flex justify-end mt-5"
    >
      <n-button
        type="primary"
        size="large"
        class="px-6 transition-all duration-200 active:scale-[0.97] cursor-pointer"
        @click="handleSave"
      >
        <template #icon>
          <Save :size="15" />
        </template>
        {{ $t('forms.save') }} frps.toml
      </n-button>
    </div>
  </div>
</template>
