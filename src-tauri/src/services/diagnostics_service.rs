use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::time::timeout;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::adapters::event_sink::{redact_secrets, redact_toml_for_export};
use crate::adapters::filesystem::AppPaths;
use crate::adapters::frp_admin::{HealthProbe, ProbeOutcome};
use crate::adapters::port_probe::{probe_bind, PortProbeStatus};
use crate::adapters::sidecar::{SidecarAdapter, SUPPORTED_FRP_VERSION};
use crate::domain::config::{ConfigKind, ConfigSnapshot, ValidationSeverity};
use crate::domain::diagnostics::action;
use crate::domain::diagnostics::{
    DiagnosticResult, DiagnosticStatus, DiagnosticsReport,
};
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::process::{ProcessKind, ProcessPhase};
use crate::domain::settings::{AppSettings, LocalMonitorPrefs, LogPolicy};
use crate::services::app_settings::AppSettingsStore;
use crate::services::config_repository::ConfigRepository;
use crate::services::process_supervisor::ProcessSupervisor;

pub struct DiagnosticsService {
    paths: AppPaths,
    config: Arc<ConfigRepository>,
    settings: Arc<AppSettingsStore>,
    sidecar: Arc<dyn SidecarAdapter>,
    health: Arc<dyn HealthProbe>,
    processes: Arc<ProcessSupervisor>,
    app_version: String,
}

impl DiagnosticsService {
    pub fn new(
        paths: AppPaths,
        config: Arc<ConfigRepository>,
        settings: Arc<AppSettingsStore>,
        sidecar: Arc<dyn SidecarAdapter>,
        health: Arc<dyn HealthProbe>,
        processes: Arc<ProcessSupervisor>,
        app_version: impl Into<String>,
    ) -> Self {
        Self {
            paths,
            config,
            settings,
            sidecar,
            health,
            processes,
            app_version: app_version.into(),
        }
    }

    /// Run all diagnostics. Order is fixed for UI; independent checks may run concurrently.
    pub async fn run_all(&self) -> Result<DiagnosticsReport, CommandError> {
        let started_at = Utc::now();

        let (frpc_sidecar, frps_sidecar) = tokio::join!(
            self.check_sidecar(ProcessKind::Frpc),
            self.check_sidecar(ProcessKind::Frps),
        );

        let frpc_snapshot = self.config.load(ConfigKind::Frpc).ok();
        let frps_snapshot = self.config.load(ConfigKind::Frps).ok();

        let config_frpc = check_config(ConfigKind::Frpc, frpc_snapshot.as_ref());
        let config_frps = check_config(ConfigKind::Frps, frps_snapshot.as_ref());

        let frpc_running = self.is_managed_running(ProcessKind::Frpc).await;
        let frps_running = self.is_managed_running(ProcessKind::Frps).await;

        let mut port_results = Vec::new();
        if let Some(snapshot) = frps_snapshot.as_ref() {
            port_results.extend(check_frps_ports(snapshot, frps_running));
        }
        if let Some(snapshot) = frpc_snapshot.as_ref() {
            port_results.extend(check_frpc_ports(snapshot, frpc_running));
        }

        let connectivity = match frpc_snapshot.as_ref() {
            Some(snapshot) => check_server_connectivity(snapshot).await,
            None => DiagnosticResult {
                id: "connectivity.frpc.server".into(),
                status: DiagnosticStatus::Warning,
                title_key: Some("connectivityServer".into()),
                detail: "frpc configuration could not be loaded".into(),
                suggested_action: action::FIX_CONFIG.into(),
            },
        };

        let health_frpc = match frpc_snapshot.as_ref() {
            Some(snapshot) => self.check_admin_health(ProcessKind::Frpc, snapshot).await,
            None => admin_unavailable(ProcessKind::Frpc),
        };
        let health_frps = match frps_snapshot.as_ref() {
            Some(snapshot) => self.check_admin_health(ProcessKind::Frps, snapshot).await,
            None => admin_unavailable(ProcessKind::Frps),
        };

        let config_dir = check_directory_rw("filesystem.configDir", "configDir", &self.paths.config_dir);
        let log_dir = check_directory_rw("filesystem.logDir", "logDir", &self.paths.log_dir);

        let versions = version_summary(
            &self.app_version,
            &frpc_sidecar,
            &frps_sidecar,
        );

        let mut results = vec![frpc_sidecar, frps_sidecar, config_frpc, config_frps];
        results.extend(port_results);
        results.push(connectivity);
        results.push(health_frpc);
        results.push(health_frps);
        results.push(config_dir);
        results.push(log_dir);
        results.push(versions);

        let finished_at = Utc::now();
        Ok(DiagnosticsReport {
            started_at: started_at.to_rfc3339(),
            finished_at: finished_at.to_rfc3339(),
            results,
        })
    }

