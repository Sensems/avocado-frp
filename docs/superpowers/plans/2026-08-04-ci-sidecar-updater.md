# CI、Sidecar 与 Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补齐 WP5：应用版本单一来源、sidecar manifest + SHA256 准备校验、PR 质量门禁、Release 在缺 updater 签名 Secret 时 fail-closed，以及 Settings 可检查/下载/确认安装的 Tauri updater（无平台代码签名）。

**Architecture:** 以 `tauri.conf.json` 的应用版本为 SSOT，脚本校验 `package.json` / `Cargo.toml`。FRP 版本与各 triple 的 URL/SHA256 放在独立 `sidecar.manifest.json`；`scripts/prepare-sidecars` 在本地与 CI 打包前下载或校验仓库内二进制。`tauri-plugin-updater` 负责检查/下载/安装；前端仅经 `tauriClient`；安装前走既有 `prepare_shutdown` / `stop_all`。Release 必须提供 `TAURI_SIGNING_PRIVATE_KEY`（及密码），否则明确失败。

**Tech Stack:** GitHub Actions、pnpm 9、Node 20、Rust stable、`tauri-apps/tauri-action`、`tauri-plugin-updater`、现有 `SidecarAdapter` / `tauriClient` / Settings 占位。

## Global Constraints

- 完整 updater 实机验收在 Windows；macOS/Linux 以 CI 构建 + sidecar 校验为准。
- 不修改、删除或暂存 `.codegraph/`、`.cursor/`、`.superpowers/` 及其他无关未跟踪文件。
- 不创建 Git commit，除非用户在执行阶段明确授权；每个任务只提供建议提交信息。
- **本工作包不新增单元测试或组件测试**（选项 A）；验证以 CI 绿、`pnpm typecheck` / `pnpm build`、`cargo fmt --check` / `clippy -D warnings`，以及 Windows 手动 updater 流程为准。可运行**已有** `pnpm test:run` / `cargo test`，但不得为过门禁新写测试套件。
- **不新增 ESLint 工程**仅为满足设计里的 `pnpm lint`；PR 门禁用 `typecheck` + `build` 代替 lint（若后续已有 lint 脚本再接入）。
- `tauriClient.ts` 仍是唯一允许直接 `invoke`/`listen` 的前端模块。
- **不做** Windows Authenticode、Apple Developer ID、notarization；README 必须说明 SmartScreen/Gatekeeper 仍可能警告。
- Updater 私钥只存在于 CI Secrets / 本地受控环境；不得写入仓库、日志或诊断包。
- 在四个目标的 prepare/校验全部稳定前，**不删除**仓库内 `src-tauri/bin/*` sidecar。
- 支持目标仅：`x86_64-pc-windows-msvc`、`x86_64-unknown-linux-gnu`、`aarch64-apple-darwin`、`x86_64-apple-darwin`（不含 linux-aarch64）。
- 诊断里的 `UPDATER_DEFERRED_WP5` 在 updater 可用后改为真实状态或移除误导文案。
- 保持 WP2–WP4 进程/配置/日志契约；安装更新前必须停止本应用 sidecar。

### Locked decision: FRP pin

当前代码 `SUPPORTED_FRP_VERSION = "0.61.1"`，但 Windows 仓库二进制曾报告 `0.67.0`。Task 0 **必须先统一**：

1. 对四个目标的 `frpc`/`frps` 运行 `--version`（能跑的平台本地跑；不能跑的在 CI/文档中记录）。
2. 选定**单一** `frpVersion` 写入 manifest，并同步 `SidecarAdapter::SUPPORTED_FRP_VERSION`、README、部署导出说明。
3. 若仓库二进制版本不一致：要么替换为同版本官方 release，要么全部升级到同一新版本——禁止 manifest 与 inspect 期望互相矛盾。
4. 优先策略：**以将进入 manifest 的官方 release 版本为准**；仓库内文件必须通过 SHA256 与该版本匹配，否则 prepare 脚本覆盖/下载。

---

## File Structure

