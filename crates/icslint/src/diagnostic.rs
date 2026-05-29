//! Diagnostic model — what every lint rule produces.
//!
//! A `Diagnostic` is a single finding from a single rule against a single
//! input source. Diagnostics are output rows, not errors: collecting many
//! into a `Vec` is the normal mode.
//!
//! ADR-026 §"Severity levels" governs the [`Severity`] variants and how
//! they map to process exit codes.

use serde::Serialize;

/// Severity of a single diagnostic.
///
/// Maps to process exit codes via [`exit_code_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable rule identifier (e.g. `"RFC5545/required-uid"`).
    pub rule: &'static str,
    pub severity: Severity,
    /// 1-based source line of the offending construct, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub message: String,
}

impl Diagnostic {
    pub fn new(rule: &'static str, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule,
            severity,
            line: None,
            message: message.into(),
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}

/// Compute the process exit code for a slice of diagnostics, given the
/// `warnings_as_errors` flag.
///
/// - `0` — no diagnostics, or only info-level.
/// - `1` — at least one warning emitted (and not promoted).
/// - `2` — at least one error emitted, or any warning when `warnings_as_errors`.
pub fn exit_code_for(diags: &[Diagnostic], warnings_as_errors: bool) -> i32 {
    let has_error = diags.iter().any(|d| d.severity == Severity::Error);
    let has_warning = diags.iter().any(|d| d.severity == Severity::Warning);
    if has_error || (warnings_as_errors && has_warning) {
        2
    } else if has_warning {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_clean_is_zero() {
        assert_eq!(exit_code_for(&[], false), 0);
    }

    #[test]
    fn exit_code_only_info_is_zero() {
        let diags = vec![Diagnostic::new("test/info", Severity::Info, "fyi")];
        assert_eq!(exit_code_for(&diags, false), 0);
    }

    #[test]
    fn exit_code_warning_is_one() {
        let diags = vec![Diagnostic::new("test/warn", Severity::Warning, "uh")];
        assert_eq!(exit_code_for(&diags, false), 1);
    }

    #[test]
    fn exit_code_error_is_two() {
        let diags = vec![Diagnostic::new("test/err", Severity::Error, "no")];
        assert_eq!(exit_code_for(&diags, false), 2);
    }

    #[test]
    fn exit_code_warning_promoted_with_w_flag() {
        let diags = vec![Diagnostic::new("test/warn", Severity::Warning, "uh")];
        assert_eq!(exit_code_for(&diags, true), 2);
    }

    #[test]
    fn exit_code_error_dominates_warning() {
        let diags = vec![
            Diagnostic::new("test/warn", Severity::Warning, "uh"),
            Diagnostic::new("test/err", Severity::Error, "no"),
        ];
        assert_eq!(exit_code_for(&diags, false), 2);
    }
}