    /// Build a redacted diagnostics zip at `path` (file path, typically `*.zip`).
    pub async fn export_pack(&self, path: impl AsRef<Path>) -> Result<String, CommandError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(map_pack_io)?;
            }
        }

        let report = self.run_all().await?;
        let frpc_process = self.processes.snapshot(ProcessKind::Frpc).await;
        let frps_process = self.processes.snapshot(ProcessKind::Frps).await;
        let settings_export = AppSettingsExport::from(&self.settings.get());

        let file = File::create(path).map_err(map_pack_io)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        write_zip_json(&mut zip, "report.json", &report, options)?;
        write_zip_json(&mut zip, "process-frpc.json", &frpc_process, options)?;
        write_zip_json(&mut zip, "process-frps.json", &frps_process, options)?;
        write_zip_json(&mut zip, "app-settings.json", &settings_export, options)?;

        for kind in [ConfigKind::Frpc, ConfigKind::Frps] {
            let name = match kind {
                ConfigKind::Frpc => "frpc.toml.redacted",
                ConfigKind::Frps => "frps.toml.redacted",
            };
            match self.config.load(kind) {
                Ok(snapshot) => {
                    let redacted = redact_toml_for_export(snapshot.raw());
                    write_zip_text(&mut zip, name, &redacted, options)?;
                }
                Err(error) => {
                    let placeholder = format!(
                        "# {} config unavailable: {}\n",
                        kind_label_config(kind),
                        error.message
                    );
                    write_zip_text(&mut zip, name, &placeholder, options)?;
                }
            }
        }

        for log_path in collect_managed_log_files(&self.paths.log_dir) {
            let Some(file_name) = log_path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let entry_name = format!("logs/{file_name}");
            let mut bytes = Vec::new();
            match File::open(&log_path).and_then(|mut file| file.read_to_end(&mut bytes)) {
                Ok(_) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let redacted = redact_secrets(&text);
                    write_zip_text(&mut zip, &entry_name, &redacted, options)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(map_pack_io(error)),
            }
        }

        zip.finish().map_err(|error| {
            CommandError::new(
                ErrorCode::ConfigIo,
                "failed to finalize diagnostics pack",
                true,
            )
            .with_detail(error.to_string())
        })?;

        Ok(format!("exported diagnostics pack to {}", path.display()))
    }

    async fn check_sidecar(&self, kind: ProcessKind) -> DiagnosticResult {
        let (id, title_key) = match kind {
            ProcessKind::Frpc => ("sidecar.frpc", "sidecarFrpc"),
            ProcessKind::Frps => ("sidecar.frps", "sidecarFrps"),
        };
        match self.sidecar.inspect(kind).await {
            Ok(info) => DiagnosticResult {
                id: id.into(),
                status: DiagnosticStatus::Pass,
                title_key: Some(title_key.into()),
                detail: format!(
                    "{} sidecar OK (version {})",
                    kind_label(kind),
                    info.version
                ),
                suggested_action: action::NONE.into(),
            },
            Err(error) => {
                let detail = match (&error.detail, error.message.as_str()) {
                    (Some(detail), message) => format!("{message}: {detail}"),
                    (None, message) => message.to_string(),
                };
                DiagnosticResult {
                    id: id.into(),
                    status: DiagnosticStatus::Fail,
                    title_key: Some(title_key.into()),
                    detail,
                    suggested_action: action::REINSTALL_SIDECAR.into(),
                }
            }
        }
    }

    async fn check_admin_health(
        &self,
        kind: ProcessKind,
        snapshot: &ConfigSnapshot,
    ) -> DiagnosticResult {
        let (id, title_key) = match kind {
            ProcessKind::Frpc => ("health.frpc.admin", "healthFrpcAdmin"),
            ProcessKind::Frps => ("health.frps.admin", "healthFrpsAdmin"),
        };
        match self.health.probe(snapshot).await {
            Ok(ProbeOutcome::Healthy) => DiagnosticResult {
                id: id.into(),
                status: DiagnosticStatus::Pass,
                title_key: Some(title_key.into()),
                detail: format!("{} Admin API healthy", kind_label(kind)),
                suggested_action: action::NONE.into(),
            },
            Ok(ProbeOutcome::NotConfigured) => DiagnosticResult {
                id: id.into(),
                status: DiagnosticStatus::Warning,
                title_key: Some(title_key.into()),
                detail: format!(
                    "{} Admin API (webServer) is not configured",
                    kind_label(kind)
                ),
                suggested_action: action::CONFIGURE_WEBSERVER.into(),
            },
            Err(error) => {
                let detail = match (&error.detail, error.message.as_str()) {
                    (Some(detail), message) => format!("{message}: {detail}"),
                    (None, message) => message.to_string(),
                };
                let suggested_action = if web_server_configured(snapshot) {
                    let running = self.is_managed_running(kind).await;
                    admin_probe_error_action(&error, running)
                } else {
                    action::CONFIGURE_WEBSERVER
                };
                DiagnosticResult {
                    id: id.into(),
                    status: DiagnosticStatus::Fail,
                    title_key: Some(title_key.into()),
                    detail,
                    suggested_action: suggested_action.into(),
                }
            }
        }
    }

    async fn is_managed_running(&self, kind: ProcessKind) -> bool {
        let snapshot = self.processes.snapshot(kind).await;
        matches!(
            snapshot.phase,
            ProcessPhase::Starting
                | ProcessPhase::Healthy
                | ProcessPhase::Degraded
                | ProcessPhase::Stopping
        )
    }
}

