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

/// `RFC5545/required-dtstart` — RFC 5545 §3.6.1: `DTSTART` is REQUIRED on
/// every `VEVENT` when the parent `VCALENDAR` does not carry a `METHOD`
/// property (i.e. the calendar represents events directly rather than
/// scheduling messages). Without `DTSTART` consumers cannot place the
/// event on a timeline.
pub struct RequiredDtstart;

impl Rule for RequiredDtstart {
    fn id(&self) -> &'static str {
        "RFC5545/required-dtstart"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            if !scan.has("DTSTART") {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has no DTSTART (RFC 5545 §3.6.1 requires DTSTART when VCALENDAR has no METHOD)",
                            idx + 1
                        ),
                    )
                    .with_line(scan.begin_line),
                );
            }
        }
    }
}

/// `RFC5545/duplicate-dtstart` — RFC 5545 §3.6.1: `DTSTART` is allowed at
/// most once per `VEVENT`. A second `DTSTART` either silently overwrites
/// the first or breaks consumers that expect a single anchor.
pub struct DuplicateDtstart;

impl Rule for DuplicateDtstart {
    fn id(&self) -> &'static str {
        "RFC5545/duplicate-dtstart"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            let occurrences = scan.lines_of("DTSTART");
            if occurrences.len() > 1 {
                let lines: Vec<String> = occurrences.iter().map(|l| l.to_string()).collect();
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has {} DTSTART occurrences (lines {}); RFC 5545 §3.6.1 allows at most one",
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

/// `RFC5545/conflicting-end-and-duration` — RFC 5545 §3.6.1: `DTEND` and
/// `DURATION` are mutually exclusive within a `VEVENT`. When both are
/// present, the effective end is ambiguous and consumers disagree on
/// precedence.
pub struct ConflictingEndAndDuration;

impl Rule for ConflictingEndAndDuration {
    fn id(&self) -> &'static str {
        "RFC5545/conflicting-end-and-duration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            if scan.has("DTEND") && scan.has("DURATION") {
                // Point at the second-property's line so the diagnostic
                // sits next to whichever property the author probably
                // added last. Falls back to BEGIN:VEVENT if either lookup
                // somehow returns no lines.
                let line = scan
                    .lines_of("DURATION")
                    .first()
                    .copied()
                    .or_else(|| scan.lines_of("DTEND").first().copied())
                    .unwrap_or(scan.begin_line);
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has both DTEND and DURATION; RFC 5545 §3.6.1 allows only one",
                            idx + 1
                        ),
                    )
                    .with_line(line),
                );
            }
        }
    }
}

/// `RFC5545/end-before-start` — RFC 5545 §3.8.2.2: when present, `DTEND`
/// MUST be later than `DTSTART`. For DATE-valued events `DTEND` is the
/// non-inclusive end, so a single-day event has `DTEND = DTSTART + 1`;
/// `DTEND <= DTSTART` produces a zero- or negative-duration event that
/// most clients silently drop or render incorrectly.
pub struct EndBeforeStart;

impl Rule for EndBeforeStart {
    fn id(&self) -> &'static str {
        "RFC5545/end-before-start"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        // Needs typed comparison of DTSTART vs DTEND. If the typed parser
        // could not promote the calendar, every event with missing or
        // malformed date already surfaces as parse-error / required-*
        // diagnostics from sibling rules; this rule then no-ops.
        let Some(cal) = ctx.calendar else {
            return;
        };
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            let Some(event) = cal.events.get(idx) else {
                continue;
            };
            if event.dtend <= event.dtstart {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has DTEND ({}) not later than DTSTART ({}); RFC 5545 §3.8.2.2 requires DTEND > DTSTART",
                            idx + 1,
                            event.dtend,
                            event.dtstart
                        ),
                    )
                    .with_line(
                        scan.lines_of("DTEND")
                            .first()
                            .copied()
                            .unwrap_or(scan.begin_line),
                    ),
                );
            }
        }
    }
}

/// `RFC5545/empty-summary` — `SUMMARY` is optional, but when it is
/// present its value should be non-empty. An empty `SUMMARY:` line is
/// almost always a merge artifact or a forgotten template placeholder;
/// most calendar UIs render such events with a blank title.
pub struct EmptySummary;