```text
sidecar.manifest.json                 # FRP ver + per-triple URL + sha256 for frpc/frps
scripts/
  check-versions.mjs                  # app version SSOT
  prepare-sidecars.mjs                # download/verify/place into src-tauri/bin
  verify-sidecar-arch.mjs             # optional file/arch smoke
.github/workflows/
  ci.yml                              # PR gate (new)
  release.yml                         # upgrade: versions, prepare, signing fail-closed
src-tauri/
  tauri.conf.json                     # version SSOT; plugins.updater endpoints + pubkey
  capabilities/default.json           # updater permissions
  Cargo.toml                          # tauri-plugin-updater; version aligned
  src/lib.rs                          # register updater plugin
  src/commands/updater.rs             # thin wrappers if needed beyond plugin
  src/adapters/sidecar.rs             # SUPPORTED_FRP_VERSION from manifest pin
src/
  services/tauriClient.ts             # check/download/install + updater://state-changed
  domain/updater.ts
  stores/updater.ts                   # optional
  features/settings/SettingsPage.vue  # enable Check Updates + progress/confirm
  i18n.ts
package.json                          # version aligned; scripts: check:versions, prepare:sidecars
README.md                             # SmartScreen/Gatekeeper + updater signing boundary
```

---

### Task 0: 应用版本 SSOT + FRP 版本钉扎

**Files:**
- Modify: `src-tauri/Cargo.toml`（对齐 `1.0.0` 或当前 tauri.conf）
- Modify: `package.json` / `src-tauri/tauri.conf.json`（若需）
- Create: `scripts/check-versions.mjs`
- Modify: `package.json` scripts: `"check:versions": "node scripts/check-versions.mjs"`
- Modify: `src-tauri/src/adapters/sidecar.rs`（`SUPPORTED_FRP_VERSION`）
- Modify: Settings 硬编码 `APP_VERSION` → 从 `package.json` 或 build 注入常量读取
- Modify: README 中 FRP 版本说明

**Interfaces:**
- Produces: `pnpm check:versions` 在三者不一致时 exit ≠ 0
- Produces: 单一 `FRP_VERSION` 字符串供 Task 1 manifest 使用（写在报告或 `sidecar.manifest.json` 草稿）

- [ ] **Step 1: 对齐应用版本**

以 `src-tauri/tauri.conf.json` → `version` 为准，改 `package.json` 与 `Cargo.toml` `[package].version`。

- [ ] **Step 2: check-versions 脚本**

比较三处字符串全等；打印清晰 diff。

- [ ] **Step 3: 钉扎 FRP 版本**

按 Global Constraints「Locked decision」执行；更新 `SUPPORTED_FRP_VERSION`。

- [ ] **Step 4: 验证**

```powershell
pnpm check:versions
pnpm typecheck
cargo check --manifest-path "src-tauri/Cargo.toml"
```

Suggested commit message if authorized:

```text
chore: align app versions and pin supported FRP release
```

---

### Task 1: Sidecar manifest + prepare/verify 脚本

**Files:**
- Create: `sidecar.manifest.json`
- Create: `scripts/prepare-sidecars.mjs`
- Modify: `package.json` → `"prepare:sidecars": "node scripts/prepare-sidecars.mjs"`
- Optional: `scripts/verify-sidecar-arch.mjs`

**Interfaces:**
- Manifest shape（示例）:

```json
{
  "frpVersion": "X.Y.Z",
  "artifacts": {
    "x86_64-pc-windows-msvc": {
      "frpc": { "url": "https://...", "sha256": "..." },
      "frps": { "url": "https://...", "sha256": "..." }
    },
    "x86_64-unknown-linux-gnu": { "frpc": {}, "frps": {} },
    "aarch64-apple-darwin": { "frpc": {}, "frps": {} },
    "x86_64-apple-darwin": { "frpc": {}, "frps": {} }
  }
}
```

- `prepare-sidecars` 行为：
  - 参数：`--target <triple>`（默认检测 host）或 `--all`
  - 若本地 `src-tauri/bin/frpc-<triple>[.exe]` 已存在且 SHA256 匹配 → 跳过下载
  - 否则下载到临时文件 → 校验 SHA256 → 原子替换到 `src-tauri/bin/`
  - SHA256 失败 → 非零退出，**不得**保留未校验文件
  - 可选：执行 `--version` 并断言包含 `frpVersion`