fn check_config(kind: ConfigKind, snapshot: Option<&ConfigSnapshot>) -> DiagnosticResult {
    let (id, title_key) = match kind {
        ConfigKind::Frpc => ("config.frpc", "configFrpc"),
        ConfigKind::Frps => ("config.frps", "configFrps"),
    };
    let Some(snapshot) = snapshot else {
        return DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: format!("{} configuration could not be loaded", kind_label_config(kind)),
            suggested_action: action::FIX_CONFIG.into(),
        };
    };
    let issues = match snapshot {
        ConfigSnapshot::Frpc { issues, .. } | ConfigSnapshot::Frps { issues, .. } => issues,
    };
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == ValidationSeverity::Error)
        .collect();
    let warnings: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == ValidationSeverity::Warning)
        .collect();
    if !errors.is_empty() {
        let summary = errors
            .iter()
            .take(3)
            .map(|issue| {
                issue
                    .path
                    .as_ref()
                    .map(|path| format!("{} ({path})", issue.code))
                    .unwrap_or_else(|| issue.code.clone())
            })
            .collect::<Vec<_>>()
            .join("; ");
        return DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: format!(
                "{} has {} validation error(s): {summary}",
                kind_label_config(kind),
                errors.len()
            ),
            suggested_action: action::FIX_CONFIG.into(),
        };
    }
    if !warnings.is_empty() {
        return DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Warning,
            title_key: Some(title_key.into()),
            detail: format!(
                "{} has {} validation warning(s)",
                kind_label_config(kind),
                warnings.len()
            ),
            suggested_action: action::FIX_CONFIG.into(),
        };
    }
    DiagnosticResult {
        id: id.into(),
        status: DiagnosticStatus::Pass,
        title_key: Some(title_key.into()),
        detail: format!(
            "{} TOML validated (revision {})",
            kind_label_config(kind),
            snapshot.revision()
        ),
        suggested_action: action::NONE.into(),
    }
}

