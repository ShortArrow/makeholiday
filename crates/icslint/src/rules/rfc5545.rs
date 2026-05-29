//! `RFC5545/*` lint rules — RFC 5545 cardinality and required fields.
//!
//! Each rule's docstring links the RFC 5545 section it enforces.

use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{DiagnosticSink, LintContext, Rule};

/// `RFC5545/required-uid` — RFC 5545 §3.6.1: `UID` is REQUIRED on every
/// `VEVENT`. Calendar items without a `UID` cannot be reliably
/// deduplicated or referenced between clients.
pub struct RequiredUid;

impl Rule for RequiredUid {
    fn id(&self) -> &'static str {
        "RFC5545/required-uid"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            if !scan.has("UID") {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has no UID (RFC 5545 §3.6.1 requires UID on every VEVENT)",
                            idx + 1
                        ),
                    )
                    .with_line(scan.begin_line),
                );
                continue;
            }
            // Present but blank — the typed parser tolerates this, so
            // catch it via the typed view if available.
            if let Some(cal) = ctx.calendar {
                if let Some(event) = cal.events.get(idx) {
                    if event.uid.trim().is_empty() {
                        sink.push(
                            Diagnostic::new(
                                self.id(),
                                self.default_severity(),
                                format!("VEVENT #{} has an empty UID (RFC 5545 §3.6.1)", idx + 1),
                            )
                            .with_line(scan.begin_line),
                        );
                    }
                }
            }
        }
    }
}

/// `RFC5545/required-dtstamp` — RFC 5545 §3.6.1: `DTSTAMP` is REQUIRED on
/// every `VEVENT`. Without it, clients cannot order revisions of the
/// same event reliably.
pub struct RequiredDtstamp;

impl Rule for RequiredDtstamp {
    fn id(&self) -> &'static str {
        "RFC5545/required-dtstamp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            if !scan.has("DTSTAMP") {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has no DTSTAMP (RFC 5545 §3.6.1 requires DTSTAMP on every VEVENT)",
                            idx + 1
                        ),
                    )
                    .with_line(scan.begin_line),
                );
            }
        }
    }
}

/// `RFC5545/duplicate-summary` — RFC 5545 §3.6.1: `SUMMARY` is allowed at
/// most once per `VEVENT`. Multiple occurrences usually indicate a merge
/// artifact and silently lose information when consumers pick "last wins".
pub struct DuplicateSummary;

impl Rule for DuplicateSummary {
    fn id(&self) -> &'static str {
        "RFC5545/duplicate-summary"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            let occurrences = scan.lines_of("SUMMARY");
            if occurrences.len() > 1 {
                let lines: Vec<String> = occurrences.iter().map(|l| l.to_string()).collect();
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has {} SUMMARY occurrences (lines {}); RFC 5545 §3.6.1 allows at most one",
                            idx + 1,
                            occurrences.len(),
                            lines.join(", ")
                        ),
                    )
                    .with_line(occurrences[1]),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Severity;
    use crate::lint;

    fn vevent_with(properties: &str) -> String {
        let mut s = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        s.push_str("BEGIN:VEVENT\r\n");
        s.push_str(properties);
        s.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        s
    }

    const REQUIRED_FIELDS_AFTER_UID: &str = "\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:s\r\n";

    #[test]
    fn vevent_with_uid_passes_required_uid() {
        let mut props = String::from("UID:event-1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/required-uid"),
            "no required-uid diagnostic expected; got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_without_uid_triggers_required_uid() {
        // UID line omitted; ics-core's typed parser tolerates this (uid
        // becomes empty string) so the rule fires.
        let diags = lint(&vevent_with(REQUIRED_FIELDS_AFTER_UID));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/required-uid")
            .collect();
        assert_eq!(hits.len(), 1, "exactly one diagnostic; got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("VEVENT #1"));
    }

    #[test]
    fn vevent_with_whitespace_only_uid_triggers_required_uid() {
        let mut props = String::from("UID:   \r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/required-uid")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn vevent_without_dtstamp_triggers_required_dtstamp() {
        // ics-core's typed parser would error on missing DTSTAMP; icslint
        // catches it via the raw walker so the fail-fast surfaces as a
        // RFC5545/required-dtstamp diagnostic instead of just parse-error.
        let props = "\
UID:e1\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:s\r\n";
        let diags = lint(&vevent_with(props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/required-dtstamp")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("VEVENT #1"));
    }

    #[test]
    fn vevent_with_dtstamp_passes_required_dtstamp() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/required-dtstamp"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_with_duplicate_summary_triggers_duplicate_summary() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTEND;VALUE=DATE:20260430\r\n");
        props.push_str("SUMMARY:original\r\n");
        props.push_str("SUMMARY:overwritten\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/duplicate-summary")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(
            hits[0].message.contains("2 SUMMARY occurrences"),
            "expected count phrase; got {:?}",
            hits[0].message
        );
    }

    #[test]
    fn vevent_with_single_summary_passes_duplicate_summary() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/duplicate-summary"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn duplicate_summary_diagnostic_points_at_second_occurrence_line() {
        // Lines 1..3 = VCALENDAR header, 4 = BEGIN:VEVENT,
        // 5 = UID, 6 = DTSTAMP, 7 = DTSTART, 8 = DTEND,
        // 9 = first SUMMARY, 10 = second SUMMARY.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\n");
        input.push_str("DTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        input.push_str("DTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:original\r\n");
        input.push_str("SUMMARY:overwritten\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&input);
        let hit = diags
            .iter()
            .find(|d| d.rule == "RFC5545/duplicate-summary")
            .unwrap();
        assert_eq!(hit.line, Some(10));
    }

    #[test]
    fn second_vevent_without_uid_is_reported_with_its_index() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        // Event 1 — fine.
        input.push_str("BEGIN:VEVENT\r\nUID:ok-1\r\n");
        input.push_str(REQUIRED_FIELDS_AFTER_UID);
        input.push_str("END:VEVENT\r\n");
        // Event 2 — missing UID.
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str(REQUIRED_FIELDS_AFTER_UID);
        input.push_str("END:VEVENT\r\n");
        input.push_str("END:VCALENDAR\r\n");
        let diags = lint(&input);
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/required-uid")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0].message.contains("VEVENT #2"),
            "second-event index expected; got {:?}",
            hits[0].message
        );
    }
}
