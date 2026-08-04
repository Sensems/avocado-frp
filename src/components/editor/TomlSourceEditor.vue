<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { basicSetup } from 'codemirror'
import { EditorView } from '@codemirror/view'
import { Compartment, EditorState } from '@codemirror/state'
import { StreamLanguage } from '@codemirror/language'
import { toml } from '@codemirror/legacy-modes/mode/toml'
import { oneDark } from '@codemirror/theme-one-dark'

import { useTheme } from '@/composables/useTheme'

const props = withDefaults(
  defineProps<{
    modelValue: string
    disabled?: boolean
  }>(),
  {
    disabled: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const { isDark } = useTheme()

const hostRef = ref<HTMLElement | null>(null)
let view: EditorView | null = null
let applyingExternal = false

const themeCompartment = new Compartment()
const editableCompartment = new Compartment()

const themeExtensions = (dark: boolean) => (dark ? [oneDark] : [])

const editableExtensions = (disabled: boolean) => [
  EditorView.editable.of(!disabled),
  EditorState.readOnly.of(disabled),
]

const mountEditor = () => {
  if (!hostRef.value || view) return

  view = new EditorView({
    parent: hostRef.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        basicSetup,
        StreamLanguage.define(toml),
        themeCompartment.of(themeExtensions(isDark.value)),
        editableCompartment.of(editableExtensions(props.disabled)),
        EditorView.theme({
          '&': {
            height: '100%',
            fontSize: '12px',
          },
          '.cm-scroller': {
            fontFamily:
              "var(--font-mono, 'Fira Code', 'JetBrains Mono', ui-monospace, monospace)",
            lineHeight: '1.5',
            overflow: 'auto',
          },
          '&.cm-focused': {
            outline: 'none',
          },
        }),
        EditorView.updateListener.of((update) => {
          if (!update.docChanged || applyingExternal) return
          emit('update:modelValue', update.state.doc.toString())
        }),
      ],
    }),
  })
}

onMounted(() => {
  mountEditor()
})

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

watch(
  () => props.modelValue,
  (next) => {
    if (!view) return
    const current = view.state.doc.toString()
    if (current === next) return
    applyingExternal = true
    view.dispatch({
      changes: { from: 0, to: current.length, insert: next },
    })
    applyingExternal = false
  },
)

watch(isDark, (dark) => {
  view?.dispatch({
    effects: themeCompartment.reconfigure(themeExtensions(dark)),
  })
})

watch(
  () => props.disabled,
  (disabled) => {
    view?.dispatch({
      effects: editableCompartment.reconfigure(editableExtensions(disabled)),
    })
  },
)
</script>

<template>
  <div
    ref="hostRef"
    class="toml-source-editor"
    :class="{ 'toml-source-editor--disabled': disabled }"
    :aria-disabled="disabled || undefined"
  />
</template>

<style scoped>
.toml-source-editor {
  min-height: 280px;
  height: min(52vh, 520px);
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
  overflow: hidden;
}

.toml-source-editor--disabled {
  opacity: 0.72;
}

.toml-source-editor :deep(.cm-editor) {
  height: 100%;
}

.toml-source-editor :deep(.cm-editor.cm-focused) {
  outline: 2px solid color-mix(in srgb, var(--ops-accent) 45%, transparent);
  outline-offset: -1px;
}
</style>