fn check_frps_ports(snapshot: &ConfigSnapshot, managed_running: bool) -> Vec<DiagnosticResult> {
    let ConfigSnapshot::Frps { known, .. } = snapshot else {
        return Vec::new();
    };
    let mut results = Vec::new();
    if let Some(port) = known.bind_port {
        results.push(port_result(
            "ports.frps.bind",
            "portsFrpsBind",
            "0.0.0.0",
            port,
            managed_running,
        ));
    }
    if let Some(port) = known.vhost_http_port {
        results.push(port_result(
            "ports.frps.vhostHttp",
            "portsFrpsVhostHttp",
            "0.0.0.0",
            port,
            managed_running,
        ));
    }
    if let Some(port) = known.vhost_https_port {
        results.push(port_result(
            "ports.frps.vhostHttps",
            "portsFrpsVhostHttps",
            "0.0.0.0",
            port,
            managed_running,
        ));
    }
    if let Some(port) = known.web_server.port {
        let addr = known
            .web_server
            .addr
            .as_deref()
            .unwrap_or("127.0.0.1");
        results.push(port_result(
            "ports.frps.webServer",
            "portsFrpsWebServer",
            addr,
            port,
            managed_running,
        ));
    }
    results
}

fn check_frpc_ports(snapshot: &ConfigSnapshot, managed_running: bool) -> Vec<DiagnosticResult> {
    let ConfigSnapshot::Frpc { known, .. } = snapshot else {
        return Vec::new();
    };
    let mut results = Vec::new();
    if let Some(port) = known.web_server.port {
        let addr = known
            .web_server
            .addr
            .as_deref()
            .unwrap_or("127.0.0.1");
        results.push(port_result(
            "ports.frpc.webServer",
            "portsFrpcWebServer",
            addr,
            port,
            managed_running,
        ));
    }

    let mut local_details = Vec::new();
    let mut remote_details = Vec::new();
    let mut local_status = DiagnosticStatus::Pass;
    let mut local_action = action::NONE;
    let mut remote_status = DiagnosticStatus::Pass;
    let mut remote_action = action::NONE;

    for proxy in &known.proxies {
        let name = proxy
            .name
            .as_deref()
            .unwrap_or(proxy.source_name.as_str());
        if let Some(port) = proxy.local_port {
            let addr = proxy.local_ip.as_deref().unwrap_or("127.0.0.1");
            let probe = probe_bind(addr, port);
            match probe.status {
                PortProbeStatus::Available => {
                    local_details.push(format!("{name} local {addr}:{port} available"));
                }
                PortProbeStatus::Occupied if managed_running => {
                    local_details.push(format!(
                        "{name} local {addr}:{port} occupied (managed process running)"
                    ));
                    elevate(&mut local_status, DiagnosticStatus::Warning);
                }
                PortProbeStatus::Occupied => {
                    local_details.push(format!("{name} local {addr}:{port} occupied"));
                    elevate(&mut local_status, DiagnosticStatus::Fail);
                    local_action = action::STOP_CONFLICTING_PROCESS;
                }
                PortProbeStatus::Error => {
                    local_details.push(format!("{name} local: {}", probe.detail));
                    elevate(&mut local_status, DiagnosticStatus::Warning);
                    local_action = action::CHANGE_PORT;
                }
            }
        }
        if let Some(port) = proxy.remote_port {
            let probe = probe_bind("0.0.0.0", port);
            match probe.status {
                PortProbeStatus::Available => {
                    remote_details.push(format!("{name} remote :{port} available locally"));
                }
                PortProbeStatus::Occupied if managed_running => {
                    remote_details.push(format!(
                        "{name} remote :{port} occupied locally (managed process running)"
                    ));
                    elevate(&mut remote_status, DiagnosticStatus::Warning);
                }
                PortProbeStatus::Occupied => {
                    remote_details.push(format!("{name} remote :{port} occupied locally"));
                    elevate(&mut remote_status, DiagnosticStatus::Fail);
                    remote_action = action::CHANGE_PORT;
                }
                PortProbeStatus::Error => {
                    remote_details.push(format!("{name} remote: {}", probe.detail));
                    elevate(&mut remote_status, DiagnosticStatus::Warning);
                    remote_action = action::CHANGE_PORT;
                }
            }
        }
    }

    if !local_details.is_empty() {
        results.push(DiagnosticResult {
            id: "ports.frpc.proxyLocal".into(),
            status: local_status,
            title_key: Some("portsFrpcProxyLocal".into()),
            detail: local_details.join("; "),
            suggested_action: if matches!(local_status, DiagnosticStatus::Pass) {
                action::NONE.into()
            } else {
                local_action.into()
            },
        });
    }

    if !remote_details.is_empty() {
        results.push(DiagnosticResult {
            id: "ports.frpc.proxyRemote".into(),
            status: remote_status,
            title_key: Some("portsFrpcProxyRemote".into()),
            detail: remote_details.join("; "),
            suggested_action: if matches!(remote_status, DiagnosticStatus::Pass) {
                action::NONE.into()
            } else {
                remote_action.into()
            },
        });
    }

    results
}

