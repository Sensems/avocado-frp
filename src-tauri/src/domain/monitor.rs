use serde::{Deserialize, Serialize};

use super::settings::AppSettings;

/// Structured local-monitor / traffic status for Overview empty states.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MonitorStatus {
    Disabled,
    ProcessStopped,
    PortConflict,
    AuthFailed,
    Timeout,
    Ok,
    NotConfigured,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FrpcTrafficResult {
    pub status: MonitorStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

impl FrpcTrafficResult {
    pub fn status_only(status: MonitorStatus) -> Self {
        Self {
            status,
            body: None,
        }
    }

    pub fn ok(body: String) -> Self {
        Self {
            status: MonitorStatus::Ok,
            body: Some(body),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyLocalMonitorRequest {
    pub enabled: bool,
    pub addr: String,
    pub port: u16,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyLocalMonitorResult {
    pub settings: AppSettings,
    pub config_patched: bool,
    pub pending_restart: bool,
}
