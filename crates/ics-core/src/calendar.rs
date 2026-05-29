use crate::event::VEvent;
use crate::raw::{RawComponent, RawProperty};
use crate::vcalendar::VCalendar;

pub fn format_vevent(event: &VEvent) -> String {
    // TRANSP precedence: prefer the typed `transp` field if set;
    // otherwise derive from the Microsoft busystatus when present
    // (vendor-specific fallback); otherwise omit TRANSP altogether.
    let ms_busystatus = event.microsoft.as_ref().and_then(|m| m.busystatus);
    let transp_value = event
        .transp
        .map(|t| t.ics_value())
        .or_else(|| ms_busystatus.map(|b| b.transp()));
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}", event.uid),
        format!("DTSTAMP:{}", event.dtstamp.format("%Y%m%dT%H%M%SZ")),
        format!("DTSTART;VALUE=DATE:{}", event.dtstart.format("%Y%m%d")),
        format!("DTEND;VALUE=DATE:{}", event.dtend.format("%Y%m%d")),
        format!("SUMMARY:{}", event.summary),
    ];
    if let Some(v) = transp_value {
        lines.push(format!("TRANSP:{v}"));
    }
    if let Some(bs) = ms_busystatus {
        lines.push(format!("X-MICROSOFT-CDO-BUSYSTATUS:{}", bs.cdo_value()));
    }
    if let Some(class) = event.class {
        lines.push(format!("CLASS:{}", class.ics_value()));
    }
    if !event.categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", event.categories.join(",")));
    }
    if let Some(ref icon) = event.icon {
        lines.push(format!("X-MAKEHOLIDAY-ICON:{icon}"));
    }
    // Round-trip unknown properties at the tail of the component, sorted
    // by their captured source_index per ADR-018.
    let mut unknown_sorted: Vec<&RawProperty> = event.unknown.iter().collect();
    unknown_sorted.sort_by_key(|p| p.source_index);
    for p in unknown_sorted {
        lines.push(format_raw_property(p));
    }
    // Nested unrecognized components (VALARM, ...) preserved verbatim.
    for comp in &event.unrecognized_components {
        format_raw_component(comp, &mut lines);
    }
    lines.push("END:VEVENT".to_string());
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

