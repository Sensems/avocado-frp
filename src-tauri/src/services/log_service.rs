use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::adapters::event_sink::EventSink;
use crate::adapters::filesystem::{map_config_io, AppPaths};
use crate::domain::error::CommandError;
use crate::domain::events::{ConfigChangedEvent, LogEntry, LogStream};
use crate::domain::process::{ProcessKind, ProcessSnapshot};
use crate::domain::settings::LogPolicy;
use crate::services::app_settings::AppSettingsStore;

pub struct LogService {
    log_dir: PathBuf,
    settings: Arc<AppSettingsStore>,
    write_lock: Mutex<()>,
}

/// EventSink adapter that persists redacted log lines via [`LogService`].
pub struct FileLogSink {
    logs: Arc<LogService>,
}

impl FileLogSink {
    pub fn new(logs: Arc<LogService>) -> Self {
        Self { logs }
    }
}

impl EventSink for FileLogSink {
    fn process_changed(&self, _snapshot: &ProcessSnapshot) -> Result<(), CommandError> {
        Ok(())
    }

    fn config_changed(&self, _event: &ConfigChangedEvent) -> Result<(), CommandError> {
        Ok(())
    }

    fn log_entry(&self, entry: &LogEntry) -> Result<(), CommandError> {
        self.logs.append(entry)
    }
}

impl LogService {
    pub fn new(paths: AppPaths, settings: Arc<AppSettingsStore>) -> Self {
        Self {
            log_dir: paths.log_dir,
            settings,
            write_lock: Mutex::new(()),
        }
    }

    pub fn append(&self, entry: &LogEntry) -> Result<(), CommandError> {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let policy = self.settings.get().log_policy;
        std::fs::create_dir_all(&self.log_dir).map_err(map_config_io)?;

        let active_path = self.active_path(entry.kind);
        let line = format_line(entry);
        let incoming = line.len() as u64;

        let current_len = match std::fs::metadata(&active_path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(map_config_io(error)),
        };

        // Close any metadata handle before rename: do not keep exclusive opens across rotate.
        if current_len > 0 && current_len.saturating_add(incoming) > policy.max_file_bytes {
            self.rotate(entry.kind, policy.max_rotated_files)?;
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)
            .map_err(map_config_io)?;
        file.write_all(line.as_bytes()).map_err(map_config_io)?;
        Ok(())
    }

    pub fn delete_disk_logs(&self, kind: Option<ProcessKind>) -> Result<(), CommandError> {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if !self.log_dir.exists() {
            return Ok(());
        }

        let kinds: Vec<ProcessKind> = match kind {
            Some(value) => vec![value],
            None => vec![ProcessKind::Frpc, ProcessKind::Frps],
        };

        let entries = std::fs::read_dir(&self.log_dir).map_err(map_config_io)?;
        for entry in entries {
            let entry = entry.map_err(map_config_io)?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if kinds.iter().any(|k| is_managed_log_name(*k, name)) {
                remove_if_present(&path)?;
            }
        }
        Ok(())
    }

    /// Optional cache hook; current writes always read policy from [`AppSettingsStore`].
    pub fn set_policy(&self, _policy: LogPolicy) {}

    fn rotate(&self, kind: ProcessKind, max_rotated_files: u32) -> Result<(), CommandError> {
        let base = active_filename(kind);
        let active = self.log_dir.join(base);
        if !active.exists() {
            return Ok(());
        }

        let oldest = self.log_dir.join(format!("{base}.{max_rotated_files}"));
        remove_if_present(&oldest)?;

        for index in (1..max_rotated_files).rev() {
            let from = self.log_dir.join(format!("{base}.{index}"));
            let to = self.log_dir.join(format!("{base}.{}", index + 1));
            if from.exists() {
                std::fs::rename(&from, &to).map_err(map_config_io)?;
            }
        }

        std::fs::rename(&active, self.log_dir.join(format!("{base}.1"))).map_err(map_config_io)?;
        Ok(())
    }

    fn active_path(&self, kind: ProcessKind) -> PathBuf {
        self.log_dir.join(active_filename(kind))
    }
}

fn active_filename(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Frpc => "frpc.log",
        ProcessKind::Frps => "frps.log",
    }
}

fn is_managed_log_name(kind: ProcessKind, name: &str) -> bool {
    let base = active_filename(kind);
    if name == base {
        return true;
    }
    let Some(suffix) = name.strip_prefix(base) else {
        return false;
    };
    let Some(number) = suffix.strip_prefix('.') else {
        return false;
    };
    !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
}

fn format_line(entry: &LogEntry) -> String {
    let marker = match entry.stream {
        LogStream::Stdout => "",
        LogStream::Stderr => " ERROR:",
    };
    format!(
        "[{}]{} {}\n",
        entry.timestamp,
        marker,
        entry.text.trim_end()
    )
}

fn remove_if_present(path: &Path) -> Result<(), CommandError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(map_config_io(error)),
    }
}
