# 安全基线 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 恢复当前跨平台发布基线，建立最小自动化测试，并修复退出清理、语言持久化、规则编辑和反馈一致性问题。

**Architecture:** 本工作包只做可独立验收的止损改动，不提前实现完整 ProcessSupervisor 或 ConfigRepository。前端先提取可测试的纯函数，Rust 先建立幂等 `stop_all` 和统一退出入口；这些接口将在下一个工作包中平滑替换为正式服务层。

**Tech Stack:** Vue 3.5、TypeScript 5.6、Vite 6、Vitest 4、Vue Test Utils、Naive UI、vue-i18n 11、Tauri 2、Rust 2021、toml_edit 0.25、pnpm。

## Global Constraints

- 完整运行验收在 Windows 本机进行；macOS/Linux 在本工作包只恢复构建输入，不做实机交互承诺。
- 不修改或暂存现有未跟踪目录 `.codegraph/` 与 `.cursor/`。
- 只恢复用户已明确授权的两个 macOS frpc 文件，不使用 `git reset --hard`、`git checkout -- .` 或其他批量还原命令。
- 不创建 Git commit，除非用户在执行阶段明确授权；每个任务只准备可审阅的 checkpoint 和建议提交信息。
- 所有行为改动遵循 TDD：先看到目标测试失败，再写最小实现，再运行目标测试和相关回归。
- 本工作包不重做 Dashboard 视觉，不引入 Pinia、CodeMirror、updater 或新的 Rust 服务目录。
- 托盘退出、窗口退出和系统退出必须调用同一个 Rust 清理函数。
- 当前 stop 实现使用 Tauri `CommandChild::kill`；等待、超时和健康状态机属于下一个 ProcessSupervisor 工作包。
- 所有新增中英文文案必须同时添加，禁止新增硬编码中文操作反馈。

---

## File Structure

本工作包锁定以下文件职责：

```text
src/lib/preferences.ts
  语言偏好 key、旧 key 迁移和持久化纯函数。

src/lib/preferences.test.ts
  语言偏好读取、迁移和写入测试。

src/domain/proxyRule.ts
  规则表单类型、已有配置到表单的归一化、字段校验。

src/domain/proxyRule.test.ts
  域名数组回填、端口和条件字段校验测试。

src/components/ProtocolForm.vue
  使用 proxyRule 领域函数呈现字段错误并发出 typed payload。

src/i18n.ts
  使用 preferences 读取 locale，并补齐规则/退出/反馈文案。

src/App.vue
  使用 preferences 持久化 locale；退出前调用 prepare_shutdown。

src/views/Dashboard.vue
  使用准确的保存/删除反馈和国际化操作标签。

src-tauri/src/config_parser.rs
  提取可单测的 TOML 语法校验函数。

src-tauri/src/process_manager.rs
  幂等 stop_slot、stop_all_state、stop_all 和最小 StopSummary。

src-tauri/src/lib.rs
  注册 prepare_shutdown；托盘和 RunEvent 使用统一清理函数。

vite.config.ts / package.json / pnpm-lock.yaml
  Vitest + jsdom + Vue Test Utils 测试基础设施。

README.md
  同步托盘和退出行为的真实能力。
```

---

### Task 1: 恢复 macOS frpc 发布输入

**Files:**
- Restore: `src-tauri/bin/frpc-aarch64-apple-darwin`
- Restore: `src-tauri/bin/frpc-x86_64-apple-darwin`

**Interfaces:**
- Consumes: Git `HEAD` 中已追踪的两个 sidecar 文件。
- Produces: 现有 `tauri.conf.json` 的 `externalBin: ["bin/frpc", "bin/frps"]` 再次具备 macOS 双架构输入。

- [ ] **Step 1: 确认只存在已知删除**

Run:

```powershell
git status --short --branch
git diff --stat
```

Expected:

```text
 D src-tauri/bin/frpc-aarch64-apple-darwin
 D src-tauri/bin/frpc-x86_64-apple-darwin
```

允许同时看到未跟踪的 `.codegraph/`、`.cursor/` 和本次新增的 `docs/`；不得还原或暂存它们。

- [ ] **Step 2: 精确恢复两个文件**

Run:

```powershell
git restore --source=HEAD -- "src-tauri/bin/frpc-aarch64-apple-darwin" "src-tauri/bin/frpc-x86_64-apple-darwin"
```

- [ ] **Step 3: 验证四个发布目标的基础文件**

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

Expected: exit code 0 and no output.

- [ ] **Step 4: 准备审阅 checkpoint**

