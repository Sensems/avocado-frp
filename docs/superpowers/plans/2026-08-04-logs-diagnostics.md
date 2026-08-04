# 日志与诊断 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落地 LogService（轮转/保留/磁盘删除）与 DiagnosticsService（逐项检查 + 脱敏诊断包），启用 Settings 日志策略与本地监控配置，补齐流量空态，使 Logs & Diagnostics 从 WP3 壳变为可运维能力。

**Architecture:** Rust 侧新增 `AppSettingsStore`（与 FRP TOML 分离的 `app-settings.json`）、`LogService`（替换 `LegacyFileLogSink` 的落盘/轮转）、`DiagnosticsService`（组装 sidecar/配置/端口/连通/目录检查）。前端经 `tauriClient` 调用；Logs 页扩展为 Logs | Diagnostics 双面板；Settings 启用轮转控件与本地监控开关。继续复用 WP2 脱敏与 WP3 Pinia logs store。

**Tech Stack:** Tauri 2、Rust（`serde_json`、现有 `EventSink`、可选 `zip`）、Vue 3.5、Pinia、Naive UI、现有 `tauriClient` / domain / ops tokens。

## Global Constraints

- 完整运行验收在 Windows 本机进行；macOS/Linux 仅依赖 `cargo check` / 前端构建。
- 不修改、删除或暂存 `.codegraph/`、`.cursor/` 及其他与工作包无关的未跟踪文件。
- 不创建 Git commit，除非用户在执行阶段明确授权；每个任务只提供建议提交信息。
- **本工作包不新增单元测试或组件测试**（与 WP2/WP3 选项 A 一致）；验证以 `cargo check`、`pnpm typecheck`、`pnpm build` 和手动验收清单为准。现有 WP1 测试文件可保留，不得作为门禁阻塞。
- `tauriClient.ts` 仍是唯一允许直接 `invoke`/`listen` 的前端模块。
- 应用设置与 FRP 配置分离：日志策略与本地监控偏好写入 `AppPaths.config_dir/app-settings.json`，**不得**写入 frpc/frps TOML 的未知应用元数据键。
- **日志策略锁定设计 §10：** 默认单文件 **10 MB** 后轮转；每进程最多保留 **7** 个历史文件（`frpc.log` / `frpc.log.1`…）。Settings 文案改为“大小 + 历史文件数”，**不再**使用“保留天数”。允许范围：大小 1–100 MB，历史文件数 1–30。
- 新日志策略只影响**后续写入**；首次启用或首次打开 Settings 日志区时，用 dialog/alert 向用户说明：不会自动删除当前已有日志。
- 清屏只清 UI 缓冲；删除磁盘日志必须单独确认，且确认文案明确不可恢复。
- 日志与诊断导出不得包含明文 `token` / `password` / `secret`（及同类键）；复用并必要时加强 `redact_secrets`。
- 诊断结果状态仅为 `pass` | `warning` | `fail`，且必须附带可执行的 `suggestedAction`（中英文经前端 i18n 映射或后端返回稳定 action code）。
- Updater / sidecar manifest / CI 属于 WP5；Settings「检查更新」保持禁用占位。
- 视觉与 a11y 延续 WP3 ops tokens：无玻璃拟态/发光；图标按钮 `aria-label`；异步按钮真实 loading；中英文同步。
- 不破坏 WP2 进程相位契约与 WP3 表单/源码 Apply 契约。

---

## File Structure

```text
src-tauri/src/
  domain/
    settings.rs              # AppSettings, LogPolicy, LocalMonitorPrefs
    diagnostics.rs           # DiagnosticCheckId, DiagnosticStatus, DiagnosticResult, DiagnosticsReport
  adapters/
    event_sink.rs            # Composite 继续；LegacyFileLogSink → 委托 LogService 或删除
    port_probe.rs            # 新建：本机端口占用探测（bind 试探）
  services/
    app_settings.rs          # AppSettingsStore load/save/defaults
    log_service.rs           # append + rotate + prune + delete_disk
    diagnostics_service.rs   # run_all / export_pack
  commands/
    settings.rs              # get/set app settings
    logs.rs                  # delete_disk_logs（export 可从 support 迁入或保留）
    diagnostics.rs           # run_diagnostics, export_diagnostics_pack
  lib.rs                     # register services + commands

src/
  domain/
    settings.ts
    diagnostics.ts
  services/
    tauriClient.ts           # 新 API 包装
    errorMapper.ts           # 必要时补错误码文案
  stores/
    settings.ts              # 可选：跨页共享 app settings
    diagnostics.ts           # 最近一次报告 + running
  features/
    logs/LogsPage.vue        # 增加 Diagnostics 面板/Tab；删除磁盘确认
    diagnostics/             # 可放 DiagnosticsPanel.vue 供 Logs 页嵌入
      DiagnosticsPanel.vue
    settings/SettingsPage.vue# 启用日志策略 + 本地监控；更新文案
    overview/OverviewPage.vue# 流量空态细化（未启用/未启动/冲突/认证/超时）
  components/shell/AppSidebar.vue  # 导航文案可改为 Logs & Diagnostics（路由仍 /logs）
  i18n.ts
  components/TrafficChart.vue      # 消费结构化 traffic/monitor 状态

README.md
```

