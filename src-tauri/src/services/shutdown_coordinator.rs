use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use crate::domain::error::CommandError;
use crate::domain::process::StopAllResult;
use crate::services::process_supervisor::ProcessSupervisor;

#[async_trait]
pub trait ShutdownTarget: Send + Sync {
    async fn stop_all(&self) -> Result<StopAllResult, CommandError>;
}

#[async_trait]
impl ShutdownTarget for ProcessSupervisor {
    async fn stop_all(&self) -> Result<StopAllResult, CommandError> {
        ProcessSupervisor::stop_all(self).await
    }
}

pub struct ShutdownCoordinator {
    started: AtomicBool,
    completed: AtomicBool,
    target: Arc<dyn ShutdownTarget>,
    result: Mutex<Option<Result<StopAllResult, CommandError>>>,
    notify: Notify,
}

impl ShutdownCoordinator {
    pub fn new(target: Arc<dyn ShutdownTarget>) -> Self {
        Self {
            started: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            target,
            result: Mutex::new(None),
            notify: Notify::new(),
        }
    }

    pub fn is_completed(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }

    pub async fn prepare(&self) -> Result<StopAllResult, CommandError> {
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let result = self.target.stop_all().await;
            *self.result.lock().await = Some(result.clone());
            self.completed.store(true, Ordering::Release);
            self.notify.notify_waiters();
            return result;
        }

        loop {
            let notified = self.notify.notified();
            if self.completed.load(Ordering::Acquire) {
                return self
                    .result
                    .lock()
                    .await
                    .clone()
                    .expect("completed shutdown must have a result");
            }
            notified.await;
        }
    }
}
