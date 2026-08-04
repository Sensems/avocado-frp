# 专业运维控制台 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用专业运维控制台替换单页 Dashboard：左侧导航、顶部状态条、Overview/Client/Server 主流程、Pinia stores，以及表单与 CodeMirror TOML 双模式（显式 Apply），同时保持 WP2 typed Tauri API 不变。

**Architecture:** Vue Router 承载五个页面；Pinia 只保存跨页共享的 process/config/logs 状态；草稿与 Modal 留在页面本地。所有 `invoke`/`listen` 继续只经 `src/services/tauriClient.ts`。Client/Server 共用双模式编辑器组件：表单走 known-field patch，源码走 validate → preview → Apply。

**Tech Stack:** Vue 3.5、TypeScript 5.6、Vite 6、Vue Router 5、Pinia 3、Naive UI、Tailwind 4、CodeMirror 6（`@codemirror/lang-toml` + `vue-codemirror` 或等效）、lucide-vue-next、现有 `tauriClient` / domain 契约。

## Global Constraints

- 完整运行验收在 Windows 本机进行；目标窗口优先 `1024×768`，允许更大窗口的高信息密度布局。
- 不修改、删除或暂存 `.codegraph/`、`.cursor/` 及其他与工作包无关的未跟踪文件。
- 不创建 Git commit，除非用户在执行阶段明确授权；每个任务只提供可审阅 checkpoint 和建议提交信息。
- **本工作包不新增单元测试或组件测试**；验证以 `pnpm typecheck`、`pnpm build` 和手动验收清单为准。现有 WP1 测试文件可保留，但不得作为门禁阻塞。
- 不改动 Rust `ConfigRepository` / `ProcessSupervisor` 契约，除非发现前端无法绕过的 bug；发现时先报告再改。
- `tauriClient.ts` 仍是唯一允许直接 `invoke`/`listen` 的模块。
- 视觉方向遵循设计规范 §8.1：中性深浅色表面、清晰分隔；**禁止**大面积渐变、发光、玻璃拟态；绿色=健康/成功，黄色=降级，红色=故障，蓝色=选中/信息。
- 表单模式与源码模式切换**不得隐式覆盖**配置；未 Apply 的源码草稿只存在本地；离开脏页面前必须确认。
- Logs & Diagnostics 完整 LogService/DiagnosticsService 属于 WP4；本包只做 Logs/Settings **可用壳**（筛选、清 UI 缓冲、导出、主题/语言/自启、恢复备份入口）。
- Updater、sidecar manifest、CI 属于 WP5；Settings 可显示当前应用版本占位，不实现检查更新。
- 删除旧 Dashboard 主流程前，新 Overview/Client/Server 必须可完成配置与启停。
- 所有新增中英文文案必须同时添加；图标按钮必须有 `aria-label`。
- 代理规则的编辑/删除按钮常显，不依赖 hover。
- 异步动作按钮执行时禁用并显示真实 loading；错误经 `errorMapper` 展示并给出下一步。

---

## File Structure

