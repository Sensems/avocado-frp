use std::net::{IpAddr, SocketAddr, TcpListener};

/// Result of a local bind occupancy probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortProbeStatus {
    /// `TcpListener::bind` succeeded; listener is dropped immediately.
    Available,
    /// Bind failed (typically address already in use).
    Occupied,
    /// Bind failed for a non-occupancy reason (permission, invalid address, etc.).
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortProbeResult {
    pub status: PortProbeStatus,
    pub detail: String,
}

/// Probe whether `(addr, port)` can be bound locally. Does not retain the listener.
pub fn probe_bind(addr: &str, port: u16) -> PortProbeResult {
    let ip: IpAddr = match addr.parse() {
        Ok(ip) => ip,
        Err(_) if addr.eq_ignore_ascii_case("localhost") => IpAddr::from([127, 0, 0, 1]),
        Err(_) => {
            return PortProbeResult {
                status: PortProbeStatus::Error,
                detail: format!("invalid bind address {addr}"),
            };
        }
    };
    let socket = SocketAddr::new(ip, port);
    match TcpListener::bind(socket) {
        Ok(listener) => {
            drop(listener);
            PortProbeResult {
                status: PortProbeStatus::Available,
                detail: format!("{addr}:{port} is available"),
            }
        }
        Err(error) => {
            let kind = error.kind();
            let occupied = matches!(
                kind,
                std::io::ErrorKind::AddrInUse | std::io::ErrorKind::AddrNotAvailable
            ) || error.to_string().to_ascii_lowercase().contains("in use");
            if occupied {
                PortProbeResult {
                    status: PortProbeStatus::Occupied,
                    detail: format!("{addr}:{port} is in use ({kind:?})"),
                }
            } else {
                PortProbeResult {
                    status: PortProbeStatus::Error,
                    detail: format!("{addr}:{port} bind failed ({kind:?})"),
                }
            }
        }
    }
}
