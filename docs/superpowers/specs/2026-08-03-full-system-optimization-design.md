# Avocado FRP 全系统优化设计

日期：2026-08-03  
状态：已完成方案确认，待书面审阅  
目标版本：现有 Tauri 2 + Vue 3 应用的渐进式升级

## 1. 背景

Avocado FRP 当前采用 Vue 3 WebView、Tauri 2 Rust 宿主和 frpc/frps sidecar 的双进程架构。整体技术方向适合桌面 FRP 管理工具，主要问题集中在应用服务层缺失：

- 窗口关闭与托盘退出没有统一停止 sidecar。
- frps 配置通过字符串插值生成，已有配置不会回填表单。
- 配置直接覆盖写入，没有备份、原子替换和恢复机制。
- 进程状态只有布尔值，spawn 成功即显示运行中。
- `Dashboard.vue` 同时负责页面、配置转换、进程编排和用户反馈。
- 表单校验、错误恢复、frps 日志和诊断能力不足。
- 项目没有自动化测试和 PR 质量门禁。
- sidecar 发布依赖仓库内二进制，缺少完整性与 checksum 校验。
- 当前工作树中的两个 macOS frpc 二进制被删除，需要先恢复。

本设计不更换 Tauri、Vue 或 Rust，而是在现有边界上补齐可测试的应用服务、配置事务、进程监督和专业运维控制台。

## 2. 已确认的产品与工程决策

1. 采用分阶段垂直切片，不做一次性整体重写。
2. Windows 完成完整本机运行验收；macOS 和 Linux 通过 CI 构建及冒烟检查。
3. 同时提供可视化表单和原始 TOML 编辑器。
4. 两种编辑模式显式切换；源码必须经过“校验 → 查看差异 → 应用”才会保存。
5. 表单只修改已知字段，未覆盖的高级 TOML 字段必须无损保留。
6. 界面全面重做为专业运维控制台，强调状态、故障和操作反馈。
7. 集成 Tauri updater，但不包含 Windows Authenticode、macOS 代码签名或公证。
8. Tauri updater 自身所需的更新包签名密钥仍必须由 CI Secret 提供。
9. 当前两个被删除的 macOS frpc 文件先从 Git 恢复，后续再迁移到自动下载与校验。

## 3. 目标与非目标

### 3.1 目标

- 应用退出后不留下由本应用启动的 frpc/frps 进程。
- 配置写入具备语法校验、语义校验、未知字段保留、备份、原子替换和恢复能力。
- 进程状态准确表达启动、健康、降级、停止和异常退出。
- 用户能够从界面完成配置、启停、日志查看、诊断和故障恢复。
- 核心行为有自动化测试，并在 PR 合并前执行。
- Release CI 能验证所有目标平台的 sidecar 完整性并生成 updater 产物。

### 3.2 非目标

- 不更换 Tauri、Vue、Naive UI 或 ECharts。
- 不拆分为微服务，也不引入远程后端。
- 不在本阶段实现 Windows/macOS 平台代码签名。
- 不保证 macOS/Linux 的完整实机桌面交互，仅保证 CI 构建与可自动化的冒烟检查。
- 不把所有 FRP 高级配置都做成表单；高级能力通过原始 TOML 模式保留。
- 不在应用内托管或同步用户配置到云端。

## 4. 目标架构

### 4.1 分层

```text
Vue Feature UI
  │ typed actions / view state
  ▼
Pinia Application Stores + Use-case Services
  │ typed Tauri client
  ▼
Thin Tauri Commands / Typed Events
  │
  ├── ProcessSupervisor
  ├── ConfigRepository
  ├── LogService
  └── DiagnosticsService
        │
        ├── Sidecar adapter
        ├── Filesystem adapter
        └── FRP Admin API adapter
```

Vue 页面不直接拼接 TOML、不直接组合 stop/save/start，也不在多个位置散落 `invoke` 和事件名称。Rust command 只负责参数反序列化、调用服务和序列化结果。

### 4.2 Rust 模块

建议的渐进式目录：

```text
src-tauri/src/
  commands/
    config.rs
    process.rs
    diagnostics.rs
    logs.rs
  domain/
    config.rs
    error.rs
    process.rs
  services/
    config_repository.rs
    process_supervisor.rs
    diagnostics_service.rs
    log_service.rs
  adapters/
    sidecar.rs
    filesystem.rs
    frp_admin.rs
  lib.rs
  main.rs
```

