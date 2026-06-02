//! JSON reporter — stable schema per ADR-026 §"Output formats".
//!
//! Emits a single JSON array on stdout. Each element has the shape:
//!
//! ```json
//! {
//!   "file": "path/to/file.ics",
//!   "line": 42,
//!   "rule": "RFC5545/required-uid",
//!   "severity": "error",
//!   "message": "VEVENT #1 has no UID..."
//! }
//! ```
//!
//! `line` is omitted when the rule could not localize the finding to
//! a specific line. Severity values are `"info"` / `"warning"` /
//! `"error"` (lowercased). When there are zero diagnostics the output
//! is `[]\n` so the consumer can always parse the stream as JSON.

use std::io::{self, Write};
use std::path::PathBuf;

use serde::Serialize;

use crate::reporter::Reporter;
use crate::{Diagnostic, Severity};

#[derive(Serialize)]
struct JsonRow<'a> {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    rule: &'a str,
    severity: Severity,
    message: &'a str,
}

pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn write(&self, w: &mut dyn Write, diagnostics: &[(PathBuf, Diagnostic)]) -> io::Result<()> {
        let rows: Vec<JsonRow<'_>> = diagnostics
            .iter()
            .map(|(path, diag)| JsonRow {
                file: path.display().to_string(),
                line: diag.line,
                rule: diag.rule,
                severity: diag.severity,
                message: &diag.message,
            })
            .collect();
        serde_json::to_writer(&mut *w, &rows).map_err(io::Error::other)?;
        writeln!(w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn json_emits_array_with_required_keys() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("X/y", Severity::Error, "boom").with_line(42);
        let mut buf: Vec<u8> = Vec::new();
        JsonReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let parsed: Value = serde_json::from_slice(&buf).expect("valid json");
        let arr = parsed.as_array().expect("top-level array");
        assert_eq!(arr.len(), 1);
        let row = &arr[0];
        assert_eq!(row["file"], "a.ics");
        assert_eq!(row["line"], 42);
        assert_eq!(row["rule"], "X/y");
        assert_eq!(row["severity"], "error");
        assert_eq!(row["message"], "boom");
    }

    #[test]
    fn json_omits_line_when_unknown() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("X/y", Severity::Warning, "fyi");
        let mut buf: Vec<u8> = Vec::new();
        JsonReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let parsed: Value = serde_json::from_slice(&buf).expect("valid json");
        assert!(parsed[0].get("line").is_none());
    }

    #[test]
    fn json_empty_input_yields_empty_array() {
        let mut buf: Vec<u8> = Vec::new();
        JsonReporter.write(&mut buf, &[]).expect("write");
        let parsed: Value = serde_json::from_slice(&buf).expect("valid json");
        assert_eq!(parsed.as_array().map(|a| a.len()), Some(0));
    }

    #[test]
    fn json_serializes_severity_as_lowercase() {
        let path = PathBuf::from("a.ics");
        let diags = vec![
            (path.clone(), Diagnostic::new("a/info", Severity::Info, "i")),
            (
                path.clone(),
                Diagnostic::new("a/warn", Severity::Warning, "w"),
            ),
            (path, Diagnostic::new("a/err", Severity::Error, "e")),
        ];
        let mut buf: Vec<u8> = Vec::new();
        JsonReporter.write(&mut buf, &diags).expect("write");
        let parsed: Value = serde_json::from_slice(&buf).expect("valid json");
        assert_eq!(parsed[0]["severity"], "info");
        assert_eq!(parsed[1]["severity"], "warning");
        assert_eq!(parsed[2]["severity"], "error");
    }
}