fn port_result(
    id: &str,
    title_key: &str,
    addr: &str,
    port: u16,
    managed_running: bool,
) -> DiagnosticResult {
    let probe = probe_bind(addr, port);
    match probe.status {
        PortProbeStatus::Available => DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Pass,
            title_key: Some(title_key.into()),
            detail: probe.detail,
            suggested_action: action::NONE.into(),
        },
        PortProbeStatus::Occupied if managed_running => DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Warning,
            title_key: Some(title_key.into()),
            detail: format!("{} — managed FRP process is running", probe.detail),
            suggested_action: action::NONE.into(),
        },
        PortProbeStatus::Occupied => DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: probe.detail,
            suggested_action: action::STOP_CONFLICTING_PROCESS.into(),
        },
        PortProbeStatus::Error => DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: probe.detail,
            suggested_action: action::CHANGE_PORT.into(),
        },
    }
}

async fn check_server_connectivity(snapshot: &ConfigSnapshot) -> DiagnosticResult {
    let ConfigSnapshot::Frpc { known, .. } = snapshot else {
        return DiagnosticResult {
            id: "connectivity.frpc.server".into(),
            status: DiagnosticStatus::Warning,
            title_key: Some("connectivityServer".into()),
            detail: "not an frpc configuration".into(),
            suggested_action: action::FIX_CONFIG.into(),
        };
    };
    let (Some(addr), Some(port)) = (known.server_addr.as_deref(), known.server_port) else {
        return DiagnosticResult {
            id: "connectivity.frpc.server".into(),
            status: DiagnosticStatus::Warning,
            title_key: Some("connectivityServer".into()),
            detail: "serverAddr/serverPort not configured".into(),
            suggested_action: action::CHECK_SERVER_ADDR.into(),
        };
    };
    let target = format!("{addr}:{port}");
    let connect_result = timeout(
        Duration::from_secs(2),
        TcpStream::connect((addr, port)),
    )
    .await;
    match connect_result {
        Ok(Ok(_stream)) => DiagnosticResult {
            id: "connectivity.frpc.server".into(),
            status: DiagnosticStatus::Pass,
            title_key: Some("connectivityServer".into()),
            detail: format!("TCP connect to {target} succeeded"),
            suggested_action: action::NONE.into(),
        },
        Ok(Err(error)) => DiagnosticResult {
            id: "connectivity.frpc.server".into(),
            status: DiagnosticStatus::Fail,
            title_key: Some("connectivityServer".into()),
            detail: format!("TCP connect to {target} failed: {error}"),
            suggested_action: action::CHECK_SERVER_ADDR.into(),
        },
        Err(_) => DiagnosticResult {
            id: "connectivity.frpc.server".into(),
            status: DiagnosticStatus::Fail,
            title_key: Some("connectivityServer".into()),
            detail: format!("TCP connect to {target} timed out"),
            suggested_action: action::CHECK_SERVER_ADDR.into(),
        },
    }
}