旧的 `config_parser.rs` 和 `process_manager.rs` 在对应服务稳定并有测试后删除，迁移期间不保留两套对外 command。

#### ProcessSupervisor

职责：

- 持有 frpc/frps 的运行记录和 `CommandChild`。
- 在一个临界区内完成“检查状态并预占 Starting”，消除双启窗口。
- 启动前验证配置快照。
- 采集 stdout、stderr、退出码和启动时间。
- 维护状态机并发出 typed event。
- 提供幂等的 start、stop、restart 和 stop_all。
- 应用退出时等待受控停止，超时后强制终止。

每个进程的状态：

```rust
enum ProcessPhase {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Stopping,
    Crashed,
}
```

状态快照至少包含：

- `kind`: frpc 或 frps
- `phase`
- `pid`（可用时）
- `started_at`
- `uptime_seconds`
- `config_revision`
- `last_exit_code`
- `last_error`

spawn 只将状态置为 `Starting`。进程通过存活窗口后进入 `Healthy`；若进程存活但可选监控接口不可用，则进入 `Degraded`。启动窗口内退出直接进入 `Crashed`。

#### ConfigRepository

职责：

- 读取和解析 frpc/frps TOML。
- 以 `toml_edit::DocumentMut` 作为配置文档来源。
- 提取表单所需的已知字段，但保留未知表、键、注释和顺序。
- 执行语法与语义校验。
- 根据修订号应用表单 patch 或完整源码。
- 生成保存前差异。
- 创建备份并执行同目录安全替换。
- 在保存或重启失败时恢复上一版本。

代理规则编辑以“配置修订号 + 原始数组位置 + 原始名称”定位。编辑已有规则时只修改表单覆盖的键，规则内其他未知键保持不变；显式删除规则时才删除整个代理表。

#### LogService

职责：

- 将进程输出转换为统一 `LogEntry`。
- 同时推送到前端和写入文件。
- 按进程、级别和时间筛选。
- 按大小或日期轮转并执行保留策略。
- 导出日志或经过敏感信息脱敏的诊断包。

日志不得记录 token、密码或 updater 私钥。诊断导出对 `token`、`password`、`secret` 等键进行脱敏。

#### DiagnosticsService

检查项：

- 目标平台 sidecar 是否存在、可执行且版本匹配。
- frpc/frps TOML 的语法和语义。
- bind、local、remote、dashboard 端口是否被占用。
- FRP 服务地址是否可连接。
- 可选 Admin API 是否可用。
- 配置目录和日志目录是否可读写。
- 当前应用、sidecar 和 updater 版本。

每个检查返回 `pass`、`warning` 或 `fail`，并附带用户可执行的修复动作。

### 4.3 Vue 模块

建议目录：

```text
src/
  app/
    router/
    stores/
      process.ts
      config.ts
      logs.ts
  services/
    tauriClient.ts
    errorMapper.ts
  domain/
    config.ts
    process.ts
    diagnostics.ts
    errors.ts
  features/
    overview/
    client/
    server/
    logs/
    diagnostics/
    settings/
  components/
    shell/
    status/
    editor/
    feedback/
```

使用 Pinia 管理跨页面共享的进程状态、配置快照和日志筛选。表单的未保存草稿、Modal 开关和临时输入仍由组件本地管理，避免把所有 UI 状态放进全局 store。

`tauriClient.ts` 是唯一允许调用 `invoke` 和 `listen` 的模块，对 command、参数、返回值和事件 payload 提供 TypeScript 类型。

## 5. 领域契约

### 5.1 ConfigSnapshot

```ts
interface ConfigSnapshot {
  kind: "frpc" | "frps"
  raw: string
  revision: string
  known: FrpcKnownConfig | FrpsKnownConfig
  issues: ValidationIssue[]
  backupAvailable: boolean
}
```

`revision` 来自文件内容 hash 或等价的单调修订值。任何保存请求都必须带上读取时的 revision；若文件被其他进程修改，返回 `CONFIG_CONFLICT`，前端要求用户重新加载或查看差异。

### 5.2 ProcessSnapshot

```ts
type ProcessPhase =
  | "stopped"
  | "starting"
  | "healthy"
  | "degraded"
  | "stopping"
  | "crashed"

interface ProcessSnapshot {
  kind: "frpc" | "frps"
  phase: ProcessPhase
  pid?: number
  startedAt?: string
  uptimeSeconds: number
  configRevision?: string
  lastExitCode?: number
  lastError?: CommandError
}
```

