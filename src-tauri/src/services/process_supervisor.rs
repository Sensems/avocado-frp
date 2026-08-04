use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{watch, Mutex};

use crate::adapters::event_sink::EventSink;
use crate::adapters::frp_admin::{HealthProbe, ProbeOutcome};
use crate::adapters::sidecar::{SidecarAdapter, SidecarChild, SidecarEvent};
use crate::domain::config::ConfigKind;
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::events::{LogEntry, LogStream};
use crate::domain::process::{ProcessKind, ProcessPhase, ProcessSnapshot, StopAllResult};
use crate::services::config_repository::ConfigRepository;

#[derive(Debug, Clone, Copy)]
pub struct SupervisorTiming {
    pub startup_grace: Duration,
    pub stop_timeout: Duration,
}

impl Default for SupervisorTiming {
    fn default() -> Self {
        Self {
            startup_grace: Duration::from_secs(1),
            stop_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Clone)]
pub struct ProcessSupervisor {
    inner: Arc<SupervisorInner>,
}

struct SupervisorInner {
    repository: Arc<ConfigRepository>,
    sidecar: Arc<dyn SidecarAdapter>,
    health: Arc<dyn HealthProbe>,
    events: Arc<dyn EventSink>,
    timing: SupervisorTiming,
    frpc: Arc<Mutex<ProcessRecord>>,
    frps: Arc<Mutex<ProcessRecord>>,
}

struct ProcessRecord {
    generation: u64,
    phase: ProcessPhase,
    pid: Option<u32>,
    child: Option<Box<dyn SidecarChild>>,
    started_at: Option<DateTime<Utc>>,
    config_revision: Option<String>,
    last_exit_code: Option<i32>,
    last_error: Option<CommandError>,
    termination: Option<watch::Receiver<TerminationState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminationState {
    Pending,
    Exited(Option<i32>),
}

impl ProcessRecord {
    fn new() -> Self {
        Self {
            generation: 0,
            phase: ProcessPhase::Stopped,
            pid: None,
            child: None,
            started_at: None,
            config_revision: None,
            last_exit_code: None,
            last_error: None,
            termination: None,
        }
    }

    fn snapshot(&self, kind: ProcessKind) -> ProcessSnapshot {
        let uptime_seconds = if matches!(
            self.phase,
            ProcessPhase::Starting
                | ProcessPhase::Healthy
                | ProcessPhase::Degraded
                | ProcessPhase::Stopping
        ) {
            self.started_at
                .map(|started_at| {
                    Utc::now()
                        .signed_duration_since(started_at)
                        .num_seconds()
                        .max(0) as u64
                })
                .unwrap_or(0)
        } else {
            0
        };
        ProcessSnapshot {
            kind,
            phase: self.phase,
            pid: self.pid,
            started_at: self.started_at.map(|value| value.to_rfc3339()),
            uptime_seconds,
            config_revision: self.config_revision.clone(),
            last_exit_code: self.last_exit_code,
            last_error: self.last_error.clone(),
        }
    }

    fn reset_for_start(&mut self) {
        self.phase = ProcessPhase::Starting;
        self.pid = None;
        self.child = None;
        self.started_at = None;
        self.config_revision = None;
        self.last_exit_code = None;
        self.last_error = None;
        self.termination = None;
    }

    fn normalize_stopped(&mut self) {
        self.phase = ProcessPhase::Stopped;
        self.pid = None;
        self.child = None;
        self.started_at = None;
        self.config_revision = None;
        self.termination = None;
    }
}

impl ProcessSupervisor {
    pub fn new(
        repository: Arc<ConfigRepository>,
        sidecar: Arc<dyn SidecarAdapter>,
        health: Arc<dyn HealthProbe>,
        events: Arc<dyn EventSink>,
        timing: SupervisorTiming,
    ) -> Self {
        Self {
            inner: Arc::new(SupervisorInner {
                repository,
                sidecar,
                health,
                events,
                timing,
                frpc: Arc::new(Mutex::new(ProcessRecord::new())),
                frps: Arc::new(Mutex::new(ProcessRecord::new())),
            }),
        }
    }

    pub async fn snapshot(&self, kind: ProcessKind) -> ProcessSnapshot {
        self.slot(kind).lock().await.snapshot(kind)
    }