---

### Task 0: AppSettingsStore（日志策略 + 本地监控偏好）

**Files:**
- Create: `src-tauri/src/domain/settings.rs`
- Create: `src-tauri/src/services/app_settings.rs`
- Create: `src-tauri/src/commands/settings.rs`
- Modify: `src-tauri/src/services/mod.rs`, `commands/mod.rs`, `lib.rs`
- Create: `src/domain/settings.ts`
- Modify: `src/services/tauriClient.ts`
- Modify: `src/i18n.ts`（首次策略说明文案可先加 key）

**Interfaces:**
- Produces:
  ```ts
  type LogPolicy = { maxFileBytes: number; maxRotatedFiles: number }
  type LocalMonitorPrefs = {
    enabled: boolean
    addr: string          // default "127.0.0.1"
    port: number          // default 7400
    // password optional; never log plaintext
  }
  type AppSettings = {
    schemaVersion: number // start at 1
    logPolicy: LogPolicy
    localMonitor: LocalMonitorPrefs
    logPolicyNoticeShown: boolean
  }
  ```
  - Rust: `get_app_settings` / `update_app_settings(patch)` → `AppSettings`
  - Defaults: `maxFileBytes = 10 * 1024 * 1024`, `maxRotatedFiles = 7`, monitor disabled, addr loopback
  - Clamp on write: bytes ∈ [1MiB, 100MiB], files ∈ [1, 30]；非 loopback `addr` → `CONFIG_INVALID` 或专用错误码并 suggestedAction
- Consumes: `AppPaths.config_dir`

- [ ] **Step 1: 定义 domain + 默认值**

`app-settings.json` 不存在时返回 defaults 并**不强制写盘**，直到用户首次保存或首次确认策略说明后写入（二选一：保存时写；计划采用「update 时写，get 可纯内存 defaults」）。

- [ ] **Step 2: AppSettingsStore**

```rust
// services/app_settings.rs — 形状
pub struct AppSettingsStore { path: PathBuf, inner: Mutex<AppSettings> }
impl AppSettingsStore {
    pub fn load_or_default(paths: &AppPaths) -> Result<Self, CommandError>;
    pub fn get(&self) -> AppSettings;
    pub fn update(&self, patch: AppSettingsPatch) -> Result<AppSettings, CommandError>;
}
```

原子写：`NamedTempFile::new_in(config_dir)` + `persist`（与 ConfigRepository 同套路）。

- [ ] **Step 3: commands + tauriClient**

```ts
getAppSettings(): Promise<AppSettings>
updateAppSettings(patch: Partial<...> | AppSettingsPatch): Promise<AppSettings>
```

- [ ] **Step 4: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
pnpm typecheck
```

Suggested commit message if authorized:

```text
feat: add app-settings store for log policy and local monitor
```

---

### Task 1: LogService 轮转与保留

**Files:**
- Create: `src-tauri/src/services/log_service.rs`
- Modify: `src-tauri/src/adapters/event_sink.rs`（`LegacyFileLogSink` 改为调用 `LogService`，或删除并由 Composite 直接持有 LogService sink）
- Modify: `src-tauri/src/lib.rs`（注入共享 `Arc<LogService>`，订阅 settings 变更或每次 append 读当前 policy）
- Modify: `src-tauri/src/services/mod.rs`

**Interfaces:**
- Produces:
  ```rust
  impl LogService {
      pub fn append(&self, entry: &LogEntry) -> Result<(), CommandError>;
      pub fn delete_disk_logs(&self, kind: Option<ProcessKind>) -> Result<(), CommandError>;
      pub fn set_policy(&self, policy: LogPolicy); // or read from AppSettingsStore each write
  }
  ```
- 文件命名：活跃 `frpc.log` / `frps.log`；轮转后 `frpc.log.1` … 升序，超出 `maxRotatedFiles` 删除最旧。
- 轮转触发：append 前若活跃文件 `len + incoming > maxFileBytes`，先 rotate 再写。
- 继续在 Composite 入口走既有 `redact_secrets`，落盘与 `log://entry` 均为脱敏后文本。
- Consumes: `AppPaths.log_dir`, `LogPolicy`

