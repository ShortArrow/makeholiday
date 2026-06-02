//! `structure/*` lint rules — physical-layer well-formedness.
//!
//! ADR-026 §"Structure" lists four rules. All four are implemented
//! here; none require source-span enrichment from ics-core (the
//! ADR's migration plan over-estimated). The rules walk `ctx.source`
//! directly so they see the physical line shape — CRLF vs LF,
//! pre-fold line widths, and the BEGIN/END nesting before unfolding
//! collapses anything.

use ics_core::parser::unfold::unfold;

use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{DiagnosticSink, LintContext, Rule};

/// `structure/unfolded-long-line` — a physical line exceeds 75 octets.
/// RFC 5545 §3.1 caps physical lines at 75 octets and requires longer
/// content to be folded; unfolded long lines break naive line-by-line
/// readers and some downstream pipelines.
pub struct UnfoldedLongLine;

impl Rule for UnfoldedLongLine {
    fn id(&self) -> &'static str {
        "structure/unfolded-long-line"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, raw) in ctx.source.split('\n').enumerate() {
            let line = raw.trim_end_matches('\r');
            // The final empty split after a trailing LF is not a real
            // physical line; skip it so a well-formed file doesn't
            // produce a phantom diagnostic.
            if line.is_empty() && idx + 1 == ctx.source.split('\n').count() {
                continue;
            }
            if line.len() > 75 {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "physical line is {} octets; RFC 5545 §3.1 caps lines at 75 octets and requires folding",
                            line.len()
                        ),
                    )
                    .with_line((idx + 1) as u32),
                );
            }
        }
    }
}

/// `structure/crlf-violation` — source uses bare `\n` instead of
/// `\r\n`. RFC 5545 §3.1 requires CRLF. Most consumers tolerate LF,
/// but the violation is silently lossy for any tool that round-trips
/// through strict serializers.
pub struct CrlfViolation;

impl Rule for CrlfViolation {
    fn id(&self) -> &'static str {
        "structure/crlf-violation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        let bytes = ctx.source.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b != b'\n' {
                continue;
            }
            let preceded_by_cr = i > 0 && bytes[i - 1] == b'\r';
            if preceded_by_cr {
                continue;
            }
            // Fire once at the first offending line and stop — re-
            // emitting on every LF would flood the output and tell the
            // author nothing new. Line number is computed by counting
            // preceding LFs.
            let line_no = bytes[..i].iter().filter(|&&c| c == b'\n').count() as u32 + 1;
            sink.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "source uses bare LF line endings; RFC 5545 §3.1 requires CRLF".to_string(),
                )
                .with_line(line_no),
            );
            return;
        }
    }
}

/// `structure/orphan-end` — `END:NAME` without a matching `BEGIN:NAME`.
/// Either a stray line or a mismatched nesting; both break component
/// reconstruction on the consumer side.
pub struct OrphanEnd;

impl Rule for OrphanEnd {
    fn id(&self) -> &'static str {
        "structure/orphan-end"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        let logical = unfold(ctx.source);
        let mut stack: Vec<String> = Vec::new();
        for (i, raw) in logical.iter().enumerate() {
            let line = raw.trim();
            if let Some(name) = line.strip_prefix("BEGIN:") {
                stack.push(name.to_string());
            } else if let Some(name) = line.strip_prefix("END:") {
                match stack.last() {
                    Some(top) if top == name => {
                        stack.pop();
                    }
                    _ => {
                        sink.push(
                            Diagnostic::new(
                                self.id(),
                                self.default_severity(),
                                format!(
                                    "END:{name} has no matching BEGIN at the same nesting level"
                                ),
                            )
                            .with_line((i + 1) as u32),
                        );
                    }
                }
            }
        }
    }
}

/// `structure/empty-calendar` — `VCALENDAR` contains zero components
/// (no `VEVENT`, no nested `VTIMEZONE`, etc.). Valid per RFC 5545 but
/// almost always an accidental empty file or stripped export.
pub struct EmptyCalendar;

