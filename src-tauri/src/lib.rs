// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod config_parser;
mod process_manager;

#[tauri::command]
fn export_deploy_script(path: &str, toml_content: &str) -> Result<(), String> {
    let dir = std::path::Path::new(path);
    std::fs::write(dir.join("frps.toml"), toml_content).map_err(|e| e.to_string())?;
    
    let sh_content = format!(r#"#!/bin/bash
echo "Installing frps..."
mkdir -p /etc/frp
cp frps.toml /etc/frp/frps.toml
wget https://github.com/fatedier/frp/releases/download/v0.61.1/frp_0.61.1_linux_amd64.tar.gz -O /tmp/frp.tar.gz
tar -zxvf /tmp/frp.tar.gz -C /tmp
cp /tmp/frp_0.61.1_linux_amd64/frps /usr/bin/frps
chmod +x /usr/bin/frps

cat <<EOF > /etc/systemd/system/frps.service
[Unit]
Description=Frp Server Service
After=network.target

[Service]
Type=simple
User=nobody
Restart=on-failure
RestartSec=5s
ExecStart=/usr/bin/frps -c /etc/frp/frps.toml
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable frps
systemctl start frps
echo "Frps installed and started successfully!"
"#);
    std::fs::write(dir.join("install.sh"), sh_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 导出日志文件到用户指定目录
#[tauri::command]
fn export_logs(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let config_dir = config_parser::get_config_dir(&app)?;
    let log_dir = config_dir.join("logs");

    if !log_dir.exists() {
        return Err("暂无日志文件".to_string());
    }

    let dest = std::path::Path::new(&path);
    let mut count = 0u32;

    // 遍历日志目录，复制所有 .log 文件
    let entries = std::fs::read_dir(&log_dir).map_err(|e| format!("读取日志目录失败: {}", e))?;
    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().and_then(|ext| ext.to_str()) == Some("log") {
            let file_name = entry.file_name();
            std::fs::copy(&file_path, dest.join(&file_name))
                .map_err(|e| format!("复制日志文件失败: {}", e))?;
            count += 1;
        }
    }

    if count == 0 {
        return Err("暂无日志文件".to_string());
    }

    Ok(format!("成功导出 {} 个日志文件", count))
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(process_manager::AppState {
            frpc_process: std::sync::Mutex::new(None),
            frps_process: std::sync::Mutex::new(None),
        })
        .setup(|app| {
            use tauri::Manager;
            use tauri::menu::{Menu, MenuItem};
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            tauri::tray::TrayIconBuilder::new()
                .tooltip("Frp Desktop Plus")
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id.as_ref() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            config_parser::read_frpc_config,
            config_parser::save_frpc_config,
            config_parser::read_frps_config,
            config_parser::save_frps_config,
            process_manager::start_frpc,
            process_manager::stop_frpc,
            process_manager::start_frps,
            process_manager::stop_frps,
            process_manager::get_frpc_status,
            process_manager::get_frps_status,
            export_deploy_script,
            export_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

