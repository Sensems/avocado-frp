# 进程与配置服务层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用可测试的 `ConfigRepository`、`ProcessSupervisor` 和结构化 IPC 契约替换当前的完整 TOML 覆盖写入、布尔进程状态与字符串错误，同时保持现有 Dashboard 可用。

**Architecture:** Vue 只能通过 `src/services/tauriClient.ts` 调用 Tauri；Rust command 只做参数反序列化和服务调用。配置以 `toml_edit::DocumentMut` 为唯一可修改来源，通过 revision、最小路径 patch、备份和同目录原子替换保证无损保存；进程由可注入 sidecar adapter 的状态机统一管理，保存并重启由独立事务服务协调和回滚。

**Tech Stack:** Vue 3.5、TypeScript 5.6、Vite 6、Vitest 3、Vue Test Utils、Tauri 2、Rust 2021、`toml_edit` 0.25、`tauri-plugin-shell` 2、Tokio、pnpm。

## Global Constraints

- 执行本工作包前必须恢复 `src-tauri/bin/frpc-aarch64-apple-darwin` 和 `src-tauri/bin/frpc-x86_64-apple-darwin`；当前工作树仍将它们显示为删除。
- Windows 负责完整运行验收；macOS/Linux 在本工作包只要求 Rust 测试和现有构建输入保持可用。
- 不修改、删除或暂存 `.codegraph/`、`.cursor/` 及其他与工作包无关的未跟踪文件。
- 不创建 Git commit，除非用户在执行阶段明确授权；每个任务只提供可审阅 checkpoint 和建议提交信息。
- 所有行为改动遵循 TDD：先运行目标测试并看到预期失败，再写最小实现，再运行目标测试和相关回归。
- 本工作包不引入 Pinia 页面 store、CodeMirror、运维控制台重做、完整 LogService、DiagnosticsService、CI、sidecar 下载器或 updater。
- `DocumentMut` 是表单 patch 的唯一文档来源；禁止先反序列化为普通对象再整体 stringify，也禁止调用 `DocumentMut::fmt()` 重排完整文件。
- 表单 patch 只修改显式提交的已知字段；缺失字段表示“不变”，`null` 表示“删除该已知键”，具体值表示“设置该键”。Rust 的 `Option<Option<T>>` 字段必须使用 `serde_with::rust::double_option`，不能依赖 Serde 默认 Option 行为。
- proxy 编辑必须同时携带读取时 revision、原数组位置和原名称；索引或名称不匹配时返回 `CONFIG_CONFLICT`。
- revision 使用原始 UTF-8 文件内容的 SHA-256；缺失文件按空字节计算 revision。
- 每次成功替换前保存一个同目录 `<name>.toml.bak`；备份和目标文件都通过同目录临时文件、flush、`sync_all` 和原子 persist 更新。
- 所有 command 错误返回可序列化 `CommandError`；前端只根据 `code` 选择 i18n 文案，不解析 Rust 字符串。
- `start_process` 的重复调用返回 `PROCESS_ALREADY_RUNNING`；`stop_process` 对 `Stopped` 幂等并返回当前快照；`restart_process` 对 `Stopped` 等价于 start。
- 生产默认启动存活窗口为 1 秒，停止等待上限为 5 秒；测试必须可注入更短时限。
- 只有配置了本地监控端点且探测失败时才进入 `Degraded`；没有配置监控端点但进程通过存活窗口时进入 `Healthy`。
- `tauriClient.ts` 是 `src/` 中唯一允许直接导入 `invoke` 或 `listen` 的模块；Tauri plugin 自身的高层 API 不受此限制。
- 新事件固定为 `process://state-changed`、`config://changed` 和临时 `log://entry`；日志轮转、持久化与脱敏诊断留到后续 LogService 工作包。
- 迁移完成后删除旧 config/process commands，不并行保留两套对外 IPC。
- token、password、secret 不得进入普通日志、错误 detail 或表单差异摘要；源码模式返回完整 raw 属于用户主动高级操作。

## Locked Domain Contracts

### Config commands

```text
get_config_snapshot(kind) -> ConfigSnapshot
validate_config_source(kind, raw) -> ValidationReport
preview_config_change(request) -> ConfigPreview
apply_config_change(request) -> ConfigSnapshot
restore_config_backup(kind, expectedRevision) -> ConfigSnapshot
save_config_and_restart(request) -> SaveAndRestartResult
```

`ConfigChangeRequest` 是 discriminated union：

```ts
type ConfigChangeRequest =
  | {
      kind: 'frpc'
      expectedRevision: string
      change:
        | { mode: 'patch'; patch: FrpcConfigPatch }
        | { mode: 'source'; raw: string }
    }
  | {
      kind: 'frps'
      expectedRevision: string
      change:
        | { mode: 'patch'; patch: FrpsConfigPatch }
        | { mode: 'source'; raw: string }
    }
```

### Process commands

```text
get_process_snapshot(kind) -> ProcessSnapshot
start_process(kind) -> ProcessSnapshot
stop_process(kind) -> ProcessSnapshot
restart_process(kind) -> ProcessSnapshot
stop_all_processes() -> StopAllResult
prepare_shutdown() -> StopAllResult
```

### Save-and-restart result

启动新配置失败属于“事务已执行但未应用”的结果，不丢失恢复详情：

```ts
interface SaveAndRestartResult {
  applied: boolean
  config: ConfigSnapshot
  process: ProcessSnapshot
  failure?: CommandError
  recovery?: {
    configRestored: boolean
    processRestored: boolean
    error?: CommandError
  }
}
```

## File Structure

```text
src-tauri/src/
  adapters/
    mod.rs
    event_sink.rs
    filesystem.rs
    frp_admin.rs
    sidecar.rs
  commands/
    mod.rs
    config.rs
    process.rs
    support.rs
  domain/
    mod.rs
    config.rs
    error.rs
    events.rs
    process.rs
  services/
    mod.rs
    config_repository.rs
    config_transaction.rs
    process_supervisor.rs
    shutdown_coordinator.rs
  lib.rs

src-tauri/tests/fixtures/
  complex-frpc.toml
  complex-frps.toml
  invalid-frpc.toml

src/
  domain/
    config.ts
    config.test.ts
    errors.ts
    process.ts
  services/
    errorMapper.ts
    errorMapper.test.ts
    tauriClient.ts
    tauriClient.test.ts
  composables/
    useProcessStatus.ts
    useProcessStatus.test.ts
    useAppLogs.ts
  App.vue
  views/Dashboard.vue
  components/FrpcConfigForm.vue
  components/FrpsConfigForm.vue
  components/ProtocolForm.vue
  components/TrafficChart.vue
```

`config_parser.rs` 和 `process_manager.rs` 只在迁移期间作为现有代码来源；新 command 注册并完成前后端切换后删除。

---

### Task 0: 恢复并锁定安全基线

**Files:**
- Restore: `src-tauri/bin/frpc-aarch64-apple-darwin`
- Restore: `src-tauri/bin/frpc-x86_64-apple-darwin`
- Verify only: all WP1 files

**Interfaces:**
- Consumes: 当前 WP1 未提交改动。
- Produces: 可开始 WP2 的绿色基线，不改变任何 WP1 行为。

- [ ] **Step 1: 记录当前工作树并确认已知删除**

Run:

```powershell
git status --short --branch
git diff --name-status -- "src-tauri/bin"
```

Expected: 两个 macOS frpc 文件显示为 `D`；不得把 `.codegraph/`、`.cursor/` 或其他未跟踪文件加入后续命令。

- [ ] **Step 2: 精确恢复两个 sidecar**

Run:

```powershell
git restore --source=HEAD -- "src-tauri/bin/frpc-aarch64-apple-darwin" "src-tauri/bin/frpc-x86_64-apple-darwin"
git diff --name-status -- "src-tauri/bin"
```

Expected: 第二条命令没有输出。

- [ ] **Step 3: 验证八个现有 sidecar 输入**

Run:

```powershell
$required = @(
  "src-tauri/bin/frpc-x86_64-pc-windows-msvc.exe",
  "src-tauri/bin/frps-x86_64-pc-windows-msvc.exe",
  "src-tauri/bin/frpc-x86_64-unknown-linux-gnu",
  "src-tauri/bin/frps-x86_64-unknown-linux-gnu",
  "src-tauri/bin/frpc-aarch64-apple-darwin",
  "src-tauri/bin/frps-aarch64-apple-darwin",
  "src-tauri/bin/frpc-x86_64-apple-darwin",
  "src-tauri/bin/frps-x86_64-apple-darwin"
)
$missing = $required | Where-Object { -not (Test-Path $_) }
if ($missing) { throw "Missing sidecars: $($missing -join ', ')" }
```

Expected: exit code 0。

- [ ] **Step 4: 重跑 WP1 自动化门禁**

Run:

