//! `text/*` lint rules — TEXT-value encoding hygiene.
//!
//! ADR-026 §"Text encoding" defines five rules; four are implemented
//! here. `text/non-utf8-bytes` is deferred: the icslint `lint()` entry
//! point takes `&str`, so by the time control reaches a rule the input
//! has already been validated as UTF-8 (in `main.rs`, via
//! `fs::read_to_string`). Surfacing a non-UTF-8 input as a diagnostic
//! rather than an internal-error exit needs a `lint_bytes(&[u8])`
//! variant — a v0.2.x point-release item.

use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{DiagnosticSink, LintContext, Rule};

/// `text/bom` — source begins with a UTF-8 byte-order mark
/// (`U+FEFF`). Outlook is the usual producer. Consumers are split:
/// most tolerate the BOM, but some scripting pipelines that read the
/// file via `head` / `grep` will see a stray prefix character.
pub struct Bom;

impl Rule for Bom {
    fn id(&self) -> &'static str {
        "text/bom"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        if ctx.source.starts_with('\u{FEFF}') {
            sink.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "input begins with UTF-8 BOM (U+FEFF); most consumers tolerate it but some scripting pipelines do not".to_string(),
                )
                .with_line(1),
            );
        }
    }
}

/// `text/double-escape` — `SUMMARY` raw value contains a literal
/// `\\,` or `\\;` substring. This is the canonical signature of a
/// producer that ran the escape pass twice (first turning `,` into
/// `\,`, then turning the `\` into `\\`). Result on the consumer side
/// is a stray backslash in the rendered title.
pub struct DoubleEscape;

impl Rule for DoubleEscape {
    fn id(&self) -> &'static str {
        "text/double-escape"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if prop.name != "SUMMARY" {
                    continue;
                }
                if prop.value.contains("\\\\,") || prop.value.contains("\\\\;") {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} SUMMARY contains a `\\\\,` or `\\\\;` pattern; producer likely escaped TEXT values twice",
                                idx + 1
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// `text/unescaped-comma-in-summary` — `SUMMARY` raw value contains
/// an unescaped `,`. RFC 5545 §3.3.11 requires literal commas inside
/// a TEXT value to be backslash-escaped (`\,`). Producers that skip
/// the escape pass silently truncate the title at the first comma in
/// strict consumers.
pub struct UnescapedCommaInSummary;

impl Rule for UnescapedCommaInSummary {
    fn id(&self) -> &'static str {
        "text/unescaped-comma-in-summary"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if prop.name == "SUMMARY" && contains_unescaped(&prop.value, ',') {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} SUMMARY contains an unescaped `,`; RFC 5545 §3.3.11 requires `\\,` in TEXT values",
                                idx + 1
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// `text/unescaped-semicolon-in-summary` — `SUMMARY` raw value
/// contains an unescaped `;`. Same RFC 5545 §3.3.11 obligation as
/// commas, same silent-truncation failure mode in strict consumers.
pub struct UnescapedSemicolonInSummary;

impl Rule for UnescapedSemicolonInSummary {
    fn id(&self) -> &'static str {
        "text/unescaped-semicolon-in-summary"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink) {
        for (idx, scan) in ctx.vevent_scans.iter().enumerate() {
            for prop in &scan.properties {
                if prop.name == "SUMMARY" && contains_unescaped(&prop.value, ';') {
                    sink.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!(
                                "VEVENT #{} SUMMARY contains an unescaped `;`; RFC 5545 §3.3.11 requires `\\;` in TEXT values",
                                idx + 1
                            ),
                        )
                        .with_line(prop.line),
                    );
                }
            }
        }
    }
}

