use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::process::ProcessKind;

pub const SUPPORTED_FRP_VERSION: &str = "0.61.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarEvent {
    Stdout(String),
    Stderr(String),
    Error(String),
    Terminated(Option<i32>),
}

impl SidecarEvent {
    pub fn from_stdout(bytes: Vec<u8>) -> Self {
        Self::Stdout(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn from_stderr(bytes: Vec<u8>) -> Self {
        Self::Stderr(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarInfo {
    pub kind: ProcessKind,
    pub version: String,
}

pub struct SpawnedSidecar {
    pub pid: u32,
    pub child: Box<dyn SidecarChild>,
    pub events: tokio::sync::mpsc::Receiver<SidecarEvent>,
}

#[async_trait]
pub trait SidecarChild: Send {
    async fn request_stop(self: Box<Self>) -> Result<(), CommandError>;
}

#[async_trait]
pub trait SidecarAdapter: Send + Sync {
    async fn inspect(&self, kind: ProcessKind) -> Result<SidecarInfo, CommandError>;

    async fn spawn(
        &self,
        kind: ProcessKind,
        config_path: &Path,
    ) -> Result<SpawnedSidecar, CommandError>;

    async fn force_kill(&self, pid: u32) -> Result<(), CommandError>;
}

pub struct TauriSidecarAdapter {
    app: tauri::AppHandle,
}

impl TauriSidecarAdapter {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }

    fn command(
        &self,
        kind: ProcessKind,
    ) -> Result<tauri_plugin_shell::process::Command, CommandError> {
        self.app
            .shell()
            .sidecar(sidecar_name(kind))
            .map_err(|_| sidecar_missing(kind))
    }
}

struct TauriSidecarChild {
    child: Option<CommandChild>,
}

#[async_trait]
impl SidecarChild for TauriSidecarChild {
    async fn request_stop(mut self: Box<Self>) -> Result<(), CommandError> {
        let child = self.child.take().ok_or_else(|| {
            CommandError::new(
                ErrorCode::ProcessNotRunning,
                "sidecar process is no longer available",
                true,
            )
        })?;
        child.kill().map_err(|_| {
            CommandError::new(
                ErrorCode::StopTimeout,
                "failed to request sidecar shutdown",
                true,
            )
        })
    }
}

#[async_trait]
impl SidecarAdapter for TauriSidecarAdapter {
    async fn inspect(&self, kind: ProcessKind) -> Result<SidecarInfo, CommandError> {
        let (mut events, child) = self
            .command(kind)?
            .args(["--version"])
            .spawn()
            .map_err(|_| sidecar_missing(kind))?;

        let output = tokio::time::timeout(Duration::from_secs(3), async {
            let mut output = String::new();
            while let Some(event) = events.recv().await {
                match event {
                    CommandEvent::Stdout(bytes) | CommandEvent::Stderr(bytes) => {
                        output.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    CommandEvent::Error(_) => {
                        return Err(CommandError::new(
                            ErrorCode::SidecarMissing,
                            "failed to inspect FRP sidecar",
                            true,
                        ));
                    }
                    CommandEvent::Terminated(_) => break,
                    _ => {}
                }
            }
            Ok(output)
        })
        .await;

        let output = match output {
            Ok(result) => result?,
            Err(_) => {
                let _ = child.kill();
                return Err(CommandError::new(
                    ErrorCode::SidecarIncompatible,
                    "FRP sidecar version check timed out",
                    true,
                ));
            }
        };
        let version = parse_version(&output).ok_or_else(|| {
            CommandError::new(
                ErrorCode::SidecarIncompatible,
                "FRP sidecar did not report a compatible version",
                true,
            )
        })?;
        if version != SUPPORTED_FRP_VERSION {
            return Err(CommandError::new(
                ErrorCode::SidecarIncompatible,
                "FRP sidecar version is incompatible",
                true,
            )
            .with_detail(format!("expected {SUPPORTED_FRP_VERSION}, found {version}")));
        }
        Ok(SidecarInfo { kind, version })
    }

    async fn spawn(
        &self,
        kind: ProcessKind,
        config_path: &Path,
    ) -> Result<SpawnedSidecar, CommandError> {
        let config_path = config_path.to_string_lossy().into_owned();
        let (mut source, child) = self
            .command(kind)?
            .args(["-c", &config_path])
            .spawn()
            .map_err(|_| {
                CommandError::new(ErrorCode::SpawnFailed, "failed to start FRP sidecar", true)
            })?;
        let pid = child.pid();
        let (sender, receiver) = tokio::sync::mpsc::channel(128);
        tauri::async_runtime::spawn(async move {
            while let Some(event) = source.recv().await {
                let translated = match event {
                    CommandEvent::Stdout(bytes) => SidecarEvent::from_stdout(bytes),
                    CommandEvent::Stderr(bytes) => SidecarEvent::from_stderr(bytes),
                    CommandEvent::Error(error) => SidecarEvent::Error(error),
                    CommandEvent::Terminated(payload) => SidecarEvent::Terminated(payload.code),
                    _ => continue,
                };
                let terminated = matches!(translated, SidecarEvent::Terminated(_));
                if sender.send(translated).await.is_err() || terminated {
                    break;
                }
            }
        });
        Ok(SpawnedSidecar {
            pid,
            child: Box::new(TauriSidecarChild { child: Some(child) }),
            events: receiver,
        })
    }

    async fn force_kill(&self, pid: u32) -> Result<(), CommandError> {
        let killed = tokio::task::spawn_blocking(move || {
            use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

            let mut system = System::new();
            let process_id = Pid::from_u32(pid);
            system.refresh_processes(ProcessesToUpdate::Some(&[process_id]), true);
            system.process(process_id).is_none_or(|process| {
                process
                    .kill_with(Signal::Kill)
                    .unwrap_or_else(|| process.kill())
            })
        })
        .await
        .map_err(|_| {
            CommandError::new(
                ErrorCode::StopTimeout,
                "sidecar force-stop task failed",
                false,
            )
        })?;

        if killed {
            Ok(())
        } else {
            Err(CommandError::new(
                ErrorCode::StopTimeout,
                "failed to force-stop sidecar process",
                false,
            ))
        }
    }
}

fn sidecar_name(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Frpc => "frpc",
        ProcessKind::Frps => "frps",
    }
}

fn sidecar_missing(kind: ProcessKind) -> CommandError {
    CommandError::new(
        ErrorCode::SidecarMissing,
        format!("{} sidecar is unavailable", sidecar_name(kind)),
        true,
    )
}

fn parse_version(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|token| {
        let candidate =
            token.trim_matches(|character: char| !(character.is_ascii_digit() || character == '.'));
        let valid = candidate.split('.').count().ge(&3)
            && candidate
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.');
        valid.then(|| candidate.to_string())
    })
}
