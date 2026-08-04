use std::fmt;

use serde::{Deserialize, Serialize};

pub const APP_SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
pub const DEFAULT_MAX_ROTATED_FILES: u32 = 7;
pub const DEFAULT_MONITOR_ADDR: &str = "127.0.0.1";
pub const DEFAULT_MONITOR_PORT: u16 = 7400;
/// Used when a monitor password is set but user is omitted.
/// FRP 0.61.x defaults both to ""; official examples use `"admin"`.
pub const DEFAULT_MONITOR_USER: &str = "admin";
pub const MIN_MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const MAX_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
pub const MIN_MAX_ROTATED_FILES: u32 = 1;
pub const MAX_MAX_ROTATED_FILES: u32 = 30;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogPolicy {
    pub max_file_bytes: u64,
    pub max_rotated_files: u32,
}

impl Default for LogPolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_rotated_files: DEFAULT_MAX_ROTATED_FILES,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalMonitorPrefs {
    pub enabled: bool,
    pub addr: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl fmt::Debug for LocalMonitorPrefs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalMonitorPrefs")
            .field("enabled", &self.enabled)
            .field("addr", &self.addr)
            .field("port", &self.port)
            .field("user", &self.user)
            .field(
                "password",
                &self.password.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl Default for LocalMonitorPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            addr: DEFAULT_MONITOR_ADDR.to_string(),
            port: DEFAULT_MONITOR_PORT,
            user: None,
            password: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,
    pub log_policy: LogPolicy,
    pub local_monitor: LocalMonitorPrefs,
    pub log_policy_notice_shown: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: APP_SETTINGS_SCHEMA_VERSION,
            log_policy: LogPolicy::default(),
            local_monitor: LocalMonitorPrefs::default(),
            log_policy_notice_shown: false,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogPolicyPatch {
    #[serde(default)]
    pub max_file_bytes: Option<u64>,
    #[serde(default)]
    pub max_rotated_files: Option<u32>,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalMonitorPrefsPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub addr: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub user: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub password: Option<Option<String>>,
}

impl fmt::Debug for LocalMonitorPrefsPatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalMonitorPrefsPatch")
            .field("enabled", &self.enabled)
            .field("addr", &self.addr)
            .field("port", &self.port)
            .field("user", &self.user)
            .field(
                "password",
                &self.password.as_ref().map(|value| {
                    value.as_ref().map(|_| "[redacted]")
                }),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    #[serde(default)]
    pub log_policy: Option<LogPolicyPatch>,
    #[serde(default)]
    pub local_monitor: Option<LocalMonitorPrefsPatch>,
    #[serde(default)]
    pub log_policy_notice_shown: Option<bool>,
}
