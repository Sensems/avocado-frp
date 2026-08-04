<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import {
  useDialog,
  useMessage,
  NButton,
  NInput,
  NSelect,
  NModal,
  NPopconfirm,
} from 'naive-ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Edit2, ExternalLink, Plus, Trash2 } from 'lucide-vue-next'

import ConfigModeToggle, {
  type ConfigEditorMode,
} from '@/components/editor/ConfigModeToggle.vue'
import ConfigOpsBar from '@/components/editor/ConfigOpsBar.vue'
import SourceApplyPanel from '@/components/editor/SourceApplyPanel.vue'
import TomlSourceEditor from '@/components/editor/TomlSourceEditor.vue'
import {
  attachBeforeUnload,
  unsavedGuard,
  type UnsavedDialogApi,
} from '@/components/feedback/unsavedGuard'
import FrpcConfigForm from '@/components/FrpcConfigForm.vue'
import ProtocolForm from '@/components/ProtocolForm.vue'
import {
  buildFrpcGlobalPatch,
  buildProxyAddPatch,
  buildProxyDeletePatch,
  buildProxyUpdatePatch,
  type FrpcConfigPatch,
  type FrpcGlobalFormData,
  type ProxyRuleKnown,
  type ProxySelector,
} from '@/domain/config'
import {
  PROTOCOL_TYPES,
  type ProxyRuleSavePayload,
} from '@/domain/proxyRule'
import {
  getCommandErrorI18nKey,
  normalizeCommandError,
} from '@/services/errorMapper'
import { useConfigStore } from '@/stores/config'
import { useProcessStore } from '@/stores/process'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog() as UnsavedDialogApi

const configStore = useConfigStore()
const processStore = useProcessStore()
const { frpc: frpcSnapshot } = storeToRefs(configStore)

const mode = ref<ConfigEditorMode>('form')
const formDirty = ref(false)
const busy = ref(false)

const sourceDraft = ref('')
const sourceBaseline = ref('')

const showAddForm = ref(false)
const editMode = ref(false)
const editIndex = ref(-1)
const editingData = shallowRef<ProxyRuleKnown>()

const searchQuery = ref('')
const protocolFilter = ref<string | null>(null)

const formRef = ref<{
  getFormData: () => FrpcGlobalFormData
  isDirty: () => boolean
} | null>(null)

const sourceApplyRef = ref<{
  apply: () => Promise<boolean>
  applyAndRestart: () => Promise<boolean>
} | null>(null)

const sourceDirty = computed(
  () => sourceDraft.value !== sourceBaseline.value,
)

const pageDirty = computed(() =>
  mode.value === 'form' ? formDirty.value : sourceDirty.value,
)

const leaveDirty = computed(() => formDirty.value || sourceDirty.value)

const protocolOptions = computed(() =>
  PROTOCOL_TYPES.map((type) => ({
    label: type.toUpperCase(),
    value: type,
  })),
)

const filteredProxies = computed(() => {
  const proxies = frpcSnapshot.value?.known.proxies ?? []
  const query = searchQuery.value.trim().toLowerCase()
  const protocol = protocolFilter.value

  return proxies.filter((proxy) => {
    if (protocol && (proxy.type ?? '').toLowerCase() !== protocol) {
      return false
    }
    if (!query) return true
    const haystack = [
      proxy.name,
      proxy.type,
      proxy.localIp,
      proxy.localPort,
      proxy.remotePort,
      ...(proxy.customDomains ?? []),
    ]
      .filter((part) => part != null && part !== '')
      .join(' ')
      .toLowerCase()
    return haystack.includes(query)
  })
})

const showCommandError = (error: unknown) => {
  message.error(t(getCommandErrorI18nKey(normalizeCommandError(error))))
}

const handleConflictReload = async (code: string) => {
  if (code !== 'CONFIG_CONFLICT') return
  message.warning(t('errors.CONFIG_CONFLICT'))
  try {
    await configStore.load('frpc')
  } catch (error) {
    showCommandError(error)
  }
}

const applyFrpcPatch = async (patch: FrpcConfigPatch): Promise<boolean> => {
  const current = frpcSnapshot.value
  if (!current) {
    try {
      await configStore.load('frpc')
    } catch {
      return false
    }
  }
  const snapshot = frpcSnapshot.value
  if (!snapshot) return false

  busy.value = true
  try {
    const result = await configStore.applyPatch(
      'frpc',
      snapshot.revision,
      patch,
    )
    if (!result.ok) {
      showCommandError(result.error)
      await handleConflictReload(result.error.code)
      return false
    }
    return true
  } finally {
    busy.value = false
  }
}

