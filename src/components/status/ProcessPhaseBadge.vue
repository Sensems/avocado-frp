<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  AlertTriangle,
  CheckCircle2,
  CircleOff,
  Loader2,
  PauseCircle,
  XCircle,
} from 'lucide-vue-next'

import type { ProcessPhase, ProcessSnapshot } from '@/domain/process'

const props = defineProps<{
  snapshot?: ProcessSnapshot | null
  phase?: ProcessPhase | null
}>()

const { t } = useI18n()

const resolvedPhase = computed<ProcessPhase>(
  () => props.snapshot?.phase ?? props.phase ?? 'stopped',
)

const label = computed(() => t(`status.phase.${resolvedPhase.value}`))

const icon = computed<Component>(() => {
  switch (resolvedPhase.value) {
    case 'healthy':
      return CheckCircle2
    case 'degraded':
      return AlertTriangle
    case 'starting':
      return Loader2
    case 'stopping':
      return PauseCircle
    case 'crashed':
      return XCircle
    default:
      return CircleOff
  }
})

const toneClass = computed(() => {
  switch (resolvedPhase.value) {
    case 'healthy':
      return 'phase-badge--ok'
    case 'degraded':
    case 'starting':
    case 'stopping':
      return 'phase-badge--warn'
    case 'crashed':
      return 'phase-badge--danger'
    default:
      return 'phase-badge--muted'
  }
})

const spinIcon = computed(
  () =>
    resolvedPhase.value === 'starting' || resolvedPhase.value === 'stopping',
)
</script>

<template>
  <span
    class="phase-badge"
    :class="toneClass"
    :title="label"
    :aria-label="label"
  >
    <component
      :is="icon"
      class="phase-badge__icon"
      :class="{ 'phase-badge__icon--spin': spinIcon }"
      :size="14"
      aria-hidden="true"
    />
    <span class="phase-badge__text">{{ label }}</span>
  </span>
</template>

<style scoped>
.phase-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--ops-text);
  font-variant-numeric: tabular-nums;
}

.phase-badge__icon {
  flex-shrink: 0;
}

.phase-badge__icon--spin {
  animation: phase-spin 1s linear infinite;
}

.phase-badge__text {
  line-height: 1;
}

.phase-badge--ok {
  color: var(--ops-ok);
}

.phase-badge--warn {
  color: var(--ops-warn);
}

.phase-badge--danger {
  color: var(--ops-danger);
}

.phase-badge--muted {
  color: var(--ops-muted);
}

@keyframes phase-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