### 5.3 CommandError

```ts
interface CommandError {
  code:
    | "CONFIG_INVALID"
    | "CONFIG_CONFLICT"
    | "CONFIG_IO"
    | "SIDECAR_MISSING"
    | "SIDECAR_INCOMPATIBLE"
    | "PORT_CONFLICT"
    | "PROCESS_ALREADY_RUNNING"
    | "PROCESS_NOT_RUNNING"
    | "SPAWN_FAILED"
    | "HEALTHCHECK_FAILED"
    | "STOP_TIMEOUT"
    | "PERMISSION_DENIED"
    | "NETWORK_UNREACHABLE"
    | "UPDATE_FAILED"
    | "UNKNOWN"
  message: string
  detail?: string
  recoverable: boolean
  suggestedAction?: string
}
```

Rust 使用可序列化 enum/struct 返回该契约。前端根据 `code` 选择 i18n 文案，不依赖解析 Rust 字符串。

### 5.4 事件

- `process://state-changed`：完整 `ProcessSnapshot`
- `log://entry`：单条 `LogEntry`
- `config://changed`：配置种类和新 revision
- `diagnostics://progress`：检查进度
- `updater://state-changed`：检查、下载和安装状态

事件 payload 必须有版本稳定的字段，不直接发送无法约束的 `any`。

## 6. 配置编辑与保存事务

### 6.1 表单模式

1. 页面加载 `ConfigSnapshot`。
2. 表单从 `known` 初始化，未知字段不进入表单。
3. 用户保存时提交 known-field patch 和 revision。
4. Rust 在最新 `DocumentMut` 上应用 patch。
5. 执行语法与语义校验。
6. 返回 diff 摘要；普通表单保存可直接确认，危险变化需要再次确认。
7. 使用备份和安全替换写入。
8. 返回新的 `ConfigSnapshot`。

语义校验至少包含：

- 端口必须为 1–65535。
- proxy 名称非空且唯一。
- HTTP/HTTPS 必须有有效域名。
- 非 HTTP 规则必须有 remotePort。
- 已知情况下检测重复 remotePort。
- serverAddr/bindAddr 采用可接受的主机或 IP 格式。
- 鉴权方法与所需字段匹配。

### 6.2 源码模式

1. CodeMirror 6 编辑原始 TOML。
2. 输入时只做本地语法提示，不写文件。
3. 点击“校验”调用 Rust 完整校验并返回行列位置。
4. 校验通过后显示与当前 revision 的差异。
5. 用户点击“应用”后执行保存事务。
6. revision 已变化时拒绝覆盖，并提供重新加载与差异查看。

源码模式显示完整 token 属于用户主动进入的高级操作。表单模式继续使用密码输入框；日志和诊断导出始终脱敏。

### 6.3 运行中保存

运行中保存提供：

- 仅保存：写入文件，状态条显示“新配置待重启生效”。
- 保存并重启：执行受控事务。

保存并重启流程：

1. 保留旧配置和旧 revision。
2. 停止目标进程。
3. 保存新配置。
4. 启动并等待健康窗口。
5. 成功后提交新 revision。
6. 失败时恢复旧配置并尝试恢复旧进程。
7. UI 同时展示新配置失败原因、配置回滚结果和旧进程恢复结果。

## 7. 进程生命周期

### 7.1 启动

1. 在 supervisor 锁内检查状态并预占 `Starting`。
2. 锁外读取并验证配置。
3. sidecar 不存在或版本不符时进入 `Crashed`，返回结构化错误。
4. spawn child 并启动输出事件泵。
5. 启动存活窗口内若收到 terminated，则进入 `Crashed`。
6. 存活窗口通过后进入 `Healthy`；仅监控接口失败时进入 `Degraded`。

启动请求重复到达时只能有一个进入 spawn，其余返回 `PROCESS_ALREADY_RUNNING` 或当前快照。

### 7.2 停止

- stop 是幂等操作。
- `Stopped` 再次 stop 返回当前状态，不视为异常。
- 停止时进入 `Stopping`，请求终止并等待 terminated。
- 超时后强制终止并返回 `STOP_TIMEOUT` 详情。
- 收到 terminated 后清理 child、记录退出码并进入 `Stopped` 或 `Crashed`。

