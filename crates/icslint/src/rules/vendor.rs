//! `vendor/*` lint rules — vendor-extension hygiene.
//!
//! ADR-026 §"Vendor extension hygiene" defines five rules; four are
//! implemented here. `vendor/typed-clash` is deferred until additional
//! vendor profiles grow typed cross-vendor slots — `microsoft.busystatus`
//! is the only typed bundle field today, so no clash is structurally
//! representable in v0.2.0.

use ics_core::{google, icloud, microsoft};

use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{DiagnosticSink, LintContext, Rule};

/// `vendor/microsoft-only` — property uses the `X-MICROSOFT-*` prefix.
/// Recognized by Outlook / Exchange / Microsoft 365 but inconsistently
/// honored by other clients. Info-level: makes the portability
/// trade-off visible without discouraging the extension.
pub struct MicrosoftOnly;

impl Rule for MicrosoftOnly {
    fn id(&self) -> &'static str {
        "vendor/microsoft-only"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if microsoft::owns_property(&prop.name) {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} uses {} (X-MICROSOFT-* extensions are honored by Outlook / Exchange but support outside that family is uneven)",
                                idx + 1,
                                prop.name
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// `vendor/google-only` — property uses an `X-GOOGLE-*` /
/// `X-GOOGLE-CALENDAR-*` prefix. Google Calendar honors these but they
/// rarely round-trip through other clients.
pub struct GoogleOnly;

impl Rule for GoogleOnly {
    fn id(&self) -> &'static str {
        "vendor/google-only"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if google::owns_property(&prop.name) {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} uses {} (X-GOOGLE-* extensions are Google Calendar-specific)",
                                idx + 1,
                                prop.name
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// `vendor/icloud-only` — property uses an `X-APPLE-*` /
/// `X-CALENDARSERVER-*` prefix. iCloud and CalendarServer honor these
/// but other clients usually ignore them.
pub struct IcloudOnly;

impl Rule for IcloudOnly {
    fn id(&self) -> &'static str {
        "vendor/icloud-only"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if icloud::owns_property(&prop.name) {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} uses {} (X-APPLE-* / X-CALENDARSERVER-* extensions are iCloud / CalendarServer-specific)",
                                idx + 1,
                                prop.name
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// `vendor/unrecognized-x` — `X-*` property does not match any known
/// vendor profile (microsoft / google / icloud). Either a private
/// extension or a typo of a known vendor prefix. Warning rather than
/// info: private extensions are fine, but they should be deliberate.
pub struct UnrecognizedX;

impl Rule for UnrecognizedX {
    fn id(&self) -> &'static str {
        "vendor/unrecognized-x"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if prop.name.starts_with("X-")
                    && !microsoft::owns_property(&prop.name)
                    && !google::owns_property(&prop.name)
                    && !icloud::owns_property(&prop.name)
                {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} uses {} which matches no known vendor prefix; either deliberate private extension or typo",
                                idx + 1,
                                prop.name
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
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

    const REQUIRED_FIELDS: &str = "\
UID:e1\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:s\r\n";

    // ---------- vendor/microsoft-only ----------

    #[test]
    fn clean_vevent_does_not_trigger_microsoft_only() {
        let diags = lint(&vevent_with(REQUIRED_FIELDS));
        assert!(
            !diags.iter().any(|d| d.rule == "vendor/microsoft-only"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn x_microsoft_property_triggers_microsoft_only() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/microsoft-only")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Info);
        assert!(hits[0].message.contains("X-MICROSOFT-CDO-BUSYSTATUS"));
    }

    #[test]
    fn multiple_x_microsoft_properties_yield_multiple_diagnostics() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        props.push_str("X-MICROSOFT-CDO-IMPORTANCE:1\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/microsoft-only")
            .collect();
        assert_eq!(hits.len(), 2, "got {:?}", diags);
    }

    // ---------- vendor/google-only ----------

    #[test]
    fn x_google_property_triggers_google_only() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-GOOGLE-CONFERENCEPROPERTIES:foo\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/google-only")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Info);
    }

    // ---------- vendor/icloud-only ----------

    #[test]
    fn x_apple_property_triggers_icloud_only() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-APPLE-CALENDAR-COLOR:#FF0000\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/icloud-only")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Info);
    }

    #[test]
    fn x_calendarserver_property_triggers_icloud_only() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-CALENDARSERVER-ACCESS:CONFIDENTIAL\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/icloud-only")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
    }

    // ---------- vendor/unrecognized-x ----------

    #[test]
    fn x_custom_property_triggers_unrecognized_x() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-CUSTOM-COLOR:blue\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "vendor/unrecognized-x")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
        assert!(hits[0].message.contains("X-CUSTOM-COLOR"));
    }

    #[test]
    fn known_vendor_property_does_not_trigger_unrecognized_x() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        props.push_str("X-GOOGLE-CONFERENCEPROPERTIES:foo\r\n");
        props.push_str("X-APPLE-CALENDAR-COLOR:#FF0000\r\n");
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "vendor/unrecognized-x"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn mixed_vendor_and_unrecognized_each_fire_independently() {
        let mut props = String::from(REQUIRED_FIELDS);
        props.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        props.push_str("X-CUSTOM-COLOR:blue\r\n");
        let diags = lint(&vevent_with(&props));
        let ms_hits = diags
            .iter()
            .filter(|d| d.rule == "vendor/microsoft-only")
            .count();
        let unrec_hits = diags
            .iter()
            .filter(|d| d.rule == "vendor/unrecognized-x")
            .count();
        assert_eq!(ms_hits, 1);
        assert_eq!(unrec_hits, 1);
    }

    // ---------- diagnostic line attribution ----------

    #[test]
    fn microsoft_diagnostic_points_at_the_property_line() {
        // Lines 1-3 = VCALENDAR header, 4 = BEGIN:VEVENT,
        // 5 = UID, 6 = DTSTAMP, 7 = DTSTART, 8 = DTEND, 9 = SUMMARY,
        // 10 = X-MICROSOFT-CDO-BUSYSTATUS.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str(REQUIRED_FIELDS);
        input.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&input);
        let hit = diags
            .iter()
            .find(|d| d.rule == "vendor/microsoft-only")
            .unwrap();
        assert_eq!(hit.line, Some(10));
    }
}
