use tauri::State;

use crate::adapters::port_probe::{probe_bind, PortProbeStatus};
use crate::domain::config::{
    ConfigChangeRequest, ConfigKind, ConfigSnapshot, FrpcChange, FrpcConfigPatch,
    WebServerConfigPatch,
};
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::monitor::{ApplyLocalMonitorRequest, ApplyLocalMonitorResult};
use crate::domain::process::{ProcessKind, ProcessPhase};
use crate::domain::settings::{
    AppSettings, AppSettingsPatch, LocalMonitorPrefsPatch, DEFAULT_MONITOR_ADDR,
    DEFAULT_MONITOR_USER,
};
use crate::AppServices;

#[tauri::command]
pub fn get_app_settings(services: State<'_, AppServices>) -> Result<AppSettings, CommandError> {
    Ok(services.settings.get())
}

#[tauri::command]
pub fn update_app_settings(
    patch: AppSettingsPatch,
    services: State<'_, AppServices>,
) -> Result<AppSettings, CommandError> {
    services.settings.update(patch)
}

/// Persist local-monitor prefs and sync frpc `webServer` via revisioned apply_change.
/// Enabling probes the port then writes addr/port/user/password; disabling clears
/// webServer fields (and removes an empty table) so the Admin port does not stay open.
#[tauri::command]
pub async fn apply_local_monitor(
    request: ApplyLocalMonitorRequest,
    services: State<'_, AppServices>,
) -> Result<ApplyLocalMonitorResult, CommandError> {
    let addr = request.addr.trim().to_string();
    if addr.is_empty() {
        return Err(CommandError::new(
            ErrorCode::ConfigInvalid,
            "local monitor address is required",
            true,
        )
        .with_suggested_action("use a loopback address such as 127.0.0.1"));
    }
    if request.port == 0 {
        return Err(CommandError::new(
            ErrorCode::ConfigInvalid,
            "local monitor port must be between 1 and 65535",
            true,
        )
        .with_suggested_action("choose a port in the range 1–65535"));
    }

    let password = request
        .password
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    // FRP 0.61.x: user/password both default to ""; examples use "admin".
    // Password-only Settings input → write DEFAULT_MONITOR_USER so Basic auth matches.
    let user = match request
        .user
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(value) => Some(value),
        None if password.is_some() => Some(DEFAULT_MONITOR_USER.to_string()),
        None => None,
    };

    let snapshot = services.config.load(ConfigKind::Frpc)?;
    let process = services.processes.snapshot(ProcessKind::Frpc).await;
    let pending_restart = process_may_need_restart(&process);

    if !request.enabled {
        let settings = services.settings.update(AppSettingsPatch {
            local_monitor: Some(LocalMonitorPrefsPatch {
                enabled: Some(false),
                addr: Some(addr),
                port: Some(request.port),
                user: Some(user),
                password: Some(password),
            }),
            ..Default::default()
        })?;

        let change = ConfigChangeRequest::Frpc {
            expected_revision: snapshot.revision().to_string(),
            change: FrpcChange::Patch {
                patch: FrpcConfigPatch {
                    web_server: Some(WebServerConfigPatch {
                        addr: Some(None),
                        port: Some(None),
                        user: Some(None),
                        password: Some(None),
                    }),
                    ..Default::default()
                },
            },
        };
        services.transactions.apply_change(change).await?;

        return Ok(ApplyLocalMonitorResult {
            settings,
            config_patched: true,
            pending_restart,
        });
    }

    // Enabling: probe before any disk write (except when our running frpc already owns the port).
    let owned = monitor_port_owned_by_frpc(&snapshot, &process, &addr, request.port);
    if !owned {
        let probe = probe_bind(&normalize_probe_addr(&addr), request.port);
        match probe.status {
            PortProbeStatus::Available => {}
            PortProbeStatus::Occupied => {
                return Err(CommandError::new(
                    ErrorCode::PortConflict,
                    "local monitor port is already in use",
                    true,
                )
                .with_detail(probe.detail)
                .with_suggested_action("choose a different port or stop the conflicting process"));
            }
            PortProbeStatus::Error => {
                return Err(CommandError::new(
                    ErrorCode::ConfigInvalid,
                    "failed to probe local monitor port",
                    true,
                )
                .with_detail(probe.detail)
                .with_suggested_action("verify the loopback address and port"));
            }
        }
    }

    let settings = services.settings.update(AppSettingsPatch {
        local_monitor: Some(LocalMonitorPrefsPatch {
            enabled: Some(true),
            addr: Some(addr.clone()),
            port: Some(request.port),
            user: Some(user.clone()),
            password: Some(password.clone()),
        }),
        ..Default::default()
    })?;

    let change = ConfigChangeRequest::Frpc {
        expected_revision: snapshot.revision().to_string(),
        change: FrpcChange::Patch {
            patch: FrpcConfigPatch {
                web_server: Some(WebServerConfigPatch {
                    addr: Some(Some(addr)),
                    port: Some(Some(request.port)),
                    user: Some(user),
                    password: Some(password),
                }),
                ..Default::default()
            },
        },
    };

    services.transactions.apply_change(change).await?;

    Ok(ApplyLocalMonitorResult {
        settings,
        config_patched: true,
        pending_restart,
    })
}

fn process_may_need_restart(process: &crate::domain::process::ProcessSnapshot) -> bool {
    matches!(
        process.phase,
        ProcessPhase::Starting
            | ProcessPhase::Healthy
            | ProcessPhase::Degraded
            | ProcessPhase::Stopping
    )
}

fn normalize_probe_addr(addr: &str) -> String {
    if addr.eq_ignore_ascii_case("localhost") {
        DEFAULT_MONITOR_ADDR.to_string()
    } else {
        addr.to_string()
    }
}

fn monitor_port_owned_by_frpc(
    snapshot: &ConfigSnapshot,
    process: &crate::domain::process::ProcessSnapshot,
    addr: &str,
    port: u16,
) -> bool {
    if !process_may_need_restart(process) {
        return false;
    }
    let ConfigSnapshot::Frpc { known, .. } = snapshot else {
        return false;
    };
    let Some(current_port) = known.web_server.port else {
        return false;
    };
    if current_port != port {
        return false;
    }
    let current_addr = known
        .web_server
        .addr
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_MONITOR_ADDR);
    addrs_equivalent(current_addr, addr)
}

fn addrs_equivalent(left: &str, right: &str) -> bool {
    fn normalize(value: &str) -> &str {
        if value.eq_ignore_ascii_case("localhost") {
            DEFAULT_MONITOR_ADDR
        } else {
            value
        }
    }
    normalize(left).eq_ignore_ascii_case(normalize(right))
}