```powershell
pnpm test:run
pnpm typecheck
pnpm build
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: 前端 9 个基线测试、Rust 3 个基线测试及所有构建/检查通过。

Suggested commit message if the user later authorizes commits:

```text
fix: restore the cross-platform safety baseline
```

---

### Task 1: 建立 Rust 与 TypeScript 领域契约

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/error.rs`
- Create: `src-tauri/src/domain/config.rs`
- Create: `src-tauri/src/domain/process.rs`
- Create: `src-tauri/src/domain/events.rs`
- Create: `src/domain/errors.ts`
- Create: `src/domain/config.ts`
- Create: `src/domain/process.ts`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `CommandError`, `ConfigSnapshot`, `ConfigChangeRequest`, `ConfigPreview`, `ProcessSnapshot`, `StopAllResult`, typed event payload。
- Consumed by: Tasks 2–9。

- [ ] **Step 1: 写 Rust 序列化失败测试**

在 `src-tauri/src/domain/error.rs` 和 `process.rs` 的 `#[cfg(test)]` 中先写：

```rust
#[test]
fn command_error_uses_stable_error_code_and_camel_case_fields() {
    let error = CommandError::new(
        ErrorCode::ConfigConflict,
        "configuration changed",
        true,
    )
    .with_detail("reload before saving");

    let json = serde_json::to_value(error).unwrap();
    assert_eq!(json["code"], "CONFIG_CONFLICT");
    assert_eq!(json["recoverable"], true);
    assert_eq!(json["detail"], "reload before saving");
}

#[test]
fn process_snapshot_serializes_frontend_contract() {
    let snapshot = ProcessSnapshot::stopped(ProcessKind::Frpc);
    let json = serde_json::to_value(snapshot).unwrap();

    assert_eq!(json["kind"], "frpc");
    assert_eq!(json["phase"], "stopped");
    assert_eq!(json["uptimeSeconds"], 0);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" domain
```

Expected: FAIL，因为 `domain` 模块和契约尚不存在。

- [ ] **Step 3: 通过 cargo add 增加运行时依赖**

Run:

```powershell
cargo add --manifest-path "src-tauri/Cargo.toml" thiserror sha2 tempfile async-trait similar serde_with
cargo add --manifest-path "src-tauri/Cargo.toml" tokio --features macros,rt-multi-thread,sync,time
```

不得手写猜测版本；使用执行时包管理器解析的最新兼容版本。

- [ ] **Step 4: 实现结构化错误**

`src-tauri/src/domain/error.rs` 至少包含：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ConfigInvalid,
    ConfigConflict,
    ConfigIo,
    SidecarMissing,
    SidecarIncompatible,
    PortConflict,
    ProcessAlreadyRunning,
    ProcessNotRunning,
    SpawnFailed,
    HealthcheckFailed,
    StopTimeout,
    PermissionDenied,
    NetworkUnreachable,
    UpdateFailed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