    pub async fn start(&self, kind: ProcessKind) -> Result<ProcessSnapshot, CommandError> {
        let slot = self.slot(kind);
        let (generation, starting) = {
            let mut record = slot.lock().await;
            if !matches!(record.phase, ProcessPhase::Stopped | ProcessPhase::Crashed) {
                return Err(CommandError::new(
                    ErrorCode::ProcessAlreadyRunning,
                    "FRP process is already active",
                    true,
                ));
            }
            record.generation = record.generation.wrapping_add(1);
            record.reset_for_start();
            (record.generation, record.snapshot(kind))
        };
        self.emit_process(&starting);

        let config_kind = config_kind(kind);
        let config = match self.inner.repository.load_validated(config_kind) {
            Ok(config) => config,
            Err(error) => return Err(self.fail_start(kind, generation, error).await),
        };
        if let Err(error) = self.inner.sidecar.inspect(kind).await {
            return Err(self.fail_start(kind, generation, error).await);
        }
        let config_path = self.inner.repository.paths().config_path(config_kind);
        let spawned = match self.inner.sidecar.spawn(kind, &config_path).await {
            Ok(spawned) => spawned,
            Err(error) => return Err(self.fail_start(kind, generation, error).await),
        };

        let (termination_sender, mut termination_receiver) =
            watch::channel(TerminationState::Pending);
        let crate::adapters::sidecar::SpawnedSidecar { pid, child, events } = spawned;
        let mut child = Some(child);
        let installed = {
            let mut record = slot.lock().await;
            if record.generation != generation || record.phase != ProcessPhase::Starting {
                false
            } else {
                record.pid = Some(pid);
                record.child = child.take();
                record.started_at = Some(Utc::now());
                record.config_revision = Some(config.revision().to_string());
                record.termination = Some(termination_receiver.clone());
                true
            }
        };
        if !installed {
            if let Some(child) = child {
                child.request_stop().await?;
            }
            return Err(CommandError::new(
                ErrorCode::ProcessNotRunning,
                "FRP start was cancelled",
                true,
            ));
        }
        self.emit_process(&self.snapshot(kind).await);
        self.spawn_event_pump(kind, generation, events, termination_sender);

        if let Some(exit_code) =
            wait_for_startup_exit(&mut termination_receiver, self.inner.timing.startup_grace).await
        {
            let error = CommandError::new(
                ErrorCode::SpawnFailed,
                "FRP sidecar exited during startup",
                true,
            )
            .with_detail(exit_code.map_or_else(
                || "exit code unavailable".to_string(),
                |code| format!("exit code {code}"),
            ));
            let snapshot = self.snapshot(kind).await;
            if snapshot.phase != ProcessPhase::Crashed {
                return Err(self.fail_start(kind, generation, error).await);
            }
            return Err(error);
        }

        let (phase, health_error) = match self.inner.health.probe(&config).await {
            Ok(ProbeOutcome::Healthy | ProbeOutcome::NotConfigured) => {
                (ProcessPhase::Healthy, None)
            }
            Err(error) => (ProcessPhase::Degraded, Some(error)),
        };
        let snapshot = {
            let mut record = slot.lock().await;
            if record.generation != generation || record.phase != ProcessPhase::Starting {
                return Err(record.last_error.clone().unwrap_or_else(|| {
                    CommandError::new(
                        ErrorCode::SpawnFailed,
                        "FRP sidecar exited during startup",
                        true,
                    )
                }));
            }
            record.phase = phase;
            record.last_error = health_error;
            record.snapshot(kind)
        };
        self.emit_process(&snapshot);
        Ok(snapshot)
    }

