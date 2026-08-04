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
