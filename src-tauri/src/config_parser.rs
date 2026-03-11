use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use toml_edit::DocumentMut;

pub fn get_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resolver = app.path();
    let mut config_dir = resolver.app_config_dir()
        .map_err(|e| format!("无法获取应用配置目录: {}", e))?;
        
    #[cfg(debug_assertions)]
    {
        config_dir = config_dir.join("dev_data");
    }
    
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("无法创建配置目录: {}", e))?;
    }
    
    Ok(config_dir)
}

#[tauri::command]
pub fn read_frpc_config(app: AppHandle) -> Result<String, String> {
    let config_dir = get_config_dir(&app)?;
    let file_path = config_dir.join("frpc.toml");
    
    if !file_path.exists() {
        return Ok(String::new());
    }
    
    fs::read_to_string(file_path)
        .map_err(|e| format!("读取 frpc.toml 失败: {}", e))
}

#[tauri::command]
pub fn save_frpc_config(app: AppHandle, config_content: String) -> Result<(), String> {
    let config_dir = get_config_dir(&app)?;
    let file_path = config_dir.join("frpc.toml");
    
    // 基础验证是否为有效的 toml
    let _parsed: DocumentMut = config_content.parse()
        .map_err(|e| format!("无效的 TOML 格式: {}", e))?;
        
    fs::write(file_path, config_content)
        .map_err(|e| format!("保存 frpc.toml 失败: {}", e))
}

#[tauri::command]
pub fn read_frps_config(app: AppHandle) -> Result<String, String> {
    let config_dir = get_config_dir(&app)?;
    let file_path = config_dir.join("frps.toml");
    
    if !file_path.exists() {
        return Ok(String::new());
    }
    
    fs::read_to_string(file_path)
        .map_err(|e| format!("读取 frps.toml 失败: {}", e))
}

#[tauri::command]
pub fn save_frps_config(app: AppHandle, config_content: String) -> Result<(), String> {
    let config_dir = get_config_dir(&app)?;
    let file_path = config_dir.join("frps.toml");
    
    let _parsed: DocumentMut = config_content.parse()
        .map_err(|e| format!("无效的 TOML 格式: {}", e))?;
        
    fs::write(file_path, config_content)
        .map_err(|e| format!("保存 frps.toml 失败: {}", e))
}
