<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import * as echarts from 'echarts'

import type { MonitorStatus } from '@/domain/monitor'
import { tauriClient } from '@/services/tauriClient'

const props = withDefaults(
  defineProps<{
    /**
     * When false, skip polling and show process_stopped (or parent-driven empty).
     * Prefer leaving true so the command returns structured status including disabled.
     */
    active?: boolean
  }>(),
  { active: true },
)

const { t } = useI18n()

const chartRef = ref<HTMLElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const xData = ref<string[]>([])
const upData = ref<number[]>([])
const downData = ref<number[]>([])
const monitorStatus = ref<MonitorStatus | null>(null)
const hasSamples = ref(false)

let timer: ReturnType<typeof setInterval> | undefined
let lastTotalIn = 0
let lastTotalOut = 0

const showEmpty = computed(
  () =>
    !props.active ||
    monitorStatus.value !== 'ok' ||
    !hasSamples.value,
)

const emptyText = computed(() => {
  if (!props.active) {
    return t('overview.trafficStatus.process_stopped')
  }
  const status = monitorStatus.value
  if (status && status !== 'ok') {
    return t(`overview.trafficStatus.${status}`)
  }
  return t('overview.trafficWaiting')
})

const showChart = computed(
  () =>
    props.active &&
    monitorStatus.value === 'ok' &&
    hasSamples.value,
)

const resizeChart = () => chartInstance?.resize()

const resetSeries = () => {
  xData.value = []
  upData.value = []
  downData.value = []
  lastTotalIn = 0
  lastTotalOut = 0
  hasSamples.value = false
}

const chartOption = () => ({
  tooltip: {
    trigger: 'axis' as const,
    backgroundColor: 'var(--ops-surface)',
    borderColor: 'var(--ops-border)',
    borderWidth: 1,
    textStyle: { color: 'var(--ops-text)', fontSize: 12 },
    padding: [8, 12],
  },
  grid: { left: '3%', right: '4%', bottom: '3%', top: '12%', containLabel: true },
  xAxis: {
    type: 'category' as const,
    boundaryGap: false,
    data: xData.value,
    axisLine: { lineStyle: { color: 'var(--ops-border)' } },
    axisLabel: { color: 'var(--ops-muted)', fontSize: 10 },
    axisTick: { show: false },
  },
  yAxis: {
    type: 'value' as const,
    name: 'KB/s',
    nameTextStyle: { color: 'var(--ops-muted)', fontSize: 10 },
    axisLine: { show: false },
    axisLabel: { color: 'var(--ops-muted)', fontSize: 10 },
    splitLine: {
      lineStyle: { color: 'var(--ops-border)', type: 'dashed' as const },
    },
  },
  series: [
    {
      name: t('overview.trafficUp'),
      type: 'line' as const,
      smooth: true,
      showSymbol: false,
      lineStyle: { width: 2, color: 'var(--ops-ok)' },
      itemStyle: { color: 'var(--ops-ok)' },
      data: upData.value,
    },
    {
      name: t('overview.trafficDown'),
      type: 'line' as const,
      smooth: true,
      showSymbol: false,
      lineStyle: { width: 2, color: 'var(--ops-accent)' },
      itemStyle: { color: 'var(--ops-accent)' },
      data: downData.value,
    },
  ],
})

const initChart = () => {
  if (!chartRef.value) return
  if (!chartInstance) {
    chartInstance = echarts.init(chartRef.value)
  }
  chartInstance.setOption(chartOption())
}

const stopPolling = () => {
  if (timer) {
    clearInterval(timer)
    timer = undefined
  }
}