const proxySelector = (proxy: ProxyRuleKnown): ProxySelector => ({
  index: proxy.sourceIndex,
  originalName: proxy.sourceName,
})

const onStart = async () => {
  const result = await processStore.start('frpc')
  if (result.ok) {
    message.success(t('feedback.startSuccess', { name: 'frpc' }))
  } else {
    message.error(
      t('feedback.startFail', {
        name: 'frpc',
        error: t(getCommandErrorI18nKey(result.error)),
      }),
    )
  }
}

const onStop = async () => {
  const result = await processStore.stop('frpc')
  if (result.ok) {
    message.success(t('feedback.stopSuccess', { name: 'frpc' }))
  } else {
    message.error(
      t('feedback.stopFail', {
        name: 'frpc',
        error: t(getCommandErrorI18nKey(result.error)),
      }),
    )
  }
}

const resolveGlobalForm = (): FrpcGlobalFormData | null => {
  if (formRef.value) return formRef.value.getFormData()
  const known = frpcSnapshot.value?.known
  if (!known) return null
  return {
    serverAddr: known.serverAddr ?? '',
    serverPort: known.serverPort ?? null,
    authMethod: known.auth.method ?? null,
    token: known.auth.token ?? '',
  }
}

const onSaveGlobal = async (configData: FrpcGlobalFormData) => {
  if (await applyFrpcPatch(buildFrpcGlobalPatch(configData))) {
    message.success(t('feedback.saveSuccess'))
  }
}

const onSourceApplied = (raw: string) => {
  sourceDraft.value = raw
  sourceBaseline.value = raw
}

const onSourceConflict = () => {
  const raw = frpcSnapshot.value?.raw ?? ''
  sourceDraft.value = raw
  sourceBaseline.value = raw
}

const onSave = async () => {
  if (mode.value === 'source') {
    if (!sourceApplyRef.value) {
      message.info(t('editor.opsBarSourceHint'))
      return
    }
    busy.value = true
    try {
      await sourceApplyRef.value.apply()
    } finally {
      busy.value = false
    }
    return
  }

  const formData = resolveGlobalForm()
  if (!formData) return
  await onSaveGlobal(formData)
}

const reportSaveAndRestartFailure = (
  error: Parameters<typeof getCommandErrorI18nKey>[0],
  recovery?: {
    configRestored: boolean
    processRestored: boolean
  },
) => {
  message.error(
    t('feedback.saveAndRestartFail', {
      error: t(getCommandErrorI18nKey(error)),
    }),
  )
  if (recovery) {
    message.warning(
      t('feedback.recoveryHint', {
        config: recovery.configRestored
          ? t('feedback.recovered')
          : t('feedback.notRecovered'),
        process: recovery.processRestored
          ? t('feedback.recovered')
          : t('feedback.notRecovered'),
      }),
    )
  }
}

const onSaveAndRestart = async () => {
  const current = frpcSnapshot.value
  if (!current) return

  busy.value = true
  try {
    if (mode.value === 'source') {
      if (!sourceApplyRef.value) {
        message.info(t('editor.opsBarSourceHint'))
        return
      }
      await sourceApplyRef.value.applyAndRestart()
      return
    }

    const formData = resolveGlobalForm()
    if (!formData) return

    const result = await configStore.saveAndRestart({
      kind: 'frpc',
      expectedRevision: current.revision,
      change: {
        mode: 'patch',
        patch: buildFrpcGlobalPatch(formData),
      },
    })
    if (!result.ok) {
      reportSaveAndRestartFailure(result.error, result.recovery)
      await handleConflictReload(result.error.code)
      return
    }
    message.success(t('feedback.saveAndRestartSuccess'))
  } finally {
    busy.value = false
  }
}

	const onEnterSource = () => {
	  // Form is torn down via v-if; clear so leave-guard does not false-positive.
	  formDirty.value = false
	  const raw = frpcSnapshot.value?.raw ?? ''
	  sourceDraft.value = raw
	  sourceBaseline.value = raw
	}

const onDiscardSource = () => {
  sourceDraft.value = sourceBaseline.value
}

const handleAddRule = () => {
  editMode.value = false
  editIndex.value = -1
  editingData.value = undefined
  showAddForm.value = true
}

const handleEditRule = (proxy: ProxyRuleKnown) => {
  const index =
    frpcSnapshot.value?.known.proxies.findIndex(
      (item) =>
        item.sourceIndex === proxy.sourceIndex &&
        item.sourceName === proxy.sourceName,
    ) ?? -1
  editMode.value = true
  editIndex.value = index
  editingData.value = proxy
  showAddForm.value = true
}