```text
src/
  main.ts                          # createPinia()
  App.vue                          # 缩为 Provider + 窗口控件；壳移到 AppShell
  router/index.ts                  # Overview/Client/Server/Logs/Settings 路由
  assets/
    ops-tokens.css                 # 运维控制台 CSS 变量与基础表面
    main.css                       # 引入 tokens；移除 glass/glow 依赖
  stores/
    process.ts                     # ProcessSnapshot + start/stop/restart
    config.ts                      # frpc/frps ConfigSnapshot + apply/save-restart
    logs.ts                        # log://entry 缓冲与筛选
  features/
    overview/OverviewPage.vue
    client/ClientPage.vue
    server/ServerPage.vue
    logs/LogsPage.vue
    settings/SettingsPage.vue
  components/
    shell/
      AppShell.vue                 # 左侧导航 + 顶栏 + 内容区
      AppSidebar.vue
      GlobalStatusBar.vue          # frpc/frps phase、待重启、最近故障
    status/
      ProcessPhaseBadge.vue
      PendingRestartBanner.vue
    editor/
      ConfigModeToggle.vue
      TomlSourceEditor.vue         # CodeMirror 6
      ConfigOpsBar.vue             # Start/Stop/Save/Save&Restart
      SourceApplyPanel.vue         # Validate / Preview / Apply
    feedback/
      UnsavedGuardDialog.ts       # beforeRouteLeave / beforeunload helper
  views/
    Dashboard.vue                  # 迁移完成后删除或改为 redirect
  composables/
    useProcessStatus.ts            # 薄封装转调 Pinia，或删除后全改 stores
    useAppLogs.ts                  # 同上
    useTheme.ts                    # 可保留
  services/
    tauriClient.ts                 # 只消费，不散落 invoke
    errorMapper.ts
  domain/
    config.ts / process.ts / errors.ts / proxyRule.ts
  i18n.ts
README.md
```

---

### Task 0: 接入 Pinia、路由壳与视觉 tokens

**Files:**
- Modify: `src/main.ts`
- Modify: `src/router/index.ts`
- Create: `src/assets/ops-tokens.css`
- Modify: `src/assets/main.css`
- Create: `src/components/shell/AppShell.vue`
- Create: `src/components/shell/AppSidebar.vue`
- Create: `src/components/shell/GlobalStatusBar.vue`
- Modify: `src/App.vue`
- Create: placeholder pages under `src/features/*/`
- Modify: `src/i18n.ts`

**Interfaces:**
- Produces: `/overview` `/client` `/server` `/logs` `/settings` 路由；壳布局可导航。
- Consumes: 现有主题/语言控件逻辑。

- [ ] **Step 1: 注册 Pinia**

```ts
// src/main.ts
import { createPinia } from 'pinia'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.use(naive)
app.mount('#app')
```

- [ ] **Step 2: 定义运维 tokens**

在 `ops-tokens.css` 锁定变量（示例值可微调，但语义不可变）：

```css
:root {
  --ops-bg: #f4f6f8;
  --ops-surface: #ffffff;
  --ops-border: #d7dee7;
  --ops-text: #142033;
  --ops-muted: #5b6b7c;
  --ops-accent: #2563eb;
  --ops-ok: #15803d;
  --ops-warn: #a16207;
  --ops-danger: #b91c1c;
  --ops-radius: 8px;
  --ops-gap: 16px;
  --ops-sidebar-width: 220px;
  --ops-topbar-height: 48px;
  --ops-control-height: 36px;
}

.dark {
  --ops-bg: #0f1419;
  --ops-surface: #171d25;
  --ops-border: #2a3441;
  --ops-text: #e7edf5;
  --ops-muted: #9aa8b5;
}
```

从 `main.css` 移除或停用 `.glass-card`、大面积渐变背景、状态灯发光动画的默认使用。

- [ ] **Step 3: 建立路由与占位页**

```ts
// router/index.ts
routes: [
  { path: '/', redirect: '/overview' },
  { path: '/overview', name: 'overview', component: () => import('@/features/overview/OverviewPage.vue') },
  { path: '/client', name: 'client', component: () => import('@/features/client/ClientPage.vue') },
  { path: '/server', name: 'server', component: () => import('@/features/server/ServerPage.vue') },
  { path: '/logs', name: 'logs', component: () => import('@/features/logs/LogsPage.vue') },
  { path: '/settings', name: 'settings', component: () => import('@/features/settings/SettingsPage.vue') },
]
```

每个占位页先渲染页面标题即可。

- [ ] **Step 4: 实现 AppShell**

结构：

```text
┌──────── sidebar ────────┬──────── top status bar ────────┐
│ Overview                │ frpc phase | frps phase | fault │
│ Client                  ├────────────────────────────────┤
│ Server                  │                                │
│ Logs                    │         <router-view />          │
│ Settings                │                                │
└─────────────────────────┴────────────────────────────────┘
```