Run:

```powershell
git status --short
```

Expected: 两个 `D` 条目消失；`.codegraph/`、`.cursor/` 和 `docs/` 保持未跟踪/未暂存状态。

Suggested commit message if the user later authorizes commits:

```text
fix: restore macOS frpc release inputs
```

---

### Task 2: 建立前端测试基线并修复语言持久化

**Files:**
- Modify: `package.json:6-10,33-41`
- Modify: `pnpm-lock.yaml`
- Modify: `vite.config.ts:1-40`
- Create: `src/lib/preferences.ts`
- Create: `src/lib/preferences.test.ts`
- Modify: `src/i18n.ts:1,260-265`
- Modify: `src/App.vue:4-25`

**Interfaces:**
- Consumes: 浏览器 `Storage`、vue-i18n 的 `locale` ref。
- Produces:
  - `LANGUAGE_STORAGE_KEY: "avocado-frp-lang"`
  - `LEGACY_LANGUAGE_STORAGE_KEY: "frp-desktop-lang"`
  - `loadLocale(storage?: StorageLike): SupportedLocale`
  - `persistLocale(locale: string, storage?: StorageLike): SupportedLocale`

- [ ] **Step 1: 安装测试依赖**

Run:

```powershell
pnpm add -D vitest @vue/test-utils jsdom
```

Expected: `package.json` 和 `pnpm-lock.yaml` 更新，命令 exit code 0。

- [ ] **Step 2: 增加测试脚本和 Vite 测试配置**

Update `package.json` scripts to:

```json
{
  "scripts": {
    "dev": "vite",
    "typecheck": "vue-tsc --noEmit",
    "build": "pnpm typecheck && vite build",
    "test": "vitest",
    "test:run": "vitest run",
    "preview": "vite preview",
    "tauri": "tauri"
  }
}
```

Replace the first import in `vite.config.ts`:

```ts
import { defineConfig } from "vitest/config";
```

Add this `test` property as a sibling of `server` inside the returned config object:

```ts
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    clearMocks: true,
    restoreMocks: true,
  },
```

Do not enable Vitest globals; every test imports `describe`, `it`, and `expect`.

- [ ] **Step 3: 写语言偏好的失败测试**

Create `src/lib/preferences.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import {
  LANGUAGE_STORAGE_KEY,
  LEGACY_LANGUAGE_STORAGE_KEY,
  loadLocale,
  persistLocale,
} from "./preferences";

describe("language preferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("defaults to zh and writes the canonical key", () => {
    expect(loadLocale()).toBe("zh");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh");
  });

  it("migrates the legacy language key", () => {
    localStorage.setItem(LEGACY_LANGUAGE_STORAGE_KEY, "en");

    expect(loadLocale()).toBe("en");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("en");
    expect(localStorage.getItem(LEGACY_LANGUAGE_STORAGE_KEY)).toBeNull();
  });

  it("prefers a valid canonical value over the legacy value", () => {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, "zh");
    localStorage.setItem(LEGACY_LANGUAGE_STORAGE_KEY, "en");

    expect(loadLocale()).toBe("zh");
  });

  it("normalizes unsupported values before persisting", () => {
    expect(persistLocale("fr")).toBe("zh");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh");
  });
});
```

- [ ] **Step 4: 运行测试并确认红灯**

Run:

```powershell
pnpm test:run -- src/lib/preferences.test.ts
```

Expected: FAIL because `src/lib/preferences.ts` does not exist.

- [ ] **Step 5: 实现语言偏好模块**

Create `src/lib/preferences.ts`:

```ts
export const LANGUAGE_STORAGE_KEY = "avocado-frp-lang";
export const LEGACY_LANGUAGE_STORAGE_KEY = "frp-desktop-lang";

export const SUPPORTED_LOCALES = ["zh", "en"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

type StorageLike = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const isSupportedLocale = (value: string | null): value is SupportedLocale =>
  value !== null && SUPPORTED_LOCALES.includes(value as SupportedLocale);

export const loadLocale = (
  storage: StorageLike = localStorage,
): SupportedLocale => {
  const canonical = storage.getItem(LANGUAGE_STORAGE_KEY);
  const legacy = storage.getItem(LEGACY_LANGUAGE_STORAGE_KEY);
  const locale = isSupportedLocale(canonical)
    ? canonical
    : isSupportedLocale(legacy)
      ? legacy
      : "zh";

  storage.setItem(LANGUAGE_STORAGE_KEY, locale);
  if (legacy !== null) {
    storage.removeItem(LEGACY_LANGUAGE_STORAGE_KEY);
  }
  return locale;
};

export const persistLocale = (
  locale: string,
  storage: StorageLike = localStorage,
): SupportedLocale => {
  const normalized = isSupportedLocale(locale) ? locale : "zh";
  storage.setItem(LANGUAGE_STORAGE_KEY, normalized);
  return normalized;
};
```