const handleDeleteRule = async (proxy: ProxyRuleKnown) => {
  const saved = await applyFrpcPatch({
    proxyOperations: [buildProxyDeletePatch(proxySelector(proxy))],
  })
  if (saved) {
    message.success(t('feedback.deleteSuccess'))
  }
}

const handleSaveRule = async (payload: ProxyRuleSavePayload) => {
  let operation
  if (
    payload.editMode &&
    typeof payload.editIndex === 'number' &&
    payload.editIndex >= 0
  ) {
    const original = frpcSnapshot.value?.known.proxies[payload.editIndex]
    if (!original) return
    operation = buildProxyUpdatePatch(proxySelector(original), payload)
  } else {
    operation = buildProxyAddPatch(payload)
  }

  const saved = await applyFrpcPatch({ proxyOperations: [operation] })
  if (saved) {
    showAddForm.value = false
    message.success(t('feedback.ruleSaveSuccess'))
  }
}

const handleJump = (proxy: ProxyRuleKnown) => {
  try {
    const proxyType = proxy.type ?? 'tcp'
    const serverIp = frpcSnapshot.value?.known.serverAddr || '127.0.0.1'
    let url = ''
    if (proxyType === 'http' || proxyType === 'https') {
      const domain = proxy.customDomains?.[0] || '127.0.0.1'
      url = `${proxyType}://${domain}`
    } else if (proxyType === 'tcp') {
      url = `http://${serverIp}:${proxy.remotePort}`
    } else {
      url = `${proxyType}://${serverIp}:${proxy.remotePort}`
    }
    void openUrl(url)
  } catch (error) {
    showCommandError(error)
  }
}

const isHttpProxy = (proxy: ProxyRuleKnown) => {
  const type = proxy.type ?? ''
  return type === 'http' || type === 'https'
}

onBeforeRouteLeave(unsavedGuard(() => leaveDirty.value, dialog, t))

let detachBeforeUnload: (() => void) | undefined

onMounted(() => {
  detachBeforeUnload = attachBeforeUnload(() => leaveDirty.value)
  if (!frpcSnapshot.value) {
    void configStore.load('frpc').catch(() => {
      /* App shell already surfaces load failures */
    })
  }
})

onUnmounted(() => {
  detachBeforeUnload?.()
})
</script>