fn check_directory_rw(id: &str, title_key: &str, path: &Path) -> DiagnosticResult {
    if let Err(error) = std::fs::create_dir_all(path) {
        return DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: format!("cannot create {}: {error}", path.display()),
            suggested_action: action::FIX_DIRECTORY_PERMISSIONS.into(),
        };
    }
    let probe = path.join(".avocado-diag-write-probe");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            DiagnosticResult {
                id: id.into(),
                status: DiagnosticStatus::Pass,
                title_key: Some(title_key.into()),
                detail: format!("{} is readable/writable", path.display()),
                suggested_action: action::NONE.into(),
            }
        }
        Err(error) => DiagnosticResult {
            id: id.into(),
            status: DiagnosticStatus::Fail,
            title_key: Some(title_key.into()),
            detail: format!("cannot write {}: {error}", path.display()),
            suggested_action: action::FIX_DIRECTORY_PERMISSIONS.into(),
        },
    }
}

fn version_summary(
    app_version: &str,
    frpc: &DiagnosticResult,
    frps: &DiagnosticResult,
) -> DiagnosticResult {
    let frpc_ver = extract_version_hint(&frpc.detail).unwrap_or_else(|| {
        if frpc.status == DiagnosticStatus::Pass {
            SUPPORTED_FRP_VERSION.to_string()
        } else {
            "unavailable".into()
        }
    });
    let frps_ver = extract_version_hint(&frps.detail).unwrap_or_else(|| {
        if frps.status == DiagnosticStatus::Pass {
            SUPPORTED_FRP_VERSION.to_string()
        } else {
            "unavailable".into()
        }
    });
    let version_problem = frpc_ver == "unavailable"
        || frps_ver == "unavailable"
        || frpc_ver != SUPPORTED_FRP_VERSION
        || frps_ver != SUPPORTED_FRP_VERSION;
    let (status, suggested_action) = if version_problem {
        (DiagnosticStatus::Warning, action::REINSTALL_SIDECAR)
    } else {
        (DiagnosticStatus::Pass, action::NONE)
    };
    DiagnosticResult {
        id: "versions.summary".into(),
        status,
        title_key: Some("versionsSummary".into()),
        detail: format!(
            "app {app_version}; frpc {frpc_ver}; frps {frps_ver}; expected sidecar {SUPPORTED_FRP_VERSION}; updater check not configured (WP5)"
        ),
        suggested_action: suggested_action.into(),
    }
}

fn extract_version_hint(detail: &str) -> Option<String> {
    detail
        .split("version ")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn web_server_configured(snapshot: &ConfigSnapshot) -> bool {
    match snapshot {
        ConfigSnapshot::Frpc { known, .. } => known.web_server.port.is_some(),
        ConfigSnapshot::Frps { known, .. } => known.web_server.port.is_some(),
    }
}

fn admin_probe_error_action(error: &CommandError, process_running: bool) -> &'static str {
    let detail = error.detail.as_deref().unwrap_or("");
    let message = error.message.as_str();
    if message.contains("not loopback") || detail.contains("not loopback") {
        return action::FIX_CONFIG;
    }
    if detail.starts_with("HTTP 401") || detail.starts_with("HTTP 403") {
        return action::CHECK_ADMIN_AUTH;
    }
    if detail.starts_with("HTTP ") {
        return action::CHECK_ADMIN_API;
    }
    if detail.contains("connection failed") || detail.contains("request timed out") {
        if process_running {
            return action::CHECK_ADMIN_API;
        }
        return action::START_PROCESS;
    }
    action::CHECK_ADMIN_API
}