- [ ] **Step 6: 让应用统一使用偏好模块**

At the top of `src/i18n.ts`, add:

```ts
import { loadLocale } from "@/lib/preferences";
```

Change the i18n locale initialization to:

```ts
export const i18n = createI18n({
  legacy: false,
  locale: loadLocale(),
  fallbackLocale: "en",
  messages,
});
```

In `src/App.vue`, add:

```ts
import { persistLocale } from "@/lib/preferences";
```

Replace the locale watcher with:

```ts
watch(locale, (value) => {
  persistLocale(value);
});
```

- [ ] **Step 7: 运行目标测试和类型检查**

Run:

```powershell
pnpm test:run -- src/lib/preferences.test.ts
pnpm typecheck
```

Expected: 4 tests PASS; `vue-tsc` exit code 0。

- [ ] **Step 8: 准备审阅 checkpoint**

Run:

```powershell
git diff -- package.json pnpm-lock.yaml vite.config.ts src/lib/preferences.ts src/lib/preferences.test.ts src/i18n.ts src/App.vue
```

Suggested commit message if authorized:

```text
fix: persist and migrate language preference
```

---

### Task 3: 修复规则回填、校验和操作反馈

**Files:**
- Create: `src/domain/proxyRule.ts`
- Create: `src/domain/proxyRule.test.ts`
- Modify: `src/components/ProtocolForm.vue:1-33,50-93`
- Modify: `src/i18n.ts:36-90,163-217`
- Modify: `src/views/Dashboard.vue:193-251,411-437`

**Interfaces:**
- Consumes: 从 TOML 解析得到的现有 proxy 对象。
- Produces:
  - `ProxyRuleForm`
  - `ProxyRuleSource`
  - `ProxyRuleSavePayload`
  - `ProxyRuleErrors`
  - `toProxyRuleForm(source?): ProxyRuleForm`
  - `validateProxyRuleForm(form): ProxyRuleErrors`

- [ ] **Step 1: 写规则领域函数的失败测试**

Create `src/domain/proxyRule.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  toProxyRuleForm,
  validateProxyRuleForm,
  type ProxyRuleForm,
} from "./proxyRule";

const validTcpRule = (): ProxyRuleForm => ({
  name: "ssh",
  type: "tcp",
  localIp: "127.0.0.1",
  localPort: "22",
  remotePort: "6000",
  customDomains: "",
});

describe("toProxyRuleForm", () => {
  it("joins customDomains arrays for editing", () => {
    expect(
      toProxyRuleForm({
        name: "web",
        type: "http",
        localIp: "127.0.0.1",
        localPort: 8080,
        customDomains: ["a.example.com", "b.example.com"],
      }).customDomains,
    ).toBe("a.example.com, b.example.com");
  });

  it("normalizes numeric ports to input strings", () => {
    const form = toProxyRuleForm({
      name: "ssh",
      type: "tcp",
      localPort: 22,
      remotePort: 6000,
    });

    expect(form.localPort).toBe("22");
    expect(form.remotePort).toBe("6000");
  });
});

describe("validateProxyRuleForm", () => {
  it("accepts a valid TCP rule", () => {
    expect(validateProxyRuleForm(validTcpRule())).toEqual({});
  });

  it("rejects out-of-range ports", () => {
    const errors = validateProxyRuleForm({
      ...validTcpRule(),
      localPort: "0",
      remotePort: "70000",
    });

    expect(errors.localPort).toBe("invalidPort");
    expect(errors.remotePort).toBe("invalidPort");
  });

  it("requires domains for HTTP rules", () => {
    const errors = validateProxyRuleForm({
      ...validTcpRule(),
      type: "http",
      remotePort: "",
      customDomains: " ",
    });

    expect(errors.customDomains).toBe("required");
    expect(errors.remotePort).toBeUndefined();
  });
});
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```powershell
pnpm test:run -- src/domain/proxyRule.test.ts
```

Expected: FAIL because `src/domain/proxyRule.ts` does not exist.

- [ ] **Step 3: 实现规则类型、归一化和校验**

Create `src/domain/proxyRule.ts`:

```ts
export const PROTOCOL_TYPES = [
  "tcp",
  "udp",
  "http",
  "https",
  "stcp",
  "xtcp",
] as const;

