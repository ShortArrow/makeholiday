//! GitHub Actions reporter — workflow command annotations.
//!
//! Emits one annotation per diagnostic in GitHub Actions' workflow
//! command shape:
//!
//! ```text
//! ::error file=path/to/file.ics,line=42,title=RFC5545/required-uid::VEVENT #1 has no UID...
//! ::warning file=path/to/file.ics,line=12,title=text/bom::input begins with UTF-8 BOM...
//! ::notice file=path/to/file.ics,title=structure/empty-calendar::VCALENDAR has no components...
//! ```
//!
//! `Info` severity maps to `notice` because GitHub Actions has only
//! three command-level severities (`error` / `warning` / `notice`).
//! When `line` is `None` the `line=` parameter is omitted, but the
//! annotation still anchors to the file.
//!
//! Message bodies are encoded per GitHub's workflow command rules:
//! `%` → `%25`, `\r` → `%0D`, `\n` → `%0A`. Without this, a literal
//! newline in a diagnostic message would close the annotation early
//! and trail garbage into the next log line.

use std::io::{self, Write};
use std::path::PathBuf;

use crate::reporter::Reporter;
use crate::{Diagnostic, Severity};

pub struct GithubReporter;

impl Reporter for GithubReporter {
    fn write(&self, w: &mut dyn Write, diagnostics: &[(PathBuf, Diagnostic)]) -> io::Result<()> {
        for (path, diag) in diagnostics {
            let severity = severity_keyword(diag.severity);
            let file = path.display().to_string();
            let title = diag.rule;
            let message = encode_message(&diag.message);
            match diag.line {
                Some(n) => writeln!(
                    w,
                    "::{severity} file={file},line={n},title={title}::{message}"
                )?,
                None => writeln!(w, "::{severity} file={file},title={title}::{message}")?,
            }
        }
        Ok(())
    }
}

fn severity_keyword(s: Severity) -> &'static str {
    match s {
        Severity::Info => "notice",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// Apply the workflow-command escape rules. Order matters: `%` first,
/// then the CR / LF (which we want to encode as their percent forms,
/// not as percent-encoded CR / LF of the percent character).
fn encode_message(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_emits_command_with_line() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("RFC5545/required-uid", Severity::Error, "boom").with_line(42);
        let mut buf: Vec<u8> = Vec::new();
        GithubReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(
            s,
            "::error file=a.ics,line=42,title=RFC5545/required-uid::boom\n"
        );
    }

    #[test]
    fn github_omits_line_when_unknown() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new("text/bom", Severity::Info, "fyi");
        let mut buf: Vec<u8> = Vec::new();
        GithubReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let s = String::from_utf8(buf).unwrap();
        // Info maps to GitHub's `notice` command.
        assert_eq!(s, "::notice file=a.ics,title=text/bom::fyi\n");
    }

    #[test]
    fn github_maps_severities_to_workflow_keywords() {
        assert_eq!(severity_keyword(Severity::Info), "notice");
        assert_eq!(severity_keyword(Severity::Warning), "warning");
        assert_eq!(severity_keyword(Severity::Error), "error");
    }

    #[test]
    fn github_encodes_percent_and_newlines_in_message() {
        let path = PathBuf::from("a.ics");
        let diag = Diagnostic::new(
            "X/y",
            Severity::Warning,
            "first line\nsecond line with 50% off",
        );
        let mut buf: Vec<u8> = Vec::new();
        GithubReporter
            .write(&mut buf, &[(path, diag)])
            .expect("write");
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("first line%0Asecond line with 50%25 off"),
            "expected escaped body in {s:?}"
        );
        // The single trailing newline from writeln! is still a real
        // newline so the next log line starts cleanly.
        assert!(s.ends_with('\n'));
    }
}