- [ ] **Step 1: 实现 append + rotate + prune**

注意 Windows 文件锁：轮转时关闭句柄后再 `rename`；不要长期占用独占锁阻碍 rotate。

- [ ] **Step 2: 接线 EventSink**

保持 `ProcessSupervisor::emit_log` → `EventSink::log_entry` 路径不变；仅替换文件落盘实现。

- [ ] **Step 3: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
```

手动（可在 Task 7 统一做）：临时把 `maxFileBytes` 调到很小（如 1MiB）灌日志，确认出现 `.1` 且历史数量 ≤ 策略。

Suggested commit message if authorized:

```text
feat: rotate and retain process log files via LogService
```

---

### Task 2: 磁盘日志删除与 Logs 页确认流

**Files:**
- Create or Modify: `src-tauri/src/commands/logs.rs`（或 `support.rs`）
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/tauriClient.ts`
- Modify: `src/features/logs/LogsPage.vue`
- Modify: `src/stores/logs.ts`（可选：删除磁盘后不强制清 UI；或提供“同时清 UI”勾选）
- Modify: `src/i18n.ts`

**Interfaces:**
- Produces: `delete_disk_logs({ kind?: 'frpc' | 'frps' | null })` — `null`/缺省表示两者
- UI：与「清屏」分离的危险按钮；Naive `dialog.warning` 确认；成功 toast；失败走 `errorMapper`
- 删除范围：活跃文件 + 已轮转历史（`frpc.log*` / `frps.log*`），不得误删其他文件

- [ ] **Step 1: Rust command**

- [ ] **Step 2: LogsPage 双按钮语义**

- Clear UI → 现有 `clearUiBuffer` + 既有 hint  
- Delete disk → 确认 → `tauriClient.deleteDiskLogs`

- [ ] **Step 3: 验证**

```powershell
pnpm typecheck
pnpm build
rg -n "from '@tauri-apps/api/(core|event)'|\\binvoke\\(|\\blisten\\(" src
```

Expected: invoke/listen 仅 `tauriClient.ts`。

Suggested commit message if authorized:

```text
feat: add confirmed disk log deletion from Logs page
```

---

### Task 3: Diagnostics domain + DiagnosticsService

**Files:**
- Create: `src-tauri/src/domain/diagnostics.rs`
- Create: `src-tauri/src/adapters/port_probe.rs`
- Create: `src-tauri/src/services/diagnostics_service.rs`
- Create: `src-tauri/src/commands/diagnostics.rs`
- Modify: `lib.rs`, mods
- Create: `src/domain/diagnostics.ts`
- Modify: `src/services/tauriClient.ts`
- Modify: `src/i18n.ts`（检查项标题与 suggestedAction 映射）

**Interfaces:**
- Produces:
  ```ts
  type DiagnosticStatus = 'pass' | 'warning' | 'fail'
  type DiagnosticResult = {
    id: string
    status: DiagnosticStatus
    titleKey?: string      // or stable id for i18n
    detail: string
    suggestedAction: string // stable code or human text; prefer code
  }
  type DiagnosticsReport = {
    startedAt: string
    finishedAt: string
    results: DiagnosticResult[]
  }
  ```
- Commands: `run_diagnostics()` → `DiagnosticsReport`  
  Optional event: `diagnostics://progress` with partial result（有则接，无也可同步一次返回）
- Check 清单（设计 §4.2，本包必须覆盖）：
  1. sidecar 存在 / 可执行 / 版本匹配（复用 `SidecarAdapter::inspect`，期望版本与仓库 FRP **0.61.1** 一致）
  2. frpc/frps TOML 语法与语义（`ConfigRepository` validate / snapshot.issues）
  3. bind / local / remote / dashboard（webServer）端口占用（`port_probe`）
  4. FRP serverAddr:serverPort 可连接（TCP connect，短超时；失败可为 warning/fail）
  5. 可选 Admin API（`HealthProbe`：NotConfigured → warning；error → fail/warn）
  6. 配置目录与日志目录可读写
  7. 应用版本 + sidecar 版本摘要（updater 版本检查标注 WP5 / skip 或 warning「未配置」）
- Consumes: `AppServices` 内已有 adapters/repos；**不要**在诊断里静默改配置。

- [ ] **Step 1: domain + port_probe**

