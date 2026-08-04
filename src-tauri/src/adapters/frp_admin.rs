use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;

use crate::domain::config::ConfigSnapshot;
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::process::ProcessKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    Healthy,
    NotConfigured,
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn probe(&self, snapshot: &ConfigSnapshot) -> Result<ProbeOutcome, CommandError>;
}

pub struct FrpAdminAdapter {
    client: reqwest::Client,
}

impl FrpAdminAdapter {
    pub fn new() -> Result<Self, CommandError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|_| {
                CommandError::new(
                    ErrorCode::HealthcheckFailed,
                    "failed to initialize FRP health client",
                    false,
                )
            })?;
        Ok(Self { client })
    }

    pub async fn get_frpc_traffic(
        &self,
        snapshot: &ConfigSnapshot,
    ) -> Result<String, CommandError> {
        let endpoint = admin_endpoint(snapshot, "/api/status")?.ok_or_else(|| {
            CommandError::new(
                ErrorCode::HealthcheckFailed,
                "FRP client monitoring endpoint is not configured",
                true,
            )
            .with_detail("not_configured")
            .with_suggested_action("enable local monitor in Settings and save")
        })?;
        let response = self.send(&endpoint).await?;
        response.text().await.map_err(map_probe_error)
    }

    async fn send(&self, endpoint: &AdminEndpoint) -> Result<reqwest::Response, CommandError> {
        let mut request = self.client.get(&endpoint.url);
        // FRP 0.61.x enables HTTP Basic auth whenever webServer.user or
        // webServer.password is non-empty (both default to ""). Password-only
        // configs still require Authorization; missing user means "".
        if endpoint.password.is_some() || endpoint.user.is_some() {
            request = request.basic_auth(
                endpoint.user.as_deref().unwrap_or(""),
                endpoint.password.as_deref(),
            );
        }
        let response = request.send().await.map_err(map_probe_error)?;
        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(CommandError::new(
                ErrorCode::HealthcheckFailed,
                "FRP monitoring authentication failed",
                true,
            )
            .with_detail(format!("HTTP {}", status.as_u16()))
            .with_suggested_action("verify webServer user/password"))
        } else {
            Err(CommandError::new(
                ErrorCode::HealthcheckFailed,
                "FRP monitoring endpoint returned an error",
                true,
            )
            .with_detail(format!("HTTP {}", status.as_u16())))
        }
    }
}

#[async_trait]
impl HealthProbe for FrpAdminAdapter {
    async fn probe(&self, snapshot: &ConfigSnapshot) -> Result<ProbeOutcome, CommandError> {
        let path = match snapshot.kind() {
            crate::domain::config::ConfigKind::Frpc => "/api/status",
            crate::domain::config::ConfigKind::Frps => "/api/serverinfo",
        };
        let Some(endpoint) = admin_endpoint(snapshot, path)? else {
            return Ok(ProbeOutcome::NotConfigured);
        };
        self.send(&endpoint).await?;
        Ok(ProbeOutcome::Healthy)
    }
}

struct AdminEndpoint {
    url: String,
    user: Option<String>,
    password: Option<String>,
}

/// Resolve Admin API endpoint from the live config snapshot (never a hardcoded host:port).
fn admin_endpoint(
    snapshot: &ConfigSnapshot,
    path: &str,
) -> Result<Option<AdminEndpoint>, CommandError> {
    let (kind, web_server) = match snapshot {
        ConfigSnapshot::Frpc {
            known: frpc_known, ..
        } => (ProcessKind::Frpc, &frpc_known.web_server),
        ConfigSnapshot::Frps {
            known: frps_known, ..
        } => (ProcessKind::Frps, &frps_known.web_server),
    };
    let Some(port) = web_server.port else {
        return Ok(None);
    };
    let address = web_server
        .addr
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(crate::domain::settings::DEFAULT_MONITOR_ADDR);
    if !is_loopback(address) {
        return Err(CommandError::new(
            ErrorCode::HealthcheckFailed,
            "FRP monitoring is restricted to loopback addresses",
            true,
        )
        .with_detail(match kind {
            ProcessKind::Frpc => "frpc webServer.addr is not loopback",
            ProcessKind::Frps => "frps webServer.addr is not loopback",
        }));
    }
    let host = if address.contains(':') && !address.starts_with('[') {
        format!("[{address}]")
    } else {
        address.to_string()
    };
    Ok(Some(AdminEndpoint {
        url: format!("http://{host}:{port}{path}"),
        user: non_empty(web_server.user.as_deref()),
        password: non_empty(web_server.password.as_deref()),
    }))
}

fn is_loopback(address: &str) -> bool {
    address.eq_ignore_ascii_case("localhost")
        || address
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn map_probe_error(error: reqwest::Error) -> CommandError {
    let detail = if error.is_timeout() {
        "request timed out"
    } else if error.is_connect() {
        "connection failed"
    } else {
        "request failed"
    };
    CommandError::new(
        ErrorCode::HealthcheckFailed,
        "FRP monitoring request failed",
        true,
    )
    .with_detail(detail)
}
