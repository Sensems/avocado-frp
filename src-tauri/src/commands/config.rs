use tauri::State;

use crate::domain::config::{
    ConfigChangeRequest, ConfigKind, ConfigPreview, ConfigSnapshot, SaveAndRestartResult,
    ValidationReport,
};
use crate::domain::error::CommandError;
use crate::AppServices;

#[tauri::command]
pub fn get_config_snapshot(
    kind: ConfigKind,
    services: State<'_, AppServices>,
) -> Result<ConfigSnapshot, CommandError> {
    services.config.load(kind)
}

#[tauri::command]
pub fn validate_config_source(
    kind: ConfigKind,
    raw: String,
    services: State<'_, AppServices>,
) -> ValidationReport {
    services.config.validate_source(kind, &raw)
}

#[tauri::command]
pub fn preview_config_change(
    request: ConfigChangeRequest,
    services: State<'_, AppServices>,
) -> Result<ConfigPreview, CommandError> {
    services.config.preview(request)
}

#[tauri::command]
pub async fn apply_config_change(
    request: ConfigChangeRequest,
    services: State<'_, AppServices>,
) -> Result<ConfigSnapshot, CommandError> {
    services.transactions.apply_change(request).await
}

#[tauri::command]
pub async fn restore_config_backup(
    kind: ConfigKind,
    expected_revision: String,
    services: State<'_, AppServices>,
) -> Result<ConfigSnapshot, CommandError> {
    services
        .transactions
        .restore_backup(kind, &expected_revision)
        .await
}

#[tauri::command]
pub async fn save_config_and_restart(
    request: ConfigChangeRequest,
    services: State<'_, AppServices>,
) -> Result<SaveAndRestartResult, CommandError> {
    services.transactions.save_and_restart(request).await
}