端口占用：尝试 `TcpListener::bind((addr, port))`；若失败且进程非本应用占用则 `fail`/`warning` 并给修复动作（改端口/停止占用进程）。注意：仅探测，不保留 listener。

- [ ] **Step 2: DiagnosticsService::run_all**

并行能并行则并行（tokio），但报告顺序固定便于 UI。

- [ ] **Step 3: command + tauriClient**

```ts
runDiagnostics(): Promise<DiagnosticsReport>
```

- [ ] **Step 4: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
pnpm typecheck
```

Suggested commit message if authorized:

```text
feat: add DiagnosticsService with actionable check results
```

---

### Task 4: 脱敏诊断包导出

**Files:**
- Modify: `src-tauri/src/services/diagnostics_service.rs`
- Modify: `src-tauri/src/commands/diagnostics.rs`
- Modify: `src-tauri/Cargo.toml`（若用 zip：添加 `zip` 依赖）
- Modify: `src/services/tauriClient.ts`
- Modify: Logs/Diagnostics UI（Task 5 可接线；本任务至少后端可用）
- Strengthen: `redact_secrets` 或新增 `redact_toml_for_export(raw: &str) -> String`（按行/键名脱敏 `token`/`password`/`secret`）

**Interfaces:**
- Produces: `export_diagnostics_pack(path: string)`  
  包内容建议：
  - `report.json`（DiagnosticsReport）
  - `process-frpc.json` / `process-frps.json`（snapshot，无秘密）
  - `frpc.toml.redacted` / `frps.toml.redacted`
  - `logs/frpc.log` / `logs/frps.log` 最近片段或全量（已是落盘脱敏文本）；若复制历史 `.N` 一并纳入
  - `app-settings.json`（可导出非秘密字段；monitor password 必须脱敏或省略）
- 路径由前端 `tauri-plugin-dialog` 保存选择后传入（与 `export_logs` 一致）
- **验收硬条件：** 包内全文搜索不得出现用户配置里的明文 token/password

- [ ] **Step 1: redacted config export helper**

- [ ] **Step 2: 打包写入 path（zip 或目录；优先 zip 单文件）**

- [ ] **Step 3: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
```

Suggested commit message if authorized:

```text
feat: export redacted diagnostics packs
```

---

### Task 5: Diagnostics UI + Settings 日志策略

**Files:**
- Create: `src/features/diagnostics/DiagnosticsPanel.vue`（或等价）
- Create: `src/stores/diagnostics.ts`
- Modify: `src/features/logs/LogsPage.vue`（Tab：Logs | Diagnostics）
- Modify: `src/features/settings/SettingsPage.vue`
- Create or Modify: `src/stores/settings.ts`
- Modify: `src/components/shell/AppSidebar.vue` / i18n `nav.logs` → Logs & Diagnostics（可选）
- Modify: `src/i18n.ts`

**Interfaces:**
- Diagnostics 面板：Run 按钮、逐项列表（图标+文本状态，不只靠颜色）、detail、suggestedAction、Export pack、上次运行时间
- Settings：启用日志策略控件（MB 数字输入 + 历史文件数）；保存走 `updateAppSettings`
- 首次展示策略区：若 `!logPolicyNoticeShown`，dialog 说明「只影响后续写入」→ 确认后 `logPolicyNoticeShown=true` 并持久化
- 「检查更新」仍禁用并标 WP5

- [ ] **Step 1: diagnostics store + panel**

- [ ] **Step 2: LogsPage tabs**

- [ ] **Step 3: Settings log policy UI**

替换 disabled placeholder；更新 `logRetentionDesc` 文案为大小+文件数（中英）。

- [ ] **Step 4: 验证**

```powershell
pnpm typecheck
pnpm build
```

Suggested commit message if authorized:

```text
feat: wire diagnostics panel and log policy settings
```

---

### Task 6: 动态本地监控与流量空态

**Files:**
- Modify: `src-tauri/src/adapters/frp_admin.rs`（严格从 config snapshot +/或 AppSettings 解析 webServer；禁止硬编码 `127.0.0.1:7400`）
- Modify: `src-tauri/src/commands/support.rs` / 新 monitor 命令（可选 `get_monitor_status`）
- Modify: Client 表单或 Settings：启用本地监控时通过 **config patch** 写入 frpc `webServer`（addr/port/password），保存前 `port_probe`；冲突拒绝并提示
- Modify: `src/components/TrafficChart.vue`、`src/features/overview/OverviewPage.vue`
- Modify: `src/domain/process.ts` 或新建 monitor 类型
- Modify: `src/i18n.ts`

