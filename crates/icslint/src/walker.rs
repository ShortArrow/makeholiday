//! Raw `VEVENT` block walker.
//!
//! Some lint rules (`RFC5545/required-dtstamp`, `RFC5545/duplicate-summary`,
//! and others in the families below) need access to the *raw property
//! sequence* of each `VEVENT` block — before ics-core's typed parser
//! collapses duplicates and errors out on missing required fields. This
//! module provides that view on top of `ics_core::parser::unfold` and
//! `ics_core::parser::line::parse_logical_line`.
//!
//! The walker tolerates nested `BEGIN:`/`END:` blocks (e.g. `VALARM`)
//! inside a `VEVENT` and does not surface their properties as event-level
//! properties. It is intentionally lenient — malformed or partial input
//! still produces a best-effort scan rather than an error.

use ics_core::parser::line::parse_logical_line;
use ics_core::parser::unfold::unfold;

/// One property occurrence inside a `VEVENT` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProperty {
    /// 1-based logical-line number after unfolding.
    pub line: u32,
    /// Property name, UPPERCASE-normalized (matches
    /// `LogicalLine::name`).
    pub name: String,
}

/// Property sequence of one `VEVENT` block, in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVEventScan {
    /// 1-based logical line of the `BEGIN:VEVENT` marker.
    pub begin_line: u32,
    pub properties: Vec<RawProperty>,
}

impl RawVEventScan {
    pub fn count(&self, name: &str) -> usize {
        self.properties.iter().filter(|p| p.name == name).count()
    }

    pub fn has(&self, name: &str) -> bool {
        self.properties.iter().any(|p| p.name == name)
    }

    /// Lines on which the given property name appears (in source order).
    pub fn lines_of(&self, name: &str) -> Vec<u32> {
        self.properties
            .iter()
            .filter(|p| p.name == name)
            .map(|p| p.line)
            .collect()
    }
}

/// Walk `source` and return one [`RawVEventScan`] per `VEVENT` block.
///
/// Properties inside nested blocks (e.g. `VALARM` inside the event) are
/// not surfaced as event-level properties.
pub fn walk_vevents(source: &str) -> Vec<RawVEventScan> {
    let logical = unfold(source);
    let mut scans: Vec<RawVEventScan> = Vec::new();
    let mut i = 0;
    while i < logical.len() {
        if logical[i].trim() == "BEGIN:VEVENT" {
            let begin_line = (i + 1) as u32;
            let (props, next) = scan_vevent_body(&logical, i + 1);
            scans.push(RawVEventScan {
                begin_line,
                properties: props,
            });
            i = next;
            continue;
        }
        i += 1;
    }
    scans
}

fn scan_vevent_body(logical: &[String], start: usize) -> (Vec<RawProperty>, usize) {
    let mut props = Vec::new();
    let mut i = start;
    while i < logical.len() {
        let line = logical[i].trim();
        if line == "END:VEVENT" {
            return (props, i + 1);
        }
        if let Some(name) = line.strip_prefix("BEGIN:") {
            let end_marker = format!("END:{name}");
            i = skip_block_body(logical, i + 1, &end_marker);
            continue;
        }
        if let Some(ll) = parse_logical_line(line) {
            props.push(RawProperty {
                line: (i + 1) as u32,
                name: ll.name,
            });
        }
        i += 1;
    }
    (props, i)
}

fn skip_block_body(logical: &[String], start: usize, end_marker: &str) -> usize {
    let mut i = start;
    while i < logical.len() {
        if logical[i].trim() == end_marker {
            return i + 1;
        }
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_calendar_yields_no_scans() {
        let src = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\nEND:VCALENDAR\r\n";
        assert!(walk_vevents(src).is_empty());
    }

    #[test]
    fn single_vevent_collects_all_properties() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let scans = walk_vevents(&src);
        assert_eq!(scans.len(), 1);
        let names: Vec<_> = scans[0]
            .properties
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(names, vec!["UID", "DTSTAMP", "DTSTART", "DTEND", "SUMMARY"]);
    }

    #[test]
    fn duplicate_summary_is_visible_in_scan() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:one\r\n");
        src.push_str("SUMMARY:two\r\n");
        src.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let scans = walk_vevents(&src);
        assert_eq!(scans[0].count("SUMMARY"), 2);
    }

    #[test]
    fn nested_valarm_properties_do_not_leak_to_vevent_scope() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\n");
        src.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        src.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        src.push_str("SUMMARY:s\r\n");
        src.push_str("BEGIN:VALARM\r\n");
        src.push_str("ACTION:DISPLAY\r\n");
        src.push_str("DESCRIPTION:reminder\r\n");
        src.push_str("END:VALARM\r\n");
        src.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let scans = walk_vevents(&src);
        let names: Vec<_> = scans[0]
            .properties
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(names, vec!["UID", "DTSTAMP", "DTSTART", "DTEND", "SUMMARY"]);
    }

    #[test]
    fn line_numbers_track_unfolded_position() {
        // Use \r\n line endings so the unfolder produces one logical line per
        // physical line; the property of interest is on logical line 6.
        let mut src = String::from("BEGIN:VCALENDAR\r\n"); // 1
        src.push_str("VERSION:2.0\r\n"); // 2
        src.push_str("PRODID:-//x//y\r\n"); // 3
        src.push_str("BEGIN:VEVENT\r\n"); // 4
        src.push_str("UID:e1\r\n"); // 5
        src.push_str("DTSTAMP:20260101T000000Z\r\n"); // 6
        src.push_str("DTSTART;VALUE=DATE:20260429\r\n"); // 7
        src.push_str("DTEND;VALUE=DATE:20260430\r\n"); // 8
        src.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let scans = walk_vevents(&src);
        assert_eq!(scans[0].begin_line, 4);
        assert_eq!(scans[0].lines_of("DTSTAMP"), vec![6]);
    }

    #[test]
    fn two_vevents_each_get_own_scan() {
        let mut src = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y\r\n");
        src.push_str("BEGIN:VEVENT\r\nUID:e1\r\nDTSTAMP:20260101T000000Z\r\nEND:VEVENT\r\n");
        src.push_str("BEGIN:VEVENT\r\nUID:e2\r\nEND:VEVENT\r\n");
        src.push_str("END:VCALENDAR\r\n");
        let scans = walk_vevents(&src);
        assert_eq!(scans.len(), 2);
        assert!(scans[0].has("DTSTAMP"));
        assert!(!scans[1].has("DTSTAMP"));
    }
}