- 侧栏使用 `RouterLink`，当前路由有清晰选中态（蓝色信息色，不用紫色）。
- 顶栏暂用静态 “—” 占位 process phase，Task 1 接入 store 后替换。
- `App.vue` 保留无边框窗口控件与 Provider；主内容改为 `<AppShell />`。

- [ ] **Step 5: 补齐导航 i18n**

中英文：`nav.overview`、`nav.client`、`nav.server`、`nav.logs`、`nav.settings`。

- [ ] **Step 6: 验证壳可导航**

Run:

```powershell
pnpm typecheck
pnpm build
```

Expected: PASS。手动 `pnpm tauri dev` 确认五页可切换，1024×768 下无横向溢出。

Suggested commit message if authorized:

```text
feat: scaffold ops console shell and routes
```

---

### Task 1: Pinia process/config/logs stores

**Files:**
- Create: `src/stores/process.ts`
- Create: `src/stores/config.ts`
- Create: `src/stores/logs.ts`
- Modify: `src/composables/useProcessStatus.ts`（改为 store 适配或删除调用点）
- Modify: `src/composables/useAppLogs.ts`
- Modify: `src/components/shell/GlobalStatusBar.vue`
- Modify: `src/App.vue`

**Interfaces:**
- Produces:
  - `useProcessStore()`：`frpc`/`frps` snapshots、`init`、`start`/`stop`/`restart`、`pendingRestart`
  - `useConfigStore()`：snapshots、`load`、`applyPatch`、`applySource`、`saveAndRestart`、`restoreBackup`
  - `useLogsStore()`：entries、filters、`clearUiBuffer`、`init`
- Consumes: `tauriClient` only。

- [ ] **Step 1: 实现 process store**

```ts
// stores/process.ts — 关键形状
export const useProcessStore = defineStore('process', () => {
  const frpc = ref<ProcessSnapshot | null>(null)
  const frps = ref<ProcessSnapshot | null>(null)
  const lastFault = ref<CommandError | null>(null)
  const pendingRestart = ref<{ frpc: boolean; frps: boolean }>({
    frpc: false,
    frps: false,
  })

  async function init() { /* get both snapshots + onProcessStateChanged */ }
  async function start(kind: ProcessKind) { /* loading + errorMapper */ }
  async function stop(kind: ProcessKind) { /* ... */ }
  async function restart(kind: ProcessKind) { /* ... */ }

  const frpcRunning = computed(() =>
    ['starting', 'healthy', 'degraded', 'stopping'].includes(frpc.value?.phase ?? ''),
  )
  // frpsRunning 同理

  return { frpc, frps, lastFault, pendingRestart, frpcRunning, frpsRunning, init, start, stop, restart }
})
```

- [ ] **Step 2: 实现 config store**

要求：

- `loadAll()` / `load(kind)`
- 订阅 `onConfigChanged` 刷新对应 snapshot
- `applyChange(request)` 成功后更新 snapshot，并在进程处于运行态时把 `pendingRestart[kind]=true`
- `saveAndRestart(request)` 成功后清除对应 pending；失败用 `errorMapper` + recovery 详情展示所需字段原样返回给调用方
- 不在 store 内保存表单草稿

- [ ] **Step 3: 实现 logs store**

- 监听 `log://entry`
- 每进程默认最多 1000 条 UI 缓冲（设计默认）；超出丢弃最旧
- 筛选字段：`source`、`level`（由 type out/err 映射）、`query`
- `clearUiBuffer()` 只清内存，不删磁盘
- `pauseScroll` 标志供 Logs 页使用

- [ ] **Step 4: 接线全局状态条与启动初始化**

`App.vue` / `AppShell` 在 mounted 时：

```ts
await Promise.allSettled([
  processStore.init(),
  configStore.loadAll(),
  logsStore.init(),
])
```

