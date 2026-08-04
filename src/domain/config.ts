import type { ProxyRuleForm } from './proxyRule'

export type ConfigKind = 'frpc' | 'frps'

export type PatchValue<T> = T | null | undefined

export type ValidationSeverity = 'error' | 'warning'

export interface ValidationIssue {
  severity: ValidationSeverity
  code: string
  message: string
  path?: string
  line?: number
  column?: number
}

export interface ValidationReport {
  issues: ValidationIssue[]
}

export interface AuthKnownConfig {
  method?: string
  token?: string
}

export interface WebServerKnownConfig {
  addr?: string
  port?: number
  user?: string
  password?: string
}

export interface ProxyRuleKnown {
  name?: string
  type?: string
  localIp?: string
  localPort?: number
  remotePort?: number
  customDomains?: string[]
  sourceIndex: number
  sourceName: string
}

export interface FrpcKnownConfig {
  serverAddr?: string
  serverPort?: number
  auth: AuthKnownConfig
  webServer: WebServerKnownConfig
  proxies: ProxyRuleKnown[]
}

export interface FrpsKnownConfig {
  bindPort?: number
  vhostHTTPPort?: number
  vhostHTTPSPort?: number
  auth: AuthKnownConfig
  webServer: WebServerKnownConfig
}

export type ConfigSnapshot =
  | {
      kind: 'frpc'
      raw: string
      revision: string
      known: FrpcKnownConfig
      issues: ValidationIssue[]
      backupAvailable: boolean
    }
  | {
      kind: 'frps'
      raw: string
      revision: string
      known: FrpsKnownConfig
      issues: ValidationIssue[]
      backupAvailable: boolean
    }

export type FrpcConfigSnapshot = Extract<ConfigSnapshot, { kind: 'frpc' }>
export type FrpsConfigSnapshot = Extract<ConfigSnapshot, { kind: 'frps' }>

export interface AuthConfigPatch {
  method?: PatchValue<string>
  token?: PatchValue<string>
}

export interface WebServerConfigPatch {
  addr?: PatchValue<string>
  port?: PatchValue<number>
  user?: PatchValue<string>
  password?: PatchValue<string>
}

export interface ProxyRulePatch {
  name?: PatchValue<string>
  type?: PatchValue<string>
  localIP?: PatchValue<string>
  localPort?: PatchValue<number>
  remotePort?: PatchValue<number>
  customDomains?: PatchValue<string[]>
}

export interface ProxySelector {
  index: number
  originalName: string
}

export type ProxyOperation =
  | { op: 'add'; rule: ProxyRulePatch }
  | { op: 'update'; selector: ProxySelector; patch: ProxyRulePatch }
  | { op: 'delete'; selector: ProxySelector }

export interface FrpcConfigPatch {
  serverAddr?: PatchValue<string>
  serverPort?: PatchValue<number>
  auth?: AuthConfigPatch
  webServer?: WebServerConfigPatch
  proxyOperations?: ProxyOperation[]
}

export interface FrpsConfigPatch {
  bindPort?: PatchValue<number>
  vhostHTTPPort?: PatchValue<number>
  vhostHTTPSPort?: PatchValue<number>
  auth?: AuthConfigPatch
  webServer?: WebServerConfigPatch
}

export type FrpcChange =
  | { mode: 'patch'; patch: FrpcConfigPatch }
  | { mode: 'source'; raw: string }

export type FrpsChange =
  | { mode: 'patch'; patch: FrpsConfigPatch }
  | { mode: 'source'; raw: string }

export type ConfigChangeRequest =
  | {
      kind: 'frpc'
      expectedRevision: string
      change: FrpcChange
    }
  | {
      kind: 'frps'
      expectedRevision: string
      change: FrpsChange
    }

export interface ConfigDiff {
  unified: string
  changedPaths: string[]
  requiresConfirmation: boolean
}

export interface ConfigPreview {
  diff: ConfigDiff
  issues: ValidationIssue[]
}

export interface SaveAndRestartRecovery {
  configRestored: boolean
  processRestored: boolean
  error?: import('./errors').CommandError
}

export interface SaveAndRestartResult {
  applied: boolean
  config: ConfigSnapshot
  process: import('./process').ProcessSnapshot
  failure?: import('./errors').CommandError
  recovery?: SaveAndRestartRecovery
}

export type PortInput = string | number | null | undefined

export interface FrpcGlobalFormData {
  serverAddr: string
  serverPort: PortInput
  authMethod: string | null
  token: string
}

export interface FrpsFormData {
  bindPort: PortInput
  vhostHttpPort: PortInput
  vhostHttpsPort: PortInput
  authMethod: string | null
  token: string
  dashboardPort: PortInput
  dashboardUser: string
  dashboardPwd: string
}

const stringPatchValue = (
  value: string | null | undefined,
): string | null => (value === null || value === undefined || value === '' ? null : value)

const portPatchValue = (value: PortInput): number | null =>
  value === null || value === undefined || value === '' ? null : Number(value)

export const buildFrpcGlobalPatch = (
  form: FrpcGlobalFormData,
): FrpcConfigPatch => ({
  serverAddr: stringPatchValue(form.serverAddr),
  serverPort: portPatchValue(form.serverPort),
  auth: {
    method: stringPatchValue(form.authMethod),
    token: stringPatchValue(form.token),
  },
  // webServer is applied only via Settings → Enable local monitor (explicit save).
})

export const buildFrpsPatch = (form: FrpsFormData): FrpsConfigPatch => ({
  bindPort: portPatchValue(form.bindPort),
  vhostHTTPPort: portPatchValue(form.vhostHttpPort),
  vhostHTTPSPort: portPatchValue(form.vhostHttpsPort),
  auth: {
    method: stringPatchValue(form.authMethod),
    token: stringPatchValue(form.token),
  },
  webServer: {
    port: portPatchValue(form.dashboardPort),
    user: stringPatchValue(form.dashboardUser),
    password: stringPatchValue(form.dashboardPwd),
  },
})

const buildProxyRulePatch = (form: ProxyRuleForm): ProxyRulePatch => {
  const isHttp = form.type === 'http' || form.type === 'https'
  const customDomains = form.customDomains
    .split(',')
    .map((domain) => domain.trim())
    .filter(Boolean)

  return {
    name: stringPatchValue(form.name),
    type: form.type,
    localIP: stringPatchValue(form.localIp),
    localPort: portPatchValue(form.localPort),
    remotePort: isHttp ? null : portPatchValue(form.remotePort),
    customDomains: isHttp && customDomains.length > 0 ? customDomains : null,
  }
}

export const buildProxyAddPatch = (form: ProxyRuleForm): ProxyOperation => ({
  op: 'add',
  rule: buildProxyRulePatch(form),
})

export const buildProxyUpdatePatch = (
  selector: ProxySelector,
  form: ProxyRuleForm,
): ProxyOperation => ({
  op: 'update',
  selector,
  patch: buildProxyRulePatch(form),
})

export const buildProxyDeletePatch = (
  selector: ProxySelector,
): ProxyOperation => ({
  op: 'delete',
  selector,
})
