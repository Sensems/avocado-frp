<script setup lang="ts">
import {
    BookOpen,
    Layers,
    Monitor,
    Server,
    Rocket,
    Lightbulb,
    HelpCircle,
    ChevronRight,
    Gamepad2,
    Globe,
    HardDrive,
    Code,
    ScreenShare,
} from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
    <div class="flex flex-col gap-6 max-w-4xl mx-auto">
        <!-- 什么是 FRP -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-emerald-500/10 border border-emerald-500/20">
                        <BookOpen :size="16" class="text-emerald-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.whatIsFrp') }}</span>
                </div>
            </template>
            <p class="text-sm leading-relaxed text-slate-700 dark:text-slate-300">
                {{ t('guide.whatIsFrpDesc') }}
            </p>
        </n-card>

        <!-- 内网穿透 -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-sky-500/10 border border-sky-500/20">
                        <Globe :size="16" class="text-sky-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.whatIsNat') }}</span>
                </div>
            </template>
            <p class="text-sm leading-relaxed text-slate-700 dark:text-slate-300">
                {{ t('guide.whatIsNatDesc') }}
            </p>
        </n-card>

        <!-- 客户端 vs 服务端 -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-violet-500/10 border border-violet-500/20">
                        <Layers :size="16" class="text-violet-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.clientServer') }}</span>
                </div>
            </template>
            <div class="flex flex-col gap-3">
                <div
                    class="flex items-start gap-3 p-3 rounded-xl bg-emerald-50 border border-emerald-200/60 dark:bg-emerald-500/5 dark:border-emerald-500/10">
                    <Monitor :size="18" class="text-emerald-400 mt-0.5 shrink-0" />
                    <p class="text-sm leading-relaxed text-slate-700 dark:text-slate-300">
                        {{ t('guide.clientDesc') }}
                    </p>
                </div>
                <div
                    class="flex items-start gap-3 p-3 rounded-xl bg-violet-50 border border-violet-200/60 dark:bg-violet-500/5 dark:border-violet-500/10">
                    <Server :size="18" class="text-violet-400 mt-0.5 shrink-0" />
                    <p class="text-sm leading-relaxed text-slate-700 dark:text-slate-300">
                        {{ t('guide.serverDesc') }}
                    </p>
                </div>
            </div>
        </n-card>

        <!-- 快速上手 -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-amber-500/10 border border-amber-500/20">
                        <Rocket :size="16" class="text-amber-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.quickStart') }}</span>
                </div>
            </template>
            <div class="flex flex-col gap-4">
                <!-- Step 1 -->
                <div class="flex gap-3 items-start">
                    <div
                        class="w-7 h-7 rounded-full bg-emerald-500/15 border border-emerald-500/20 flex items-center justify-center shrink-0 mt-0.5">
                        <span class="text-xs font-bold text-emerald-400">1</span>
                    </div>
                    <div>
                        <p class="text-sm font-semibold text-slate-700 dark:text-slate-200">
                            {{ t('guide.step1Title') }}
                        </p>
                        <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400 mt-1">
                            {{ t('guide.step1Desc') }}
                        </p>
                    </div>
                </div>
                <!-- Step 2 -->
                <div class="flex gap-3 items-start">
                    <div
                        class="w-7 h-7 rounded-full bg-sky-500/15 border border-sky-500/20 flex items-center justify-center shrink-0 mt-0.5">
                        <span class="text-xs font-bold text-sky-400">2</span>
                    </div>
                    <div>
                        <p class="text-sm font-semibold text-slate-700 dark:text-slate-200">
                            {{ t('guide.step2Title') }}
                        </p>
                        <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400 mt-1">
                            {{ t('guide.step2Desc') }}
                        </p>
                    </div>
                </div>
                <!-- Step 3 -->
                <div class="flex gap-3 items-start">
                    <div
                        class="w-7 h-7 rounded-full bg-amber-500/15 border border-amber-500/20 flex items-center justify-center shrink-0 mt-0.5">
                        <span class="text-xs font-bold text-amber-400">3</span>
                    </div>
                    <div>
                        <p class="text-sm font-semibold text-slate-700 dark:text-slate-200">
                            {{ t('guide.step3Title') }}
                        </p>
                        <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400 mt-1">
                            {{ t('guide.step3Desc') }}
                        </p>
                    </div>
                </div>
            </div>
        </n-card>

        <!-- 常见使用场景 -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-pink-500/10 border border-pink-500/20">
                        <Lightbulb :size="16" class="text-pink-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.scenarios') }}</span>
                </div>
            </template>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div
                    class="flex items-center gap-2.5 p-3 rounded-xl bg-slate-50 dark:bg-white/3 border border-slate-200/60 dark:border-white/6 transition-colors hover:border-emerald-500/30">
                    <ScreenShare :size="15" class="text-emerald-400 shrink-0" />
                    <span class="text-xs text-slate-700 dark:text-slate-300">{{ t('guide.scenario1') }}</span>
                </div>
                <div
                    class="flex items-center gap-2.5 p-3 rounded-xl bg-slate-50 dark:bg-white/3 border border-slate-200/60 dark:border-white/6 transition-colors hover:border-sky-500/30">
                    <Globe :size="15" class="text-sky-400 shrink-0" />
                    <span class="text-xs text-slate-700 dark:text-slate-300">{{ t('guide.scenario2') }}</span>
                </div>
                <div
                    class="flex items-center gap-2.5 p-3 rounded-xl bg-slate-50 dark:bg-white/3 border border-slate-200/60 dark:border-white/6 transition-colors hover:border-violet-500/30">
                    <HardDrive :size="15" class="text-violet-400 shrink-0" />
                    <span class="text-xs text-slate-700 dark:text-slate-300">{{ t('guide.scenario3') }}</span>
                </div>
                <div
                    class="flex items-center gap-2.5 p-3 rounded-xl bg-slate-50 dark:bg-white/3 border border-slate-200/60 dark:border-white/6 transition-colors hover:border-amber-500/30">
                    <Code :size="15" class="text-amber-400 shrink-0" />
                    <span class="text-xs text-slate-700 dark:text-slate-300">{{ t('guide.scenario4') }}</span>
                </div>
                <div
                    class="flex items-center gap-2.5 p-3 rounded-xl bg-slate-50 dark:bg-white/3 border border-slate-200/60 dark:border-white/6 transition-colors hover:border-pink-500/30 sm:col-span-2">
                    <Gamepad2 :size="15" class="text-pink-400 shrink-0" />
                    <span class="text-xs text-slate-700 dark:text-slate-300">{{ t('guide.scenario5') }}</span>
                </div>
            </div>
        </n-card>

        <!-- FAQ -->
        <n-card class="glass-card" size="medium">
            <template #header>
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg flex items-center justify-center bg-cyan-500/10 border border-cyan-500/20">
                        <HelpCircle :size="16" class="text-cyan-400" />
                    </div>
                    <span class="text-sm font-semibold">{{ t('guide.faq') }}</span>
                </div>
            </template>
            <n-collapse arrow-placement="right" class="faq-collapse">
                <n-collapse-item :title="t('guide.faq1q')" name="1">
                    <template #arrow>
                        <ChevronRight :size="14" />
                    </template>
                    <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400">
                        {{ t('guide.faq1a') }}
                    </p>
                </n-collapse-item>
                <n-collapse-item :title="t('guide.faq2q')" name="2">
                    <template #arrow>
                        <ChevronRight :size="14" />
                    </template>
                    <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400">
                        {{ t('guide.faq2a') }}
                    </p>
                </n-collapse-item>
                <n-collapse-item :title="t('guide.faq3q')" name="3">
                    <template #arrow>
                        <ChevronRight :size="14" />
                    </template>
                    <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400">
                        {{ t('guide.faq3a') }}
                    </p>
                </n-collapse-item>
                <n-collapse-item :title="t('guide.faq4q')" name="4">
                    <template #arrow>
                        <ChevronRight :size="14" />
                    </template>
                    <p class="text-xs leading-relaxed text-slate-800 dark:text-slate-400">
                        {{ t('guide.faq4a') }}
                    </p>
                </n-collapse-item>
            </n-collapse>
        </n-card>
    </div>
</template>

<style scoped>
@reference "@/assets/main.css";

.faq-collapse :deep(.n-collapse-item__header-main) {
    @apply text-slate-800 dark:text-slate-200 transition-colors duration-300;
    font-size: 13px;
    font-weight: 600;
}
</style>