`GlobalStatusBar` 显示：

- frpc/frps `ProcessPhaseBadge`
- 若 `pendingRestart.*` 显示“新配置待重启”
- 若 `lastFault` 显示简短故障文案 + 跳转 Logs

- [ ] **Step 5: 迁移旧 composable 调用点**

全局搜索 `useProcessStatus` / `useAppLogs`，改为 store；若保留 composable，只能薄封装 store，不得再直接 listen。

- [ ] **Step 6: 验证**

Run:

```powershell
pnpm typecheck
pnpm build
rg -n "from '@tauri-apps/api/(core|event)'|\binvoke\(|\blisten\(" src
```

Expected: 第一条只命中 `src/services/tauriClient.ts`；typecheck/build PASS。

Suggested commit message if authorized:

```text
feat: add pinia stores for process config and logs
```

---

### Task 2: Overview 页面

**Files:**
- Create: `src/features/overview/OverviewPage.vue`
- Create: `src/components/status/ProcessPhaseBadge.vue`
- Create: `src/components/status/PendingRestartBanner.vue`
- Modify: `src/components/TrafficChart.vue`（可选嵌入 Overview）
- Modify: `src/i18n.ts`

**Interfaces:**
- Consumes: process/config/logs stores。
- Produces: 一屏运维摘要与常用动作。

- [ ] **Step 1: ProcessPhaseBadge**

输入 `ProcessSnapshot`，输出带图标+文本的状态：Stopped / Starting / Healthy / Degraded / Stopping / Crashed。颜色只用 ops tokens；不可仅靠颜色区分。

- [ ] **Step 2: Overview 卡片布局**

一屏包含：

1. frpc 卡片：phase、uptime、configRevision、lastExitCode、lastError
2. frps 卡片：同上
3. 配置健康：`issues` 中 error/warning 计数
4. 最近日志/故障：logs store 最新 5 条 err
5. 流量摘要：复用 `TrafficChart`（监控未启用时显示明确空态，不静默失败）
6. 动作：Start/Stop frpc、Start/Stop frps；若 pendingRestart 显示 Save&Restart 入口（跳 Client/Server 或直接调 store）

- [ ] **Step 3: 可访问动作**

所有图标按钮 `aria-label`；loading 时禁用对应按钮。

- [ ] **Step 4: 验证**

```powershell
pnpm typecheck
pnpm build
```

手动：启动后 Overview 状态随 `process://state-changed` 更新。

Suggested commit message if authorized:

```text
feat: add operations overview page
```

---

### Task 3: 共享 ConfigOpsBar 与未保存保护

**Files:**
- Create: `src/components/editor/ConfigOpsBar.vue`
- Create: `src/components/editor/ConfigModeToggle.vue`
- Create: `src/components/feedback/unsavedGuard.ts`
- Modify: `src/router/index.ts`（如需全局 beforeEach 辅助）

**Interfaces:**
- `ConfigOpsBar` props: `kind`, `mode`, `dirty`, `busy`
- emits / callbacks: `start`, `stop`, `save`, `saveAndRestart`
- `unsavedGuard(isDirty)` 供页面 `onBeforeRouteLeave` 使用

- [ ] **Step 1: ConfigOpsBar**

顶部固定操作栏（Client/Server 共用）：

- Start / Stop（绑定 process store）
- Save（仅保存）
- Save & Restart（调 `configStore.saveAndRestart`）
- 右侧显示当前 revision 短哈希与 `PendingRestartBanner`

按钮在 `busy` 时 disabled + loading。

- [ ] **Step 2: ConfigModeToggle**

值为 `'form' | 'source'`。切换时：

- form → source：用当前 `snapshot.raw` 初始化编辑器草稿
- source → form：若源码草稿 dirty，弹出确认“丢弃未应用的源码更改？”；确认后丢弃草稿回到 form，**绝不**自动 Apply

- [ ] **Step 3: unsavedGuard**

