use serde::{Deserialize, Serialize};

use super::config::{ConfigKind, ConfigSnapshot};
use super::process::{ProcessKind, ProcessSnapshot};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStateChangedEvent {
    #[serde(flatten)]
    pub snapshot: ProcessSnapshot,
}

impl From<ProcessSnapshot> for ProcessStateChangedEvent {
    fn from(snapshot: ProcessSnapshot) -> Self {
        Self { snapshot }
    }
}

impl From<&ProcessSnapshot> for ProcessStateChangedEvent {
    fn from(snapshot: &ProcessSnapshot) -> Self {
        Self {
            snapshot: snapshot.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangedEvent {
    pub kind: ConfigKind,
    pub revision: String,
}

impl From<&ConfigSnapshot> for ConfigChangedEvent {
    fn from(snapshot: &ConfigSnapshot) -> Self {
        Self {
            kind: snapshot.kind(),
            revision: snapshot.revision().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// Minimal log payload for the temporary `log://entry` event.
/// Timestamps are RFC 3339 strings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub kind: ProcessKind,
    pub stream: LogStream,
    pub text: String,
    pub timestamp: String,
}