impl Rule for EmptyCalendar {
    fn id(&self) -> &'static str {
        "structure/empty-calendar"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        let Some(cal) = ctx.calendar else {
            return;
        };
        if cal.events.is_empty() && cal.unrecognized_components.is_empty() {
            sink.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "VCALENDAR has no components (no VEVENT, no nested blocks); likely an accidental empty export".to_string(),
                )
                .with_line(1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Severity;
    use crate::lint;

    // ---------- structure/unfolded-long-line ----------

    #[test]
    fn short_lines_do_not_trigger_unfolded_long_line() {
        let src = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\nEND:VCALENDAR\r\n";
        let diags = lint(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.rule == "structure/unfolded-long-line"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn line_over_75_octets_triggers_unfolded_long_line() {
        // PRODID with a 90-octet body — well over the 75 cap.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n");
        let long_value = "x".repeat(90);
        src.push_str(&format!("PRODID:{long_value}\r\n"));
        src.push_str("END:VCALENDAR\r\n");
        let diags = lint(&src);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "structure/unfolded-long-line")
            .collect();
        assert!(!hits.is_empty(), "expected at least one hit; got {diags:?}");
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    #[test]
    fn folded_long_line_does_not_trigger_unfolded_long_line() {
        // The producer folded the long content properly; each physical
        // line stays under 75 octets.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:This is a long event title spread across many\r\n");
        src.push_str(" physical lines per RFC 5545 section 3.1 folding rules\r\n");
        src.push_str(" so no individual physical line exceeds the cap.\r\n");
        src.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&src);
        assert!(
            !diags
                .iter()
                .any(|d| d.rule == "structure/unfolded-long-line"),
            "got {:?}",
            diags
        );
    }

    // ---------- structure/crlf-violation ----------

    #[test]
    fn crlf_source_does_not_trigger_crlf_violation() {
        let src = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\nEND:VCALENDAR\r\n";
        let diags = lint(src);
        assert!(
            !diags.iter().any(|d| d.rule == "structure/crlf-violation"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn lf_only_source_triggers_crlf_violation_once() {
        let src = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//x//y\nEND:VCALENDAR\n";
        let diags = lint(src);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "structure/crlf-violation")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
        // First bare LF is at end of line 1.
        assert_eq!(hits[0].line, Some(1));
    }

    // ---------- structure/orphan-end ----------

    #[test]
    fn well_formed_calendar_has_no_orphan_end() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\nUID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&src);
        assert!(
            !diags.iter().any(|d| d.rule == "structure/orphan-end"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn stray_end_at_top_level_triggers_orphan_end() {
        // Extra END:VEVENT before any BEGIN — pure orphan.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("END:VEVENT\r\n");
        src.push_str("END:VCALENDAR\r\n");
        let diags = lint(&src);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "structure/orphan-end")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("END:VEVENT"));
    }

    #[test]
    fn mismatched_end_inside_block_triggers_orphan_end() {
        // BEGIN:VEVENT but END:VTODO inside it.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("END:VTODO\r\n");
        src.push_str("END:VEVENT\r\n");
        src.push_str("END:VCALENDAR\r\n");
        let diags = lint(&src);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "structure/orphan-end")
            .collect();
        assert!(!hits.is_empty(), "got {:?}", diags);
        assert!(hits.iter().any(|d| d.message.contains("END:VTODO")));
    }

    // ---------- structure/empty-calendar ----------

    #[test]
    fn empty_vcalendar_triggers_empty_calendar() {
        let src = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\nEND:VCALENDAR\r\n";
        let diags = lint(src);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "structure/empty-calendar")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Info);
    }

    #[test]
    fn vcalendar_with_event_does_not_trigger_empty_calendar() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\nUID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&src);
        assert!(
            !diags.iter().any(|d| d.rule == "structure/empty-calendar"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vcalendar_with_only_vtimezone_does_not_trigger_empty_calendar() {
        // VTIMEZONE is a component (lands in unrecognized_components),
        // so the calendar is not actually empty.
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VTIMEZONE\r\nTZID:Asia/Tokyo\r\nEND:VTIMEZONE\r\n");
        src.push_str("END:VCALENDAR\r\n");
        let diags = lint(&src);
        assert!(
            !diags.iter().any(|d| d.rule == "structure/empty-calendar"),
            "got {:?}",
            diags
        );
    }
}