### 7.3 应用退出

窗口关闭、托盘退出和系统退出必须调用同一个 shutdown coordinator：

1. 如果没有运行中进程，直接退出。
2. 如果用户选择留守后台，只隐藏窗口。
3. 如果用户选择停止并退出，调用 `stop_all`。
4. 等待两个进程完成清理。
5. 超时后强制终止并记录日志。
6. 最后由 Tauri 正常退出，不使用直接 `std::process::exit(0)` 绕过清理。

## 8. 专业运维控制台

### 8.1 视觉系统

- 使用中性深浅色表面和清晰分隔，不使用大面积渐变、发光或玻璃拟态。
- 绿色仅表示健康或成功，黄色表示降级，红色表示故障，蓝色表示选中和信息动作。
- 使用统一字号、间距、圆角和交互高度。
- 优先保证 1024×768 桌面窗口；设置合理最小窗口尺寸，并支持更大窗口的高信息密度布局。
- 所有交互元素有可见 hover、pressed 和 focus 状态。

### 8.2 应用壳

- 左侧导航：Overview、Client、Server、Logs & Diagnostics、Settings。
- 顶部全局状态条：frpc/frps 状态、待重启配置、更新状态和最近故障。
- 页面切换不丢失未保存草稿；离开前进行未保存保护。

### 8.3 Overview

- frpc/frps 状态、运行时长、配置 revision 和最近退出码。
- 流量摘要与趋势。
- 配置健康摘要。
- 最近错误和诊断警告。
- 启动、停止、保存并重启、运行诊断等常用动作。

### 8.4 Client

- 顶部固定操作栏：启动、停止、保存、保存并重启。
- “可视化配置 / TOML 源码”双模式。
- 规则表格支持搜索、协议筛选、新增、编辑和删除。
- 编辑与删除按钮常显，不依赖 hover。
- 表单错误显示在对应字段附近。

### 8.5 Server

- 与 Client 使用相同的双模式编辑和操作栏。
- 直接展示 frps 日志、最后退出码和部署状态。
- 部署脚本导出前显示目标版本、平台和 checksum 策略。

### 8.6 Logs & Diagnostics

- 日志按 frpc/frps、级别和关键字筛选。
- 支持暂停自动滚动、清屏、复制、导出。
- 清屏只清 UI 缓冲，不删除磁盘日志；删除磁盘日志需要单独确认。
- 诊断逐项展示 pass/warning/fail、技术详情和修复动作。
- 可导出脱敏诊断包。

### 8.7 Settings

- 开机自启及“启动时隐藏到托盘”。
- 主题、语言和无障碍选项。
- 日志轮转大小、保留天数。
- 配置导入、导出和恢复备份。
- 当前版本、FRP 版本、检查更新和更新进度。

### 8.8 可访问性与反馈

- 完整键盘导航和符合视觉顺序的 Tab 顺序。
- 图标按钮提供 aria-label。
- 可见焦点样式。
- 颜色不是唯一状态表达，同时显示文本与图标。
- 异步按钮执行时禁用并显示真实 loading。
- 所有错误提供下一步动作，不使用静默 catch。
- 中英文 toast、dialog、系统文件选择标题和图表图例保持一致。

## 9. 流量监控

不再无条件把端口 7400 写入用户配置。新增显式的“启用本地监控”设置：

- 默认仅绑定 127.0.0.1。
- 端口可配置，并在保存前检测冲突。
- 若 FRP 版本支持鉴权，则配置本地监控认证。
- Rust 从当前配置快照获取地址、端口和认证，而不是使用硬编码 URL。
- 监控不可用但主进程存活时显示 `Degraded`，并解释原因。
- UI 提供未启用、未启动、端口冲突、认证失败和请求超时的独立空态。

## 10. 日志与配置保留

默认策略：

- 单日志文件达到 10 MB 后轮转。
- 每个进程最多保留 7 个历史文件。
- 设置页允许用户调整，但必须设置合理上限。
- UI 内存日志使用有界缓冲，默认每个进程 1000 条。
- 配置每次成功替换前保留一个最新 `.bak`。
- 用户可在设置页恢复备份，恢复前展示差异。
- 配置引入 `schemaVersion` 仅用于本应用元数据时，不得写入 FRP 无法识别的位置；应用自身迁移元数据优先存放在独立状态文件。