fn admin_unavailable(kind: ProcessKind) -> DiagnosticResult {
    let (id, title_key) = match kind {
        ProcessKind::Frpc => ("health.frpc.admin", "healthFrpcAdmin"),
        ProcessKind::Frps => ("health.frps.admin", "healthFrpsAdmin"),
    };
    DiagnosticResult {
        id: id.into(),
        status: DiagnosticStatus::Warning,
        title_key: Some(title_key.into()),
        detail: format!(
            "{} configuration unavailable for Admin API probe",
            kind_label(kind)
        ),
        suggested_action: action::CONFIGURE_WEBSERVER.into(),
    }
}

fn elevate(current: &mut DiagnosticStatus, next: DiagnosticStatus) {
    let rank = |status: DiagnosticStatus| match status {
        DiagnosticStatus::Pass => 0,
        DiagnosticStatus::Warning => 1,
        DiagnosticStatus::Fail => 2,
    };
    if rank(next) > rank(*current) {
        *current = next;
    }
}

fn kind_label(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Frpc => "frpc",
        ProcessKind::Frps => "frps",
    }
}

fn kind_label_config(kind: ConfigKind) -> &'static str {
    match kind {
        ConfigKind::Frpc => "frpc",
        ConfigKind::Frps => "frps",
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsExport {
    schema_version: u32,
    log_policy: LogPolicy,
    local_monitor: LocalMonitorPrefsExport,
    log_policy_notice_shown: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalMonitorPrefsExport {
    enabled: bool,
    addr: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    /// Present only when a password was configured; value is always redacted.
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'static str>,
}

impl From<&AppSettings> for AppSettingsExport {
    fn from(settings: &AppSettings) -> Self {
        Self {
            schema_version: settings.schema_version,
            log_policy: settings.log_policy.clone(),
            local_monitor: LocalMonitorPrefsExport::from(&settings.local_monitor),
            log_policy_notice_shown: settings.log_policy_notice_shown,
        }
    }
}

impl From<&LocalMonitorPrefs> for LocalMonitorPrefsExport {
    fn from(prefs: &LocalMonitorPrefs) -> Self {
        Self {
            enabled: prefs.enabled,
            addr: prefs.addr.clone(),
            port: prefs.port,
            user: prefs.user.clone(),
            password: prefs.password.as_ref().map(|_| "***"),
        }
    }
}

fn write_zip_json<T: Serialize>(
    zip: &mut ZipWriter<File>,
    name: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), CommandError> {
    let body = serde_json::to_string_pretty(value).map_err(|error| {
        CommandError::new(ErrorCode::Unknown, "failed to serialize diagnostics pack entry", true)
            .with_detail(error.to_string())
    })?;
    write_zip_text(zip, name, &body, options)
}

fn write_zip_text(
    zip: &mut ZipWriter<File>,
    name: &str,
    body: &str,
    options: SimpleFileOptions,
) -> Result<(), CommandError> {
    zip.start_file(name, options).map_err(map_zip_error)?;
    zip.write_all(body.as_bytes()).map_err(map_pack_io)?;
    Ok(())
}

fn collect_managed_log_files(log_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if is_managed_log_name(name) {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn is_managed_log_name(name: &str) -> bool {
    for base in ["frpc.log", "frps.log"] {
        if name == base {
            return true;
        }
        if let Some(suffix) = name.strip_prefix(base).and_then(|rest| rest.strip_prefix('.')) {
            if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

fn map_pack_io(error: std::io::Error) -> CommandError {
    let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
        ErrorCode::PermissionDenied
    } else {
        ErrorCode::ConfigIo
    };
    CommandError::new(code, "diagnostics pack file operation failed", true)
        .with_detail(format!("{:?}", error.kind()))
}

fn map_zip_error(error: zip::result::ZipError) -> CommandError {
    CommandError::new(ErrorCode::ConfigIo, "failed to write diagnostics pack entry", true)
        .with_detail(error.to_string())
}