export type ProtocolType = (typeof PROTOCOL_TYPES)[number];

export interface ProxyRuleSource {
  name?: string;
  type?: string;
  localIp?: string;
  localIP?: string;
  localPort?: string | number;
  remotePort?: string | number;
  customDomains?: string | string[];
}

export interface ProxyRuleForm {
  name: string;
  type: ProtocolType;
  localIp: string;
  localPort: string;
  remotePort: string;
  customDomains: string;
}

export interface ProxyRuleSavePayload extends ProxyRuleForm {
  editMode?: boolean;
  editIndex?: number;
}

export type ProxyRuleField =
  | "name"
  | "localIp"
  | "localPort"
  | "remotePort"
  | "customDomains";

export type ProxyRuleErrorCode = "required" | "invalidPort";
export type ProxyRuleErrors = Partial<
  Record<ProxyRuleField, ProxyRuleErrorCode>
>;

const asInputString = (value: string | number | undefined): string =>
  value === undefined ? "" : String(value);

const normalizeProtocol = (value: string | undefined): ProtocolType =>
  PROTOCOL_TYPES.includes(value as ProtocolType)
    ? (value as ProtocolType)
    : "tcp";

const isValidPort = (value: string): boolean => {
  const number = Number(value);
  return Number.isInteger(number) && number >= 1 && number <= 65535;
};

export const toProxyRuleForm = (
  source?: ProxyRuleSource,
): ProxyRuleForm => ({
  name: source?.name ?? "",
  type: normalizeProtocol(source?.type),
  localIp: source?.localIp ?? source?.localIP ?? "127.0.0.1",
  localPort: asInputString(source?.localPort),
  remotePort: asInputString(source?.remotePort),
  customDomains: Array.isArray(source?.customDomains)
    ? source.customDomains.join(", ")
    : source?.customDomains ?? "",
});

export const validateProxyRuleForm = (
  form: ProxyRuleForm,
): ProxyRuleErrors => {
  const errors: ProxyRuleErrors = {};
  const isHttp = form.type === "http" || form.type === "https";

  if (!form.name.trim()) errors.name = "required";
  if (!form.localIp.trim()) errors.localIp = "required";
  if (!isValidPort(form.localPort)) errors.localPort = "invalidPort";

  if (isHttp) {
    const domains = form.customDomains
      .split(",")
      .map((domain) => domain.trim())
      .filter(Boolean);
    if (domains.length === 0) errors.customDomains = "required";
  } else if (!isValidPort(form.remotePort)) {
    errors.remotePort = "invalidPort";
  }

  return errors;
};
```

This baseline preserves the current STCP/XTCP form behavior. Their protocol-specific fields are handled by the typed ConfigRepository work package rather than guessed here.

- [ ] **Step 4: 让 ProtocolForm 使用 typed state 和可见错误**

Replace the `<script setup>` block in `src/components/ProtocolForm.vue` with:

```vue
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { GitBranch, Save, X } from "lucide-vue-next";
import {
  PROTOCOL_TYPES,
  toProxyRuleForm,
  validateProxyRuleForm,
  type ProxyRuleErrors,
  type ProxyRuleField,
  type ProxyRuleSavePayload,
  type ProxyRuleSource,
} from "@/domain/proxyRule";

const props = defineProps<{
  initialData?: ProxyRuleSource;
  editMode?: boolean;
  editIndex?: number;
}>();

const emit = defineEmits<{
  (event: "save", payload: ProxyRuleSavePayload): void;
  (event: "cancel"): void;
}>();

const { t } = useI18n();
const form = ref(toProxyRuleForm(props.initialData));
const errors = ref<ProxyRuleErrors>({});
const protocolTypes = PROTOCOL_TYPES;
const isHttp = computed(
  () => form.value.type === "http" || form.value.type === "https",
);

watch(
  () => props.initialData,
  (value) => {
    form.value = toProxyRuleForm(value);
    errors.value = {};
  },
  { deep: true },
);

const validationStatus = (field: ProxyRuleField) =>
  errors.value[field] ? "error" : undefined;

const validationFeedback = (field: ProxyRuleField) => {
  const code = errors.value[field];
  return code ? t(`forms.validation.${code}`) : undefined;
};