## 11. 自动更新与版本

### 11.1 范围

- 集成 Tauri updater 插件。
- 设置页支持手动检查更新。
- 可配置启动后静默检查，但不自动安装。
- 展示版本、发布日期和 release notes。
- 下载时显示进度；安装前要求用户确认并处理运行中进程。

### 11.2 签名边界

Tauri updater 的产物签名与 Windows/macOS 平台代码签名是两件不同的事。本阶段：

- 必须配置 updater 公钥。
- CI 通过 `TAURI_SIGNING_PRIVATE_KEY` 和对应密码 Secret 生成更新签名。
- 不配置 Windows Authenticode。
- 不配置 Apple Developer ID 或 notarization。
- README 明确说明 SmartScreen/Gatekeeper 警告仍可能存在。

### 11.3 版本单一来源

以 `src-tauri/tauri.conf.json` 的应用版本为发布来源，通过检查脚本验证 `package.json` 和 `Cargo.toml` 一致。FRP sidecar 版本存放在单独 manifest 中，不与应用版本混用。

## 12. Sidecar 供应链

### 12.1 迁移步骤

1. 从 Git 恢复当前删除的两个 macOS frpc 文件，先恢复现有发布能力。
2. 建立 sidecar manifest，记录 FRP 版本、目标 triple、下载 URL 和 SHA256。
3. 增加准备脚本，按目标平台下载并校验 frpc/frps。
4. CI 在打包前运行准备脚本和架构检查。
5. 所有目标稳定后，再单独决定是否从 Git 历史和当前树移除大二进制。

在自动下载流程被所有目标验证前，不删除其余仓库内 sidecar。

### 12.2 支持目标

- Windows x86_64
- Linux x86_64
- macOS aarch64
- macOS x86_64

Linux aarch64 不在本阶段承诺范围。

## 13. 测试策略

### 13.1 Rust 单元测试

- 表单 patch 保留未知顶层字段、未知表、注释和规则内未知键。
- 原始 TOML 特殊字符、空值和非法语法。
- 端口、规则名、协议条件和鉴权语义校验。
- revision 冲突。
- 原子保存、备份和恢复。
- ProcessSupervisor 双启互斥。
- start、stop、restart 和 stop_all 状态转换。
- stop 幂等与退出超时。
- 错误到 `CommandError` 的映射。

### 13.2 前端单元与组件测试

使用 Vitest 和 Vue Test Utils：

- Pinia process/config/log stores。
- typed Tauri client 的 command 和 event 适配。
- 表单初始化、字段校验和 known-field patch。
- 源码校验、diff、应用和 revision 冲突。
- 运行中“仅保存 / 保存并重启”。
- 进程状态呈现和错误恢复动作。
- 未保存保护。
- i18n 持久化和所有关键反馈文案。
- 键盘操作与 aria-label。

### 13.3 集成测试

使用 fake sidecar 覆盖：

- 正常启动并持续运行。
- 启动后立即退出。
- 输出 stdout/stderr。
- 非零退出码。
- 停止无响应并触发超时。
- 并发启动请求。

集成测试不连接真实公网 FRP 服务。

### 13.4 冒烟测试

Windows 本机：

- 加载、编辑、保存和恢复配置。
- 启动、日志、停止、重启。
- 退出后无残留进程。
- 托盘留守与停止退出。
- 诊断与日志导出。
- 检查更新。

CI：

- Windows、Linux、macOS 双架构能够准备正确 sidecar 并完成打包。
- 安装包或 bundle 中 sidecar 文件存在且架构匹配。

## 14. CI 与质量门禁

新增 PR 工作流：

```text
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --check
cargo clippy -- -D warnings
cargo test
sidecar manifest validation
```

Release 工作流：

- 验证应用版本一致性。
- 准备并校验目标 sidecar。
- 构建 Windows、Linux、macOS aarch64/x86_64。
- 生成普通安装资产和 updater 资产。
- 生成 checksum 与 updater signature。
- 缺少 updater 签名 Secret 时明确失败，不发布不可更新的半成品。

## 15. 五个工作包

### 工作包 1：安全基线

范围：

- 恢复两个 macOS frpc 文件。
- 建立前后端测试框架。
- 统一退出路径并实现 stop_all。
- 修复语言 storage key、customDomains 回填和现有错误文案。
- 为当前行为建立最小回归测试。

