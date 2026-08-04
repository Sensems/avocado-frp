use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};

use sha2::{Digest, Sha256};
use similar::TextDiff;
use toml_edit::{value as toml_value, Array, DocumentMut, Item, Table, TableLike, Value};

use crate::adapters::event_sink::redact_secrets;
use crate::adapters::filesystem::{backup_path, AppPaths, ConfigFilesystem};
use crate::domain::config::{
    AuthConfigPatch, AuthKnownConfig, ConfigChangeRequest, ConfigDiff, ConfigKind, ConfigPreview,
    ConfigSnapshot, FrpcChange, FrpcConfigPatch, FrpcKnownConfig, FrpsChange, FrpsConfigPatch,
    FrpsKnownConfig, ProxyOperation, ProxyRuleKnown, ProxyRulePatch, ProxySelector,
    ValidationIssue, ValidationReport, ValidationSeverity, WebServerConfigPatch,
    WebServerKnownConfig,
};
use crate::domain::error::{CommandError, ErrorCode};

pub struct ConfigRepository {
    paths: AppPaths,
    filesystem: Arc<dyn ConfigFilesystem>,
    frpc_lock: Mutex<()>,
    frps_lock: Mutex<()>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfigFileState {
    pub existed: bool,
    pub raw: String,
    pub revision: String,
}

struct Candidate {
    raw: String,
    issues: Vec<ValidationIssue>,
    changed_paths: Vec<String>,
    requires_confirmation: bool,
    source_mode: bool,
}

impl ConfigRepository {
    pub fn new(paths: AppPaths, filesystem: Arc<dyn ConfigFilesystem>) -> Self {
        Self {
            paths,
            filesystem,
            frpc_lock: Mutex::new(()),
            frps_lock: Mutex::new(()),
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn load(&self, kind: ConfigKind) -> Result<ConfigSnapshot, CommandError> {
        let path = self.paths.config_path(kind);
        let raw = self.filesystem.read_utf8(&path)?.unwrap_or_default();
        let backup_available = self.filesystem.exists(&backup_path(&path));
        Ok(snapshot_from_raw(kind, raw, backup_available))
    }

    pub fn load_validated(&self, kind: ConfigKind) -> Result<ConfigSnapshot, CommandError> {
        let snapshot = self.load(kind)?;
        if let Some(issue) = snapshot_issues(&snapshot)
            .iter()
            .find(|issue| issue.severity == ValidationSeverity::Error)
        {
            return Err(invalid_config_error(issue));
        }
        Ok(snapshot)
    }

    pub fn validate_source(&self, kind: ConfigKind, raw: &str) -> ValidationReport {
        ValidationReport {
            issues: parse_and_validate(kind, raw)
                .map(|(_, issues)| issues)
                .unwrap_or_else(|issue| vec![issue]),
        }
    }

    pub fn preview(&self, request: ConfigChangeRequest) -> Result<ConfigPreview, CommandError> {
        let kind = request.kind();
        let _guard = self.lock(kind)?;
        let current = self
            .filesystem
            .read_utf8(&self.paths.config_path(kind))?
            .unwrap_or_default();
        ensure_revision(&current, request.expected_revision())?;
        let candidate = candidate_for_request(&current, &request)?;
        Ok(preview_for_candidate(&current, candidate))
    }

    pub fn apply(&self, request: ConfigChangeRequest) -> Result<ConfigSnapshot, CommandError> {
        let kind = request.kind();
        let _guard = self.lock(kind)?;
        let path = self.paths.config_path(kind);
        let current = self.filesystem.read_utf8(&path)?.unwrap_or_default();
        ensure_revision(&current, request.expected_revision())?;
        let candidate = candidate_for_request(&current, &request)?;
        reject_errors(&candidate.issues)?;
        self.filesystem
            .replace_with_backup(&path, candidate.raw.as_bytes())?;
        self.load(kind)
    }

    pub fn restore_backup(
        &self,
        kind: ConfigKind,
        expected_revision: &str,
    ) -> Result<ConfigSnapshot, CommandError> {
        let _guard = self.lock(kind)?;
        let path = self.paths.config_path(kind);
        let current = self.filesystem.read_utf8(&path)?.unwrap_or_default();
        ensure_revision(&current, expected_revision)?;
        let backup = self
            .filesystem
            .read_utf8(&backup_path(&path))?
            .ok_or_else(|| {
                CommandError::new(
                    ErrorCode::ConfigIo,
                    "configuration backup is not available",
                    true,
                )
            })?;
        let report = self.validate_source(kind, &backup);
        reject_errors(&report.issues)?;
        self.filesystem.restore_backup(&path)?;
        self.load(kind)
    }

    pub(crate) fn capture(&self, kind: ConfigKind) -> Result<ConfigFileState, CommandError> {
        let path = self.paths.config_path(kind);
        let raw = self.filesystem.read_utf8(&path)?;
        let existed = raw.is_some();
        let raw = raw.unwrap_or_default();
        Ok(ConfigFileState {
            existed,
            revision: revision(&raw),
            raw,
        })
    }

    pub(crate) fn restore_state(
        &self,
        kind: ConfigKind,
        candidate_revision: &str,
        state: ConfigFileState,
    ) -> Result<ConfigSnapshot, CommandError> {
        let _guard = self.lock(kind)?;
        let path = self.paths.config_path(kind);
        let current = self.filesystem.read_utf8(&path)?.unwrap_or_default();
        ensure_revision(&current, candidate_revision)?;
        if state.existed {
            self.filesystem
                .atomic_replace(&path, state.raw.as_bytes())?;
        } else {
            self.filesystem.remove(&path)?;
        }
        let restored = self.load(kind)?;
        if restored.revision() != state.revision {
            return Err(CommandError::new(
                ErrorCode::ConfigConflict,
                "configuration recovery revision mismatch",
                false,
            ));
        }
        Ok(restored)
    }

    fn lock(&self, kind: ConfigKind) -> Result<MutexGuard<'_, ()>, CommandError> {
        match kind {
            ConfigKind::Frpc => self.frpc_lock.lock(),
            ConfigKind::Frps => self.frps_lock.lock(),
        }
        .map_err(|_| {
            CommandError::new(
                ErrorCode::Unknown,
                "configuration transaction lock is unavailable",
                true,
            )
        })
    }
}

pub fn revision(raw: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(raw.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[usize::from(byte >> 4)] as char);
        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

fn snapshot_from_raw(kind: ConfigKind, raw: String, backup_available: bool) -> ConfigSnapshot {
    match parse_and_validate(kind, &raw) {
        Ok((document, issues)) => match kind {
            ConfigKind::Frpc => ConfigSnapshot::Frpc {
                known: extract_frpc(&document),
                revision: revision(&raw),
                raw,
                issues,
                backup_available,
            },
            ConfigKind::Frps => ConfigSnapshot::Frps {
                known: extract_frps(&document),
                revision: revision(&raw),
                raw,
                issues,
                backup_available,
            },
        },
        Err(issue) => match kind {
            ConfigKind::Frpc => ConfigSnapshot::Frpc {
                revision: revision(&raw),
                raw,
                known: FrpcKnownConfig::default(),
                issues: vec![issue],
                backup_available,
            },
            ConfigKind::Frps => ConfigSnapshot::Frps {
                revision: revision(&raw),
                raw,
                known: FrpsKnownConfig::default(),
                issues: vec![issue],
                backup_available,
            },
        },
    }
}

fn parse_and_validate(
    kind: ConfigKind,
    raw: &str,
) -> Result<(DocumentMut, Vec<ValidationIssue>), ValidationIssue> {
    let document = raw.parse::<DocumentMut>().map_err(|error| {
        let (line, column) = error
            .span()
            .map(|span| line_column(raw, span.start))
            .unwrap_or((1, 1));
        ValidationIssue {
            severity: ValidationSeverity::Error,
            code: "TOML_SYNTAX".to_string(),
            message: "configuration contains invalid TOML syntax".to_string(),
            path: None,
            line: Some(line),
            column: Some(column),
        }
    })?;
    let issues = validate_document(kind, &document);
    Ok((document, issues))
}

fn line_column(raw: &str, offset: usize) -> (usize, usize) {
    let prefix = &raw.as_bytes()[..offset.min(raw.len())];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len() + 1, |index| prefix.len() - index);
    (line, column)
}

fn extract_frpc(document: &DocumentMut) -> FrpcKnownConfig {
    let root = document.as_table();
    let proxies = root
        .get("proxies")
        .and_then(Item::as_array_of_tables)
        .map(|tables| {
            tables
                .iter()
                .enumerate()
                .map(|(index, table)| {
                    let name = table_string(table, "name");
                    ProxyRuleKnown {
                        source_index: index,
                        source_name: name.clone().unwrap_or_default(),
                        name,
                        proxy_type: table_string(table, "type"),
                        local_ip: table_string(table, "localIP")
                            .or_else(|| table_string(table, "localIp")),
                        local_port: table_u16(table, "localPort"),
                        remote_port: table_u16(table, "remotePort"),
                        custom_domains: table_strings(table, "customDomains"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    FrpcKnownConfig {
        server_addr: table_string(root, "serverAddr"),
        server_port: table_u16(root, "serverPort"),
        auth: extract_auth(root),
        web_server: extract_web_server(root),
        proxies,
    }
}

fn extract_frps(document: &DocumentMut) -> FrpsKnownConfig {
    let root = document.as_table();
    FrpsKnownConfig {
        bind_port: table_u16(root, "bindPort"),
        vhost_http_port: table_u16(root, "vhostHTTPPort"),
        vhost_https_port: table_u16(root, "vhostHTTPSPort"),
        auth: extract_auth(root),
        web_server: extract_web_server(root),
    }
}

fn extract_auth(root: &dyn TableLike) -> AuthKnownConfig {
    root.get("auth")
        .and_then(Item::as_table_like)
        .map(|auth| AuthKnownConfig {
            method: table_string(auth, "method"),
            token: table_string(auth, "token"),
        })
        .unwrap_or_default()
}

fn extract_web_server(root: &dyn TableLike) -> WebServerKnownConfig {
    root.get("webServer")
        .and_then(Item::as_table_like)
        .map(|web_server| WebServerKnownConfig {
            addr: table_string(web_server, "addr"),
            port: table_u16(web_server, "port"),
            user: table_string(web_server, "user"),
            password: table_string(web_server, "password"),
        })
        .unwrap_or_default()
}

fn table_string(table: &dyn TableLike, key: &str) -> Option<String> {
    table.get(key).and_then(Item::as_str).map(ToOwned::to_owned)
}

fn table_u16(table: &dyn TableLike, key: &str) -> Option<u16> {
    table
        .get(key)
        .and_then(Item::as_integer)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn table_strings(table: &dyn TableLike, key: &str) -> Option<Vec<String>> {
    table.get(key).and_then(Item::as_array).map(|array| {
        array
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn validate_document(kind: ConfigKind, document: &DocumentMut) -> Vec<ValidationIssue> {
    let root = document.as_table();
    let mut issues = Vec::new();
    match kind {
        ConfigKind::Frpc => {
            validate_host(root, "serverAddr", "serverAddr", &mut issues);
            validate_port(root, "serverPort", "serverPort", &mut issues);
            validate_proxies(root, &mut issues);
        }
        ConfigKind::Frps => {
            validate_host(root, "bindAddr", "bindAddr", &mut issues);
            validate_port(root, "bindPort", "bindPort", &mut issues);
            validate_port(root, "vhostHTTPPort", "vhostHTTPPort", &mut issues);
            validate_port(root, "vhostHTTPSPort", "vhostHTTPSPort", &mut issues);
        }
    }
    validate_auth(root, &mut issues);
    if let Some(web_server) = root.get("webServer").and_then(Item::as_table_like) {
        validate_host(web_server, "addr", "webServer.addr", &mut issues);
        validate_port(web_server, "port", "webServer.port", &mut issues);
    }
    issues
}

fn validate_auth(root: &dyn TableLike, issues: &mut Vec<ValidationIssue>) {
    let Some(auth) = root.get("auth").and_then(Item::as_table_like) else {
        return;
    };
    let Some(method_item) = auth.get("method") else {
        return;
    };
    let Some(method) = method_item.as_str() else {
        push_error(
            issues,
            "AUTH_METHOD_INVALID",
            "auth method must be a string",
            "auth.method",
        );
        return;
    };
    if method.eq_ignore_ascii_case("token") {
        if auth
            .get("token")
            .and_then(Item::as_str)
            .is_none_or(|token| token.trim().is_empty())
        {
            push_error(
                issues,
                "AUTH_TOKEN_REQUIRED",
                "token authentication requires a non-empty token",
                "auth.token",
            );
        }
    } else if !method.eq_ignore_ascii_case("oidc") {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Warning,
            code: "AUTH_METHOD_ADVANCED".to_string(),
            message: "unrecognized authentication method is preserved as advanced configuration"
                .to_string(),
            path: Some("auth.method".to_string()),
            line: None,
            column: None,
        });
    }
}

fn validate_proxies(root: &dyn TableLike, issues: &mut Vec<ValidationIssue>) {
    let Some(proxies) = root.get("proxies").and_then(Item::as_array_of_tables) else {
        return;
    };
    let mut names = HashSet::new();
    let mut remote_ports = HashSet::new();
    for (index, proxy) in proxies.iter().enumerate() {
        let prefix = format!("proxies[{index}]");
        let name = proxy.get("name").and_then(Item::as_str);
        match name {
            Some(name) if !name.trim().is_empty() => {
                if !names.insert(name.to_string()) {
                    push_error(
                        issues,
                        "PROXY_NAME_DUPLICATE",
                        "proxy names must be unique",
                        &format!("{prefix}.name"),
                    );
                }
            }
            _ => push_error(
                issues,
                "PROXY_NAME_REQUIRED",
                "proxy name must be a non-empty string",
                &format!("{prefix}.name"),
            ),
        }

        validate_port(proxy, "localPort", &format!("{prefix}.localPort"), issues);
        validate_port(proxy, "remotePort", &format!("{prefix}.remotePort"), issues);

        let proxy_type = proxy
            .get("type")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(proxy_type.as_str(), "http" | "https") {
            let has_domain = proxy
                .get("customDomains")
                .and_then(Item::as_array)
                .is_some_and(|domains| {
                    domains.iter().any(|domain| {
                        domain
                            .as_str()
                            .is_some_and(|value| !value.trim().is_empty())
                    })
                });
            if !has_domain {
                push_error(
                    issues,
                    "PROXY_DOMAIN_REQUIRED",
                    "HTTP proxies require at least one custom domain",
                    &format!("{prefix}.customDomains"),
                );
            }
        } else {
            let remote_port = proxy.get("remotePort").and_then(Item::as_integer);
            if remote_port.is_none_or(|port| !(1..=65_535).contains(&port)) {
                push_error(
                    issues,
                    "PROXY_REMOTE_PORT_REQUIRED",
                    "non-HTTP proxies require a valid remote port",
                    &format!("{prefix}.remotePort"),
                );
            } else if let Some(port) = remote_port {
                if !remote_ports.insert(port) {
                    push_error(
                        issues,
                        "PROXY_REMOTE_PORT_DUPLICATE",
                        "proxy remote ports must be unique",
                        &format!("{prefix}.remotePort"),
                    );
                }
            }
        }
    }
}

fn validate_port(table: &dyn TableLike, key: &str, path: &str, issues: &mut Vec<ValidationIssue>) {
    let Some(item) = table.get(key) else {
        return;
    };
    if item
        .as_integer()
        .is_none_or(|port| !(1..=65_535).contains(&port))
    {
        push_error(
            issues,
            "PORT_INVALID",
            "port must be an integer between 1 and 65535",
            path,
        );
    }
}

fn validate_host(table: &dyn TableLike, key: &str, path: &str, issues: &mut Vec<ValidationIssue>) {
    let Some(item) = table.get(key) else {
        return;
    };
    if item.as_str().is_none_or(|host| !is_valid_host(host)) {
        push_error(
            issues,
            "HOST_INVALID",
            "address must be a valid IP address, localhost, or DNS name",
            path,
        );
    }
}

fn is_valid_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok() {
        return true;
    }
    let host = host.strip_suffix('.').unwrap_or(host);
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

fn push_error(issues: &mut Vec<ValidationIssue>, code: &str, message: &str, path: &str) {
    issues.push(ValidationIssue {
        severity: ValidationSeverity::Error,
        code: code.to_string(),
        message: message.to_string(),
        path: Some(path.to_string()),
        line: None,
        column: None,
    });
}

fn candidate_for_request(
    current: &str,
    request: &ConfigChangeRequest,
) -> Result<Candidate, CommandError> {
    let (raw, changed_paths, requires_confirmation, source_mode) = match request {
        ConfigChangeRequest::Frpc { change, .. } => match change {
            FrpcChange::Patch { patch } => {
                let mut document = parse_for_patch(ConfigKind::Frpc, current)?;
                apply_frpc_patch(&mut document, patch)?;
                let paths = frpc_changed_paths(patch);
                let confirmation = paths.iter().any(|path| {
                    path.starts_with("server")
                        || path.starts_with("auth")
                        || path.starts_with("webServer")
                        || path.ends_with(".$delete")
                });
                (document.to_string(), paths, confirmation, false)
            }
            FrpcChange::Source { raw } => (raw.clone(), vec!["$source".to_string()], true, true),
        },
        ConfigChangeRequest::Frps { change, .. } => match change {
            FrpsChange::Patch { patch } => {
                let mut document = parse_for_patch(ConfigKind::Frps, current)?;
                apply_frps_patch(&mut document, patch)?;
                let paths = frps_changed_paths(patch);
                let confirmation = paths.iter().any(|path| {
                    path.starts_with("bind")
                        || path.starts_with("vhost")
                        || path.starts_with("auth")
                        || path.starts_with("webServer")
                });
                (document.to_string(), paths, confirmation, false)
            }
            FrpsChange::Source { raw } => (raw.clone(), vec!["$source".to_string()], true, true),
        },
    };
    let issues = parse_and_validate(request.kind(), &raw)
        .map(|(_, issues)| issues)
        .map_err(|issue| invalid_config_error(&issue))?;
    reject_errors(&issues)?;
    Ok(Candidate {
        raw,
        issues,
        changed_paths,
        requires_confirmation,
        source_mode,
    })
}

fn parse_for_patch(kind: ConfigKind, current: &str) -> Result<DocumentMut, CommandError> {
    parse_and_validate(kind, current)
        .map(|(document, _)| document)
        .map_err(|issue| invalid_config_error(&issue))
}

fn preview_for_candidate(current: &str, candidate: Candidate) -> ConfigPreview {
    let (before, after) = if candidate.source_mode {
        (current.to_string(), candidate.raw.clone())
    } else {
        (redact_secrets(current), redact_secrets(&candidate.raw))
    };
    let unified = TextDiff::from_lines(&before, &after)
        .unified_diff()
        .header("current", "candidate")
        .to_string();
    ConfigPreview {
        diff: ConfigDiff {
            unified,
            changed_paths: candidate.changed_paths,
            requires_confirmation: candidate.requires_confirmation,
        },
        issues: candidate.issues,
    }
}

fn apply_frpc_patch(
    document: &mut DocumentMut,
    patch: &FrpcConfigPatch,
) -> Result<(), CommandError> {
    let root = document.as_table_mut();
    apply_string(root, "serverAddr", &patch.server_addr);
    apply_u16(root, "serverPort", &patch.server_port);
    if let Some(auth) = &patch.auth {
        let table = ensure_table(root, "auth")?;
        apply_auth_patch(table, auth);
    }
    if let Some(web_server) = &patch.web_server {
        apply_web_server_section(root, web_server)?;
    }
    for operation in &patch.proxy_operations {
        apply_proxy_operation(root, operation)?;
    }
    Ok(())
}

fn apply_frps_patch(
    document: &mut DocumentMut,
    patch: &FrpsConfigPatch,
) -> Result<(), CommandError> {
    let root = document.as_table_mut();
    apply_u16(root, "bindPort", &patch.bind_port);
    apply_u16(root, "vhostHTTPPort", &patch.vhost_http_port);
    apply_u16(root, "vhostHTTPSPort", &patch.vhost_https_port);
    if let Some(auth) = &patch.auth {
        let table = ensure_table(root, "auth")?;
        apply_auth_patch(table, auth);
    }
    if let Some(web_server) = &patch.web_server {
        apply_web_server_section(root, web_server)?;
    }
    Ok(())
}

/// Patch webServer keys; drop the table when it becomes empty (preserves unknown keys).
fn apply_web_server_section(
    root: &mut Table,
    patch: &WebServerConfigPatch,
) -> Result<(), CommandError> {
    {
        let table = ensure_table(root, "webServer")?;
        apply_web_server_patch(table, patch);
    }
    if root
        .get("webServer")
        .and_then(Item::as_table)
        .is_some_and(toml_edit::Table::is_empty)
    {
        root.remove("webServer");
    }
    Ok(())
}

fn apply_auth_patch(table: &mut Table, patch: &AuthConfigPatch) {
    apply_string(table, "method", &patch.method);
    apply_string(table, "token", &patch.token);
}

fn apply_web_server_patch(table: &mut Table, patch: &WebServerConfigPatch) {
    apply_string(table, "addr", &patch.addr);
    apply_u16(table, "port", &patch.port);
    apply_string(table, "user", &patch.user);
    apply_string(table, "password", &patch.password);
}

fn apply_proxy_operation(root: &mut Table, operation: &ProxyOperation) -> Result<(), CommandError> {
    if root.get("proxies").is_none() {
        root.insert("proxies", Item::ArrayOfTables(Default::default()));
    }
    let proxies = root
        .get_mut("proxies")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or_else(|| {
            CommandError::new(
                ErrorCode::ConfigInvalid,
                "proxies must be an array of tables",
                true,
            )
        })?;
    match operation {
        ProxyOperation::Add { rule } => {
            let mut table = Table::new();
            apply_proxy_patch(&mut table, rule);
            proxies.push(table);
        }
        ProxyOperation::Update { selector, patch } => {
            let table = selected_proxy_mut(proxies, selector)?;
            apply_proxy_patch(table, patch);
        }
        ProxyOperation::Delete { selector } => {
            ensure_selector(proxies, selector)?;
            proxies.remove(selector.index);
        }
    }
    Ok(())
}

fn selected_proxy_mut<'a>(
    proxies: &'a mut toml_edit::ArrayOfTables,
    selector: &ProxySelector,
) -> Result<&'a mut Table, CommandError> {
    ensure_selector(proxies, selector)?;
    proxies.get_mut(selector.index).ok_or_else(proxy_conflict)
}

fn ensure_selector(
    proxies: &toml_edit::ArrayOfTables,
    selector: &ProxySelector,
) -> Result<(), CommandError> {
    let matches = proxies
        .get(selector.index)
        .and_then(|table| table.get("name"))
        .and_then(Item::as_str)
        .is_some_and(|name| name == selector.original_name);
    if matches {
        Ok(())
    } else {
        Err(proxy_conflict())
    }
}

fn proxy_conflict() -> CommandError {
    CommandError::new(
        ErrorCode::ConfigConflict,
        "proxy changed since the configuration was loaded",
        true,
    )
    .with_suggested_action("reload the configuration before saving")
}

fn apply_proxy_patch(table: &mut Table, patch: &ProxyRulePatch) {
    apply_string(table, "name", &patch.name);
    apply_string(table, "type", &patch.proxy_type);
    let local_ip_key = if table.contains_key("localIP") {
        "localIP"
    } else if table.contains_key("localIp") {
        "localIp"
    } else {
        "localIP"
    };
    apply_string(table, local_ip_key, &patch.local_ip);
    apply_u16(table, "localPort", &patch.local_port);
    apply_u16(table, "remotePort", &patch.remote_port);
    apply_string_array(table, "customDomains", &patch.custom_domains);
}

fn ensure_table<'a>(root: &'a mut Table, key: &str) -> Result<&'a mut Table, CommandError> {
    if root.get(key).is_none() {
        root.insert(key, Item::Table(Table::new()));
    }
    root.get_mut(key)
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            CommandError::new(
                ErrorCode::ConfigInvalid,
                format!("{key} must be a table"),
                true,
            )
        })
}

fn apply_string(table: &mut Table, key: &str, patch: &Option<Option<String>>) {
    match patch {
        None => {}
        Some(None) => {
            table.remove(key);
        }
        Some(Some(value)) => {
            table.insert(key, toml_value(value.clone()));
        }
    }
}

fn apply_u16(table: &mut Table, key: &str, patch: &Option<Option<u16>>) {
    match patch {
        None => {}
        Some(None) => {
            table.remove(key);
        }
        Some(Some(value)) => {
            table.insert(key, toml_value(i64::from(*value)));
        }
    }
}

fn apply_string_array(table: &mut Table, key: &str, patch: &Option<Option<Vec<String>>>) {
    match patch {
        None => {}
        Some(None) => {
            table.remove(key);
        }
        Some(Some(values)) => {
            let mut array = Array::new();
            for item in values {
                array.push(item.as_str());
            }
            table.insert(key, Item::Value(Value::Array(array)));
        }
    }
}

fn frpc_changed_paths(patch: &FrpcConfigPatch) -> Vec<String> {
    let mut paths = Vec::new();
    push_if_present(&mut paths, "serverAddr", &patch.server_addr);
    push_if_present(&mut paths, "serverPort", &patch.server_port);
    if let Some(auth) = &patch.auth {
        push_if_present(&mut paths, "auth.method", &auth.method);
        push_if_present(&mut paths, "auth.token", &auth.token);
    }
    if let Some(web_server) = &patch.web_server {
        web_server_changed_paths(&mut paths, web_server);
    }
    for operation in &patch.proxy_operations {
        match operation {
            ProxyOperation::Add { rule } => {
                paths.push("proxies.$add".to_string());
                proxy_patch_paths(&mut paths, "proxies.$add", rule);
            }
            ProxyOperation::Update { selector, patch } => {
                proxy_patch_paths(&mut paths, &format!("proxies[{}]", selector.index), patch);
            }
            ProxyOperation::Delete { selector } => {
                paths.push(format!("proxies[{}].$delete", selector.index));
            }
        }
    }
    paths
}

fn frps_changed_paths(patch: &FrpsConfigPatch) -> Vec<String> {
    let mut paths = Vec::new();
    push_if_present(&mut paths, "bindPort", &patch.bind_port);
    push_if_present(&mut paths, "vhostHTTPPort", &patch.vhost_http_port);
    push_if_present(&mut paths, "vhostHTTPSPort", &patch.vhost_https_port);
    if let Some(auth) = &patch.auth {
        push_if_present(&mut paths, "auth.method", &auth.method);
        push_if_present(&mut paths, "auth.token", &auth.token);
    }
    if let Some(web_server) = &patch.web_server {
        web_server_changed_paths(&mut paths, web_server);
    }
    paths
}

fn web_server_changed_paths(paths: &mut Vec<String>, patch: &WebServerConfigPatch) {
    push_if_present(paths, "webServer.addr", &patch.addr);
    push_if_present(paths, "webServer.port", &patch.port);
    push_if_present(paths, "webServer.user", &patch.user);
    push_if_present(paths, "webServer.password", &patch.password);
}

fn proxy_patch_paths(paths: &mut Vec<String>, prefix: &str, patch: &ProxyRulePatch) {
    push_if_present(paths, &format!("{prefix}.name"), &patch.name);
    push_if_present(paths, &format!("{prefix}.type"), &patch.proxy_type);
    push_if_present(paths, &format!("{prefix}.localIP"), &patch.local_ip);
    push_if_present(paths, &format!("{prefix}.localPort"), &patch.local_port);
    push_if_present(paths, &format!("{prefix}.remotePort"), &patch.remote_port);
    push_if_present(
        paths,
        &format!("{prefix}.customDomains"),
        &patch.custom_domains,
    );
}

fn push_if_present<T>(paths: &mut Vec<String>, path: &str, value: &Option<Option<T>>) {
    if value.is_some() {
        paths.push(path.to_string());
    }
}

fn ensure_revision(current: &str, expected: &str) -> Result<(), CommandError> {
    if revision(current) == expected {
        Ok(())
    } else {
        Err(CommandError::new(
            ErrorCode::ConfigConflict,
            "configuration changed since it was loaded",
            true,
        )
        .with_suggested_action("reload the configuration before saving"))
    }
}

fn reject_errors(issues: &[ValidationIssue]) -> Result<(), CommandError> {
    if let Some(issue) = issues
        .iter()
        .find(|issue| issue.severity == ValidationSeverity::Error)
    {
        Err(invalid_config_error(issue))
    } else {
        Ok(())
    }
}

fn invalid_config_error(issue: &ValidationIssue) -> CommandError {
    let detail = issue
        .path
        .as_ref()
        .map(|path| format!("{} at {path}", issue.code))
        .unwrap_or_else(|| issue.code.clone());
    CommandError::new(
        ErrorCode::ConfigInvalid,
        "configuration validation failed",
        true,
    )
    .with_detail(detail)
    .with_suggested_action("correct the reported configuration issue")
}

fn snapshot_issues(snapshot: &ConfigSnapshot) -> &[ValidationIssue] {
    match snapshot {
        ConfigSnapshot::Frpc { issues, .. } | ConfigSnapshot::Frps { issues, .. } => issues,
    }
}
