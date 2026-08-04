use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticResult {
    pub id: String,
    pub status: DiagnosticStatus,
    /// Stable i18n key suffix under `diagnostics.checks.*` (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_key: Option<String>,
    pub detail: String,
    /// Stable action code mapped by frontend i18n (`diagnostics.actions.*`).
    pub suggested_action: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub started_at: String,
    pub finished_at: String,
    pub results: Vec<DiagnosticResult>,
}

/// Stable suggested-action codes (prefer these over free-form text).
pub mod action {
    pub const NONE: &str = "NONE";
    pub const REINSTALL_SIDECAR: &str = "REINSTALL_SIDECAR";
    pub const FIX_CONFIG: &str = "FIX_CONFIG";
    pub const CHANGE_PORT: &str = "CHANGE_PORT";
    pub const STOP_CONFLICTING_PROCESS: &str = "STOP_CONFLICTING_PROCESS";
    pub const CHECK_SERVER_ADDR: &str = "CHECK_SERVER_ADDR";
    pub const CONFIGURE_WEBSERVER: &str = "CONFIGURE_WEBSERVER";
    pub const START_PROCESS: &str = "START_PROCESS";
    pub const CHECK_ADMIN_API: &str = "CHECK_ADMIN_API";
    pub const CHECK_ADMIN_AUTH: &str = "CHECK_ADMIN_AUTH";
    pub const FIX_DIRECTORY_PERMISSIONS: &str = "FIX_DIRECTORY_PERMISSIONS";
    /// Reserved for WP5 updater checks; kept for stable action-code parity with i18n.
    #[allow(dead_code)]
    pub const UPDATER_DEFERRED_WP5: &str = "UPDATER_DEFERRED_WP5";
}