    pub async fn stop(&self, kind: ProcessKind) -> Result<ProcessSnapshot, CommandError> {
        let slot = self.slot(kind);
        let (generation, child, pid, receiver, changed_phase) = {
            let mut record = slot.lock().await;
            match record.phase {
                ProcessPhase::Stopped => return Ok(record.snapshot(kind)),
                ProcessPhase::Starting if record.child.is_none() => {
                    record.generation = record.generation.wrapping_add(1);
                    record.normalize_stopped();
                    let snapshot = record.snapshot(kind);
                    drop(record);
                    self.emit_process(&snapshot);
                    return Ok(snapshot);
                }
                ProcessPhase::Crashed if record.child.is_none() => {
                    record.normalize_stopped();
                    let snapshot = record.snapshot(kind);
                    drop(record);
                    self.emit_process(&snapshot);
                    return Ok(snapshot);
                }
                ProcessPhase::Stopping => (
                    record.generation,
                    None,
                    record.pid,
                    record.termination.clone(),
                    false,
                ),
                _ => {
                    record.phase = ProcessPhase::Stopping;
                    (
                        record.generation,
                        record.child.take(),
                        record.pid,
                        record.termination.clone(),
                        true,
                    )
                }
            }
        };
        if changed_phase {
            self.emit_process(&self.snapshot(kind).await);
        }

        let mut stop_error = None;
        if let Some(child) = child {
            if let Err(error) = child.request_stop().await {
                stop_error = Some(error);
            }
        }

        let exited = if stop_error.is_none() {
            match receiver.clone() {
                Some(receiver) => {
                    wait_for_termination(receiver, self.inner.timing.stop_timeout).await
                }
                None => true,
            }
        } else {
            false
        };
        if exited {
            return Ok(self.snapshot(kind).await);
        }

        let timeout_error = stop_error.unwrap_or_else(|| {
            CommandError::new(
                ErrorCode::StopTimeout,
                "FRP sidecar did not stop before the timeout",
                true,
            )
        });
        if let Some(pid) = pid {
            if let Err(force_error) = self.inner.sidecar.force_kill(pid).await {
                return Err(self.mark_stop_failed(kind, generation, force_error).await);
            }
        }
        let confirmed = match receiver {
            Some(receiver) => {
                wait_for_termination(
                    receiver,
                    self.inner.timing.stop_timeout.min(Duration::from_secs(1)),
                )
                .await
            }
            None => true,
        };
        if confirmed {
            let snapshot = {
                let mut record = slot.lock().await;
                if record.generation == generation {
                    record.normalize_stopped();
                    record.last_error = Some(timeout_error.clone());
                }
                record.snapshot(kind)
            };
            self.emit_process(&snapshot);
        } else {
            let _ = self
                .mark_stop_failed(kind, generation, timeout_error.clone())
                .await;
        }
        Err(timeout_error)
    }

    pub async fn restart(&self, kind: ProcessKind) -> Result<ProcessSnapshot, CommandError> {
        let snapshot = self.snapshot(kind).await;
        if !matches!(
            snapshot.phase,
            ProcessPhase::Stopped | ProcessPhase::Crashed
        ) {
            self.stop(kind).await?;
        }
        self.start(kind).await
    }

