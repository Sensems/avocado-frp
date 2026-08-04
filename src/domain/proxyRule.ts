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

export const toProxyRuleForm = (source?: ProxyRuleSource): ProxyRuleForm => ({
  name: source?.name ?? "",
  type: normalizeProtocol(source?.type),
  localIp: source?.localIp ?? source?.localIP ?? "127.0.0.1",
  localPort: asInputString(source?.localPort),
  remotePort: asInputString(source?.remotePort),
  customDomains: Array.isArray(source?.customDomains)
    ? source.customDomains.join(", ")
    : (source?.customDomains ?? ""),
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
