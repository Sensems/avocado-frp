use serde::{Deserialize, Serialize};

use super::error::CommandError;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_snapshot_serializes_frontend_contract() {
        let snapshot = ProcessSnapshot::stopped(ProcessKind::Frpc);
        let json = serde_json::to_value(snapshot).unwrap();

        assert_eq!(json["kind"], "frpc");
        assert_eq!(json["phase"], "stopped");
        assert_eq!(json["uptimeSeconds"], 0);
    }
}