**Interfaces:**
- 监控状态枚举（前端展示用）：
  `disabled` | `process_stopped` | `port_conflict` | `auth_failed` | `timeout` | `ok` | `not_configured`
- Overview / TrafficChart 对每种状态独立空态文案（设计 §9）
- 进程存活但监控失败 → 保持/映射 `Degraded` 的既有逻辑，UI 解释原因
- Settings 或 Client 提供「启用本地监控」：默认 `127.0.0.1`，端口可配；非 loopback 拒绝
- **不要**在用户未启用时偷偷写入 `webServer` 端口

- [ ] **Step 1: 后端结构化 monitor/traffic 错误**

将 `get_frpc_traffic` 失败映射为稳定错误码或返回 `{ status, body? }`（若改返回形状，同步 TS domain + 所有调用点）。

- [ ] **Step 2: Settings/Client 启用监控 → patch + 可选 save**

优先 Settings 开关 + 应用到 frpc 配置的显式 Save（避免静默写盘）；若进程运行中置 `pendingRestart`。

- [ ] **Step 3: Overview 空态**

- [ ] **Step 4: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
pnpm typecheck
pnpm build
```

Suggested commit message if authorized:

```text
feat: add local monitor prefs and traffic empty states
```

---

### Task 7: README、死代码清理与 Windows 验收

**Files:**
- Modify: `README.md`
- Delete if still present and unused: `src/composables/useAppLogs.ts`, `src/components/ConsoleLogger.vue`（若 WP3 已删则跳过）
- Modify: `.superpowers/sdd/progress-wp4.md`（执行期账本，可选）

- [ ] **Step 1: README**

只描述已实现：轮转/保留、删除磁盘确认、诊断检查、脱敏诊断包、本地监控与流量空态。不宣称 updater/CI。

- [ ] **Step 2: 全量自动验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
pnpm typecheck
pnpm build
rg -n "from '@tauri-apps/api/(core|event)'|\\binvoke\\(|\\blisten\\(" src
rg -n "127\\.0\\.0\\.1:7400" src src-tauri/src
```

Expected：

- 前四项 PASS（clippy 若仅剩无关历史问题则先修本包引入的）
- invoke/listen 仅 `tauriClient.ts`
- 无硬编码监控 URL（测试夹具除外）

- [ ] **Step 3: Windows 手动验收清单**

1. Settings 将 `maxFileBytes` 临时调小 → 产生足够日志 → 出现轮转文件且历史数 ≤ 设定。  
2. 修改策略后说明：旧文件不被自动批量删除；仅后续写入按新策略。  
3. Logs：清屏不删磁盘；删除磁盘需确认；确认后文件消失。  
4. Run Diagnostics：人为制造 sidecar 缺失或坏 TOML / 端口占用 → 对应 fail/warning + 修复动作。  
5. Export diagnostics pack → 打开检查无明文 token/password。  
6. 未启用监控时 Overview 显示「未启用」；启用并重启 frpc 后流量可见；端口冲突/超时有独立空态。  
7. 退出/托盘退出仍无 `frpc.exe`/`frps.exe` 残留（回归）。

Suggested commit series if authorized:

```text
feat: add app-settings store for log policy and local monitor
feat: rotate and retain process log files via LogService
feat: add confirmed disk log deletion from Logs page
feat: add DiagnosticsService with actionable check results
feat: export redacted diagnostics packs
feat: wire diagnostics panel and log policy settings
feat: add local monitor prefs and traffic empty states
docs: document logs and diagnostics features
```

Do not run these commits without explicit authorization.

## Final Acceptance Checklist

- [ ] 日志达阈值后按策略轮转，历史文件数受控。
- [ ] Settings 可调整大小与历史文件数（有上下限）；文案不再写“保留天数”。
- [ ] 清屏 ≠ 删磁盘；删磁盘有确认。
- [ ] 诊断逐项 pass/warning/fail + 修复动作。
- [ ] 诊断包无明文 token/password/secret。
- [ ] 端口冲突、配置错误、sidecar 缺失均有明确结果。
- [ ] 本地监控可配置且默认 loopback；流量空态齐全。
- [ ] `cargo check`、`pnpm typecheck`、`pnpm build` 通过。
- [ ] 无新增单元测试要求；手动清单执行完毕。

## Out of Scope (explicit)

- Tauri updater、签名、Release Secret（WP5）
- sidecar 远程下载 / SHA256 manifest（WP5）
- PR CI 质量门禁（WP5）
- 新增 Vitest / Rust 单测（本包按选项 A 跳过）
- 改变 WP3 路由信息架构以外的整体视觉重做
- 自动删除用户升级前已存在的超大日志（仅后续写入生效）
