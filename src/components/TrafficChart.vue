<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'

const chartRef = ref<HTMLElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const xData = ref<string[]>([])
const upData = ref<number[]>([])
const downData = ref<number[]>([])

let timer: any = null

const initChart = () => {
    if (chartRef.value) {
        chartInstance = echarts.init(chartRef.value)
        chartInstance.setOption({
            tooltip: {
                trigger: 'axis',
                backgroundColor: 'rgba(15, 23, 42, 0.9)',
                borderColor: 'rgba(255, 255, 255, 0.08)',
                borderWidth: 1,
                textStyle: { color: '#F8FAFC', fontSize: 12, fontFamily: 'Plus Jakarta Sans' },
                padding: [10, 14],
                extraCssText: 'border-radius: 10px; backdrop-filter: blur(8px); box-shadow: 0 8px 32px rgba(0,0,0,0.3);'
            },
            grid: { left: '3%', right: '4%', bottom: '3%', top: '8%', containLabel: true },
            xAxis: {
                type: 'category',
                boundaryGap: false,
                data: xData.value,
                axisLine: { lineStyle: { color: 'rgba(148, 163, 184, 0.1)' } },
                axisLabel: { color: 'rgba(148, 163, 184, 0.5)', fontSize: 10, fontFamily: 'Plus Jakarta Sans' },
                axisTick: { show: false }
            },
            yAxis: {
                type: 'value',
                name: 'KB/s',
                nameTextStyle: { color: 'rgba(148, 163, 184, 0.5)', fontSize: 10, fontFamily: 'Plus Jakarta Sans', padding: [0, 0, 0, -30] },
                axisLine: { show: false },
                axisLabel: { color: 'rgba(148, 163, 184, 0.5)', fontSize: 10, fontFamily: 'Plus Jakarta Sans' },
                splitLine: { lineStyle: { color: 'rgba(148, 163, 184, 0.06)', type: 'dashed' } }
            },
            series: [
                {
                    name: '上行 (Up)',
                    type: 'line',
                    smooth: true,
                    showSymbol: false,
                    lineStyle: { width: 2.5, color: '#22C55E' },
                    itemStyle: { color: '#22C55E' },
                    areaStyle: {
                        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                            { offset: 0, color: 'rgba(34, 197, 94, 0.25)' },
                            { offset: 0.5, color: 'rgba(34, 197, 94, 0.08)' },
                            { offset: 1, color: 'rgba(34, 197, 94, 0)' }
                        ])
                    },
                    data: upData.value
                },
                {
                    name: '下行 (Down)',
                    type: 'line',
                    smooth: true,
                    showSymbol: false,
                    lineStyle: { width: 2.5, color: '#0EA5E9' },
                    itemStyle: { color: '#0EA5E9' },
                    areaStyle: {
                        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                            { offset: 0, color: 'rgba(14, 165, 233, 0.25)' },
                            { offset: 0.5, color: 'rgba(14, 165, 233, 0.08)' },
                            { offset: 1, color: 'rgba(14, 165, 233, 0)' }
                        ])
                    },
                    data: downData.value
                }
            ] as any
        })
    }
}

const fetchTrafficData = async () => {
    timer = setInterval(async () => {
        try {
            const response = await fetch('http://127.0.0.1:7400/api/status')
            if (!response.ok) return

            const data = await response.json()
            const now = new Date().toLocaleTimeString('en-US', { hour12: false })

            let totalIn = 0
            let totalOut = 0

            if (data?.tcp) {
                data.tcp.forEach((p: any) => {
                    totalIn += p.today_traffic_in || 0
                    totalOut += p.today_traffic_out || 0
                })
            }

            const upSpeed = totalOut / 1024
            const downSpeed = totalIn / 1024

            xData.value.push(now)
            upData.value.push(Number(upSpeed.toFixed(2)))
            downData.value.push(Number(downSpeed.toFixed(2)))

            if (xData.value.length > 30) {
                xData.value.shift()
                upData.value.shift()
                downData.value.shift()
            }

            if (chartInstance) {
                chartInstance.setOption({
                    xAxis: { data: xData.value },
                    series: [
                        { data: upData.value },
                        { data: downData.value }
                    ]
                })
            }
        } catch {
            // frpc 未启动时忽略
        }
    }, 2000)
}

onMounted(() => {
    initChart()
    fetchTrafficData()
    window.addEventListener('resize', () => chartInstance?.resize())
})

onUnmounted(() => {
    if (timer) clearInterval(timer)
    chartInstance?.dispose()
    window.removeEventListener('resize', () => chartInstance?.resize())
})
</script>

<template>
    <div class="w-full h-full relative">
        <div ref="chartRef" class="w-full h-full absolute inset-0"></div>

        <!-- Legend Badges -->
        <div class="absolute top-0 left-0 text-[11px] flex gap-2.5 select-none">
            <div
                class="flex items-center gap-1.5 bg-emerald-500/[0.08] border border-emerald-500/20 text-emerald-400 px-2.5 py-1 rounded-lg backdrop-blur-sm">
                <span class="w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(34,197,94,0.5)]"></span>
                <span class="font-medium">上行</span>
            </div>
            <div
                class="flex items-center gap-1.5 bg-sky-500/[0.08] border border-sky-500/20 text-sky-400 px-2.5 py-1 rounded-lg backdrop-blur-sm">
                <span class="w-2 h-2 rounded-full bg-sky-500 shadow-[0_0_6px_rgba(14,165,233,0.5)]"></span>
                <span class="font-medium">下行</span>
            </div>
        </div>
    </div>
</template>