impl Rule for EmptySummary {
    fn id(&self) -> &'static str {
        "RFC5545/empty-summary"
    }

    fn default_severity(&self) -> Severity {
        // Warning rather than Error: SUMMARY is not RFC-required, so an
        // empty value is a quality issue rather than a spec violation.
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        let Some(cal) = ctx.calendar else {
            return;
        };
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            if !scan.has("SUMMARY") {
                continue;
            }
            let Some(event) = cal.events.get(idx) else {
                continue;
            };
            if event.summary.trim().is_empty() {
                sink.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "VEVENT #{} has an empty SUMMARY value; either drop the property or fill it in",
                            idx + 1
                        ),
                    )
                    .with_line(
                        scan.lines_of("SUMMARY")
                            .first()
                            .copied()
                            .unwrap_or(scan.begin_line),
                    ),
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

    // ---------- RFC5545/required-dtstart ----------

    #[test]
    fn vevent_with_dtstart_passes_required_dtstart() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/required-dtstart"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_without_dtstart_triggers_required_dtstart() {
        // ics-core's typed parser would error on missing DTSTART; icslint
        // catches it via the raw walker so the failure surfaces as a
        // RFC5545/required-dtstart diagnostic alongside the parse-error.
        let props = "\
UID:e1\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:s\r\n";
        let diags = lint(&vevent_with(props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/required-dtstart")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("VEVENT #1"));
    }

    // ---------- RFC5545/duplicate-dtstart ----------

    #[test]
    fn vevent_with_single_dtstart_passes_duplicate_dtstart() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/duplicate-dtstart"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_with_duplicate_dtstart_triggers_duplicate_dtstart() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260501\r\n");
        props.push_str("DTEND;VALUE=DATE:20260430\r\n");
        props.push_str("SUMMARY:s\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/duplicate-dtstart")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(
            hits[0].message.contains("2 DTSTART occurrences"),
            "expected count phrase; got {:?}",
            hits[0].message
        );
    }

    // ---------- RFC5545/conflicting-end-and-duration ----------

    #[test]
    fn vevent_with_only_dtend_passes_conflicting_rule() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags
                .iter()
                .any(|d| d.rule == "RFC5545/conflicting-end-and-duration"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_with_both_dtend_and_duration_triggers_conflicting_rule() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTEND;VALUE=DATE:20260430\r\n");
        props.push_str("DURATION:P1D\r\n");
        props.push_str("SUMMARY:s\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/conflicting-end-and-duration")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("VEVENT #1"));
        assert!(hits[0].message.contains("DTEND and DURATION"));
    }

    // ---------- RFC5545/end-before-start ----------

    #[test]
    fn vevent_with_dtend_after_dtstart_passes_end_before_start() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/end-before-start"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_with_dtend_equal_to_dtstart_triggers_end_before_start() {
        // DATE-valued events have DTEND as the non-inclusive end; DTEND ==
        // DTSTART means a zero-duration event, which most clients drop.
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTEND;VALUE=DATE:20260429\r\n");
        props.push_str("SUMMARY:s\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/end-before-start")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
        assert!(hits[0].message.contains("VEVENT #1"));
    }

    #[test]
    fn vevent_with_dtend_before_dtstart_triggers_end_before_start() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260501\r\n");
        props.push_str("DTEND;VALUE=DATE:20260429\r\n");
        props.push_str("SUMMARY:s\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/end-before-start")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Error);
    }

    // ---------- RFC5545/empty-summary ----------

    #[test]
    fn vevent_with_nonempty_summary_passes_empty_summary() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str(REQUIRED_FIELDS_AFTER_UID);
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/empty-summary"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_without_summary_does_not_trigger_empty_summary() {
        // SUMMARY is optional; absence is not a violation.
        let props = "\
UID:e1\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n";
        let diags = lint(&vevent_with(props));
        assert!(
            !diags.iter().any(|d| d.rule == "RFC5545/empty-summary"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn vevent_with_empty_summary_triggers_empty_summary() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTEND;VALUE=DATE:20260430\r\n");
        props.push_str("SUMMARY:\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/empty-summary")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
        assert!(hits[0].message.contains("VEVENT #1"));
    }

    #[test]
    fn vevent_with_whitespace_only_summary_triggers_empty_summary() {
        let mut props = String::from("UID:e1\r\n");
        props.push_str("DTSTAMP:20260101T000000Z\r\n");
        props.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        props.push_str("DTEND;VALUE=DATE:20260430\r\n");
        props.push_str("SUMMARY:   \r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "RFC5545/empty-summary")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
    }
}