- [ ] **Step 1: 写 manifest（含真实 SHA256）**

从官方 release 资产计算或记录校验和。

- [ ] **Step 2: prepare 脚本**

- [ ] **Step 3: 本地验证**

```powershell
pnpm prepare:sidecars
pnpm check:versions
```

Suggested commit message if authorized:

```text
feat: add sidecar manifest and prepare script with sha256 checks
```

---

### Task 2: PR CI 工作流

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Triggers: `pull_request` + `push` to `main`（按需）
- Jobs（可单 job 或 frontend/rust 分离）：

```text
pnpm install --frozen-lockfile
pnpm check:versions
pnpm prepare:sidecars   # host triple at least; or validate-only mode
pnpm typecheck
pnpm build
pnpm test:run           # existing tests only; allow empty/skip if script missing — prefer run
cargo fmt --check       # in src-tauri
cargo clippy -- -D warnings
cargo test              # existing only
# manifest JSON schema / required triples present
```

- Linux job 需 webkit 依赖（可抄 `release.yml`）。
- 不要求本 PR 工作流完成四目标完整 `tauri build`（留给 Release）；至少 typecheck/build + Rust 门禁 + manifest/prepare。

- [ ] **Step 1: 编写 ci.yml**

- [ ] **Step 2: 本地等价命令跑通**

```powershell
pnpm install --frozen-lockfile
pnpm check:versions
pnpm typecheck
pnpm build
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
```

Suggested commit message if authorized:

```text
ci: add pull request quality gate workflow
```

---

### Task 3: Release 工作流 — prepare + updater 签名 fail-closed

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- 每个 matrix job 在 `tauri-action` 前：
  1. `pnpm check:versions`
  2. `pnpm prepare:sidecars --target <matrix triple>`
  3. 校验产物路径存在
- Env 必须注入：
  - `TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}`（若使用）
- **Fail-closed：** job 开头显式检查 Secret 非空；为空则 `echo` 说明并 `exit 1`，不调用 tauri-action。
- `tauri-action` 生成安装包 + updater 资产（按 Tauri 2 文档开启 updater artifacts）。
- 保留 draft release 或按现有策略；在 release body 简述校验与签名边界。

- [ ] **Step 1: 映射 matrix → triple**

| matrix | triple |
|--------|--------|
| windows | `x86_64-pc-windows-msvc` |
| linux | `x86_64-unknown-linux-gnu` |
| macos aarch64 | `aarch64-apple-darwin` |
| macos x86_64 | `x86_64-apple-darwin` |

- [ ] **Step 2: Secret 门闩 + prepare**

- [ ] **Step 3: 文档化所需 Secrets（README）**

Suggested commit message if authorized:

```text
ci: require updater signing secrets and prepare sidecars on release
```

---

### Task 4: 集成 tauri-plugin-updater（Rust/配置）

**Files:**
- Modify: `src-tauri/Cargo.toml`、`package.json` / lockfiles
- Modify: `src-tauri/tauri.conf.json` → `plugins.updater`（endpoints、pubkey）
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs`
- Create pubkey 文件或内联 conf（**公钥可进仓库**；私钥禁止）

**Interfaces:**
- Endpoints：指向 GitHub Releases 最新（或 `https://github.com/<owner>/<repo>/releases/latest/download/latest.json` 等 Tauri 推荐格式）
- 开发环境：允许缺少 endpoint 时检查失败为友好错误，不崩溃启动
- 注册插件；权限最小必要

- [ ] **Step 1: 添加依赖与 conf**

生成本地测试密钥对的步骤写入 README（`tauri signer generate`）；CI 用 Secrets 中的私钥。

- [ ] **Step 2: capabilities + lib.rs**

- [ ] **Step 3: 验证**

```powershell
cargo check --manifest-path "src-tauri/Cargo.toml"
pnpm typecheck
```

Suggested commit message if authorized:

```text
feat: integrate tauri-plugin-updater with public key config
```

---

### Task 5: 前端 Check Updates UX

