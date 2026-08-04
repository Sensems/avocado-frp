use std::sync::Arc;

use tokio::sync::Mutex;

use crate::adapters::event_sink::EventSink;
use crate::domain::config::{
    ConfigChangeRequest, ConfigKind, ConfigSnapshot, SaveAndRestartRecovery, SaveAndRestartResult,
};
use crate::domain::error::CommandError;
use crate::domain::events::ConfigChangedEvent;
use crate::domain::process::{ProcessKind, ProcessPhase};
use crate::services::config_repository::ConfigRepository;
use crate::services::process_supervisor::ProcessSupervisor;

pub struct ConfigTransactionService {
    repository: Arc<ConfigRepository>,
    processes: Arc<ProcessSupervisor>,
    events: Arc<dyn EventSink>,
    frpc_lock: Mutex<()>,
    frps_lock: Mutex<()>,
}

impl ConfigTransactionService {
    pub fn new(
        repository: Arc<ConfigRepository>,
        processes: Arc<ProcessSupervisor>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            repository,
            processes,
            events,
            frpc_lock: Mutex::new(()),
            frps_lock: Mutex::new(()),
        }
    }

    pub async fn apply_change(
        &self,
        request: ConfigChangeRequest,
    ) -> Result<ConfigSnapshot, CommandError> {
        let kind = request.kind();
        let _guard = self.transaction_lock(kind).lock().await;
        let snapshot = self.repository.apply(request)?;
        self.emit_config(&snapshot);
        Ok(snapshot)
    }

    pub async fn restore_backup(
        &self,
        kind: ConfigKind,
        expected_revision: &str,
    ) -> Result<ConfigSnapshot, CommandError> {
        let _guard = self.transaction_lock(kind).lock().await;
        let snapshot = self.repository.restore_backup(kind, expected_revision)?;
        self.emit_config(&snapshot);
        Ok(snapshot)
    }

    pub async fn save_and_restart(
        &self,
        request: ConfigChangeRequest,
    ) -> Result<SaveAndRestartResult, CommandError> {
        let kind = request.kind();
        let process_kind = process_kind(kind);
        let _guard = self.transaction_lock(kind).lock().await;
        let old_state = self.repository.capture(kind)?;
        let old_config = self.repository.load(kind)?;
        let old_process = self.processes.snapshot(process_kind).await;
        let was_active = !matches!(
            old_process.phase,
            ProcessPhase::Stopped | ProcessPhase::Crashed
        );

        self.repository.preview(request.clone())?;
        if was_active {
            self.processes.stop(process_kind).await?;
        }

        let candidate = match self.repository.apply(request) {
            Ok(candidate) => candidate,
            Err(error) => {
                if was_active {
                    let _ = self.processes.start(process_kind).await;
                }
                return Err(error);
            }
        };

        match self.processes.start(process_kind).await {
            Ok(process) => {
                self.emit_config(&candidate);
                Ok(SaveAndRestartResult {
                    applied: true,
                    config: candidate,
                    process,
                    failure: None,
                    recovery: None,
                })
            }
            Err(failure) => {
                let candidate_revision = candidate.revision().to_string();
                match self
                    .repository
                    .restore_state(kind, &candidate_revision, old_state)
                {
                    Ok(restored) => {
                        debug_assert_eq!(restored.revision(), old_config.revision());
                        let process_recovery = if was_active {
                            self.processes.start(process_kind).await
                        } else {
                            self.processes.stop(process_kind).await
                        };
                        let (process, process_restored, recovery_error) = match process_recovery {
                            Ok(process) => (process, true, None),
                            Err(error) => (
                                self.processes.snapshot(process_kind).await,
                                false,
                                Some(error),
                            ),
                        };
                        self.emit_config(&restored);
                        Ok(SaveAndRestartResult {
                            applied: false,
                            config: restored,
                            process,
                            failure: Some(failure),
                            recovery: Some(SaveAndRestartRecovery {
                                config_restored: true,
                                process_restored,
                                error: recovery_error,
                            }),
                        })
                    }
                    Err(restore_error) => Ok(SaveAndRestartResult {
                        applied: false,
                        config: candidate,
                        process: self.processes.snapshot(process_kind).await,
                        failure: Some(failure),
                        recovery: Some(SaveAndRestartRecovery {
                            config_restored: false,
                            process_restored: false,
                            error: Some(restore_error),
                        }),
                    }),
                }
            }
        }
    }

    fn transaction_lock(&self, kind: ConfigKind) -> &Mutex<()> {
        match kind {
            ConfigKind::Frpc => &self.frpc_lock,
            ConfigKind::Frps => &self.frps_lock,
        }
    }

    fn emit_config(&self, snapshot: &ConfigSnapshot) {
        if let Err(error) = self
            .events
            .config_changed(&ConfigChangedEvent::from(snapshot))
        {
            eprintln!("configuration event delivery failed: {error}");
        }
    }
}

fn process_kind(kind: ConfigKind) -> ProcessKind {
    match kind {
        ConfigKind::Frpc => ProcessKind::Frpc,
        ConfigKind::Frps => ProcessKind::Frps,
    }
}