pub fn format_calendar(cal: &VCalendar) -> String {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        format!("VERSION:{}", cal.version),
        format!("PRODID:{}", cal.prodid),
    ];
    if let Some(v) = &cal.calscale {
        lines.push(format!("CALSCALE:{v}"));
    }
    if let Some(v) = &cal.method {
        lines.push(format!("METHOD:{v}"));
    }
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    for event in &cal.events {
        out.push_str(&format_vevent(event));
    }
    // Calendar-level unrecognized components (VTIMEZONE, ...) preserved.
    let mut comp_lines: Vec<String> = Vec::new();
    for comp in &cal.unrecognized_components {
        format_raw_component(comp, &mut comp_lines);
    }
    if !comp_lines.is_empty() {
        out.push_str(&comp_lines.join("\r\n"));
        out.push_str("\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

/// Emit a `RawProperty` in `NAME[;PARAM=VALUE...]:VALUE` form.
fn format_raw_property(p: &RawProperty) -> String {
    let mut out = p.name.clone();
    for (k, v) in &p.params {
        out.push(';');
        out.push_str(k);
        out.push('=');
        out.push_str(v);
    }
    out.push(':');
    out.push_str(&p.value);
    out
}

/// Append a `RawComponent` (and its sub-components recursively) into
/// `lines` as `BEGIN:NAME ... END:NAME` block content.
fn format_raw_component(comp: &RawComponent, lines: &mut Vec<String>) {
    lines.push(format!("BEGIN:{}", comp.name));
    for p in &comp.properties {
        lines.push(format_raw_property(p));
    }
    for sub in &comp.sub_components {
        format_raw_component(sub, lines);
    }
    lines.push(format!("END:{}", comp.name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventClass;
    use crate::parser::parse_calendar;
    use crate::profile::microsoft::{EventExtensions as MsExtensions, MsBusyStatus};
    use crate::test_helpers::make_event;

    fn vcal(events: Vec<VEvent>) -> VCalendar {
        VCalendar {
            events,
            ..VCalendar::new("-//makeholiday//EN")
        }
    }

    #[test]
    fn header_contains_crlf_and_required_fields() {
        let s = format_calendar(&vcal(vec![]));
        assert!(s.contains("\r\n"), "must use CRLF");
        assert!(s.starts_with("BEGIN:VCALENDAR"));
        assert!(s.contains("VERSION:2.0"));
        assert!(s.contains("PRODID:"));
    }

    #[test]
    fn footer_is_end_vcalendar_crlf() {
        let s = format_calendar(&vcal(vec![]));
        assert!(s.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn format_vevent_single_day() {
        let mut event = make_event("test-uid-1", (2026, 1, 1), (2026, 1, 2), "元日");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Free),
        });
        let output = format_vevent(&event);
        assert!(output.contains("BEGIN:VEVENT\r\n"));
        assert!(output.contains("DTSTAMP:20260327T000000Z\r\n"));
        assert!(output.contains("DTSTART;VALUE=DATE:20260101\r\n"));
        assert!(output.contains("DTEND;VALUE=DATE:20260102\r\n"));
        assert!(output.contains("SUMMARY:元日\r\n"));
        assert!(output.contains("TRANSP:TRANSPARENT\r\n"));
        assert!(output.contains("X-MICROSOFT-CDO-BUSYSTATUS:FREE\r\n"));
        assert!(output.contains("UID:test-uid-1\r\n"));
        assert!(output.contains("END:VEVENT\r\n"));
        assert!(!output.contains("CLASS:"));
    }

    #[test]
    fn format_vevent_multi_day() {
        let event = make_event("test-uid-2", (2026, 12, 29), (2027, 1, 4), "年末年始");
        let output = format_vevent(&event);
        assert!(output.contains("DTSTART;VALUE=DATE:20261229"));
        assert!(output.contains("DTEND;VALUE=DATE:20270104"));
    }

    #[test]
    fn format_vevent_oof_private() {
        let mut event = make_event("oof-1", (2026, 8, 1), (2026, 8, 2), "不在");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Oof),
        });
        event.class = Some(EventClass::Private);
        let output = format_vevent(&event);
        assert!(output.contains("TRANSP:OPAQUE\r\n"));
        assert!(output.contains("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n"));
        assert!(output.contains("CLASS:PRIVATE\r\n"));
    }

    #[test]
    fn format_and_parse_categories_and_icon() {
        let mut event = make_event("cat-1", (2026, 6, 15), (2026, 6, 16), "出張");
        event.categories = vec!["仕事".to_string(), "出張".to_string()];
        event.icon = Some("airplane".to_string());
        let output = format_vevent(&event);
        assert!(output.contains("CATEGORIES:仕事,出張\r\n"));
        assert!(output.contains("X-MAKEHOLIDAY-ICON:airplane\r\n"));

        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].categories, vec!["仕事", "出張"]);
        assert_eq!(parsed.events[0].icon, Some("airplane".to_string()));
    }

    #[test]
    fn format_vevent_omits_optional_extensions_by_default() {
        let event = make_event("x", (2026, 1, 1), (2026, 1, 2), "元日");
        let output = format_vevent(&event);
        assert!(!output.contains("CATEGORIES:"));
        assert!(!output.contains("X-MAKEHOLIDAY-ICON:"));
    }

    #[test]
    fn format_calendar_empty() {
        let cal = format_calendar(&vcal(vec![]));
        assert!(cal.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(cal.ends_with("END:VCALENDAR\r\n"));
        assert!(!cal.contains("BEGIN:VEVENT"));
    }

    #[test]
    fn format_calendar_with_events() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ];
        let cal = format_calendar(&vcal(events));
        assert_eq!(cal.matches("BEGIN:VEVENT").count(), 2);
        assert!(cal.contains("SUMMARY:元日"));
        assert!(cal.contains("SUMMARY:建国記念の日"));
    }

    #[test]
    fn vcalendar_round_trip_preserves_calendar_fields() {
        let cal = VCalendar {
            version: "2.0".to_string(),
            prodid: "-//mh-test//EN".to_string(),
            calscale: Some("GREGORIAN".to_string()),
            method: Some("PUBLISH".to_string()),
            events: vec![],
            unrecognized_components: vec![],
        };
        let s = format_calendar(&cal);
        let parsed = parse_calendar(&s).unwrap();
        assert_eq!(parsed.version, "2.0");
        assert_eq!(parsed.prodid, "-//mh-test//EN");
        assert_eq!(parsed.calscale.as_deref(), Some("GREGORIAN"));
        assert_eq!(parsed.method.as_deref(), Some("PUBLISH"));
    }
}
