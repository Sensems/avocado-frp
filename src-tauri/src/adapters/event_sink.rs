use tauri::Emitter;

use crate::domain::error::{CommandError, ErrorCode};
use crate::domain::events::{ConfigChangedEvent, LogEntry, ProcessStateChangedEvent};
use crate::domain::process::ProcessSnapshot;

pub const PROCESS_STATE_CHANGED_EVENT: &str = "process://state-changed";
pub const CONFIG_CHANGED_EVENT: &str = "config://changed";
pub const LOG_ENTRY_EVENT: &str = "log://entry";

pub trait EventSink: Send + Sync {
    fn process_changed(&self, snapshot: &ProcessSnapshot) -> Result<(), CommandError>;
    fn config_changed(&self, event: &ConfigChangedEvent) -> Result<(), CommandError>;
    fn log_entry(&self, entry: &LogEntry) -> Result<(), CommandError>;
}

pub struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn process_changed(&self, snapshot: &ProcessSnapshot) -> Result<(), CommandError> {
        self.app
            .emit(
                PROCESS_STATE_CHANGED_EVENT,
                ProcessStateChangedEvent::from(snapshot),
            )
            .map_err(map_event_error)
    }

    fn config_changed(&self, event: &ConfigChangedEvent) -> Result<(), CommandError> {
        self.app
            .emit(CONFIG_CHANGED_EVENT, event)
            .map_err(map_event_error)
    }

    fn log_entry(&self, entry: &LogEntry) -> Result<(), CommandError> {
        self.app
            .emit(LOG_ENTRY_EVENT, entry)
            .map_err(map_event_error)
    }
}

pub struct CompositeEventSink {
    sinks: Vec<Box<dyn EventSink>>,
}

impl CompositeEventSink {
    pub fn new(sinks: Vec<Box<dyn EventSink>>) -> Self {
        Self { sinks }
    }

