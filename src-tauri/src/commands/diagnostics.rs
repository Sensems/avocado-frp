use tauri::State;

use crate::domain::diagnostics::DiagnosticsReport;
use crate::domain::error::CommandError;
use crate::AppServices;

#[tauri::command]
pub async fn run_diagnostics(
    services: State<'_, AppServices>,
) -> Result<DiagnosticsReport, CommandError> {
    services.diagnostics.run_all().await
}

#[tauri::command]
pub async fn export_diagnostics_pack(
    path: String,
    services: State<'_, AppServices>,
) -> Result<String, CommandError> {
    services.diagnostics.export_pack(path).await
}
