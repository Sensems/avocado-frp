use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ConfigInvalid,
    ConfigConflict,
    ConfigIo,
    SidecarMissing,
    SidecarIncompatible,
    PortConflict,
    ProcessAlreadyRunning,
    ProcessNotRunning,
    SpawnFailed,
    HealthcheckFailed,
    StopTimeout,
    PermissionDenied,
    NetworkUnreachable,
    UpdateFailed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

impl CommandError {
    pub fn new(code: ErrorCode, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            recoverable,
            suggested_action: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_suggested_action(mut self, action: impl Into<String>) -> Self {
        self.suggested_action = Some(action.into());
        self
    }

    pub fn from_io(error: std::io::Error) -> Self {
        let code = match error.kind() {
            std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            _ => ErrorCode::ConfigIo,
        };
        Self::new(code, error.to_string(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_uses_stable_error_code_and_camel_case_fields() {
        let error = CommandError::new(ErrorCode::ConfigConflict, "configuration changed", true)
            .with_detail("reload before saving");

        let json = serde_json::to_value(error).unwrap();
        assert_eq!(json["code"], "CONFIG_CONFLICT");
        assert_eq!(json["recoverable"], true);
        assert_eq!(json["detail"], "reload before saving");
    }
}