<template>
  <div class="client-page">
    <ConfigOpsBar
      kind="frpc"
      :mode="mode"
      :dirty="pageDirty"
      :busy="busy"
      @start="onStart"
      @stop="onStop"
      @save="onSave"
      @save-and-restart="onSaveAndRestart"
    />

    <div class="client-page__body">
      <header class="client-page__header">
        <div>
          <h2 class="ops-page__title">{{ t('nav.client') }}</h2>
          <p class="client-page__subtitle">{{ t('client.subtitle') }}</p>
        </div>
        <ConfigModeToggle
          v-model="mode"
          :source-dirty="sourceDirty"
          :form-dirty="formDirty"
          :disabled="busy"
          @enter-source="onEnterSource"
          @discard-source="onDiscardSource"
        />
      </header>

      <section
        v-if="mode === 'form'"
        class="client-page__form"
      >
        <article class="ops-card">
          <div class="ops-card__header">
            <h3 class="ops-card__title">{{ t('client.globalConfig') }}</h3>
          </div>
          <FrpcConfigForm
            ref="formRef"
            :initial-data="frpcSnapshot?.known"
            hide-save
            @update:dirty="formDirty = $event"
            @save="onSaveGlobal"
          />
        </article>

        <article class="ops-card">
          <div class="ops-card__header">
            <h3 class="ops-card__title">{{ t('client.proxyRules') }}</h3>
            <NButton
              type="primary"
              size="small"
              :disabled="busy"
              :aria-label="t('dashboard.addRule')"
              @click="handleAddRule"
            >
              <template #icon>
                <Plus :size="14" aria-hidden="true" />
              </template>
              {{ t('dashboard.addRule') }}
            </NButton>
          </div>

          <div class="client-page__filters">
            <NInput
              v-model:value="searchQuery"
              clearable
              size="small"
              :placeholder="t('client.searchRules')"
              class="client-page__search"
            />
            <NSelect
              v-model:value="protocolFilter"
              size="small"
              clearable
              :options="protocolOptions"
              :placeholder="t('client.filterProtocol')"
              class="client-page__protocol"
            />
          </div>

          <div
            v-if="filteredProxies.length > 0"
            class="proxy-list"
          >
            <div
              v-for="proxy in filteredProxies"
              :key="`${proxy.sourceIndex}:${proxy.sourceName}`"
              class="proxy-list__row"
            >
              <div class="proxy-list__main">
                <div class="proxy-list__identity">
                  <span class="proxy-list__name">{{ proxy.name }}</span>
                  <span class="proxy-list__type">{{ proxy.type }}</span>
                </div>
                <div class="proxy-list__meta">
                  <span>
                    {{ t('client.localLabel') }}
                    {{ proxy.localIp }}:{{ proxy.localPort }}
                  </span>
                  <span v-if="isHttpProxy(proxy)">
                    {{ t('client.domainLabel') }}
                    {{ proxy.customDomains?.join(', ') || '—' }}
                  </span>
                  <span v-else>
                    {{ t('client.remoteLabel') }}
                    {{ proxy.remotePort ?? '—' }}
                  </span>
                </div>
              </div>
              <div class="proxy-list__actions">
                <NButton
                  circle
                  size="small"
                  tertiary
                  type="primary"
                  :title="t('actions.openExternal')"
                  :aria-label="t('actions.openExternal')"
                  @click="handleJump(proxy)"
                >
                  <template #icon>
                    <ExternalLink :size="14" aria-hidden="true" />
                  </template>
                </NButton>
                <NButton
                  circle
                  size="small"
                  tertiary
                  type="info"
                  :title="t('actions.editRule')"
                  :aria-label="t('actions.editRule')"
                  @click="handleEditRule(proxy)"
                >
                  <template #icon>
                    <Edit2 :size="14" aria-hidden="true" />
                  </template>
                </NButton>
                <NPopconfirm
                  placement="top"
                  @positive-click="handleDeleteRule(proxy)"
                >
                  <template #trigger>
                    <NButton
                      circle
                      size="small"
                      tertiary
                      type="error"
                      :aria-label="t('actions.deleteRule')"
                    >
                      <template #icon>
                        <Trash2 :size="14" aria-hidden="true" />
                      </template>
                    </NButton>
                  </template>
                  {{ t('actions.confirmDeleteRule') }}
                </NPopconfirm>
              </div>
            </div>
          </div>
          <p
            v-else
            class="ops-card__empty"
          >
            {{ t('dashboard.ruleEmpty') }}
          </p>
        </article>
      </section>

      <section
        v-else
        class="client-page__source ops-card"
      >
        <div class="ops-card__header">
          <h3 class="ops-card__title">{{ t('editor.sourceMode') }}</h3>
        </div>
        <p class="client-page__source-hint">
          {{ t('client.sourceHint') }}
        </p>
        <TomlSourceEditor
          v-model="sourceDraft"
          :disabled="busy"
          :aria-label="t('editor.sourceMode')"
        />
        <SourceApplyPanel
          ref="sourceApplyRef"
          kind="frpc"
          :draft="sourceDraft"
          :expected-revision="frpcSnapshot?.revision ?? ''"
          :disabled="busy || !frpcSnapshot"
          @applied="onSourceApplied"
          @conflict="onSourceConflict"
        />
      </section>
    </div>

    <NModal
      v-model:show="showAddForm"
      transform-origin="center"
    >
      <ProtocolForm
        v-if="showAddForm"
        :edit-mode="editMode"
        :edit-index="editIndex"
        :initial-data="editingData"
        @save="handleSaveRule"
        @cancel="showAddForm = false"
      />
    </NModal>
  </div>
</template>

<style scoped>
.client-page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.client-page__body {
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
  padding: var(--ops-gap);
}

.client-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.ops-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--ops-text);
}

.client-page__subtitle {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--ops-muted);
}

.client-page__form {
  display: flex;
  flex-direction: column;
  gap: var(--ops-gap);
}

.ops-card {
  background: var(--ops-surface);
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  padding: 14px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ops-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.ops-card__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--ops-text);
}

.ops-card__empty {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
  padding: 24px 0;
  text-align: center;
}

.client-page__filters {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.client-page__search {
  flex: 1 1 220px;
  min-width: 160px;
}

.client-page__protocol {
  flex: 0 1 160px;
  min-width: 140px;
}

.proxy-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.proxy-list__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-bg);
}

.proxy-list__main {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.proxy-list__identity {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.proxy-list__name {
  font-size: 13px;
  font-weight: 600;
  color: var(--ops-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proxy-list__type {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--ops-muted);
  flex-shrink: 0;
}

.proxy-list__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  color: var(--ops-muted);
}

.proxy-list__actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  opacity: 1;
}

.client-page__source {
  min-width: 0;
}

.client-page__source-hint {
  margin: 0;
  font-size: 13px;
  color: var(--ops-muted);
}

@media (max-width: 720px) {
  .proxy-list__row {
    flex-direction: column;
    align-items: stretch;
  }

  .proxy-list__actions {
    justify-content: flex-end;
  }
}
</style>