验收：

- Windows 启动 frpc/frps 后从窗口和托盘退出均无残留进程。
- 基线测试和现有前端构建通过。
- macOS sidecar 文件恢复，Git diff 不再显示这两个删除。

### 工作包 2：进程与配置服务层

范围：

- ProcessSupervisor、ConfigRepository 和 structured error。
- 配置快照、revision、unknown-field-preserving patch。
- 备份、安全替换和保存并重启回滚。
- typed Tauri commands/events 和前端 client。

验收：

- fake sidecar 覆盖正常、秒退、并发、超时。
- 特殊字符和高级字段配置可保存后重新读取且不丢失。
- 启动失败会回滚配置并报告恢复结果。

### 工作包 3：专业运维控制台

范围：

- 新应用壳、导航、Overview、Client、Server。
- Pinia application stores。
- 表单与 CodeMirror TOML 双模式。
- 状态机 UI、可访问性、完整 i18n。

验收：

- 1024×768 下无关键内容遮挡或横向溢出。
- 仅键盘可完成核心配置和启停流程。
- 表单/源码切换不会隐式覆盖配置。
- frps 页面可见实时状态和日志。

### 工作包 4：日志与诊断

范围：

- LogService、轮转、筛选和脱敏导出。
- DiagnosticsService 与诊断页面。
- 动态本地监控配置和流量错误状态。

验收：

- 日志达到阈值后按策略轮转。
- 诊断包不包含明文 token/password。
- 端口冲突、配置错误、sidecar 缺失均有明确检查结果和修复动作。

### 工作包 5：CI、sidecar 与 updater

范围：

- PR 质量门禁。
- sidecar manifest、下载、SHA256 和目标检查。
- 版本一致性检查。
- Tauri updater 和 Release 产物。

验收：

- PR 工作流完整通过。
- 四个目标构建成功并包含正确 sidecar。
- Windows 可检查、下载并确认安装一个签名 updater 测试版本。
- 未配置 updater signing Secret 时 Release 明确失败。

## 16. 迁移与兼容性

- 首次升级不主动重写现有配置。
- 加载现有配置时生成 snapshot，但只有用户保存或应用源码时才写文件。
- 表单保存必须保留未知字段和注释。
- 应用自身设置存储与 FRP 配置分离。
- 新日志策略只影响后续写入，不自动删除当前日志；首次启用时向用户说明保留策略。
- updater 集成前发布的版本继续允许手动升级。
- 旧 localStorage 语言 key 在一个兼容版本内迁移到统一 key，然后删除旧 key。

## 17. 风险与缓解

### TOML 无损修改

风险：数组表、注释和 dotted key 在 patch 时可能被重排或丢失。  
缓解：以 `DocumentMut` 做最小路径修改，为真实复杂配置建立 golden tests。

### 跨平台进程终止

风险：Windows、Linux、macOS 的信号和 child 行为不同。  
缓解：把 sidecar 操作封装为 adapter；Windows 完整实测，其余平台在 CI 使用 fake sidecar 和打包冒烟。

### UI 全面重做

风险：视觉重做与领域重构同时发生会扩大回归范围。  
缓解：在工作包 2 稳定 typed API 后再替换页面；旧页面只在新主流程通过后删除。

### 自动更新但无平台签名

风险：更新包可被 updater 验签，但安装程序仍可能触发系统安全警告。  
缓解：文档明确边界；未来将平台签名单独作为发布安全项目。

### Sidecar 下载依赖外部网络

风险：GitHub Release 不可用会阻塞构建。  
缓解：固定版本与 checksum，允许受控镜像或 CI 缓存；下载失败不得回退到未校验文件。

## 18. 完成定义

只有同时满足以下条件，才视为本次全系统优化完成：

1. 五个工作包的验收项全部通过。
2. Windows 本机核心操作冒烟通过。
3. 前后端自动化测试、lint、typecheck、clippy 和构建通过。
4. 四个发布目标成功生成包含正确 sidecar 的产物。
5. 应用退出不留下由本应用启动的 sidecar。
6. 现有高级 TOML 配置通过表单往返后未知字段不丢失。
7. 配置保存失败或新配置启动失败时能够恢复并给出明确结果。
8. 核心操作支持键盘和中英文界面。
9. 日志与诊断导出不泄露已知敏感字段。
10. updater 测试版本能够完成检查、下载和安装确认流程。
