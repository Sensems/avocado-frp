use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::domain::config::ConfigKind;
use crate::domain::error::{CommandError, ErrorCode};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn from_app(app: &tauri::AppHandle) -> Result<Self, CommandError> {
        let mut config_dir = app.path().app_config_dir().map_err(|_| {
            CommandError::new(
                ErrorCode::ConfigIo,
                "application configuration path is unavailable",
                false,
            )
        })?;
        #[cfg(debug_assertions)]
        {
            config_dir = config_dir.join("dev_data");
        }
        let log_dir = config_dir.join("logs");
        Ok(Self {
            config_dir,
            log_dir,
        })
    }

    pub fn config_path(&self, kind: ConfigKind) -> PathBuf {
        self.config_dir.join(match kind {
            ConfigKind::Frpc => "frpc.toml",
            ConfigKind::Frps => "frps.toml",
        })
    }
}

pub trait ConfigFilesystem: Send + Sync {
    fn read_utf8(&self, path: &Path) -> Result<Option<String>, CommandError>;
    fn exists(&self, path: &Path) -> bool;
    fn replace_with_backup(&self, path: &Path, bytes: &[u8]) -> Result<(), CommandError>;
    fn restore_backup(&self, path: &Path) -> Result<(), CommandError>;
    fn atomic_replace(&self, path: &Path, bytes: &[u8]) -> Result<(), CommandError>;
    fn remove(&self, path: &Path) -> Result<(), CommandError>;
}

#[derive(Debug, Default)]
pub struct RealConfigFilesystem;

impl ConfigFilesystem for RealConfigFilesystem {
    fn read_utf8(&self, path: &Path) -> Result<Option<String>, CommandError> {
        match std::fs::read_to_string(path) {
            Ok(raw) => Ok(Some(raw)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(map_config_io(error)),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn replace_with_backup(&self, path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
        match std::fs::read(path) {
            Ok(current) => atomic_persist(&backup_path(path), &current)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(map_config_io(error)),
        }
        atomic_persist(path, bytes)
    }

    fn restore_backup(&self, path: &Path) -> Result<(), CommandError> {
        let backup = std::fs::read(backup_path(path)).map_err(map_config_io)?;
        atomic_persist(path, &backup)
    }

    fn atomic_replace(&self, path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
        atomic_persist(path, bytes)
    }

    fn remove(&self, path: &Path) -> Result<(), CommandError> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(map_config_io(error)),
        }
    }
}

pub fn backup_path(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(".bak");
    PathBuf::from(name)
}

fn atomic_persist(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    let parent = path.parent().ok_or_else(|| {
        CommandError::new(ErrorCode::ConfigIo, "config path has no parent", false)
    })?;
    std::fs::create_dir_all(parent).map_err(map_config_io)?;

    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(map_config_io)?;
    staged.write_all(bytes).map_err(map_config_io)?;
    staged.flush().map_err(map_config_io)?;
    staged.as_file().sync_all().map_err(map_config_io)?;
    staged
        .persist(path)
        .map_err(|error| map_config_io(error.error))?;

    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(map_config_io)?;

    Ok(())
}

pub fn map_config_io(error: std::io::Error) -> CommandError {
    let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
        ErrorCode::PermissionDenied
    } else {
        ErrorCode::ConfigIo
    };
    CommandError::new(code, "configuration I/O failed", true)
        .with_detail(format!("{:?}", error.kind()))
}