const handleSave = () => {
  errors.value = validateProxyRuleForm(form.value);
  if (Object.keys(errors.value).length > 0) return;

  emit("save", {
    ...form.value,
    editMode: props.editMode,
    editIndex: props.editIndex,
  });
};
</script>
```

For each validated `<n-form-item>`, bind status and feedback. Example for local port:

```vue
<n-form-item
  :label="$t('forms.localPort')"
  path="localPort"
  :validation-status="validationStatus('localPort')"
  :feedback="validationFeedback('localPort')"
>
  <n-input v-model:value="form.localPort" placeholder="8080" />
</n-form-item>
```

Apply the same pattern to `name`, `localIp`, `remotePort`, and `customDomains`. For `customDomains`, use this non-error fallback:

```vue
:feedback="validationFeedback('customDomains') || $t('forms.customDomainsHint')"
```

Replace the hardcoded title description with:

```vue
{{ editMode ? $t("forms.ruleEditDesc") : $t("forms.ruleCreateDesc") }}
```

- [ ] **Step 5: 补齐中英文反馈文案**

Add these keys to both locale branches in `src/i18n.ts`.

English:

```ts
feedback: {
  // keep existing keys
  deleteSuccess: "Rule deleted successfully",
  ruleSaveSuccess: "Rule saved successfully",
},
forms: {
  // keep existing keys
  editRuleTitle: "Edit Mapping Rule",
  ruleCreateDesc: "Create a mapping from a local service to the public server.",
  ruleEditDesc: "Update the selected local-to-public mapping.",
  customDomainsHint: "Separate multiple domains with commas",
  validation: {
    required: "This field is required",
    invalidPort: "Enter a port from 1 to 65535",
  },
},
actions: {
  openExternal: "Open in a new window",
  editRule: "Edit rule",
  deleteRule: "Delete rule",
  confirmDeleteRule: "Delete this rule?",
},
app: {
  // keep existing keys
  closeFailedTitle: "Unable to quit",
  closeFailedContent: "Failed to stop FRP processes: {error}",
},
```

Chinese:

```ts
feedback: {
  // keep existing keys
  deleteSuccess: "规则已删除",
  ruleSaveSuccess: "规则已保存",
},
forms: {
  // keep existing keys
  editRuleTitle: "编辑映射规则",
  ruleCreateDesc: "创建一条从本地服务到公网服务器的映射。",
  ruleEditDesc: "修改当前本地服务到公网服务器的映射。",
  customDomainsHint: "多个域名使用逗号分隔",
  validation: {
    required: "此项为必填项",
    invalidPort: "请输入 1–65535 的端口",
  },
},
actions: {
  openExternal: "在新窗口打开",
  editRule: "编辑规则",
  deleteRule: "删除规则",
  confirmDeleteRule: "确定删除该规则吗？",
},
app: {
  // keep existing keys
  closeFailedTitle: "无法退出应用",
  closeFailedContent: "停止 FRP 进程失败：{error}",
},
```

The `// keep existing keys` comments above describe merge locations; do not replace the existing objects.

- [ ] **Step 6: 修正 Dashboard 类型和文案**

Add:

```ts
import type { ProxyRuleSavePayload } from "@/domain/proxyRule";
```

Replace the complete rule save handler:

```ts
const handleSaveRule = async (payload: ProxyRuleSavePayload) => {
  try {
    const configObj: Record<string, any> =
      parsedFrpcConfig.value || { proxies: [] };
    if (!Array.isArray(configObj.proxies)) {
      configObj.proxies = [];
    }

    const newRule: Record<string, unknown> = {
      name: payload.name,
      type: payload.type,
      localIp: payload.localIp || "127.0.0.1",
      localPort: Number(payload.localPort),
    };

    if (["http", "https"].includes(payload.type)) {
      newRule.customDomains = payload.customDomains
        .split(",")
        .map((domain) => domain.trim())
        .filter(Boolean);
    } else {
      newRule.remotePort = Number(payload.remotePort);
    }

    if (
      payload.editMode &&
      typeof payload.editIndex === "number" &&
      payload.editIndex >= 0
    ) {
      configObj.proxies[payload.editIndex] = newRule;
    } else {
      configObj.proxies.push(newRule);
    }

    const updatedConfigStr = stringify(configObj);
    await invoke("save_frpc_config", {
      configContent: updatedConfigStr,
    });
    await loadFrpcConfig();
    showAddForm.value = false;
    message.success(t("feedback.ruleSaveSuccess"));
  } catch (error) {
    console.error("Failed to save proxy rule:", error);
    message.error(t("feedback.saveFail", { error: String(error) }));
  }
};
```

Replace success messages:

```ts
message.success(t("feedback.deleteSuccess"));
message.success(t("feedback.ruleSaveSuccess"));
```

Replace the rule action labels and confirmation:

```vue
:title="$t('actions.openExternal')"
:title="$t('actions.editRule')"
:aria-label="$t('actions.deleteRule')"
{{ $t("actions.confirmDeleteRule") }}
```

- [ ] **Step 7: 运行规则测试、全部前端测试和类型检查**

Run:

```powershell
pnpm test:run -- src/domain/proxyRule.test.ts
pnpm test:run
pnpm typecheck
```

Expected: 5 proxy rule tests PASS, all frontend tests PASS, typecheck exit code 0。

- [ ] **Step 8: 准备审阅 checkpoint**

Run:

```powershell
git diff -- src/domain/proxyRule.ts src/domain/proxyRule.test.ts src/components/ProtocolForm.vue src/i18n.ts src/views/Dashboard.vue
```

Suggested commit message if authorized:

```text
fix: validate and restore proxy rule form data
```

---

### Task 4: 为 Rust 配置校验建立测试缝

**Files:**
- Modify: `src-tauri/src/config_parser.rs:1-73`

**Interfaces:**
- Consumes: TOML 字符串。
- Produces: `validate_toml(config_content: &str) -> Result<(), String>`，供 frpc/frps 保存路径共享。

- [ ] **Step 1: 写 TOML 校验的失败测试**

Append to `src-tauri/src/config_parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::validate_toml;

    #[test]
    fn accepts_escaped_secret_values() {
        let config = r#"
auth.method = "token"
auth.token = "quote\"and\\slash"
"#;

        assert!(validate_toml(config).is_ok());
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(validate_toml("auth.token = \"unterminated").is_err());
    }
}
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_parser::tests
```

Expected: FAIL because `validate_toml` is not defined.

- [ ] **Step 3: 实现共享校验函数**

Add above the save commands:

```rust
fn validate_toml(config_content: &str) -> Result<(), String> {
    config_content
        .parse::<DocumentMut>()
        .map(|_| ())
        .map_err(|error| format!("无效的 TOML 格式: {}", error))
}
```

Replace the duplicated parser in both save commands with:

```rust
validate_toml(&config_content)?;
```

- [ ] **Step 4: 运行目标测试和 Rust 格式化**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" config_parser::tests
cargo fmt --manifest-path "src-tauri/Cargo.toml"
```

Expected: 2 tests PASS; formatter exit code 0。

- [ ] **Step 5: 准备审阅 checkpoint**

Run:

```powershell
git diff -- src-tauri/src/config_parser.rs
```

Suggested commit message if authorized:

```text
test: cover shared TOML syntax validation
```

---

### Task 5: 统一窗口、托盘和系统退出清理

**Files:**
- Modify: `src-tauri/src/process_manager.rs:1-198`
- Modify: `src-tauri/src/lib.rs:1-154`
- Modify: `src/App.vue:1-137`
- Modify: `src/i18n.ts:124-129,251-256`

**Interfaces:**
- Consumes: 现有 `AppState` 中的 frpc/frps `CommandChild`。
- Produces:
  - `StopSummary { frpc_stopped: bool, frps_stopped: bool }`
  - `stop_all_state(state: &AppState) -> Result<StopSummary, String>`
  - `stop_all(app: &AppHandle) -> Result<StopSummary, String>`
  - Tauri command `prepare_shutdown(app: AppHandle) -> Result<(), String>`

- [ ] **Step 1: 写空状态幂等停止的失败测试**

Append to `src-tauri/src/process_manager.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{stop_all_state, AppState, StopSummary};

    #[test]
    fn stop_all_is_idempotent_when_nothing_is_running() {
        let state = AppState::default();

        assert_eq!(stop_all_state(&state).unwrap(), StopSummary::default());
        assert_eq!(stop_all_state(&state).unwrap(), StopSummary::default());
    }
}
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" process_manager::tests
```

Expected: FAIL because `AppState::default`, `StopSummary`, and `stop_all_state` are not defined.

- [ ] **Step 3: 实现共享停止函数**

At the top of `src-tauri/src/process_manager.rs`, import `serde::Serialize` and derive `Default`:

```rust
use serde::Serialize;

