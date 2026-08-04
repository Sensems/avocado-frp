use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::adapters::filesystem::{
    map_config_io, AppPaths, ConfigFilesystem, RealConfigFilesystem,
};
use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::settings::{
    AppSettings, AppSettingsPatch, LocalMonitorPrefsPatch, LogPolicyPatch, MAX_MAX_FILE_BYTES,
    MAX_MAX_ROTATED_FILES, MIN_MAX_FILE_BYTES, MIN_MAX_ROTATED_FILES,
};

const SETTINGS_FILE_NAME: &str = "app-settings.json";

pub struct AppSettingsStore {
    path: PathBuf,
    filesystem: RealConfigFilesystem,
    inner: Mutex<AppSettings>,
}

impl AppSettingsStore {
    pub fn load_or_default(paths: &AppPaths) -> Result<Self, CommandError> {
        std::fs::create_dir_all(&paths.config_dir).map_err(map_config_io)?;
        let path = paths.config_dir.join(SETTINGS_FILE_NAME);
        let filesystem = RealConfigFilesystem;
        let settings = match filesystem.read_utf8(&path)? {
            None => AppSettings::default(),
            Some(raw) => serde_json::from_str(&raw).map_err(|error| {
                CommandError::new(
                    ErrorCode::ConfigInvalid,
                    "app settings file is invalid",
                    true,
                )
                .with_detail(error.to_string())
                .with_suggested_action("fix or delete app-settings.json and try again")
            })?,
        };

        Ok(Self {
            path,
            filesystem,
            inner: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn update(&self, patch: AppSettingsPatch) -> Result<AppSettings, CommandError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut next = guard.clone();
        apply_patch(&mut next, patch)?;
        next.schema_version = crate::domain::settings::APP_SETTINGS_SCHEMA_VERSION;
        persist(&self.filesystem, &self.path, &next)?;
        *guard = next.clone();
        Ok(next)
    }
}

fn apply_patch(settings: &mut AppSettings, patch: AppSettingsPatch) -> Result<(), CommandError> {
    if let Some(log_policy) = patch.log_policy {
        apply_log_policy_patch(&mut settings.log_policy, log_policy);
    }
    if let Some(local_monitor) = patch.local_monitor {
        apply_local_monitor_patch(&mut settings.local_monitor, local_monitor)?;
    }
    if let Some(shown) = patch.log_policy_notice_shown {
        settings.log_policy_notice_shown = shown;
    }
    if let Some(check_on_launch) = patch.check_updates_on_launch {
        settings.check_updates_on_launch = check_on_launch;
    }
    Ok(())
}

fn apply_log_policy_patch(policy: &mut crate::domain::settings::LogPolicy, patch: LogPolicyPatch) {
    if let Some(bytes) = patch.max_file_bytes {
        policy.max_file_bytes = bytes.clamp(MIN_MAX_FILE_BYTES, MAX_MAX_FILE_BYTES);
    }
    if let Some(files) = patch.max_rotated_files {
        policy.max_rotated_files = files.clamp(MIN_MAX_ROTATED_FILES, MAX_MAX_ROTATED_FILES);
    }
}

fn apply_local_monitor_patch(
    prefs: &mut crate::domain::settings::LocalMonitorPrefs,
    patch: LocalMonitorPrefsPatch,
) -> Result<(), CommandError> {
    if let Some(enabled) = patch.enabled {
        prefs.enabled = enabled;
    }
    if let Some(addr) = patch.addr {
        prefs.addr = addr;
    }
    if let Some(port) = patch.port {
        if port == 0 {
            return Err(CommandError::new(
                ErrorCode::ConfigInvalid,
                "local monitor port must be between 1 and 65535",
                true,
            )
            .with_suggested_action("choose a port in the range 1–65535"));
        }
        prefs.port = port;
    }
    if let Some(user) = patch.user {
        prefs.user = user.filter(|value| !value.is_empty());
    }
    if let Some(password) = patch.password {
        prefs.password = password.filter(|value| !value.is_empty());
    }
    validate_loopback_addr(&prefs.addr)?;
    Ok(())
}

fn validate_loopback_addr(addr: &str) -> Result<(), CommandError> {
    let trimmed = addr.trim();
    if trimmed.eq_ignore_ascii_case("localhost") {
        return Ok(());
    }
    let parsed = trimmed.parse::<IpAddr>().map_err(|_| {
        CommandError::new(
            ErrorCode::ConfigInvalid,
            "local monitor address is invalid",
            true,
        )
        .with_detail(format!("addr={trimmed}"))
        .with_suggested_action("use a loopback address such as 127.0.0.1")
    })?;
    if !parsed.is_loopback() {
        return Err(CommandError::new(
            ErrorCode::ConfigInvalid,
            "local monitor address must be loopback",
            true,
        )
        .with_detail(format!("addr={trimmed}"))
        .with_suggested_action("use a loopback address such as 127.0.0.1"));
    }
    Ok(())
}

fn persist(
    filesystem: &RealConfigFilesystem,
    path: &std::path::Path,
    settings: &AppSettings,
) -> Result<(), CommandError> {
    let bytes = serde_json::to_vec_pretty(settings).map_err(|error| {
        CommandError::new(
            ErrorCode::ConfigIo,
            "failed to serialize app settings",
            true,
        )
        .with_detail(error.to_string())
    })?;
    filesystem.atomic_replace(path, &bytes)
}
