<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { LayoutDashboard, Monitor, Server, ScrollText, Settings } from 'lucide-vue-next'

const { t } = useI18n()

const navItems = [
  { to: '/overview', name: 'overview', labelKey: 'nav.overview', icon: LayoutDashboard },
  { to: '/client', name: 'client', labelKey: 'nav.client', icon: Monitor },
  { to: '/server', name: 'server', labelKey: 'nav.server', icon: Server },
  { to: '/logs', name: 'logs', labelKey: 'nav.logs', icon: ScrollText },
  { to: '/settings', name: 'settings', labelKey: 'nav.settings', icon: Settings },
] as const
</script>

<template>
  <aside class="app-sidebar" aria-label="Main navigation">
    <nav class="app-sidebar__nav">
      <RouterLink
        v-for="item in navItems"
        :key="item.name"
        :to="item.to"
        class="app-sidebar__link"
        active-class="app-sidebar__link--active"
      >
        <component :is="item.icon" :size="16" aria-hidden="true" />
        <span>{{ t(item.labelKey) }}</span>
      </RouterLink>
    </nav>
  </aside>
</template>

<style scoped>
.app-sidebar {
  width: var(--ops-sidebar-width);
  flex-shrink: 0;
  border-right: 1px solid var(--ops-border);
  background: var(--ops-surface);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.app-sidebar__nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 12px;
}

.app-sidebar__link {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: var(--ops-control-height);
  padding: 0 12px;
  border-radius: var(--ops-radius);
  color: var(--ops-muted);
  text-decoration: none;
  font-size: 13px;
  font-weight: 500;
  transition: background-color 0.15s ease, color 0.15s ease;
}

.app-sidebar__link:hover {
  background: color-mix(in srgb, var(--ops-accent) 8%, transparent);
  color: var(--ops-text);
}

.app-sidebar__link--active {
  background: color-mix(in srgb, var(--ops-accent) 14%, transparent);
  color: var(--ops-accent);
}

.app-sidebar__link:focus-visible {
  outline: 2px solid var(--ops-accent);
  outline-offset: 2px;
}
</style>
