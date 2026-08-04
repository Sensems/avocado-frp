use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::process::{ProcessKind, ProcessSnapshot, StopAllResult};
use crate::AppServices;

#[tauri::command]
pub async fn get_process_snapshot(
    kind: ProcessKind,
    services: State<'_, AppServices>,
) -> Result<ProcessSnapshot, CommandError> {
    Ok(services.processes.snapshot(kind).await)
}

#[tauri::command]
pub async fn start_process(
    kind: ProcessKind,
    services: State<'_, AppServices>,
) -> Result<ProcessSnapshot, CommandError> {
    services.processes.start(kind).await
}

#[tauri::command]
pub async fn stop_process(
    kind: ProcessKind,
    services: State<'_, AppServices>,
) -> Result<ProcessSnapshot, CommandError> {
    services.processes.stop(kind).await
}

#[tauri::command]
pub async fn restart_process(
    kind: ProcessKind,
    services: State<'_, AppServices>,
) -> Result<ProcessSnapshot, CommandError> {
    services.processes.restart(kind).await
}

#[tauri::command]
pub async fn stop_all_processes(
    services: State<'_, AppServices>,
) -> Result<StopAllResult, CommandError> {
    services.processes.stop_all().await
}

#[tauri::command]
pub async fn prepare_shutdown(
    services: State<'_, AppServices>,
) -> Result<StopAllResult, CommandError> {
    services.shutdown.prepare().await
}