```ts
export async function confirmDiscardIfNeeded(
  dirty: boolean,
  dialog: { warning: Function },
  t: Composer['t'],
): Promise<boolean> {
  if (!dirty) return true
  // naive discrete dialog; return true only if user confirms discard
}
```

页面 `onBeforeRouteLeave` 调用它。

- [ ] **Step 4: 验证**

```powershell
pnpm typecheck
```

Suggested commit message if authorized:

```text
feat: add shared config ops bar and unsaved guard
```

---

### Task 4: Client 页面（表单模式完整迁移）

**Files:**
- Create: `src/features/client/ClientPage.vue`
- Move/adapt: `FrpcConfigForm.vue`、`ProtocolForm.vue` 到 feature 或继续从 components 引用
- Modify: `src/domain/config.ts`（如需补 patch helpers）
- Modify: `src/i18n.ts`
- Delete or redirect: `src/views/Dashboard.vue` 中的 frpc 逻辑（最终 Task 7 删除）

**Interfaces:**
- Consumes: `configStore` frpc snapshot、`processStore`、既存表单组件与 patch builders。
- Produces: 可完成全局配置、规则 CRUD、启停、仅保存。

- [ ] **Step 1: ClientPage 骨架**

```vue
<ConfigOpsBar kind="frpc" ... />
<ConfigModeToggle v-model="mode" />
<section v-if="mode === 'form'">
  <FrpcConfigForm :initial-data="snapshot?.known" @save="onSaveGlobal" />
  <!-- proxy table -->
</section>
<section v-else>
  <!-- Task 5 接入 TomlSourceEditor -->
</section>
```

- [ ] **Step 2: 规则表**

从 Dashboard 迁移动作：

- 搜索框 + 协议筛选
- 新增 / 编辑（ProtocolForm modal）/ 删除
- 编辑与删除按钮**常显**
- 保存走 `buildProxy*Patch` + `configStore.applyChange`
- 冲突时提示重新加载

- [ ] **Step 3: 全局保存与 Save&Restart**

- Save：`applyChange` patch，成功 toast；若进程运行中置 pendingRestart
- Save&Restart：`saveAndRestart`；展示 failure/recovery（若有）

- [ ] **Step 4: dirty 跟踪**

表单本地 dirty；路由离开走 unsavedGuard。

- [ ] **Step 5: 验证**

```powershell
pnpm typecheck
pnpm build
```

手动：Client 表单改端口保存 → Overview/顶栏显示待重启 → Save&Restart 后清除。

Suggested commit message if authorized:

```text
feat: migrate client page to ops console form mode
```

---

### Task 5: CodeMirror TOML 源码模式与 Apply 同步

**Files:**
- Modify: `package.json` / `pnpm-lock.yaml`
- Create: `src/components/editor/TomlSourceEditor.vue`
- Create: `src/components/editor/SourceApplyPanel.vue`
- Modify: `src/features/client/ClientPage.vue`
- Modify: `src/features/server/ServerPage.vue`（可先留接口，Task 6 填满）
- Modify: `src/i18n.ts`

**Interfaces:**
- `TomlSourceEditor` v-model:string；只本地编辑
- `SourceApplyPanel`：Validate / Preview / Apply；调用
  - `tauriClient.validateConfigSource`
  - `tauriClient.previewConfigChange`
  - `configStore` 的 source apply / saveAndRestart

- [ ] **Step 1: 安装编辑器依赖**

```powershell
pnpm add codemirror @codemirror/lang-toml @codemirror/view @codemirror/state @codemirror/theme-one-dark vue-codemirror
```

若 `vue-codemirror` 与 Vue 3.5 不兼容，改用 `@codemirror/view` 在 `onMounted` 手工挂载 EditorView（允许，但只封装在 `TomlSourceEditor.vue`）。

- [ ] **Step 2: TomlSourceEditor**

要求：

