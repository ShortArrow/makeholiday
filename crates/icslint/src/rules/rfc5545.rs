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
        let Some(cal) = ctx.calendar else {
            return;
        };
        for (idx, event) in cal.events.iter().enumerate() {
            if event.uid.trim().is_empty() {
                sink.push(Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!(
                        "VEVENT #{} has no UID (RFC 5545 §3.6.1 requires UID on every VEVENT)",
                        idx + 1
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lint;
    use crate::Severity;

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
