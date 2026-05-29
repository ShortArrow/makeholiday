//! `icslint` — Lint tool for iCalendar (RFC 5545) files.
//!
//! Library entry points: [`lint`] runs the registered rule set against a
//! source string and returns the [`Diagnostic`]s it produced.
//!
//! See ADR-026 (`docs/design/026-icslint-project-definition.md`) for the
//! project definition, rule families, severity tiers, and the relationship
//! to `ics-core` and `makeholiday`.

pub mod diagnostic;

pub use diagnostic::{Diagnostic, Severity, exit_code_for};

/// Run the lint pass over `source` and return all diagnostics produced.
///
/// The v0.2.0 rule set is fixed; future revisions may take a configuration
/// argument controlling which rules participate.
pub fn lint(source: &str) -> Vec<Diagnostic> {
    let _ = source;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_calendar_lint_is_currently_clean() {
        let diags = lint("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\nEND:VCALENDAR\r\n");
        assert!(
            diags.is_empty(),
            "scaffolding stage: no rules registered yet; got {:?}",
            diags
        );
    }
}