- 等宽字体（可用现有 Fira Code）
- 跟随 isDark 切换主题
- `v-model` 双向
- 不做自动保存、不做失焦 Apply

- [ ] **Step 3: SourceApplyPanel 流程**

固定顺序：

1. **Validate** → `validateConfigSource(kind, draft)` → 列表展示 issues（含 line/column）
2. **Preview** → 仅校验通过后 `previewConfigChange({ kind, expectedRevision, change:{ mode:'source', raw: draft }})` → 显示 unified diff（注意 token 脱敏策略遵循后端返回）
3. **Apply** → 用户确认后 `applyChange` source；成功后用新 snapshot.raw 重置草稿并清 dirty
4. revision 冲突 → 展示重新加载；禁止覆盖

提供可选 “Apply & Restart”。

- [ ] **Step 4: 模式切换契约验收点**

实现并手动确认：

- 在 source 改草稿但不 Apply，切回 form → 确认丢弃后 form 仍是旧 known
- form 保存后进 source → 看到新 raw
- 绝不因切换模式而写文件

- [ ] **Step 5: 验证**

```powershell
pnpm typecheck
pnpm build
rg -n "@codemirror|codemirror" package.json src
```

Suggested commit message if authorized:

```text
feat: add CodeMirror TOML source mode with explicit apply
```

---

### Task 6: Server 页面

**Files:**
- Create: `src/features/server/ServerPage.vue`
- Reuse: `FrpsConfigForm.vue`、`TomlSourceEditor`、`ConfigOpsBar`、`SourceApplyPanel`
- Embed: logs store 过滤 `source==='frps'` 的最近日志面板
- Modify: `src/i18n.ts`

**Interfaces:**
- 与 Client 相同的双模式与操作栏。
- 额外展示 frps phase、lastExitCode、实时日志切片、部署脚本导出。

- [ ] **Step 1: Server 表单模式**

- 加载 `configStore` frps snapshot
- `FrpsConfigForm` `:initial-data="known"` + immediate watch（WP2 已要求）
- Save / Save&Restart 走 frps patch（修掉旧字符串模板路径；必须经 store）

- [ ] **Step 2: Server 源码模式**

复用 Task 5 组件，`kind='frps'`。

- [ ] **Step 3: 运行态与日志**

页面右侧或下方固定：

- `ProcessPhaseBadge` + lastExitCode
- 最近 frps 日志（来自 logs store），链接“打开 Logs 页”

- [ ] **Step 4: 部署导出**

保留 `exportDeployScript`；导出前 dialog 说明目标示例版本（当前仓库 FRP `0.61.1`）与“checksum 策略将在 WP5 完善”，不阻塞导出。

- [ ] **Step 5: 验证**

```powershell
pnpm typecheck
pnpm build
```

手动验收（WP3 正式项）：frps 页可见实时状态和日志；表单/源码切换不隐式覆盖。

Suggested commit message if authorized:

```text
feat: add server ops page with dual-mode editing
```

---

### Task 7: Logs/Settings 壳、a11y、删除旧 Dashboard

**Files:**
- Create: `src/features/logs/LogsPage.vue`
- Create: `src/features/settings/SettingsPage.vue`
- Modify: `src/i18n.ts`
- Modify: `README.md`
- Delete: `src/views/Dashboard.vue`（或改为 redirect）
- Delete/trim: `HelpGuide.vue` 若不再挂载（可移到 Settings 帮助链接）
- Modify: `src/router/index.ts` 确保无残留 Dashboard 路由
- Modify: window min size in `src-tauri/tauri.conf.json` 若需要（建议 `minWidth: 1024`, `minHeight: 768`）

- [ ] **Step 1: Logs 页**

功能（WP3 最小集）：

- 筛选 source / level / keyword
- 暂停自动滚动
- 清屏（仅 UI）
- 复制可见行
- 导出（`tauriClient.exportLogs`）
- 明确文案：清屏不删磁盘日志

