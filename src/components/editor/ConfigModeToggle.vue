<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useDialog, NButton } from 'naive-ui'
import { FileCode, ListChecks } from 'lucide-vue-next'

import {
  confirmDiscardIfNeeded,
  confirmDiscardSourceIfNeeded,
  type UnsavedDialogApi,
} from '@/components/feedback/unsavedGuard'

export type ConfigEditorMode = 'form' | 'source'

const props = withDefaults(
  defineProps<{
    modelValue: ConfigEditorMode
    /** True when source draft differs from last applied / snapshot.raw */
    sourceDirty?: boolean
    /** True when form-mode fields differ from the loaded snapshot baseline. */
    formDirty?: boolean
    disabled?: boolean
  }>(),
  {
    sourceDirty: false,
    formDirty: false,
    disabled: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [mode: ConfigEditorMode]
  /** Parent should seed the source editor from `snapshot.raw`. */
  'enter-source': []
  /** Parent should drop the unapplied source draft (never auto-Apply). */
  'discard-source': []
}>()

const { t } = useI18n()
const dialog = useDialog() as UnsavedDialogApi

const selectForm = async () => {
  if (props.disabled || props.modelValue === 'form') return

  const ok = await confirmDiscardSourceIfNeeded(
    props.sourceDirty,
    dialog,
    t,
  )
  if (!ok) return

  if (props.sourceDirty) {
    emit('discard-source')
  }
  emit('update:modelValue', 'form')
}

const selectSource = async () => {
  if (props.disabled || props.modelValue === 'source') return

  const ok = await confirmDiscardIfNeeded(props.formDirty, dialog, t)
  if (!ok) return

  emit('update:modelValue', 'source')
  emit('enter-source')
}
</script>

<template>
  <div
    class="config-mode-toggle"
    role="group"
    :aria-label="t('editor.modeGroup')"
  >
    <NButton
      size="small"
      :type="modelValue === 'form' ? 'primary' : 'default'"
      :secondary="modelValue !== 'form'"
      :disabled="disabled"
      :aria-label="t('editor.formMode')"
      :aria-pressed="modelValue === 'form'"
      @click="selectForm"
    >
      <template #icon>
        <ListChecks :size="14" aria-hidden="true" />
      </template>
      {{ t('editor.formMode') }}
    </NButton>
    <NButton
      size="small"
      :type="modelValue === 'source' ? 'primary' : 'default'"
      :secondary="modelValue !== 'source'"
      :disabled="disabled"
      :aria-label="t('editor.sourceMode')"
      :aria-pressed="modelValue === 'source'"
      @click="selectSource"
    >
      <template #icon>
        <FileCode :size="14" aria-hidden="true" />
      </template>
      {{ t('editor.sourceMode') }}
    </NButton>
  </div>
</template>

<style scoped>
.config-mode-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
