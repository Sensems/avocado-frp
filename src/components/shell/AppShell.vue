<script setup lang="ts">
import AppSidebar from './AppSidebar.vue'
import GlobalStatusBar from './GlobalStatusBar.vue'
</script>

<template>
  <div class="app-shell">
    <AppSidebar />
    <div class="app-shell__main">
      <GlobalStatusBar />
      <section class="app-shell__content">
        <router-view />
      </section>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex: 1;
  min-height: 0;
  width: 100%;
  background: var(--ops-bg);
  color: var(--ops-text);
  overflow: hidden;
}

.app-shell__main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.app-shell__content {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--ops-bg);
}
</style>
