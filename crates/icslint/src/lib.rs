//! `icslint` — Lint tool for iCalendar (RFC 5545) files.
//!
//! Library entry points: [`lint`] runs the registered rule set against a
//! source string and returns the [`Diagnostic`]s it produced.
//!
//! See ADR-026 (`docs/design/026-icslint-project-definition.md`) for the
//! project definition, rule families, severity tiers, and the relationship
//! to `ics-core` and `makeholiday`.

pub mod diagnostic;
pub mod reporter;
pub mod rules;
pub mod walker;

pub use diagnostic::{Diagnostic, Severity, exit_code_for};

use rules::{DiagnosticSink, LintContext};
use walker::walk_vevents;

/// Run the lint pass over `source` and return all diagnostics produced.
///
/// The v0.2.0 rule set is fixed; future revisions may take a configuration
/// argument controlling which rules participate.
pub fn lint(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Try the tolerant typed parse. When ics-core's fail-fast parser
    // cannot promote the source to a typed VCalendar, we record a single
    // synthetic "parse error" diagnostic and run rules with calendar=None
    // so source-text-only rules still execute.
    let calendar = match ics_core::parse_calendar(source) {
        Ok(c) => Some(c),
        Err(e) => {
            diagnostics.push(Diagnostic::new(
                "RFC5545/parse-error",
                Severity::Error,
                e.to_string(),
            ));
            None
        }
    };

    let vevent_scans = walk_vevents(source);
    let ctx = LintContext {
        source,
        calendar: calendar.as_ref(),
        vevent_scans: &vevent_scans,
    };
    let mut sink = DiagnosticSink::from_vec(diagnostics);
    for rule in rules::all() {
        rule.visit(&ctx, &mut sink);
    }
    sink.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_calendar_with_one_event_has_no_diagnostics() {
        // A single clean event must not trigger any rule. Empty-VCALENDAR
        // fixtures used to live here, but `structure/empty-calendar`
        // (ADR-026) now fires on them by design, so the "clean = no
        // diagnostics" smoke test needs a real event.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&src);
        assert!(
            diags.is_empty(),
            "well-formed calendar should be clean; got {:?}",
            diags
        );
    }

    #[test]
    fn unparseable_input_yields_parse_error_diagnostic() {
        // No BEGIN:VCALENDAR at all — fail-fast parser cannot recover.
        let diags = lint("this is not an ics file at all");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "RFC5545/parse-error");
        assert_eq!(diags[0].severity, Severity::Error);
    }
}