**Files:**
- Create: `src/domain/updater.ts`
- Modify: `src/services/tauriClient.ts`
- Create: `src/stores/updater.ts`（可选）
- Modify: `src/features/settings/SettingsPage.vue`
- Modify: `src/i18n.ts`
- Modify: diagnostics 文案（去掉“永远 WP5 延期”的误导）
- Optional: App 启动静默 `check`（不下载、不安装）；Settings 开关可放 `AppSettings`（若加字段需小扩展 app-settings；否则 localStorage 亦可，优先 app-settings 一致性）

**Interfaces:**
- `tauriClient`: `checkForUpdates` / `downloadAndInstall`（或明确分步）+ `onUpdaterStateChanged`
- Settings：启用「检查更新」；显示当前版本、就绪版本、notes、进度；**安装前 confirm**
- Confirm 后：`prepareShutdown` / `stop_all` → 再安装
- 错误经 `errorMapper`；`UpdateFailed` 文案中英齐全
- 异步按钮真实 loading；`aria-label`

- [ ] **Step 1: tauriClient + domain**

- [ ] **Step 2: Settings UI**

- [ ] **Step 3: 可选启动静默检查**

- [ ] **Step 4: 验证**

```powershell
pnpm typecheck
pnpm build
rg -n "from '@tauri-apps/api/(core|event)'|\\binvoke\\(|\\blisten\\(" src
```

Suggested commit message if authorized:

```text
feat: enable settings update check download and confirm install
```

---

### Task 6: README、诊断收尾与验收清单

**Files:**
- Modify: `README.md`
- Modify: diagnostics i18n / `versions.summary` 行为（updater 已配置时不再永远 warning）
- Optional: `.github/PULL_REQUEST_TEMPLATE.md` 一句指向 CI

- [ ] **Step 1: README**

必须包含：

- 应用版本 SSOT 与 `pnpm check:versions`
- sidecar manifest / `pnpm prepare:sidecars`
- updater：公钥在仓、私钥在 Secrets；无 Authenticode/公证
- SmartScreen / Gatekeeper 可能警告
- 所需 GitHub Secrets 列表

- [ ] **Step 2: 全量本地门禁**

```powershell
pnpm check:versions
pnpm prepare:sidecars
pnpm typecheck
pnpm build
cargo fmt --check --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
rg -n "from '@tauri-apps/api/(core|event)'|\\binvoke\\(|\\blisten\\(" src
```

- [ ] **Step 3: 验收清单（人工 + CI）**

1. PR 工作流在示例 PR 或 `workflow_dispatch` 等价步骤上全绿。  
2. Release（或 dry-run job）在**缺少**签名 Secret 时明确失败。  
3. 配置 Secret 后四目标构建成功，bundle 内 sidecar 存在且 SHA256/版本匹配。  
4. Windows：Settings 检查更新 → 下载 → 确认 → 安装（可用签名测试通道/draft release）。  
5. 安装前运行中的 frpc/frps 被停止；退出无残留。  
6. 诊断包/日志仍无 updater 私钥。

Suggested commit message if authorized:

```text
docs: document CI sidecar prepare and updater signing boundary
```

## Final Acceptance Checklist

- [ ] `pnpm check:versions` 强制 app 三处版本一致。
- [ ] sidecar manifest 含四目标 URL + SHA256；prepare 失败不留脏文件。
- [ ] `SUPPORTED_FRP_VERSION` 与 manifest / 二进制一致。
- [ ] PR CI：typecheck、build、fmt、clippy、版本与 manifest 校验。
- [ ] Release：prepare 每目标 sidecar；缺 `TAURI_SIGNING_PRIVATE_KEY` 失败。
- [ ] Settings 可检查/下载/确认安装；安装前停 sidecar。
- [ ] README 写明签名边界与 SmartScreen/Gatekeeper。
- [ ] 无新增单元测试要求；Windows updater 手动清单执行完毕。

## Out of Scope (explicit)

- Windows Authenticode / Apple 公证
- linux-aarch64 目标
- 从 Git 删除 sidecar 大二进制（须四目标 prepare 稳定后另议）
- 新增 Vitest/Rust 单测或新 ESLint 栈
- 自动静默安装（仅允许静默检查）
- 改变 WP3/WP4 运维 UI 信息架构