impl CommandError {
    pub fn new(code: ErrorCode, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            recoverable,
            suggested_action: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
```

IO error映射必须集中在本文件或 adapter 中，不允许 command 临时拼接中文错误字符串。

- [ ] **Step 5: 实现配置 discriminated unions**

`src-tauri/src/domain/config.rs` 使用 enum 保证 `kind` 与 `known/change` 类型一致：

```rust
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigKind {
    Frpc,
    Frps,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ConfigSnapshot {
    Frpc {
        raw: String,
        revision: String,
        known: FrpcKnownConfig,
        issues: Vec<ValidationIssue>,
        #[serde(rename = "backupAvailable")]
        backup_available: bool,
    },
    Frps {
        raw: String,
        revision: String,
        known: FrpsKnownConfig,
        issues: Vec<ValidationIssue>,
        #[serde(rename = "backupAvailable")]
        backup_available: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ConfigChangeRequest {
    Frpc {
        #[serde(rename = "expectedRevision")]
        expected_revision: String,
        change: FrpcChange,
    },
    Frps {
        #[serde(rename = "expectedRevision")]
        expected_revision: String,
        change: FrpsChange,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum FrpcChange {
    Patch { patch: FrpcConfigPatch },
    Source { raw: String },
}

impl ConfigSnapshot {
    pub fn kind(&self) -> ConfigKind {
        match self {
            Self::Frpc { .. } => ConfigKind::Frpc,
            Self::Frps { .. } => ConfigKind::Frps,
        }
    }

    pub fn raw(&self) -> &str {
        match self {
            Self::Frpc { raw, .. } | Self::Frps { raw, .. } => raw,
        }
    }

    pub fn revision(&self) -> &str {
        match self {
            Self::Frpc { revision, .. } | Self::Frps { revision, .. } => revision,
        }
    }
}

impl ConfigChangeRequest {
    pub fn kind(&self) -> ConfigKind {
        match self {
            Self::Frpc { .. } => ConfigKind::Frpc,
            Self::Frps { .. } => ConfigKind::Frps,
        }
    }
}
```

同时定义：

- `FrpcKnownConfig`：`serverAddr`、`serverPort`、`auth`、`webServer`、`proxies`。
- `FrpsKnownConfig`：`bindPort`、`vhostHTTPPort`、`vhostHTTPSPort`、`auth`、`webServer`。
- `ProxyRuleKnown`：表单字段以及只读 `sourceIndex`、`sourceName`。
- `ProxySelector { index, originalName }`。
- `ProxyOperation`：`add`、`update`、`delete`。
- 所有 patch struct 使用 `#[serde(rename_all = "camelCase")]`。
- 所有 patch 字段使用 `#[serde(default, with = "serde_with::rust::double_option")] Option<Option<T>>`，明确区分 missing / null / value。
- `ValidationIssue { severity, code, message, path, line, column }`。
- `ConfigDiff { unified, changedPaths, requiresConfirmation }`。

- [ ] **Step 6: 实现进程与事件契约**

`src-tauri/src/domain/process.rs` 至少包含：

```rust
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProcessKind {
    Frpc,
    Frps,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessPhase {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Stopping,
    Crashed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub kind: ProcessKind,
    pub phase: ProcessPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<CommandError>,
}

impl ProcessSnapshot {
    pub fn stopped(kind: ProcessKind) -> Self {
        Self {
            kind,
            phase: ProcessPhase::Stopped,
            pid: None,
            started_at: None,
            uptime_seconds: 0,
            config_revision: None,
            last_exit_code: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StopAllResult {
    pub frpc: ProcessSnapshot,
    pub frps: ProcessSnapshot,
    pub errors: Vec<CommandError>,
}
```

所有含 snake_case 字段的 payload struct 都使用 `#[serde(rename_all = "camelCase")]`，所有 optional 字段都使用 `skip_serializing_if = "Option::is_none"`，确保 TypeScript 的 `field?: T` 不会收到 `null`。

`events.rs` 定义 `ProcessStateChangedEvent`、`ConfigChangedEvent` 和最小 `LogEntry`；时间戳统一为 RFC 3339。实现 `From<&ConfigSnapshot> for ConfigChangedEvent`，只复制 kind 和 revision，不复制 raw/token。

- [ ] **Step 7: 创建完全同构的 TypeScript 类型**

`src/domain/errors.ts`、`config.ts`、`process.ts` 必须与 Rust JSON 字段逐项一致。核心定义：

```ts
export type ProcessPhase =
  | 'stopped'
  | 'starting'
  | 'healthy'
  | 'degraded'
  | 'stopping'
  | 'crashed'

export interface ProcessSnapshot {
  kind: 'frpc' | 'frps'
  phase: ProcessPhase
  pid?: number
  startedAt?: string
  uptimeSeconds: number
  configRevision?: string
  lastExitCode?: number
  lastError?: CommandError
}

export type PatchValue<T> = T | null | undefined
```

- [ ] **Step 8: 注册模块并验证契约**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" domain
pnpm typecheck
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: define typed process and configuration contracts
```

---

### Task 2: 提取应用路径与原子文件适配器

**Files:**
- Create: `src-tauri/src/adapters/mod.rs`
- Create: `src-tauri/src/adapters/filesystem.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/adapters/filesystem.rs`

**Interfaces:**
- Consumes: `CommandError`、Tauri `AppHandle::path()`。
- Produces: `AppPaths`、`ConfigFilesystem`、`RealConfigFilesystem`。
- Consumed by: ConfigRepository、support commands。

- [ ] **Step 1: 写原子替换和备份失败测试**

测试至少覆盖：

```rust
#[test]
fn replace_creates_latest_backup_and_new_target() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("frpc.toml");
    std::fs::write(&target, "serverPort = 7000\n").unwrap();

    let fs = RealConfigFilesystem;
    fs.replace_with_backup(&target, b"serverPort = 7001\n")
        .unwrap();

    assert_eq!(std::fs::read_to_string(&target).unwrap(), "serverPort = 7001\n");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("frpc.toml.bak")).unwrap(),
        "serverPort = 7000\n"
    );
}

#[test]
fn restore_keeps_backup_available() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("frpc.toml");
    let backup = temp.path().join("frpc.toml.bak");
    std::fs::write(&target, "serverPort = 7001\n").unwrap();
    std::fs::write(&backup, "serverPort = 7000\n").unwrap();

    let fs = RealConfigFilesystem;
    fs.restore_backup(&target).unwrap();

    assert_eq!(std::fs::read_to_string(&target).unwrap(), "serverPort = 7000\n");
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "serverPort = 7000\n");
}
```

另建一个测试 filesystem，在“写备份后、替换目标前”注入失败，断言原目标仍保持原内容。

- [ ] **Step 2: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" filesystem
```

Expected: FAIL，因为 adapter 尚不存在。

- [ ] **Step 3: 实现 AppPaths**

```rust
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn from_app(app: &tauri::AppHandle) -> Result<Self, CommandError> {
        let mut config_dir = app
            .path()
            .app_config_dir()
            .map_err(map_config_io)?;
        #[cfg(debug_assertions)]
        {
            config_dir = config_dir.join("dev_data");
        }
        let log_dir = config_dir.join("logs");
        Ok(Self { config_dir, log_dir })
    }

    pub fn config_path(&self, kind: ConfigKind) -> PathBuf {
        self.config_dir.join(match kind {
            ConfigKind::Frpc => "frpc.toml",
            ConfigKind::Frps => "frps.toml",
        })
    }
}
```

这一步必须替代 `config_parser::get_config_dir`，否则后续删除旧模块会破坏日志导出。

- [ ] **Step 4: 定义 filesystem seam**

```rust
pub trait ConfigFilesystem: Send + Sync {
    fn read_utf8(&self, path: &Path) -> Result<Option<String>, CommandError>;
    fn exists(&self, path: &Path) -> bool;
    fn replace_with_backup(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), CommandError>;
    fn restore_backup(&self, path: &Path) -> Result<(), CommandError>;
    fn atomic_replace(&self, path: &Path, bytes: &[u8]) -> Result<(), CommandError>;
    fn remove(&self, path: &Path) -> Result<(), CommandError>;
}

#[derive(Debug, Default)]
pub struct RealConfigFilesystem;
```

ConfigRepository 依赖 `Arc<dyn ConfigFilesystem>`；失败注入测试使用实现同一 trait 的 `FaultingConfigFilesystem`，不在生产代码添加测试开关。

- [ ] **Step 5: 实现同目录原子 persist**

```rust
fn atomic_persist(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    let parent = path.parent().ok_or_else(|| {
        CommandError::new(ErrorCode::ConfigIo, "config path has no parent", false)
    })?;
    std::fs::create_dir_all(parent).map_err(map_config_io)?;

    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(map_config_io)?;
    use std::io::Write;
    staged.write_all(bytes).map_err(map_config_io)?;
    staged.flush().map_err(map_config_io)?;
    staged.as_file().sync_all().map_err(map_config_io)?;
    staged.persist(path).map_err(|error| map_config_io(error.error))?;
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(map_config_io)?;
    Ok(())
}
```

同一文件定义无敏感路径泄漏的错误映射：

```rust
fn map_config_io(error: std::io::Error) -> CommandError {
    let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
        ErrorCode::PermissionDenied
    } else {
        ErrorCode::ConfigIo
    };
    CommandError::new(code, "configuration I/O failed", true)
        .with_detail(format!("{:?}", error.kind()))
}
```

`replace_with_backup` 的固定顺序：

1. 读取现有 target（不存在则记为 `None`）。
2. 若存在，先用 `atomic_persist` 更新 `.bak`。
3. 用另一个 staged file 原子替换 target。
4. 第 3 步失败时 target 仍是旧内容。

- [ ] **Step 6: 运行 adapter 测试和 Clippy**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" filesystem
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: add atomic configuration filesystem adapter
```

---

### Task 3: 实现 ConfigRepository 读取、revision 和验证

**Files:**
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/services/config_repository.rs`
- Create: `src-tauri/tests/fixtures/complex-frpc.toml`
- Create: `src-tauri/tests/fixtures/complex-frps.toml`
- Create: `src-tauri/tests/fixtures/invalid-frpc.toml`
- Test: `src-tauri/src/services/config_repository.rs`

**Interfaces:**
- Consumes: `AppPaths`、`ConfigFilesystem`、配置领域类型。
- Produces:
  - `load(kind) -> Result<ConfigSnapshot, CommandError>`
  - `load_validated(kind) -> Result<ConfigSnapshot, CommandError>`
  - `validate_source(kind, raw) -> ValidationReport`
  - `revision(raw) -> String`
- Consumed by: patch/apply、ProcessSupervisor。

- [ ] **Step 1: 创建复杂 golden fixtures**

`complex-frpc.toml` 必须包含不会出现在当前表单中的内容：

```toml
# keep this top-level comment
serverAddr = "frp.example.com"
serverPort = 7000
transport.protocol = "quic"
auth.method = "token"
auth.token = "quote\"and\\slash"

[webServer]
addr = "127.0.0.1"
port = 7400
user = "operator"
password = "do-not-log"

[[proxies]]
name = "web"
type = "http"
localIP = "127.0.0.1"
localPort = 8080
customDomains = ["app.example.com"]
healthCheck.type = "http"
healthCheck.path = "/ready"

[[proxies]]
name = "ssh"
type = "tcp"
localIP = "127.0.0.1"
localPort = 22
remotePort = 6022
transport.useEncryption = true
```

`complex-frps.toml` 至少包含注释、dotted key、未知 transport 字段和 webServer 凭据。`invalid-frpc.toml` 使用未闭合字符串并保留可断言的行列位置。

- [ ] **Step 2: 写 load/revision/known extraction 测试**

```rust
struct RepositoryFixture {
    _temp: tempfile::TempDir,
    repository: ConfigRepository,
}

fn repository_with_file(kind: ConfigKind, raw: &str) -> RepositoryFixture {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths {
        config_dir: temp.path().to_path_buf(),
        log_dir: temp.path().join("logs"),
    };
    std::fs::write(paths.config_path(kind), raw).unwrap();
    let repository =
        ConfigRepository::new(paths, std::sync::Arc::new(RealConfigFilesystem));
    RepositoryFixture {
        _temp: temp,
        repository,
    }
}

#[test]
fn load_extracts_known_fields_without_rewriting_raw() {
    let fixture = include_str!("../../tests/fixtures/complex-frpc.toml");
    let test = repository_with_file(ConfigKind::Frpc, fixture);

    let snapshot = test.repository.load(ConfigKind::Frpc).unwrap();
    let ConfigSnapshot::Frpc { raw, revision, known, .. } = snapshot else {
        panic!("expected frpc snapshot");
    };

    assert_eq!(raw, fixture);
    assert_eq!(revision.len(), 64);
    assert_eq!(known.server_addr.as_deref(), Some("frp.example.com"));
    assert_eq!(known.proxies[0].source_index, 0);
    assert_eq!(known.proxies[0].source_name, "web");
}

#[test]
fn missing_file_returns_empty_snapshot_with_stable_revision() {
    let temp = tempfile::tempdir().unwrap();
    let repository = ConfigRepository::new(
        AppPaths {
            config_dir: temp.path().to_path_buf(),
            log_dir: temp.path().join("logs"),
        },
        std::sync::Arc::new(RealConfigFilesystem),
    );

    let first = repository.load(ConfigKind::Frpc).unwrap();
    let second = repository.load(ConfigKind::Frpc).unwrap();
    assert_eq!(first.revision(), second.revision());
    assert_eq!(
        first.revision(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
```

- [ ] **Step 3: 写语义验证测试**

逐项测试以下错误 path/code：

- 端口 0、65536 或非整数。
- 空 proxy 名称和重复 proxy 名称。
- HTTP/HTTPS 缺少非空 customDomains。
- 非 HTTP proxy 缺少 remotePort。
- 已知 remotePort 重复。
- 非法 serverAddr/bindAddr。
- `auth.method = "token"` 但 token 为空。
- 未识别的高级 auth method 不得被擅自删除；只产生 warning，不阻止无损读取。

- [ ] **Step 4: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_repository
```

Expected: FAIL，因为 repository 尚不存在。

- [ ] **Step 5: 实现 raw revision 和语法位置**

```rust
fn revision(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

fn line_column(raw: &str, offset: usize) -> (usize, usize) {
    let prefix = &raw[..offset.min(raw.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map(str::len).unwrap_or(0) + 1;
    (line, column)
}
```

解析错误从 `toml_edit` error span 转换为 `ValidationIssue`。`load` 仍返回包含原始 raw、revision、默认 known 和 issues 的 snapshot，使 UI 能修复非法文件；`load_validated` 在存在 error issue 时返回 `CONFIG_INVALID`，供启动和 patch 使用。`validate_source` 返回完整 issues，不抛掉行列信息。

- [ ] **Step 6: 实现 known extraction 和语义 validator**

要求：

- 读取时只借用 `DocumentMut`，不得修改文档。
- 同时兼容 FRP 常见的 `localIP` 与旧 UI 的 `localIp`，保存时统一修改原来存在的拼写；新建规则使用 `localIP`。
- 读取 `[[proxies]]` 时保留 source index/name 元数据。
- host validator 接受 IPv4、IPv6、`localhost` 和合法 DNS label。
- validator 只校验已知语义，不因未知高级字段而拒绝配置。

- [ ] **Step 7: 运行目标测试**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_repository
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: add revisioned configuration snapshots and validation
```

---

### Task 4: 实现无损 patch、预览、冲突和备份恢复

**Files:**
- Modify: `src-tauri/src/services/config_repository.rs`
- Modify: `src-tauri/src/domain/config.rs`
- Test: `src-tauri/src/services/config_repository.rs`
- Test fixture: `src-tauri/tests/fixtures/complex-frpc.toml`
- Test fixture: `src-tauri/tests/fixtures/complex-frps.toml`

**Interfaces:**
- Produces:
  - `preview(request) -> Result<ConfigPreview, CommandError>`
  - `apply(request) -> Result<ConfigSnapshot, CommandError>`
  - `restore_backup(kind, expected_revision) -> Result<ConfigSnapshot, CommandError>`
  - `capture(kind) -> Result<ConfigFileState, CommandError>`（内部事务 API，包含 `existed/raw/revision`）
  - `restore_state(kind, candidate_revision, state) -> Result<ConfigSnapshot, CommandError>`
- Guarantees: revision CAS、unknown-field preservation、最新 `.bak`。

- [ ] **Step 1: 写 unknown-field preservation 测试**

```rust
#[test]
fn frpc_patch_preserves_comments_unknown_tables_and_rule_fields() {
    let original = include_str!("../../tests/fixtures/complex-frpc.toml");
    let test = repository_with_file(ConfigKind::Frpc, original);
    let revision = test
        .repository
        .load(ConfigKind::Frpc)
        .unwrap()
        .revision()
        .to_string();

    let request = frpc_patch_request(
        revision,
        FrpcConfigPatch {
            server_port: Some(Some(7001)),
            proxy_operations: vec![ProxyOperation::Update {
                selector: ProxySelector {
                    index: 0,
                    original_name: "web".into(),
                },
                patch: ProxyRulePatch {
                    local_port: Some(Some(8081)),
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    );

    let saved = test.repository.apply(request).unwrap().raw().to_string();
    assert!(saved.contains("# keep this top-level comment"));
    assert!(saved.contains("transport.protocol = \"quic\""));
    assert!(saved.contains("healthCheck.path = \"/ready\""));
    assert!(saved.contains("transport.useEncryption = true"));
    assert!(saved.contains("serverPort = 7001"));
    assert!(saved.contains("localPort = 8081"));
}
```

在同一测试模块定义 `frpc_patch_request`，避免每个测试手写 discriminant：

```rust
fn frpc_patch_request(
    expected_revision: String,
    patch: FrpcConfigPatch,
) -> ConfigChangeRequest {
    ConfigChangeRequest::Frpc {
        expected_revision,
        change: FrpcChange::Patch { patch },
    }
}
```

为 frps 再写一个测试，更新 `bindPort` 后断言未知 transport、注释、webServer 其他键均保留。

- [ ] **Step 2: 写 selector 与 revision 冲突测试**

覆盖：

- 外部修改文件后，用旧 revision apply → `CONFIG_CONFLICT`，文件不变。
- index 正确但 originalName 不匹配 → `CONFIG_CONFLICT`。
- 显式 delete 才删除整个 proxy table。
- update 只修改 patch 中出现的键。
- `null` 删除已知键，missing 保持不变。

- [ ] **Step 3: 写 preview 和恢复测试**

断言：

- preview 不写文件、不创建 `.bak`。
- `changedPaths` 包含 `serverPort` 或 `proxies[0].localPort`。
- auth、bind/server 地址、监听端口或规则删除使 `requiresConfirmation=true`。
- 表单摘要不包含 token/password 的旧值或新值。
- patch 模式的 unified diff 对 token/password/secret 值脱敏；source 模式可返回完整 diff，但只允许后续高级源码编辑器展示。
- apply 创建 `.bak` 并返回新 revision。
- restore 后 raw 与旧内容逐字节一致，`.bak` 仍可用。
- 原文件不存在时 capture → apply → restore_state 会删除新目标并恢复“不存在”状态。
- 原文件存在但内容为空时 restore_state 保留空文件，不能与“不存在”混淆。

- [ ] **Step 4: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" patch
cargo test --manifest-path "src-tauri/Cargo.toml" revision_conflict
cargo test --manifest-path "src-tauri/Cargo.toml" backup
```

Expected: FAIL。

- [ ] **Step 5: 实现最小 DocumentMut 路径修改**

固定规则：

```rust
fn apply_optional_value<T>(
    table: &mut toml_edit::Table,
    key: &str,
    patch: &Option<Option<T>>,
    to_item: impl Fn(&T) -> toml_edit::Item,
) {
    match patch {
        None => {}
        Some(None) => {
            table.remove(key);
        }
        Some(Some(value)) => {
            table.insert(key, to_item(value));
        }
    }
}
```

proxy 操作使用 `doc["proxies"].as_array_of_tables_mut()` 的 `get_mut/insert/push/remove`；update 不得用新 `Table` 替换旧 Table。

- [ ] **Step 6: 实现 preview/apply CAS**

`apply` 必须在每个 ConfigKind 独立锁内完成：

1. 重新读取当前 raw。
2. 比较 `expectedRevision`。
3. 在当前 `DocumentMut` 上应用 patch，或解析 source raw。
4. 执行语法与语义验证。
5. 生成 preview/diff。
6. `replace_with_backup`。
7. 重新 load 并返回新 snapshot。

ConfigRepository 本身不发事件；Task 7 的事务服务只在成功应用或完成回滚后发出 `config://changed`。

`ConfigFileState` 不通过 Tauri 暴露：

```rust
#[derive(Debug, Clone)]
pub(crate) struct ConfigFileState {
    pub existed: bool,
    pub raw: String,
    pub revision: String,
}
```

`restore_state` 先比较当前 candidate revision；旧状态 `existed=true` 时原子写回 raw，`existed=false` 时只删除与 candidate revision 匹配的新目标，防止覆盖并发外部修改。

- [ ] **Step 7: 运行 repository 全套测试**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_repository
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: preserve advanced TOML through revisioned patches
```

---

### Task 5: 实现 sidecar、事件和监控 adapters

**Files:**
- Create: `src-tauri/src/adapters/event_sink.rs`
- Create: `src-tauri/src/adapters/sidecar.rs`
- Create: `src-tauri/src/adapters/frp_admin.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Test: corresponding module tests

**Interfaces:**
- Produces: `SidecarAdapter`、`SpawnedSidecar`、`SidecarEvent`、`EventSink`、`HealthProbe`。
- Consumed by: ProcessSupervisor。

- [ ] **Step 1: 写 adapter translation 测试**

测试 Tauri `CommandEvent` 的等价内部事件映射：

```rust
assert_eq!(
    SidecarEvent::from_stdout(b"ready\n".to_vec()),
    SidecarEvent::Stdout("ready\n".into())
);
assert_eq!(
    SidecarEvent::from_stderr(vec![0xff]),
    SidecarEvent::Stderr("\u{fffd}".into())
);
```

测试 fake adapter 能脚本化：

- 持续运行。
- 立即 `Terminated(Some(1))`。
- stdout/stderr。
- 收到 stop 后发送 terminated。
- 忽略 stop，触发 timeout。

- [ ] **Step 2: 定义 object-safe adapter**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarEvent {
    Stdout(String),
    Stderr(String),
    Error(String),
    Terminated(Option<i32>),
}

pub struct SpawnedSidecar {
    pub pid: u32,
    pub child: Box<dyn SidecarChild>,
    pub events: tokio::sync::mpsc::Receiver<SidecarEvent>,
}

#[async_trait::async_trait]
pub trait SidecarChild: Send {
    async fn request_stop(self: Box<Self>) -> Result<(), CommandError>;
}

#[async_trait::async_trait]
pub trait SidecarAdapter: Send + Sync {
    async fn inspect(&self, kind: ProcessKind) -> Result<SidecarInfo, CommandError>;

    async fn spawn(
        &self,
        kind: ProcessKind,
        config_path: &std::path::Path,
    ) -> Result<SpawnedSidecar, CommandError>;

    async fn force_kill(&self, pid: u32) -> Result<(), CommandError>;
}
```

`SidecarInfo` 至少包含 kind 和 version。Tauri 实现先执行 sidecar `--version`：

- 无法解析/定位 binary → `SIDECAR_MISSING`。
- 当前兼容版本固定为仓库现有 FRP `0.61.1`；版本不同 → `SIDECAR_INCOMPATIBLE`。
- 后续工作包 5 的 manifest 落地后，用 manifest 替换该单一版本常量，不在多个文件复制版本。

spawn 包装 `app.shell().sidecar(name).args(["-c", path]).spawn()`；`CommandChild::pid()` 填入 pid，`kill()` 实现 request_stop。timeout 后使用现有 `sysinfo` 依赖按 pid 再次强制终止并确认。Fake adapter 必须覆盖 missing 和 incompatible 两种 inspect 结果。

- [ ] **Step 3: 定义 typed EventSink**

不要使用带泛型方法的 trait，以保持 object-safe：

```rust
pub trait EventSink: Send + Sync {
    fn process_changed(&self, snapshot: &ProcessSnapshot) -> Result<(), CommandError>;
    fn config_changed(&self, event: &ConfigChangedEvent) -> Result<(), CommandError>;
    fn log_entry(&self, entry: &LogEntry) -> Result<(), CommandError>;
}
```

生产实现为 composite sink：

- `TauriEventSink` 使用 `AppHandle::emit` 推送三个稳定事件。
- `LegacyFileLogSink` 只对 `log_entry` 追加现有 `logs/frpc.log` 或 `logs/frps.log`，保留当前日志导出能力。
- `CompositeEventSink` 同时调用两者；文件轮转和保留策略仍留给后续 LogService。
- `RecordingEventSink` 在测试中记录 payload。

写日志前移除明显的 `token/password/secret = ...` 片段；event/file sink 失败不得改变进程状态机，只记录内部错误并允许前端通过 snapshot command 重新同步。

- [ ] **Step 4: 移动 FRP Admin API 访问**

将 `process_manager.rs::get_frpc_traffic` 的 hardcoded HTTP 访问移入 `frp_admin.rs`：

- endpoint 从当前 `ConfigSnapshot.known.webServer` 构造。
- 只允许 loopback 监控地址。
- 有 user/password 时使用 basic auth。
- probe 超时映射 `HEALTHCHECK_FAILED`。
- 未配置 endpoint 时返回 `NotConfigured`，不把主进程降级。

- [ ] **Step 5: 运行 adapter 测试**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" adapters
```

Expected: PASS。

Suggested commit message if authorized:

```text
refactor: isolate sidecar events and FRP monitoring
```

---

### Task 6: 实现 ProcessSupervisor 状态机

**Files:**
- Create: `src-tauri/src/services/process_supervisor.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/process_supervisor.rs`

**Interfaces:**
- Consumes: `ConfigRepository`、`SidecarAdapter`、`HealthProbe`、`EventSink`。
- Produces:
  - `snapshot(kind)`
  - `start(kind)`
  - `stop(kind)`
  - `restart(kind)`
  - `stop_all()`

- [ ] **Step 1: 写 FakeSidecar 状态转换测试**

至少创建以下 async tests：

```rust
#[tokio::test]
async fn concurrent_start_spawns_once() {
    let fake = FakeSidecarAdapter::running();
    let fixture = supervisor_fixture(fake.clone());
    let supervisor = &fixture.supervisor;

    let (first, second) = tokio::join!(
        supervisor.start(ProcessKind::Frpc),
        supervisor.start(ProcessKind::Frpc),
    );

    assert_eq!(fake.spawn_count(), 1);
    assert!(first.is_ok() ^ second.is_ok());
    let error = first.err().or_else(|| second.err()).unwrap();
    assert_eq!(error.code, ErrorCode::ProcessAlreadyRunning);
}

#[tokio::test]
async fn immediate_exit_becomes_crashed() {
    let fixture = supervisor_fixture(FakeSidecarAdapter::immediate_exit(1));
    let supervisor = &fixture.supervisor;
    let error = supervisor.start(ProcessKind::Frpc).await.unwrap_err();

    assert_eq!(error.code, ErrorCode::SpawnFailed);
    let snapshot = supervisor.snapshot(ProcessKind::Frpc).await;
    assert_eq!(snapshot.phase, ProcessPhase::Crashed);
    assert_eq!(snapshot.last_exit_code, Some(1));
}
```

测试模块定义并持有临时配置目录，防止构造完 supervisor 后 `TempDir` 被提前删除：

```rust
struct SupervisorFixture {
    _temp: tempfile::TempDir,
    supervisor: ProcessSupervisor,
}

fn supervisor_fixture(sidecar: FakeSidecarAdapter) -> SupervisorFixture {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths {
        config_dir: temp.path().to_path_buf(),
        log_dir: temp.path().join("logs"),
    };
    std::fs::write(
        paths.config_path(ConfigKind::Frpc),
        "serverAddr = \"127.0.0.1\"\nserverPort = 7000\n",
    )
    .unwrap();
    std::fs::write(
        paths.config_path(ConfigKind::Frps),
        "bindPort = 7000\n",
    )
    .unwrap();
    let repository =
        ConfigRepository::new(paths, std::sync::Arc::new(RealConfigFilesystem));
    let supervisor = ProcessSupervisor::new(
        std::sync::Arc::new(repository),
        std::sync::Arc::new(sidecar),
        std::sync::Arc::new(FakeHealthProbe::healthy()),
        std::sync::Arc::new(RecordingEventSink::default()),
        SupervisorTiming {
            startup_grace: std::time::Duration::from_millis(10),
            stop_timeout: std::time::Duration::from_millis(20),
        },
    );
    SupervisorFixture {
        _temp: temp,
        supervisor,
    }
}
```

再覆盖：

- `Starting -> Healthy`。
- configured probe failure → `Degraded`。
- stdout/stderr → `log://entry`。
- stop 幂等。
- `Healthy -> Stopping -> Stopped`。
- stop timeout → force_kill + `STOP_TIMEOUT`。
- restart。
- stop_all 对两个进程均尝试，不 fail-fast。
- 旧 generation 的 terminated 事件不能清理新进程。
- stop 在 spawn 尚未安装 child 时取消该 generation；迟到 child 必须立即终止。

- [ ] **Step 2: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" process_supervisor
```

Expected: FAIL。

- [ ] **Step 3: 实现进程记录和时限**

```rust
struct ProcessRecord {
    generation: u64,
    phase: ProcessPhase,
    pid: Option<u32>,
    child: Option<Box<dyn SidecarChild>>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    config_revision: Option<String>,
    last_exit_code: Option<i32>,
    last_error: Option<CommandError>,
}

#[derive(Debug, Clone, Copy)]
pub struct SupervisorTiming {
    pub startup_grace: std::time::Duration,
    pub stop_timeout: std::time::Duration,
}

impl Default for SupervisorTiming {
    fn default() -> Self {
        Self {
            startup_grace: std::time::Duration::from_secs(1),
            stop_timeout: std::time::Duration::from_secs(5),
        }
    }
}
```

frpc/frps 各自拥有独立 `tokio::sync::Mutex<ProcessRecord>`，避免一个进程慢操作阻塞另一个。

- [ ] **Step 4: 实现 start 的原子预占**

固定顺序：

1. 在 slot 锁内确认 `Stopped/Crashed`，递增 generation 并设置 `Starting`。
2. 释放锁。
3. load + validate 当前 config snapshot。
4. 调用 sidecar inspect，区分 missing/incompatible。
5. spawn sidecar，启动 event pump。
6. 重新加锁；若 generation/phase 已改变，立即停止迟到 child。
7. 安装 pid、child、startedAt、configRevision。
8. 等待 startup grace；期间 terminated → `Crashed`。
9. 存活后执行可选 probe，进入 `Healthy` 或 `Degraded`。
10. 每次 phase 改变都发送完整 `ProcessSnapshot`。

任何配置或 spawn 错误都必须把同一 generation 置为 `Crashed` 并记录结构化错误。

- [ ] **Step 5: 实现 event pump 与 generation 防护**

event pump 每次修改前检查：

```rust
if record.generation != generation {
    return;
}
```

`Terminated` 在 `Stopping` 时最终进入 `Stopped`，在其他运行 phase 时进入 `Crashed`；保留真实 `Option<i32>`，不得把 `None` 改写为 0。

- [ ] **Step 6: 实现 stop/restart/stop_all**

stop 固定行为：

1. `Stopped` 直接返回当前 snapshot。
2. 设置 `Stopping` 并取出 child/pid。
3. 调用 `request_stop`。
4. 等待匹配 generation 的 terminated 通知。
5. 超时时调用 `force_kill(pid)`，记录 `STOP_TIMEOUT`。
6. force kill 后确认退出则返回 Stopped snapshot；无法确认则进入 Crashed 并返回 error。

- [ ] **Step 7: 运行 supervisor 与全量 Rust 测试**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" process_supervisor
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: supervise FRP sidecars with explicit lifecycle states
```

---

### Task 7: 实现保存并重启回滚事务

**Files:**
- Create: `src-tauri/src/services/config_transaction.rs`
- Modify: `src-tauri/src/domain/config.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/config_transaction.rs`

**Interfaces:**
- Consumes: ConfigRepository、ProcessSupervisor、EventSink。
- Produces:
  - `apply_change(request) -> ConfigSnapshot`
  - `restore_backup(kind, expected_revision) -> ConfigSnapshot`
  - `save_and_restart(request) -> SaveAndRestartResult`

- [ ] **Step 1: 写事务失败矩阵测试**

至少覆盖：

```rust
#[tokio::test]
async fn failed_new_start_restores_config_and_previous_process() {
    let original = "serverPort = 7000\n";
    let fixture = transaction_fixture(original, ProcessPhase::Healthy).await;
    fixture.sidecar.fail_next_start(ErrorCode::SpawnFailed);

    let result = fixture
        .service
        .save_and_restart(change_server_port(&fixture, 7001))
        .await
        .unwrap();

    assert!(!result.applied);
    assert_eq!(fixture.read_config(), original);
    assert!(result.recovery.as_ref().unwrap().config_restored);
    assert!(result.recovery.as_ref().unwrap().process_restored);
    assert_eq!(result.process.phase, ProcessPhase::Healthy);
}
```

测试模块同时定义：

```rust
struct TransactionFixture {
    _temp: tempfile::TempDir,
    repository: std::sync::Arc<ConfigRepository>,
    sidecar: std::sync::Arc<FakeSidecarAdapter>,
    service: ConfigTransactionService,
}

fn change_server_port(
    fixture: &TransactionFixture,
    port: u16,
) -> ConfigChangeRequest {
    let revision = fixture
        .repository
        .load(ConfigKind::Frpc)
        .unwrap()
        .revision()
        .to_string();
    frpc_patch_request(
        revision,
        FrpcConfigPatch {
            server_port: Some(Some(port)),
            ..Default::default()
        },
    )
}

async fn transaction_fixture(
    original: &str,
    phase: ProcessPhase,
) -> TransactionFixture {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths {
        config_dir: temp.path().to_path_buf(),
        log_dir: temp.path().join("logs"),
    };
    std::fs::write(paths.config_path(ConfigKind::Frpc), original).unwrap();
    std::fs::write(paths.config_path(ConfigKind::Frps), "bindPort = 7000\n").unwrap();

    let repository = std::sync::Arc::new(ConfigRepository::new(
        paths,
        std::sync::Arc::new(RealConfigFilesystem),
    ));
    let sidecar = std::sync::Arc::new(FakeSidecarAdapter::running());
    let events = std::sync::Arc::new(RecordingEventSink::default());
    let supervisor = std::sync::Arc::new(ProcessSupervisor::new(
        repository.clone(),
        sidecar.clone(),
        std::sync::Arc::new(FakeHealthProbe::healthy()),
        events.clone(),
        SupervisorTiming {
            startup_grace: std::time::Duration::from_millis(10),
            stop_timeout: std::time::Duration::from_millis(20),
        },
    ));

    if phase == ProcessPhase::Healthy {
        supervisor.start(ProcessKind::Frpc).await.unwrap();
    }

    let service = ConfigTransactionService::new(
        repository.clone(),
        supervisor,
        events,
    );
    TransactionFixture {
        _temp: temp,
        repository,
        sidecar,
        service,
    }
}
```

当前测试只允许传 `Healthy` 或 `Stopped`；其他 phase 由 ProcessSupervisor 自身测试覆盖。

再测试：

- 原进程停止失败 → 不写新配置。
- 新配置 apply 失败 → 不启动。
- 新进程成功 → `applied=true`，新 revision 与 process.configRevision 一致。
- 原进程原本 Stopped，新启动失败 → 回滚配置并保持 Stopped。
- 配置回滚成功但旧进程恢复失败 → 两个恢复结果分别报告。
- 并发 save-and-restart 对同 kind 串行化并执行 revision CAS。

- [ ] **Step 2: 运行测试确认失败**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_transaction
```

Expected: FAIL。

- [ ] **Step 3: 实现每 kind 事务锁和固定顺序**

```rust
pub async fn save_and_restart(
    &self,
    request: ConfigChangeRequest,
) -> Result<SaveAndRestartResult, CommandError> {
    let kind = request.kind();
    let _guard = self.transaction_lock(kind).lock().await;
    let old_state = self.repository.capture(kind)?;
    let old_config = self.repository.load(kind)?;
    let old_process = self.processes.snapshot(kind).await;
    let was_active = !matches!(
        old_process.phase,
        ProcessPhase::Stopped | ProcessPhase::Crashed
    );
    self.repository.preview(request.clone())?;

    if was_active {
        self.processes.stop(kind).await?;
    }

    let candidate = self.repository.apply(request)?;
    match self.processes.start(kind).await {
        Ok(process) => {
            let _ = self
                .events
                .config_changed(&ConfigChangedEvent::from(&candidate));
            Ok(SaveAndRestartResult::applied(candidate, process))
        }
        Err(failure) => {
            match self
                .repository
                .restore_state(kind, candidate.revision(), old_state)
            {
                Ok(restored) => {
                    debug_assert_eq!(restored.revision(), old_config.revision());
                    let process_recovery = if was_active {
                        self.processes.start(kind).await
                    } else {
                        Ok(self.processes.snapshot(kind).await)
                    };
                    let _ = self
                        .events
                        .config_changed(&ConfigChangedEvent::from(&restored));
                    Ok(SaveAndRestartResult::rolled_back(
                        restored,
                        failure,
                        process_recovery,
                    ))
                }
                Err(restore_error) => Ok(SaveAndRestartResult::recovery_failed(
                    candidate,
                    self.processes.snapshot(kind).await,
                    failure,
                    restore_error,
                )),
            }
        }
    }
}
```

`old_config` 用于记录旧 revision，并在恢复后断言 returned snapshot revision 与它一致。`ConfigChangeRequest` 派生 `Clone`；`transaction_lock(&self, kind)` 返回 `&tokio::sync::Mutex<()>`。`SaveAndRestartResult::applied/rolled_back` 负责把配置恢复和进程恢复结果转换为固定 JSON 契约。

同一服务的 `apply_change` 和 `restore_backup` 也获取对应 transaction lock，成功后尽力发一次最终 `config://changed`；event 投递失败不把已成功的磁盘事务伪装成失败。command 不直接发事件。

- [ ] **Step 4: 控制 config changed 事件提交时机**

- 普通 `apply_config_change`：成功替换后立即发新 revision。
- save-and-restart 成功：新进程通过启动窗口后发新 revision。
- save-and-restart 失败并回滚：只发恢复后的旧 revision。
- 不得短暂向前端提交最终会回滚的新 revision。

- [ ] **Step 5: 运行事务与 repository/supervisor 回归**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_transaction
cargo test --manifest-path "src-tauri/Cargo.toml" config_repository
cargo test --manifest-path "src-tauri/Cargo.toml" process_supervisor
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: roll back failed configuration restarts
```

---

### Task 8: 建立 thin commands、AppServices 和统一 shutdown

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/config.rs`
- Create: `src-tauri/src/commands/process.rs`
- Create: `src-tauri/src/commands/support.rs`
- Create: `src-tauri/src/services/shutdown_coordinator.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/capabilities/default.json` only if Tauri reports a missing required permission

**Interfaces:**
- Consumes: repository、transaction、supervisor、AppPaths。
- Produces: Locked Domain Contracts 中列出的 Tauri commands。

- [ ] **Step 1: 写 command serde smoke tests**

测试 `ConfigChangeRequest` 能从前端 JSON 反序列化，并验证 Tauri error 可以保持对象结构：

```rust
#[test]
fn frpc_patch_request_deserializes_camel_case() {
    let value = serde_json::json!({
        "kind": "frpc",
        "expectedRevision": "abc",
        "change": {
            "mode": "patch",
            "patch": { "serverPort": 7001 }
        }
    });
    let request: ConfigChangeRequest = serde_json::from_value(value).unwrap();
    assert_eq!(request.kind(), ConfigKind::Frpc);
}
```

- [ ] **Step 2: 写 shutdown 并发幂等测试**

先为 coordinator 定义可替换目标：

```rust
#[async_trait::async_trait]
pub trait ShutdownTarget: Send + Sync {
    async fn stop_all(&self) -> Result<StopAllResult, CommandError>;
}
```

`ProcessSupervisor` 实现该 trait。测试使用计数 fake，并发调用三次 `prepare()`，断言三个调用得到相同结果且 `stop_all` 只执行一次；另测首次失败时所有等待者得到同一个 error。

- [ ] **Step 3: 实现 AppServices**

`lib.rs` 只 manage 一个服务聚合对象和 shutdown 状态：

```rust
pub struct AppServices {
    pub paths: AppPaths,
    pub config: std::sync::Arc<ConfigRepository>,
    pub processes: std::sync::Arc<ProcessSupervisor>,
    pub transactions: std::sync::Arc<ConfigTransactionService>,
    pub shutdown: std::sync::Arc<ShutdownCoordinator>,
}
```

所有生产 adapter 在 `setup` 中用同一个 `AppHandle` 构造；command 不自行 new service。

- [ ] **Step 4: 实现 thin config/process commands**

示例：

```rust
#[tauri::command]
pub async fn start_process(
    kind: ProcessKind,
    services: tauri::State<'_, AppServices>,
) -> Result<ProcessSnapshot, CommandError> {
    services.processes.start(kind).await
}

#[tauri::command]
pub fn get_config_snapshot(
    kind: ConfigKind,
    services: tauri::State<'_, AppServices>,
) -> Result<ConfigSnapshot, CommandError> {
    services.config.load(kind)
}
```

其他 command 同样只能委托一次服务方法，不拼接 stop/save/start。

配置 mutation commands 调用 `services.transactions.apply_change`、`restore_backup` 或 `save_and_restart`；read/validate/preview 调用 `services.config`。这样事件提交时机和 revision 锁不会落入 command 层。

- [ ] **Step 5: 迁移 support commands 的依赖**

- `export_logs` 使用 `AppServices.paths.log_dir`，不再调用 `config_parser::get_config_dir`。
- `get_frpc_traffic` 调用 `FrpAdminAdapter`，不再位于 process manager。
- `export_deploy_script` 保持现有行为，但移动到 `commands/support.rs` 并返回 `CommandError`。

- [ ] **Step 6: 实现异步 ShutdownCoordinator**

使用 `AtomicBool started/completed` 防止 tray、前端和 `RunEvent` 重复清理：

```rust
pub struct ShutdownCoordinator {
    started: std::sync::atomic::AtomicBool,
    completed: std::sync::atomic::AtomicBool,
    target: std::sync::Arc<dyn ShutdownTarget>,
    result: tokio::sync::Mutex<Option<Result<StopAllResult, CommandError>>>,
    notify: tokio::sync::Notify,
}
```

- 第一个 `prepare()` 调用执行 `target.stop_all()`，clone 结果写入 `result`，再设置 completed 并 `notify_waiters()`。
- 后续调用在循环中先创建 `notified()` future，再检查 completed，避免错过通知；完成后 clone 同一 result。
- `prepare_shutdown` await `prepare()` 并返回 `StopAllResult`；成功后 completed 已为 true，前端关闭窗口不会启动第二轮清理。
- tray quit 启动一次 async cleanup，完成后调用 `app.exit(0)`。
- `RunEvent::ExitRequested` 在未完成时调用 `api.prevent_exit()` 并启动同一 coordinator。
- `RunEvent::Exit` 只做最终兜底记录，不再次启动一轮清理。
- 已 completed 的后续 ExitRequested 允许正常退出。

- [ ] **Step 7: 替换 invoke_handler 注册**

注册新 commands；此时可以暂时保留旧 Rust 文件用于编译，但不得同时注册旧 config/process command。

- [ ] **Step 8: 运行 Rust 全套验证**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: PASS。

Suggested commit message if authorized:

```text
refactor: expose typed Tauri service commands
```

---

### Task 9: 建立唯一前端 Tauri client 和错误映射

**Files:**
- Create: `src/services/tauriClient.ts`
- Create: `src/services/tauriClient.test.ts`
- Create: `src/services/errorMapper.ts`
- Create: `src/services/errorMapper.test.ts`
- Modify: `src/i18n.ts`

**Interfaces:**
- Consumes: Task 1 TypeScript domain contracts。
- Produces: 所有 command wrappers、typed event subscriptions、`normalizeCommandError`。
- Consumed by: App、Dashboard、composables、TrafficChart。

- [ ] **Step 1: 写 invoke/listen adapter 失败测试**

```ts
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn()
const listenMock = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }))

describe('tauriClient', () => {
  beforeEach(() => vi.clearAllMocks())

  it('sends the typed config snapshot command', async () => {
    invokeMock.mockResolvedValue({ kind: 'frpc', revision: 'abc' })
    await tauriClient.getConfigSnapshot('frpc')
    expect(invokeMock).toHaveBeenCalledWith('get_config_snapshot', {
      kind: 'frpc',
    })
  })

  it('subscribes to the stable process event', async () => {
    const listener = vi.fn()
    await tauriClient.onProcessStateChanged(listener)
    expect(listenMock).toHaveBeenCalledWith(
      'process://state-changed',
      expect.any(Function),
    )
  })
})
```

- [ ] **Step 2: 写结构化错误归一化测试**

覆盖：

- 已经是 `CommandError` object → 原样保留。
- Tauri reject 的 `{ code, message, recoverable }` → 保留字段。
- legacy string/unknown → `UNKNOWN`，不得尝试正则解析错误码。
- error mapper 根据 code 返回 `errors.CONFIG_CONFLICT` 等 i18n key。

- [ ] **Step 3: 运行测试确认失败**

Run:

```powershell
pnpm test:run -- src/services/tauriClient.test.ts src/services/errorMapper.test.ts
```

Expected: FAIL。

- [ ] **Step 4: 实现唯一 client**

```ts
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export const tauriClient = {
  getConfigSnapshot: (kind: ConfigKind) =>
    invoke<ConfigSnapshot>('get_config_snapshot', { kind }),

  startProcess: (kind: ProcessKind) =>
    invoke<ProcessSnapshot>('start_process', { kind }),

  stopProcess: (kind: ProcessKind) =>
    invoke<ProcessSnapshot>('stop_process', { kind }),

  onProcessStateChanged: (
    listener: (snapshot: ProcessSnapshot) => void,
  ): Promise<UnlistenFn> =>
    listen<ProcessSnapshot>('process://state-changed', (event) => {
      listener(event.payload)
    }),
}
```

补齐 Locked Domain Contracts 全部方法，以及 export logs/deploy、traffic 和 `prepareShutdown` wrappers。command/event 名称只能在本文件出现一次。

- [ ] **Step 5: 增加中英文错误映射**

`src/i18n.ts` 为当前 WP2 可能返回的每个 error code 添加中英文文案；UI 使用 mapper key，不展示未经处理的 Rust message。detail 只在可展开技术详情中使用。

- [ ] **Step 6: 运行 client 测试和 typecheck**

Run:

```powershell
pnpm test:run -- src/services/tauriClient.test.ts src/services/errorMapper.test.ts
pnpm typecheck
```

Expected: PASS。

Suggested commit message if authorized:

```text
feat: centralize typed Tauri IPC in the frontend
```

---

### Task 10: 迁移现有 Dashboard、表单和进程 composables

**Files:**
- Create: `src/domain/config.test.ts`
- Modify: `src/domain/config.ts`
- Modify: `src/domain/proxyRule.ts`
- Modify: `src/composables/useProcessStatus.ts`
- Create: `src/composables/useProcessStatus.test.ts`
- Modify: `src/composables/useAppLogs.ts`
- Modify: `src/App.vue`
- Modify: `src/views/Dashboard.vue`
- Modify: `src/components/FrpcConfigForm.vue`
- Modify: `src/components/FrpsConfigForm.vue`
- Modify: `src/components/ProtocolForm.vue`
- Modify: `src/components/TrafficChart.vue`
- Modify: `package.json`
- Modify: `pnpm-lock.yaml`

**Interfaces:**
- Consumes: `tauriClient`、typed snapshots/events。
- Produces: 保持现有视觉结构的无损表单保存与准确进程状态。

- [ ] **Step 1: 写 config patch builder 测试**

```ts
it('builds an update selector without replacing unknown rule fields', () => {
  expect(
    buildProxyUpdatePatch(
      { index: 2, originalName: 'ssh' },
      {
        name: 'ssh',
        type: 'tcp',
        localIp: '127.0.0.1',
        localPort: '22',
        remotePort: '6023',
        customDomains: '',
      },
    ),
  ).toEqual({
    op: 'update',
    selector: { index: 2, originalName: 'ssh' },
    patch: {
      name: 'ssh',
      type: 'tcp',
      localIP: '127.0.0.1',
      localPort: 22,
      remotePort: 6023,
      customDomains: null,
    },
  })
})
```

再测试 frpc global、frps known fields、add/delete rule。

- [ ] **Step 2: 写 useProcessStatus 状态事件测试**

mock `tauriClient`，断言：

- init 同时加载 frpc/frps snapshot。
- `process://state-changed` 只更新对应 kind。
- `running` 从 Starting/Healthy/Degraded/Stopping 推导，Crashed/Stopped 为 false。
- start 不做乐观 `running=true`，完全使用 command 返回或事件 snapshot。

- [ ] **Step 3: 运行测试确认失败**

Run:

```powershell
pnpm test:run -- src/domain/config.test.ts src/composables/useProcessStatus.test.ts
```

Expected: FAIL。

- [ ] **Step 4: 迁移 Dashboard 配置状态**

固定替换：

- `frpcConfigContent/parsedFrpcConfig` → `frpcSnapshot`。
- `frpsConfigContent` → `frpsSnapshot`。
- load 调用 `getConfigSnapshot`。
- global/rule/frps 保存提交 patch + 当前 revision。
- delete 提交 `ProxyOperation::Delete`，不再 splice 普通 JS 对象。
- 导出部署脚本使用 `frpsSnapshot.raw`。
- 删除 `smol-toml` 的 parse/stringify import。
- 保存冲突显示“重新加载/查看差异”恢复动作；本工作包不实现新视觉页面。

- [ ] **Step 5: 修复两个配置表单的异步初始化**

`FrpcConfigForm.vue` 和 `FrpsConfigForm.vue` 都使用 typed props 和 immediate watch：

```ts
const props = defineProps<{
  initialData?: FrpsKnownConfig
}>()

watch(
  () => props.initialData,
  (value) => {
    if (!value) return
    form.value = toFrpsForm(value)
  },
  { immediate: true, deep: true },
)
```

Dashboard 必须传入 `:initial-data="frpsSnapshot?.known"`。

- [ ] **Step 6: 迁移进程、退出、日志和流量调用**

- `useProcessStatus.ts` 只调用 `tauriClient`，保存完整 snapshot 并导出 computed compatibility flags。
- `App.vue` 在根级 mounted 时调用 `initProcessStatus()`；不能依赖 Dashboard 先挂载，否则关闭判断会误报。
- `App.vue` 的退出调用 `prepareShutdown()`。
- `useAppLogs.ts` 监听 `log://entry`。
- `TrafficChart.vue` 通过 client 请求 traffic。
- 所有错误先 `normalizeCommandError` 再映射 i18n。

- [ ] **Step 7: 删除前端 TOML 重写依赖**

Run:

```powershell
pnpm remove smol-toml
```

确认项目中没有 import：

```powershell
rg -n "smol-toml|parse\(|stringify\(" "src"
```

Expected: 不再出现 `smol-toml`；其他业务合法的 parse/stringify 使用需人工确认。

- [ ] **Step 8: 验证唯一 IPC 边界**

Run:

```powershell
rg -n "@tauri-apps/api/core|@tauri-apps/api/event|\binvoke\(|\blisten\(" "src"
```

Expected: 直接 `invoke/listen` 只出现在 `src/services/tauriClient.ts` 及其 mock 测试中。

- [ ] **Step 9: 运行前端测试和构建**

Run:

```powershell
pnpm test:run
pnpm typecheck
pnpm build
```

Expected: PASS。Vite 现有 chunk-size warning 可记录，但不得出现新增 build error。

Suggested commit message if authorized:

```text
refactor: migrate the dashboard to revisioned typed services
```

---

### Task 11: 删除旧模块、同步文档并完成验收

**Files:**
- Delete: `src-tauri/src/config_parser.rs`
- Delete: `src-tauri/src/process_manager.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `README.md`
- Verify: all WP2 files

**Interfaces:**
- Consumes: 新 commands/client 已完成的迁移。
- Produces: 单一正式服务层和完整 WP2 验收证据。

- [ ] **Step 1: 证明旧 symbols 已无调用者**

Run:

```powershell
rg -n "config_parser|process_manager|read_frpc_config|save_frpc_config|read_frps_config|save_frps_config|get_frpc_status|get_frps_status|start_frpc|stop_frpc|start_frps|stop_frps|frpc-stdout|frpc-stderr|frps-stdout|frps-stderr|frpc-terminated|frps-terminated" "src" "src-tauri/src"
```

Expected: 仅允许出现在即将删除的两个旧模块；其他命中必须先迁移。

- [ ] **Step 2: 删除旧模块并清理注册**

删除两个文件及 `mod config_parser; mod process_manager;`，确认 support commands 已使用 `AppPaths`/`FrpAdminAdapter`。

- [ ] **Step 3: 同步 README**

README 只描述已实现能力：

- 配置保存保留未知字段和注释。
- revision 冲突会阻止覆盖外部修改。
- 保存前保留一个 `.bak`。
- 启动状态区分 starting/healthy/degraded/crashed。
- 保存并重启失败会尝试恢复配置和旧进程。

不提前宣称 CodeMirror、Pinia 控制台、诊断或 updater 已完成。

- [ ] **Step 4: 运行完整自动化门禁**

Run:

```powershell
pnpm test:run
pnpm typecheck
pnpm build
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Expected: 全部 exit code 0。

- [ ] **Step 5: 运行静态验收搜索**

Run:

```powershell
rg -n "@tauri-apps/api/core|@tauri-apps/api/event|\binvoke\(|\blisten\(" "src"
rg -n "read_frpc_config|save_frpc_config|read_frps_config|save_frps_config|start_frpc|stop_frpc|start_frps|stop_frps|frpc-stdout|frpc-stderr|frps-stdout|frps-stderr|frpc-terminated|frps-terminated" "src" "src-tauri/src"
git diff --check
git diff --name-status -- "src-tauri/bin"
```

Expected:

- 第一条只命中 tauriClient 和测试 mock。
- 第二条无命中。
- `git diff --check` 无 whitespace error。
- sidecar diff 无输出。

- [ ] **Step 6: Windows 手动配置验收**

使用 complex fixture 的副本：

1. 启动应用并加载包含注释、dotted key、未知字段和特殊字符 token 的配置。
2. 只修改一个全局端口，保存并重新加载。
3. 对比文件，确认注释、顺序、未知表/键、proxy 内高级字段仍存在。
4. 在外部编辑器修改文件，再用旧页面 snapshot 保存，确认收到 `CONFIG_CONFLICT` 且外部修改未被覆盖。
5. 恢复 `.bak`，确认 raw 与备份逐字节一致。

- [ ] **Step 7: Windows 手动进程验收**

1. 快速双击启动，同一 kind 只产生一个 frpc/frps 进程。
2. 正常启动后状态依次为 Starting → Healthy，监控配置错误时为 Degraded。
3. 停止两次不报错且无残留。
4. 保存并重启成功后 process.configRevision 等于新 config revision。
5. 制造新启动失败，确认 UI 分别展示失败原因、配置回滚和旧进程恢复结果。
6. 分别从窗口和 tray 退出，任务管理器中无由应用启动的 `frpc.exe`/`frps.exe`。

- [ ] **Step 8: 最终工作树审阅**

Run:

```powershell
git status --short --branch
git diff --stat
git diff -- "src-tauri/src" "src" "README.md" "package.json" "src-tauri/Cargo.toml"
```

确认：

- 没有 `.codegraph/`、`.cursor/` 或 build output 被加入计划变更。
- 没有意外删除 sidecar。
- 没有硬编码 token/password。
- 没有第二套 legacy command。

Suggested commit series if the user explicitly authorizes commits:

```text
feat: define typed process and configuration contracts
feat: preserve advanced TOML through revisioned patches
feat: supervise FRP sidecars with explicit lifecycle states
feat: roll back failed configuration restarts
refactor: migrate the frontend to typed Tauri services
docs: document safe configuration and process behavior
```

Do not run these commits without explicit authorization.

## Final Acceptance Checklist

- [ ] fake sidecar 覆盖正常运行、立即退出、stdout/stderr、非零退出、并发启动和停止超时。
- [ ] ProcessSupervisor 覆盖 start、stop、restart、stop_all、幂等和 stale generation。
- [ ] 复杂 frpc/frps TOML 表单保存后注释、顺序、未知字段和规则内高级字段不丢失。
- [ ] raw/source 校验返回准确问题位置，revision 冲突不会覆盖磁盘。
- [ ] `.bak`、原子替换和恢复测试通过。
- [ ] 新配置启动失败会恢复旧配置，并报告旧进程恢复是否成功。
- [ ] Rust 和 TypeScript 的 error/snapshot/event 字段一致。
- [ ] `tauriClient.ts` 是唯一直接 invoke/listen 边界。
- [ ] `config_parser.rs`、`process_manager.rs` 和旧 IPC/event 均已移除。
- [ ] Windows 窗口退出、tray 退出和系统退出均无 sidecar 残留。
- [ ] WP1 前端/Rust 测试和现有生产构建保持绿色。
