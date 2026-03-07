use std::sync::Mutex;
use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_shell::{ShellExt, process::{CommandChild, CommandEvent}};

pub struct AppState {
    pub frpc_process: Mutex<Option<CommandChild>>,
    pub frps_process: Mutex<Option<CommandChild>>,
}

#[tauri::command]
pub fn get_frpc_status(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    let is_running = state.frpc_process.lock().unwrap().is_some();
    is_running
}

#[tauri::command]
pub fn get_frps_status(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    let is_running = state.frps_process.lock().unwrap().is_some();
    is_running
}

#[tauri::command]
pub fn start_frpc(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    
    if state.frpc_process.lock().unwrap().is_some() {
        return Ok("frpc 已经在运行中".to_string());
    }

    let config_dir = super::config_parser::get_config_dir(&app)?;
    let config_path = config_dir.join("frpc.toml");
    let config_path_str = config_path.to_string_lossy().to_string();
    
    let (mut rx, child) = app
        .shell()
        .sidecar("frpc")
        .map_err(|e| format!("侧边进程未找到: {}", e))?
        .args(["-c", &config_path_str])
        .spawn()
        .map_err(|e| format!("启动 frpc 失败: {}", e))?;

    *state.frpc_process.lock().unwrap() = Some(child);

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let _ = app_handle.emit("frpc-stdout", String::from_utf8_lossy(&line).to_string());
                }
                CommandEvent::Stderr(line) => {
                    let _ = app_handle.emit("frpc-stderr", String::from_utf8_lossy(&line).to_string());
                }
                CommandEvent::Terminated(payload) => {
                    // 进程退出后自动清理状态
                    let state = app_handle.state::<AppState>();
                    *state.frpc_process.lock().unwrap() = None;
                    let _ = app_handle.emit("frpc-terminated", payload.code);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok("frpc 启动成功".to_string())
}

#[tauri::command]
pub fn stop_frpc(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut process_lock = state.frpc_process.lock().unwrap();

    if let Some(child) = process_lock.take() {
        let _ = child.kill().map_err(|e| format!("终止进程失败: {}", e))?;
        return Ok("frpc 进程已停止".to_string());
    }

    Ok("当前没有运行的 frpc 进程".to_string())
}

#[tauri::command]
pub fn start_frps(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    
    if state.frps_process.lock().unwrap().is_some() {
        return Ok("frps 已经在运行中".to_string());
    }

    let config_dir = super::config_parser::get_config_dir(&app)?;
    let config_path = config_dir.join("frps.toml");
    let config_path_str = config_path.to_string_lossy().to_string();
    
    let (mut rx, child) = app
        .shell()
        .sidecar("frps")
        .map_err(|e| format!("侧边进程未找到: {}", e))?
        .args(["-c", &config_path_str])
        .spawn()
        .map_err(|e| format!("启动 frps 失败: {}", e))?;

    *state.frps_process.lock().unwrap() = Some(child);

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let _ = app_handle.emit("frps-stdout", String::from_utf8_lossy(&line).to_string());
                }
                CommandEvent::Stderr(line) => {
                    let _ = app_handle.emit("frps-stderr", String::from_utf8_lossy(&line).to_string());
                }
                CommandEvent::Terminated(payload) => {
                    // 进程退出后自动清理状态
                    let state = app_handle.state::<AppState>();
                    *state.frps_process.lock().unwrap() = None;
                    let _ = app_handle.emit("frps-terminated", payload.code);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok("frps 启动成功".to_string())
}

#[tauri::command]
pub fn stop_frps(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut process_lock = state.frps_process.lock().unwrap();

    if let Some(child) = process_lock.take() {
        let _ = child.kill().map_err(|e| format!("终止进程失败: {}", e))?;
        return Ok("frps 进程已停止".to_string());
    }

    Ok("当前没有运行的 frps 进程".to_string())
}

