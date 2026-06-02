//! Human-facing reporter — compiler-style one-line-per-diagnostic.
//!
//! Lifted from the inline `report_human` that used to live in
//! `main.rs`. Output shape:
//!
//! ```text
//! path/to/file.ics:42: error: [RFC5545/required-uid] VEVENT #1 has no UID...
//! path/to/file.ics: warning: [text/bom] input begins with UTF-8 BOM...
//! ```
//!
//! When `line` is `None` the `:N` segment is omitted. No color today;
//! ADR-026 mentions `--color <WHEN>` as a future option but pinning
//! the wire format to plain text keeps the smoke tests stable.

use std::io::{self, Write};
use std::path::PathBuf;

use crate::reporter::Reporter;
use crate::{Diagnostic, Severity};

pub struct HumanReporter;

impl Reporter for HumanReporter {
    fn write(&self, w: &mut dyn Write, diagnostics: &[(PathBuf, Diagnostic)]) -> io::Result<()> {
        for (path, diag) in diagnostics {
            let severity = severity_label(diag.severity);
            match diag.line {
                Some(n) => writeln!(
                    w,
                    "{}:{}: {}: [{}] {}",
                    path.display(),
                    n,
                    severity,
                    diag.rule,
                    diag.message
                )?,
                None => writeln!(
                    w,
                    "{}: {}: [{}] {}",
                    path.display(),
                    severity,
                    diag.rule,
                    diag.message
                )?,
            }
        }
        Ok(())
    }
}

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_writes_path_severity_rule_message() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("X/y", Severity::Error, "boom").with_line(42);
        let mut buf: Vec<u8> = Vec::new();
        HumanReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s, "a.ics:42: error: [X/y] boom\n");
    }

    #[test]
    fn human_omits_line_when_unknown() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("X/y", Severity::Warning, "fyi");
        let mut buf: Vec<u8> = Vec::new();
        HumanReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "a.ics: warning: [X/y] fyi\n"
        );
    }
}
