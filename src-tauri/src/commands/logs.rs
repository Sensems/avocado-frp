use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::process::ProcessKind;
use crate::AppServices;

#[tauri::command]
pub fn delete_disk_logs(
    kind: Option<ProcessKind>,
    services: State<'_, AppServices>,
) -> Result<(), CommandError> {
    services.logs.delete_disk_logs(kind)
}
