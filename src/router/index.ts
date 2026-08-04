import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/overview' },
    { path: '/overview', name: 'overview', component: () => import('@/features/overview/OverviewPage.vue') },
    { path: '/client', name: 'client', component: () => import('@/features/client/ClientPage.vue') },
    { path: '/server', name: 'server', component: () => import('@/features/server/ServerPage.vue') },
    { path: '/logs', name: 'logs', component: () => import('@/features/logs/LogsPage.vue') },
    { path: '/settings', name: 'settings', component: () => import('@/features/settings/SettingsPage.vue') },
  ]
})

export default router