不做：轮转配置生效、脱敏诊断包（WP4）。

- [ ] **Step 2: Settings 页**

本包实现：

- 开机自启
- 主题 / 语言
- 恢复配置备份（`restoreConfigBackup`，恢复前 `preview` 或至少确认）
- 显示应用版本（从 `package.json` 或 tauri app 版本硬编码读取前端常量即可）

占位禁用并标注 “WP4/WP5”：

- 日志轮转大小/保留天数
- 检查更新

- [ ] **Step 3: 键盘与 a11y 巡检**

核对：

- Tab 顺序与视觉顺序一致
- 侧栏与主要按钮可键盘到达
- 图标按钮均有 aria-label
- 焦点环可见（勿 `outline: none` 无替代）
- 仅键盘完成：打开 Client → 改端口 → Save → Start

- [ ] **Step 4: 删除旧 Dashboard 主流程**

确认无路由/入口指向旧 tabs UI 后删除 `Dashboard.vue` 及相关死代码。Help 内容如需保留，改为 Settings 内折叠或外链。

- [ ] **Step 5: README 同步**

只描述已实现：

- 运维壳导航
- Overview/Client/Server
- 表单/源码双模式与显式 Apply
- 不宣称完整诊断、日志轮转、updater

- [ ] **Step 6: 全量验证**

Run:

```powershell
pnpm typecheck
pnpm build
rg -n "from '@tauri-apps/api/(core|event)'|\binvoke\(|\blisten\(" src
rg -n "Dashboard|glass-card|glow-" src
git diff --check
```

Expected:

- typecheck/build PASS
- invoke/listen 仅 `tauriClient.ts`
- 无旧 Dashboard 路由；无玻璃拟态主样式依赖

- [ ] **Step 7: Windows 手动验收清单**

1. 窗口 1024×768：无内容遮挡、无横向溢出。
2. 仅键盘完成 Client 配置保存与 frpc 启停。
3. Client/Server：表单 ↔ 源码切换不隐式写文件；Apply 才更新 revision。
4. 源码 Validate 失败显示行列；冲突拒绝覆盖。
5. Server 页显示 frps phase 与实时日志。
6. Overview 状态与顶栏随进程变化。
7. 离开脏页面前有确认。
8. 退出/托盘退出仍无 sidecar 残留（回归 WP1/WP2）。

Suggested commit series if authorized:

```text
feat: scaffold ops console shell and routes
feat: add pinia stores for process config and logs
feat: add operations overview and client/server pages
feat: add CodeMirror TOML source mode with explicit apply
docs: document the operations console
```

Do not run these commits without explicit authorization.

## Final Acceptance Checklist

- [ ] 左侧导航五页可切换；默认进入 Overview。
- [ ] 顶部状态条显示 frpc/frps phase、待重启、最近故障。
- [ ] Pinia stores 为跨页唯一过程/配置/日志共享状态；草稿不进 store。
- [ ] Client/Server 具备 Start/Stop/Save/Save&Restart。
- [ ] 表单/源码双模式；切换不隐式覆盖；Apply 显式提交。
- [ ] CodeMirror 源码 Validate/Preview/Apply 可用。
- [ ] 1024×768 无遮挡/横向溢出。
- [ ] 仅键盘可完成核心配置与启停。
- [ ] Server 页可见实时状态与日志。
- [ ] 旧 Dashboard 主流程已移除。
- [ ] `pnpm typecheck` 与 `pnpm build` 通过。
- [ ] 无新增单元测试要求；手动清单执行完毕。

## Out of Scope (explicit)

- LogService 轮转/保留策略落地与脱敏诊断包（WP4）
- DiagnosticsService 与诊断页逐项检查（WP4）
- 动态本地监控配置与端口冲突探测增强（WP4）
- Tauri updater、sidecar manifest、CI 门禁（WP5）
- 新增 Vitest / 组件单测（本包按选项 A 跳过）