    pub async fn stop_all(&self) -> Result<StopAllResult, CommandError> {
        let (frpc_result, frps_result) =
            tokio::join!(self.stop(ProcessKind::Frpc), self.stop(ProcessKind::Frps));
        let mut errors = Vec::new();
        let frpc = match frpc_result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                errors.push(error);
                self.snapshot(ProcessKind::Frpc).await
            }
        };
        let frps = match frps_result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                errors.push(error);
                self.snapshot(ProcessKind::Frps).await
            }
        };
        Ok(StopAllResult { frpc, frps, errors })
    }

    fn slot(&self, kind: ProcessKind) -> Arc<Mutex<ProcessRecord>> {
        match kind {
            ProcessKind::Frpc => self.inner.frpc.clone(),
            ProcessKind::Frps => self.inner.frps.clone(),
        }
    }

    fn emit_process(&self, snapshot: &ProcessSnapshot) {
        if let Err(error) = self.inner.events.process_changed(snapshot) {
            eprintln!("process event delivery failed: {error}");
        }
    }

    async fn fail_start(
        &self,
        kind: ProcessKind,
        generation: u64,
        error: CommandError,
    ) -> CommandError {
        let slot = self.slot(kind);
        let snapshot = {
            let mut record = slot.lock().await;
            if record.generation == generation && record.phase == ProcessPhase::Starting {
                record.phase = ProcessPhase::Crashed;
                record.pid = None;
                record.child = None;
                record.started_at = None;
                record.termination = None;
                record.last_error = Some(error.clone());
            }
            record.snapshot(kind)
        };
        self.emit_process(&snapshot);
        error
    }

    async fn mark_stop_failed(
        &self,
        kind: ProcessKind,
        generation: u64,
        error: CommandError,
    ) -> CommandError {
        let slot = self.slot(kind);
        let snapshot = {
            let mut record = slot.lock().await;
            if record.generation == generation && record.phase == ProcessPhase::Stopping {
                record.phase = ProcessPhase::Crashed;
                record.pid = None;
                record.child = None;
                record.started_at = None;
                record.termination = None;
                record.last_error = Some(error.clone());
            }
            record.snapshot(kind)
        };
        self.emit_process(&snapshot);
        error
    }

    fn spawn_event_pump(
        &self,
        kind: ProcessKind,
        generation: u64,
        mut source: tokio::sync::mpsc::Receiver<SidecarEvent>,
        termination: watch::Sender<TerminationState>,
    ) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut saw_termination = false;
            while let Some(event) = source.recv().await {
                match event {
                    SidecarEvent::Stdout(text) => {
                        supervisor.emit_log(kind, LogStream::Stdout, text);
                    }
                    SidecarEvent::Stderr(text) | SidecarEvent::Error(text) => {
                        supervisor.emit_log(kind, LogStream::Stderr, text);
                    }
                    SidecarEvent::Terminated(code) => {
                        supervisor
                            .handle_termination(kind, generation, code, &termination)
                            .await;
                        saw_termination = true;
                        break;
                    }
                }
            }
            if !saw_termination {
                supervisor
                    .handle_termination(kind, generation, None, &termination)
                    .await;
            }
        });
    }

    fn emit_log(&self, kind: ProcessKind, stream: LogStream, text: String) {
        let entry = LogEntry {
            kind,
            stream,
            text,
            timestamp: Utc::now().to_rfc3339(),
        };
        if let Err(error) = self.inner.events.log_entry(&entry) {
            eprintln!("log event delivery failed: {error}");
        }
    }

    async fn handle_termination(
        &self,
        kind: ProcessKind,
        generation: u64,
        code: Option<i32>,
        termination: &watch::Sender<TerminationState>,
    ) {
        let slot = self.slot(kind);
        let snapshot = {
            let mut record = slot.lock().await;
            if record.generation != generation {
                let _ = termination.send(TerminationState::Exited(code));
                return;
            }
            let stopping = record.phase == ProcessPhase::Stopping;
            record.child = None;
            record.pid = None;
            record.started_at = None;
            record.termination = None;
            record.last_exit_code = code;
            if stopping {
                record.phase = ProcessPhase::Stopped;
                record.config_revision = None;
                record.last_error = None;
            } else {
                record.phase = ProcessPhase::Crashed;
                record.last_error = Some(
                    CommandError::new(
                        ErrorCode::SpawnFailed,
                        "FRP sidecar exited unexpectedly",
                        true,
                    )
                    .with_detail(code.map_or_else(
                        || "exit code unavailable".to_string(),
                        |value| format!("exit code {value}"),
                    )),
                );
            }
            record.snapshot(kind)
        };
        self.emit_process(&snapshot);
        let _ = termination.send(TerminationState::Exited(code));
    }
}

async fn wait_for_startup_exit(
    receiver: &mut watch::Receiver<TerminationState>,
    grace: Duration,
) -> Option<Option<i32>> {
    if let TerminationState::Exited(code) = *receiver.borrow() {
        return Some(code);
    }
    match tokio::time::timeout(grace, receiver.changed()).await {
        Ok(Ok(())) => match *receiver.borrow() {
            TerminationState::Exited(code) => Some(code),
            TerminationState::Pending => None,
        },
        Ok(Err(_)) => Some(None),
        Err(_) => None,
    }
}

async fn wait_for_termination(
    mut receiver: watch::Receiver<TerminationState>,
    timeout: Duration,
) -> bool {
    if matches!(*receiver.borrow(), TerminationState::Exited(_)) {
        return true;
    }
    tokio::time::timeout(timeout, async {
        loop {
            if receiver.changed().await.is_err() {
                return false;
            }
            if matches!(*receiver.borrow(), TerminationState::Exited(_)) {
                return true;
            }
        }
    })
    .await
    .unwrap_or(false)
}

fn config_kind(kind: ProcessKind) -> ConfigKind {
    match kind {
        ProcessKind::Frpc => ConfigKind::Frpc,
        ProcessKind::Frps => ConfigKind::Frps,
    }
}
