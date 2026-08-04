use serde::{Deserialize, Serialize};

use super::error::CommandError;
use super::process::ProcessSnapshot;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigKind {
    Frpc,
    Frps,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthKnownConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebServerKnownConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyRuleKnown {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub proxy_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_domains: Option<Vec<String>>,
    pub source_index: usize,
    pub source_name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrpcKnownConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_port: Option<u16>,
    pub auth: AuthKnownConfig,
    pub web_server: WebServerKnownConfig,
    pub proxies: Vec<ProxyRuleKnown>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrpsKnownConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_port: Option<u16>,
    #[serde(rename = "vhostHTTPPort", skip_serializing_if = "Option::is_none")]
    pub vhost_http_port: Option<u16>,
    #[serde(rename = "vhostHTTPSPort", skip_serializing_if = "Option::is_none")]
    pub vhost_https_port: Option<u16>,
    pub auth: AuthKnownConfig,
    pub web_server: WebServerKnownConfig,
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

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfigPatch {
    #[serde(default, with = "serde_with::rust::double_option")]
    pub method: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub token: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebServerConfigPatch {
    #[serde(default, with = "serde_with::rust::double_option")]
    pub addr: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub port: Option<Option<u16>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub user: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub password: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyRulePatch {
    #[serde(default, with = "serde_with::rust::double_option")]
    pub name: Option<Option<String>>,
    #[serde(rename = "type", default, with = "serde_with::rust::double_option")]
    pub proxy_type: Option<Option<String>>,
    #[serde(rename = "localIP", default, with = "serde_with::rust::double_option")]
    pub local_ip: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub local_port: Option<Option<u16>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub remote_port: Option<Option<u16>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub custom_domains: Option<Option<Vec<String>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxySelector {
    pub index: usize,
    pub original_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum ProxyOperation {
    Add {
        rule: ProxyRulePatch,
    },
    Update {
        selector: ProxySelector,
        patch: ProxyRulePatch,
    },
    Delete {
        selector: ProxySelector,
    },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrpcConfigPatch {
    #[serde(default, with = "serde_with::rust::double_option")]
    pub server_addr: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub server_port: Option<Option<u16>>,
    #[serde(default)]
    pub auth: Option<AuthConfigPatch>,
    #[serde(default)]
    pub web_server: Option<WebServerConfigPatch>,
    #[serde(default)]
    pub proxy_operations: Vec<ProxyOperation>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrpsConfigPatch {
    #[serde(default, with = "serde_with::rust::double_option")]
    pub bind_port: Option<Option<u16>>,
    #[serde(
        rename = "vhostHTTPPort",
        default,
        with = "serde_with::rust::double_option"
    )]
    pub vhost_http_port: Option<Option<u16>>,
    #[serde(
        rename = "vhostHTTPSPort",
        default,
        with = "serde_with::rust::double_option"
    )]
    pub vhost_https_port: Option<Option<u16>>,
    #[serde(default)]
    pub auth: Option<AuthConfigPatch>,
    #[serde(default)]
    pub web_server: Option<WebServerConfigPatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum FrpcChange {
    Patch { patch: FrpcConfigPatch },
    Source { raw: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum FrpsChange {
    Patch { patch: FrpsConfigPatch },
    Source { raw: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

impl ConfigChangeRequest {
    pub fn kind(&self) -> ConfigKind {
        match self {
            Self::Frpc { .. } => ConfigKind::Frpc,
            Self::Frps { .. } => ConfigKind::Frps,
        }
    }

    pub fn expected_revision(&self) -> &str {
        match self {
            Self::Frpc {
                expected_revision, ..
            }
            | Self::Frps {
                expected_revision, ..
            } => expected_revision,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDiff {
    pub unified: String,
    pub changed_paths: Vec<String>,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPreview {
    pub diff: ConfigDiff,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAndRestartRecovery {
    pub config_restored: bool,
    pub process_restored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAndRestartResult {
    pub applied: bool,
    pub config: ConfigSnapshot,
    pub process: ProcessSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<CommandError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<SaveAndRestartRecovery>,
}
