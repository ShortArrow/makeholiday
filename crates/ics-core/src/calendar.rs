use crate::error::{Error, Result};
use crate::event::VEvent;

pub fn vcalendar_header() -> String {
    "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//makeholiday//EN\r\n".to_string()
}

pub fn vcalendar_footer() -> String {
    "END:VCALENDAR\r\n".to_string()
}

pub fn format_vevent(event: &VEvent) -> String {
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}", event.uid),
        format!("DTSTAMP:{}", event.dtstamp.format("%Y%m%dT%H%M%SZ")),
        format!("DTSTART;VALUE=DATE:{}", event.dtstart.format("%Y%m%d")),
        format!("DTEND;VALUE=DATE:{}", event.dtend.format("%Y%m%d")),
        format!("SUMMARY:{}", event.summary),
        format!("TRANSP:{}", event.busystatus.transp()),
        format!(
            "X-MICROSOFT-CDO-BUSYSTATUS:{}",
            event.busystatus.cdo_value()
        ),
    ];
    if let Some(class) = event.class {
        lines.push(format!("CLASS:{}", class.ics_value()));
    }
    if !event.categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", event.categories.join(",")));
    }
    if let Some(ref icon) = event.icon {
        lines.push(format!("X-MAKEHOLIDAY-ICON:{icon}"));
    }
    lines.push("END:VEVENT".to_string());
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

pub fn format_calendar(events: &[VEvent]) -> String {
    let mut out = vcalendar_header();
    for event in events {
        out.push_str(&format_vevent(event));
    }
    out.push_str(&vcalendar_footer());
    out
}

pub fn insert_event(content: &str, event: &VEvent) -> Result<String> {
    let footer = "END:VCALENDAR";
    let pos = content
        .find(footer)
        .ok_or_else(|| Error::parse("Invalid ICS: missing END:VCALENDAR"))?;
    let mut result = content[..pos].to_string();
    result.push_str(&format_vevent(event));
    result.push_str(&content[pos..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{BusyStatus, EventClass};
    use crate::parser::parse_events;
    use crate::test_helpers::make_event;

    #[test]
    fn header_contains_crlf_and_required_fields() {
        let h = vcalendar_header();
        assert!(h.contains("\r\n"), "must use CRLF");
        assert!(h.contains("BEGIN:VCALENDAR"));
        assert!(h.contains("VERSION:2.0"));
        assert!(h.contains("PRODID:"));
    }

    #[test]
    fn footer_is_end_vcalendar_crlf() {
        assert_eq!(vcalendar_footer(), "END:VCALENDAR\r\n");
    }

    #[test]
    fn format_vevent_single_day() {
        let event = make_event("test-uid-1", (2026, 1, 1), (2026, 1, 2), "元日");
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
        // Default: no CLASS output
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
        event.busystatus = BusyStatus::Oof;
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

        let cal = format_calendar(&[event.clone()]);
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed[0].categories, vec!["仕事", "出張"]);
        assert_eq!(parsed[0].icon, Some("airplane".to_string()));
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
        let cal = format_calendar(&[]);
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
        let cal = format_calendar(&events);
        assert_eq!(cal.matches("BEGIN:VEVENT").count(), 2);
        assert!(cal.contains("SUMMARY:元日"));
        assert!(cal.contains("SUMMARY:建国記念の日"));
    }

    #[test]
    fn insert_event_adds_vevent_before_footer() {
        let cal = format_calendar(&[]);
        let event = make_event("ins-1", (2026, 3, 20), (2026, 3, 21), "春分の日");
        let result = insert_event(&cal, &event).unwrap();
        assert!(result.contains("SUMMARY:春分の日"));
        assert!(result.ends_with("END:VCALENDAR\r\n"));
        let parsed = parse_events(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].summary, "春分の日");
    }
}