const pollOnce = async () => {
  try {
    const result = await tauriClient.getFrpcTraffic()
    monitorStatus.value = result.status

    if (result.status !== 'ok' || !result.body) {
      hasSamples.value = false
      return
    }

    const data = JSON.parse(result.body) as Record<string, unknown>
    const now = new Date().toLocaleTimeString('en-US', { hour12: false })

    let currentTotalIn = 0
    let currentTotalOut = 0

    Object.keys(data).forEach((proto) => {
      const rows = data[proto]
      if (!Array.isArray(rows)) return
      rows.forEach((row) => {
        const item = row as {
          today_traffic_in?: number
          today_traffic_out?: number
        }
        currentTotalIn += item.today_traffic_in || 0
        currentTotalOut += item.today_traffic_out || 0
      })
    })

    let upSpeed = 0
    let downSpeed = 0
    if (lastTotalIn > 0 || lastTotalOut > 0) {
      upSpeed = Math.max(0, (currentTotalOut - lastTotalOut) / 2 / 1024)
      downSpeed = Math.max(0, (currentTotalIn - lastTotalIn) / 2 / 1024)
    }

    lastTotalIn = currentTotalIn
    lastTotalOut = currentTotalOut

    xData.value.push(now)
    upData.value.push(Number(upSpeed.toFixed(2)))
    downData.value.push(Number(downSpeed.toFixed(2)))

    if (xData.value.length > 30) {
      xData.value.shift()
      upData.value.shift()
      downData.value.shift()
    }

    hasSamples.value = true

    if (chartInstance) {
      chartInstance.setOption({
        xAxis: { data: xData.value },
        series: [{ data: upData.value }, { data: downData.value }],
      })
    } else {
      initChart()
    }
  } catch {
    monitorStatus.value = 'timeout'
    hasSamples.value = false
  }
}

const startPolling = () => {
  stopPolling()
  if (!props.active) {
    monitorStatus.value = 'process_stopped'
    return
  }
  void pollOnce()
  timer = setInterval(() => {
    void pollOnce()
  }, 2000)
}

watch(
  () => props.active,
  (active) => {
    if (active) {
      monitorStatus.value = null
      resetSeries()
      startPolling()
      requestAnimationFrame(() => {
        initChart()
        resizeChart()
      })
    } else {
      stopPolling()
      resetSeries()
      monitorStatus.value = 'process_stopped'
      chartInstance?.clear()
    }
  },
)

onMounted(() => {
  window.addEventListener('resize', resizeChart)
  if (props.active) {
    initChart()
    startPolling()
  } else {
    monitorStatus.value = 'process_stopped'
  }
})

onUnmounted(() => {
  stopPolling()
  chartInstance?.dispose()
  chartInstance = null
  window.removeEventListener('resize', resizeChart)
})
</script>

<template>
  <div class="traffic-chart">
    <div
      ref="chartRef"
      class="traffic-chart__canvas"
      :class="{ 'traffic-chart__canvas--hidden': !showChart }"
    />

    <div
      v-if="showEmpty"
      class="traffic-chart__empty"
      role="status"
      :data-monitor-status="monitorStatus ?? 'pending'"
    >
      {{ emptyText }}
    </div>

    <div
      v-if="showChart"
      class="traffic-chart__legend"
    >
      <span class="traffic-chart__legend-item traffic-chart__legend-item--up">
        <span class="traffic-chart__swatch" aria-hidden="true" />
        {{ t('overview.trafficUp') }}
      </span>
      <span class="traffic-chart__legend-item traffic-chart__legend-item--down">
        <span class="traffic-chart__swatch" aria-hidden="true" />
        {{ t('overview.trafficDown') }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.traffic-chart {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 180px;
}

.traffic-chart__canvas {
  position: absolute;
  inset: 0;
}

.traffic-chart__canvas--hidden {
  visibility: hidden;
  pointer-events: none;
}

.traffic-chart__empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  text-align: center;
  color: var(--ops-muted);
  font-size: 13px;
  border: 1px dashed var(--ops-border);
  border-radius: var(--ops-radius);
  background: var(--ops-surface);
}

.traffic-chart__legend {
  position: absolute;
  top: 0;
  left: 0;
  display: flex;
  gap: 10px;
  font-size: 11px;
  color: var(--ops-muted);
  pointer-events: none;
  z-index: 1;
}

.traffic-chart__legend-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.traffic-chart__swatch {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}

.traffic-chart__legend-item--up {
  color: var(--ops-ok);
}

.traffic-chart__legend-item--down {
  color: var(--ops-accent);
}
</style>