/// True when `value` contains an occurrence of `needle` not preceded
/// by an odd number of backslashes (i.e., the `needle` is not part of
/// an escape sequence).
///
/// Counts backslashes immediately preceding each match. Even count
/// (including zero) means the previous backslashes pair off into
/// escaped-backslash sequences and the match itself is unescaped.
fn contains_unescaped(value: &str, needle: char) -> bool {
    let bytes: Vec<char> = value.chars().collect();
    for (i, &c) in bytes.iter().enumerate() {
        if c != needle {
            continue;
        }
        let mut backslashes = 0usize;
        let mut j = i;
        while j > 0 && bytes[j - 1] == '\\' {
            backslashes += 1;
            j -= 1;
        }
        if backslashes % 2 == 0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;
    use crate::lint;

    fn vevent_with(properties: &str) -> String {
        let mut s = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        s.push_str("BEGIN:VEVENT\r\n");
        s.push_str(properties);
        s.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        s
    }

    const PROPS_BEFORE_SUMMARY: &str = "\
UID:e1\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n";

    // ---------- contains_unescaped helper ----------

    #[test]
    fn contains_unescaped_finds_bare_comma() {
        assert!(contains_unescaped("foo,bar", ','));
    }

    #[test]
    fn contains_unescaped_skips_escaped_comma() {
        assert!(!contains_unescaped(r"foo\,bar", ','));
    }

    #[test]
    fn contains_unescaped_finds_comma_after_escaped_backslash() {
        // `\\,` = escaped backslash, then unescaped comma.
        assert!(contains_unescaped(r"foo\\,bar", ','));
    }

    #[test]
    fn contains_unescaped_skips_comma_after_three_backslashes() {
        // `\\\,` = escaped backslash + escaped comma → no fire.
        assert!(!contains_unescaped(r"foo\\\,bar", ','));
    }

    #[test]
    fn contains_unescaped_returns_false_when_needle_absent() {
        assert!(!contains_unescaped("foobar", ','));
    }

    // ---------- text/bom ----------

    #[test]
    fn calendar_without_bom_does_not_trigger_bom() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:no bom here\r\n");
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "text/bom"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn calendar_with_leading_bom_triggers_bom() {
        let mut src = String::from("\u{FEFF}BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str(PROPS_BEFORE_SUMMARY);
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let diags = lint(&src);
        let hits: Vec<_> = diags.iter().filter(|d| d.rule == "text/bom").collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Info);
        assert_eq!(hits[0].line, Some(1));
    }

    // ---------- text/double-escape ----------

    #[test]
    fn properly_escaped_summary_does_not_trigger_double_escape() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Lunch\\, dinner\r\n");
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags.iter().any(|d| d.rule == "text/double-escape"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn double_escaped_summary_triggers_double_escape() {
        // `\\\\,` in Rust source = `\\,` in actual ICS text = the
        // double-escape signature.
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Lunch\\\\, dinner\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "text/double-escape")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    #[test]
    fn double_escaped_semicolon_triggers_double_escape() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Q1\\\\; Q2\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "text/double-escape")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
    }

    // ---------- text/unescaped-comma-in-summary ----------

    #[test]
    fn properly_escaped_comma_does_not_trigger_unescaped_comma() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Lunch\\, dinner\r\n");
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags
                .iter()
                .any(|d| d.rule == "text/unescaped-comma-in-summary"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn unescaped_comma_in_summary_triggers_rule() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Lunch, dinner\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "text/unescaped-comma-in-summary")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    // ---------- text/unescaped-semicolon-in-summary ----------

    #[test]
    fn properly_escaped_semicolon_does_not_trigger_unescaped_semicolon() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Q1\\; Q2\r\n");
        let diags = lint(&vevent_with(&props));
        assert!(
            !diags
                .iter()
                .any(|d| d.rule == "text/unescaped-semicolon-in-summary"),
            "got {:?}",
            diags
        );
    }

    #[test]
    fn unescaped_semicolon_in_summary_triggers_rule() {
        let mut props = String::from(PROPS_BEFORE_SUMMARY);
        props.push_str("SUMMARY:Q1; Q2\r\n");
        let diags = lint(&vevent_with(&props));
        let hits: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "text/unescaped-semicolon-in-summary")
            .collect();
        assert_eq!(hits.len(), 1, "got {:?}", diags);
        assert_eq!(hits[0].severity, Severity::Warning);
    }
}