    fn dispatch(
        &self,
        mut operation: impl FnMut(&dyn EventSink) -> Result<(), CommandError>,
    ) -> Result<(), CommandError> {
        let mut first_error = None;
        for sink in &self.sinks {
            if let Err(error) = operation(sink.as_ref()) {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl EventSink for CompositeEventSink {
    fn process_changed(&self, snapshot: &ProcessSnapshot) -> Result<(), CommandError> {
        self.dispatch(|sink| sink.process_changed(snapshot))
    }

    fn config_changed(&self, event: &ConfigChangedEvent) -> Result<(), CommandError> {
        self.dispatch(|sink| sink.config_changed(event))
    }

    fn log_entry(&self, entry: &LogEntry) -> Result<(), CommandError> {
        let mut sanitized = entry.clone();
        sanitized.text = redact_secrets(&sanitized.text);
        self.dispatch(|sink| sink.log_entry(&sanitized))
    }
}

const SENSITIVE_KEY_MARKERS: &[&str] = &["token", "password", "secret"];

pub fn redact_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            let sensitive = SENSITIVE_KEY_MARKERS.iter().any(|key| lower.contains(key));
            if sensitive {
                if let Some(separator) = line.find('=') {
                    return format!("{}= ***", line[..separator].trim_end());
                }
                if let Some(separator) = line.find(':') {
                    return format!("{}: ***", line[..separator].trim_end());
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Redact TOML for diagnostics export by sensitive **key names** (token/password/secret).
/// Safer than whole-line substring matching so values like hostnames are preserved.
pub fn redact_toml_for_export(raw: &str) -> String {
    let mut out = Vec::new();
    let mut in_redacted_multiline: Option<MultilineQuote> = None;

    for line in raw.lines() {
        if let Some(kind) = in_redacted_multiline {
            if line_closes_multiline(line, kind) {
                in_redacted_multiline = None;
            }
            continue;
        }

        let (redacted, opens_multiline) = redact_toml_line(line);
        if let Some(kind) = opens_multiline {
            in_redacted_multiline = Some(kind);
        }
        out.push(redacted);
    }

    out.join("\n")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MultilineQuote {
    Basic,
    Literal,
}

fn redact_toml_line(line: &str) -> (String, Option<MultilineQuote>) {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
        return (line.to_string(), None);
    }

    if let Some(eq_pos) = find_unquoted_equals(line) {
        let key = line[..eq_pos].trim();
        if key_is_sensitive(key) {
            let indent_len = line.len() - trimmed.len();
            let indent = &line[..indent_len];
            let value = line[eq_pos + 1..].trim_start();
            let opens_multiline = value_opens_unclosed_multiline(value);
            return (format!("{indent}{key} = ***"), opens_multiline);
        }
    }

    (redact_inline_sensitive_assignments(line), None)
}

fn value_opens_unclosed_multiline(value: &str) -> Option<MultilineQuote> {
    if let Some(rest) = value.strip_prefix("\"\"\"") {
        if find_multiline_basic_end(rest).is_none() {
            return Some(MultilineQuote::Basic);
        }
    } else if let Some(rest) = value.strip_prefix("'''") {
        if find_multiline_literal_end(rest).is_none() {
            return Some(MultilineQuote::Literal);
        }
    }
    None
}

fn line_closes_multiline(line: &str, kind: MultilineQuote) -> bool {
    match kind {
        MultilineQuote::Basic => find_multiline_basic_end(line).is_some(),
        MultilineQuote::Literal => find_multiline_literal_end(line).is_some(),
    }
}

fn find_multiline_basic_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == b'"' && bytes[i + 1] == b'"' && bytes[i + 2] == b'"' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_multiline_literal_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\'' {
            i += 1;
            continue;
        }
        let start = i;
        let mut quotes = 0;
        while i < bytes.len() && bytes[i] == b'\'' {
            quotes += 1;
            i += 1;
        }
        if quotes >= 3 {
            return Some(start);
        }
    }
    None
}

fn key_is_sensitive(key: &str) -> bool {
    let key = key.trim().trim_matches('"').trim_matches('\'');
    let lower = key.to_ascii_lowercase();
    let leaf = lower.rsplit('.').next().unwrap_or(lower.as_str());
    SENSITIVE_KEY_MARKERS
        .iter()
        .any(|marker| leaf == *marker || leaf.ends_with(marker) || leaf.contains(marker))
}

fn find_unquoted_equals(line: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '=' if !in_single && !in_double => return Some(index),
            _ => {}
        }
    }
    None
}

/// Redact `password = "..."`, `token = ...`, etc. inside inline tables / remainder of line.
fn redact_inline_sensitive_assignments(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;
    let mut search = 0usize;
    while search < bytes.len() {
        if let Some((key_start, key_end, value_start)) = match_sensitive_assignment(bytes, search) {
            out.push_str(&line[cursor..key_start]);
            out.push_str(line[key_start..key_end].trim_end());
            out.push_str(" = ***");
            let value_end = skip_toml_value(bytes, value_start);
            cursor = value_end;
            search = value_end;
        } else {
            search += 1;
        }
    }
    out.push_str(&line[cursor..]);
    out
}

fn match_sensitive_assignment(bytes: &[u8], start: usize) -> Option<(usize, usize, usize)> {
    if start > 0 {
        let prev = bytes[start - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'-' || prev == b'.' {
            return None;
        }
    }
    let mut i = start;
    let key_start = i;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' {
            i += 1;
        } else {
            break;
        }
    }
    if i == key_start {
        return None;
    }
    let key_end = i;
    let key = std::str::from_utf8(&bytes[key_start..key_end]).ok()?;
    if !key_is_sensitive(key) {
        return None;
    }
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'=' {
        return None;
    }
    i += 1;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    Some((key_start, key_end, i))
}

fn skip_toml_value(bytes: &[u8], mut i: usize) -> usize {
    if i >= bytes.len() {
        return i;
    }
    match bytes[i] {
        b'"' => {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    return i + 1;
                }
                i += 1;
            }
            i
        }
        b'\'' => {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                i += 1;
            }
            if i < bytes.len() {
                i + 1
            } else {
                i
            }
        }
        _ => {
            while i < bytes.len() {
                let b = bytes[i];
                if b == b',' || b == b'}' || b == b'#' {
                    break;
                }
                i += 1;
            }
            i
        }
    }
}

fn map_event_error(_error: tauri::Error) -> CommandError {
    CommandError::new(
        ErrorCode::Unknown,
        "application event delivery failed",
        true,
    )
}
