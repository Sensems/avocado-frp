use std::path::Path;

use tauri::State;

use crate::domain::config::ConfigKind;
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::monitor::{FrpcTrafficResult, MonitorStatus};
use crate::domain::process::ProcessPhase;
use crate::AppServices;

const INSTALL_SCRIPT: &str = r#"#!/bin/bash
echo "Installing frps..."
mkdir -p /etc/frp
cp frps.toml /etc/frp/frps.toml
wget https://github.com/fatedier/frp/releases/download/v0.67.0/frp_0.67.0_linux_amd64.tar.gz -O /tmp/frp.tar.gz
tar -zxvf /tmp/frp.tar.gz -C /tmp
cp /tmp/frp_0.67.0_linux_amd64/frps /usr/bin/frps
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
"#;

#[tauri::command]
pub fn export_deploy_script(path: String, toml_content: String) -> Result<(), CommandError> {
    let directory = Path::new(&path);
    std::fs::write(directory.join("frps.toml"), toml_content).map_err(map_support_io)?;
    std::fs::write(directory.join("install.sh"), INSTALL_SCRIPT).map_err(map_support_io)
}

#[tauri::command]
pub fn export_logs(path: String, services: State<'_, AppServices>) -> Result<String, CommandError> {
    if !services.paths.log_dir.exists() {
        return Err(no_logs_error());
    }
    let destination = Path::new(&path);
    let mut count = 0_u32;
    let entries = std::fs::read_dir(&services.paths.log_dir).map_err(map_support_io)?;
    for entry in entries {
        let entry = entry.map_err(map_support_io)?;
        let source = entry.path();
        if source.extension().and_then(|extension| extension.to_str()) == Some("log") {
            std::fs::copy(&source, destination.join(entry.file_name())).map_err(map_support_io)?;
            count += 1;
        }
    }
    if count == 0 {
        Err(no_logs_error())
    } else {
        Ok(format!("exported {count} log file(s)"))
    }
}

#[tauri::command]
pub async fn get_frpc_traffic(
    services: State<'_, AppServices>,
) -> Result<FrpcTrafficResult, CommandError> {
    let settings = services.settings.get();
    if !settings.local_monitor.enabled {
        return Ok(FrpcTrafficResult::status_only(MonitorStatus::Disabled));
    }

    let process = services
        .processes
        .snapshot(crate::domain::process::ProcessKind::Frpc)
        .await;
    if !matches!(
        process.phase,
        ProcessPhase::Starting
            | ProcessPhase::Healthy
            | ProcessPhase::Degraded
            | ProcessPhase::Stopping
    ) {
        return Ok(FrpcTrafficResult::status_only(
            MonitorStatus::ProcessStopped,
        ));
    }

    let snapshot = services.config.load(ConfigKind::Frpc)?;
    match services.frp_admin.get_frpc_traffic(&snapshot).await {
        Ok(body) => Ok(FrpcTrafficResult::ok(body)),
        Err(error) => Ok(FrpcTrafficResult::status_only(map_traffic_error(&error))),
    }
}

fn map_traffic_error(error: &CommandError) -> MonitorStatus {
    let detail = error.detail.as_deref().unwrap_or("");
    let message = error.message.as_str();
    if detail == "not_configured"
        || message.contains("not configured")
        || detail.contains("not configured")
    {
        return MonitorStatus::NotConfigured;
    }
    if detail.starts_with("HTTP 401") || detail.starts_with("HTTP 403") {
        return MonitorStatus::AuthFailed;
    }
    if detail.contains("timed out") {
        return MonitorStatus::Timeout;
    }
    if error.code == ErrorCode::PortConflict || detail.contains("in use") {
        return MonitorStatus::PortConflict;
    }
    if detail.contains("connection failed") || detail.contains("request failed") {
        // Process is alive but Admin API unreachable — treat as timeout empty state.
        return MonitorStatus::Timeout;
    }
    MonitorStatus::NotConfigured
}

fn no_logs_error() -> CommandError {
    CommandError::new(ErrorCode::ConfigIo, "no log files are available", true)
}

fn map_support_io(error: std::io::Error) -> CommandError {
    let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
        ErrorCode::PermissionDenied
    } else {
        ErrorCode::ConfigIo
    };
    CommandError::new(code, "file operation failed", true)
        .with_detail(format!("{:?}", error.kind()))
}