#[derive(Default)]
pub struct AppState {
    pub frpc_process: Mutex<Option<CommandChild>>,
    pub frps_process: Mutex<Option<CommandChild>>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct StopSummary {
    pub frpc_stopped: bool,
    pub frps_stopped: bool,
}
```

Add these helpers before the stop commands:

```rust
fn stop_slot(
    slot: &Mutex<Option<CommandChild>>,
    process_name: &str,
) -> Result<bool, String> {
    let child = slot
        .lock()
        .map_err(|_| format!("{} 进程状态锁已损坏", process_name))?
        .take();

    match child {
        Some(child) => {
            child
                .kill()
                .map_err(|error| format!("终止 {} 失败: {}", process_name, error))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

pub fn stop_all_state(state: &AppState) -> Result<StopSummary, String> {
    let frpc_result = stop_slot(&state.frpc_process, "frpc");
    let frps_result = stop_slot(&state.frps_process, "frps");
    let mut errors = Vec::new();

    let frpc_stopped = match frpc_result {
        Ok(stopped) => stopped,
        Err(error) => {
            errors.push(error);
            false
        }
    };
    let frps_stopped = match frps_result {
        Ok(stopped) => stopped,
        Err(error) => {
            errors.push(error);
            false
        }
    };

    if errors.is_empty() {
        Ok(StopSummary {
            frpc_stopped,
            frps_stopped,
        })
    } else {
        Err(errors.join("; "))
    }
}

pub fn stop_all(app: &AppHandle) -> Result<StopSummary, String> {
    let state = app.state::<AppState>();
    stop_all_state(&state)
}
```

Refactor the existing stop commands to use `stop_slot`:

```rust
#[tauri::command]
pub fn stop_frpc(app: AppHandle) -> Result<String, String> {
    let stopped = stop_slot(&app.state::<AppState>().frpc_process, "frpc")?;
    Ok(if stopped {
        "frpc 进程已停止"
    } else {
        "当前没有运行的 frpc 进程"
    }
    .to_string())
}

#[tauri::command]
pub fn stop_frps(app: AppHandle) -> Result<String, String> {
    let stopped = stop_slot(&app.state::<AppState>().frps_process, "frps")?;
    Ok(if stopped {
        "frps 进程已停止"
    } else {
        "当前没有运行的 frps 进程"
    }
    .to_string())
}
```

- [ ] **Step 4: 运行 Rust 目标测试并确认绿灯**

Run:

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" process_manager::tests
```

Expected: 1 test PASS。

- [ ] **Step 5: 在 Rust 装配层统一退出**

Add to `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn prepare_shutdown(app: tauri::AppHandle) -> Result<(), String> {
    process_manager::stop_all(&app).map(|_| ())
}
```

Use `AppState::default()`:

```rust
.manage(process_manager::AppState::default())
```

Replace the tray quit branch:

```rust
"quit" => {
    if let Err(error) = process_manager::stop_all(app) {
        eprintln!("退出前停止 FRP 进程失败: {}", error);
    }
    app.exit(0);
}
```

Register the command:

```rust
prepare_shutdown,
```

Change the builder expression at the start of `run()` to assign the built app:

```rust
let app = tauri::Builder::default()
```

Then replace the final `.run(...).expect(...)` call by building and handling Tauri events:

```rust
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
            if let Err(error) = process_manager::stop_all(app_handle) {
                eprintln!("应用退出清理 FRP 进程失败: {}", error);
            }
        }
        _ => {}
    });
```

This follows Tauri 2's `Builder::build` + `App::run` lifecycle. Do not call `prevent_exit()` because the baseline cleanup is synchronous.

- [ ] **Step 6: 让窗口退出先调用 prepare_shutdown**

In `src/App.vue`, add:

```ts
import { invoke } from "@tauri-apps/api/core";
```

Add:

```ts
const discreteDialog = () =>
  createDiscreteApi(["dialog"], {
    configProviderProps: computed(() => ({
      theme: theme.value,
      themeOverrides: themeOverrides.value,
    })),
  }).dialog;

const quitApplication = async () => {
  try {
    await invoke("prepare_shutdown");
    await appWindow.close();
  } catch (error) {
    discreteDialog().error({
      title: t("app.closeFailedTitle"),
      content: t("app.closeFailedContent", { error: String(error) }),
      positiveText: t("forms.confirm"),
    });
  }
};
```

Add `forms.confirm` in both locales:

```ts
confirm: "Confirm",
```

```ts
confirm: "确认",
```

Replace `handleClose` with:

```ts
const handleClose = () => {
  if (frpcRunning.value || frpsRunning.value) {
    discreteDialog().warning({
      title: t("app.closeTitle"),
      content: t("app.closeRunningContent"),
      positiveText: t("app.minimizeToTray"),
      negativeText: t("app.closeApp"),
      onPositiveClick: () => appWindow.hide(),
      onNegativeClick: () => quitApplication(),
    });
  } else {
    void quitApplication();
  }
};
```

- [ ] **Step 7: 运行 Rust、前端测试和构建**

Run:

```powershell
cargo fmt --manifest-path "src-tauri/Cargo.toml"
cargo test --manifest-path "src-tauri/Cargo.toml"
pnpm test:run
pnpm build
```

Expected: all Rust tests PASS, all Vitest tests PASS, frontend build exit code 0。

- [ ] **Step 8: Windows 手动验证窗口退出**

1. 确保测试前没有其他业务需要的 frpc/frps 进程。
2. 在应用内启动一个可用的 frpc 或 frps。
3. 记录由本应用启动的进程：

```powershell
Get-CimInstance Win32_Process |
  Where-Object { $_.Name -in @("frpc.exe", "frps.exe") } |
  Select-Object ProcessId, Name, CommandLine
```

4. 点击窗口关闭，选择“退出应用”。
5. 等待 2 秒并重新运行同一查询。

Expected: 对应本应用配置路径的进程不再存在。不得终止命令行不属于本应用配置目录的其他 FRP 进程。

- [ ] **Step 9: Windows 手动验证托盘退出**

1. 重新启动应用和一个 FRP 进程。
2. 隐藏到托盘。
3. 从托盘菜单选择“退出”。
4. 等待 2 秒并执行相同的 `Get-CimInstance` 查询。

Expected: 应用与其启动的 FRP 进程都不再存在。

- [ ] **Step 10: 准备审阅 checkpoint**

Run:

```powershell
git diff -- src-tauri/src/process_manager.rs src-tauri/src/lib.rs src/App.vue src/i18n.ts
```

Suggested commit message if authorized:

```text
fix: stop FRP processes on every app exit path
```

---

### Task 6: 同步文档并完成安全基线验证

**Files:**
- Modify: `README.md:20-29,57-62`
- Verify: all files changed by Tasks 1-5

**Interfaces:**
- Consumes: 完成的安全基线行为。
- Produces: 与实际托盘、配置和退出行为一致的 README；完整验证记录。

- [ ] **Step 1: 更新 README 的托盘描述**

Replace the “即将支持” feature line with:

```md
- **托盘与后台运行**：关闭窗口时可选择留守后台；从窗口或托盘退出会先停止由应用启动的 `frpc` / `frps` 进程。
```

Under “一键启停”, add:

```md
- 退出应用会先停止由 Avocado FRP 启动的服务；如果希望服务继续运行，请选择“留守后台”。
```

Do not claim syntax highlighting is complete in this work package; the README editor claim will be corrected when the CodeMirror console work package lands.

- [ ] **Step 2: 运行完整自动化验证**

Run:

```powershell
pnpm test:run
pnpm typecheck
pnpm build
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" -- -D warnings
git diff --check
```

Expected:

- Vitest: all tests PASS.
- vue-tsc: exit code 0.
- Vite build: exit code 0.
- cargo fmt: exit code 0.
- cargo test: all tests PASS.
- cargo clippy: exit code 0 with no warnings.
- git diff --check: no output.

- [ ] **Step 3: 检查业务代码诊断**

Read IDE diagnostics for:

```text
src/lib/preferences.ts
src/lib/preferences.test.ts
src/domain/proxyRule.ts
src/domain/proxyRule.test.ts
src/components/ProtocolForm.vue
src/i18n.ts
src/App.vue
src/views/Dashboard.vue
src-tauri/src/config_parser.rs
src-tauri/src/process_manager.rs
src-tauri/src/lib.rs
```

Expected: no newly introduced diagnostics.

- [ ] **Step 4: 检查最终工作树边界**

Run:

```powershell
git status --short
git diff --stat
```

Expected:

- 两个 macOS sidecar 不再显示为删除。
- 只包含本计划明确列出的源码、测试、锁文件、README 和 docs 变更。
- `.codegraph/` 与 `.cursor/` 仍未跟踪且未暂存。
- 没有生成的 `dist/`、`target/` 或日志文件进入 Git diff。

- [ ] **Step 5: 准备工作包交付 checkpoint**

Suggested commit series if the user explicitly authorizes commits:

```text
fix: restore macOS frpc release inputs
fix: persist and migrate language preference
fix: validate and restore proxy rule form data
test: cover shared TOML syntax validation
fix: stop FRP processes on every app exit path
docs: document safe tray and exit behavior
```

Do not run these commits without explicit authorization.
