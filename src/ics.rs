use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub struct VEvent {
    pub uid: String,
    pub dtstart: NaiveDate,
    pub dtend: NaiveDate,
    pub summary: String,
}

pub fn vcalendar_header() -> String {
    "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//makeholiday//EN\r\n".to_string()
}

pub fn vcalendar_footer() -> String {
    "END:VCALENDAR\r\n".to_string()
}

pub fn format_vevent(event: &VEvent) -> String {
    format!(
        "BEGIN:VEVENT\r\nUID:{}\r\nDTSTART;VALUE=DATE:{}\r\nDTEND;VALUE=DATE:{}\r\nSUMMARY:{}\r\nTRANSP:TRANSPARENT\r\nEND:VEVENT\r\n",
        event.uid,
        event.dtstart.format("%Y%m%d"),
        event.dtend.format("%Y%m%d"),
        event.summary,
    )
}

pub fn format_calendar(events: &[VEvent]) -> String {
    let mut out = vcalendar_header();
    for event in events {
        out.push_str(&format_vevent(event));
    }
    out.push_str(&vcalendar_footer());
    out
}

pub fn parse_events(content: &str) -> Result<Vec<VEvent>, String> {
    let mut events = Vec::new();
    let normalized = content.replace("\r\n", "\n");
    let mut in_event = false;
    let mut uid = String::new();
    let mut dtstart: Option<NaiveDate> = None;
    let mut dtend: Option<NaiveDate> = None;
    let mut summary = String::new();

    for line in normalized.lines() {
        let line = line.trim();
        if line == "BEGIN:VEVENT" {
            in_event = true;
            uid.clear();
            dtstart = None;
            dtend = None;
            summary.clear();
        } else if line == "END:VEVENT" && in_event {
            let start = dtstart.ok_or("VEVENT missing DTSTART")?;
            let end = dtend.ok_or("VEVENT missing DTEND")?;
            events.push(VEvent {
                uid: uid.clone(),
                dtstart: start,
                dtend: end,
                summary: summary.clone(),
            });
            in_event = false;
        } else if in_event {
            if let Some(val) = line.strip_prefix("UID:") {
                uid = val.to_string();
            } else if let Some(val) = line.strip_prefix("DTSTART;VALUE=DATE:") {
                dtstart = Some(
                    NaiveDate::parse_from_str(val, "%Y%m%d")
                        .map_err(|e| format!("Invalid DTSTART: {e}"))?,
                );
            } else if let Some(val) = line.strip_prefix("DTEND;VALUE=DATE:") {
                dtend = Some(
                    NaiveDate::parse_from_str(val, "%Y%m%d")
                        .map_err(|e| format!("Invalid DTEND: {e}"))?,
                );
            } else if let Some(val) = line.strip_prefix("SUMMARY:") {
                summary = val.to_string();
            }
        }
    }
    Ok(events)
}

pub fn insert_event(content: &str, event: &VEvent) -> Result<String, String> {
    let footer = "END:VCALENDAR";
    let pos = content
        .find(footer)
        .ok_or_else(|| "Invalid ICS: missing END:VCALENDAR".to_string())?;
    let mut result = content[..pos].to_string();
    result.push_str(&format_vevent(event));
    result.push_str(&content[pos..]);
    Ok(result)
}

pub fn format_event_line(event: &VEvent) -> String {
    let start = event.dtstart;
    let end = event.dtend - chrono::Days::new(1);
    if start == end {
        format!("{} : {}", start.format("%Y-%m-%d"), event.summary)
    } else {
        format!(
            "{} to {} : {}",
            start.format("%Y-%m-%d"),
            end.format("%Y-%m-%d"),
            event.summary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // Step 1: vcalendar_header
    #[test]
    fn header_contains_crlf_and_required_fields() {
        let h = vcalendar_header();
        assert!(h.contains("\r\n"), "must use CRLF");
        assert!(h.contains("BEGIN:VCALENDAR"));
        assert!(h.contains("VERSION:2.0"));
        assert!(h.contains("PRODID:"));
    }

    // Step 2: vcalendar_footer
    #[test]
    fn footer_is_end_vcalendar_crlf() {
        assert_eq!(vcalendar_footer(), "END:VCALENDAR\r\n");
    }

    // Step 3: format_vevent single day
    #[test]
    fn format_vevent_single_day() {
        let event = VEvent {
            uid: "test-uid-1".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            summary: "元日".to_string(),
        };
        let output = format_vevent(&event);
        assert!(output.contains("BEGIN:VEVENT\r\n"));
        assert!(output.contains("DTSTART;VALUE=DATE:20260101\r\n"));
        assert!(output.contains("DTEND;VALUE=DATE:20260102\r\n"));
        assert!(output.contains("SUMMARY:元日\r\n"));
        assert!(output.contains("TRANSP:TRANSPARENT\r\n"));
        assert!(output.contains("UID:test-uid-1\r\n"));
        assert!(output.contains("END:VEVENT\r\n"));
    }

    // Step 4: format_vevent multi day
    #[test]
    fn format_vevent_multi_day() {
        let event = VEvent {
            uid: "test-uid-2".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 12, 29).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(),
            summary: "年末年始".to_string(),
        };
        let output = format_vevent(&event);
        assert!(output.contains("DTSTART;VALUE=DATE:20261229"));
        assert!(output.contains("DTEND;VALUE=DATE:20270104"));
    }

    // Step 5: format_calendar empty
    #[test]
    fn format_calendar_empty() {
        let cal = format_calendar(&[]);
        assert!(cal.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(cal.ends_with("END:VCALENDAR\r\n"));
        assert!(!cal.contains("BEGIN:VEVENT"));
    }

    // Step 6: format_calendar with events
    #[test]
    fn format_calendar_with_events() {
        let events = vec![
            VEvent {
                uid: "a".to_string(),
                dtstart: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                dtend: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
                summary: "元日".to_string(),
            },
            VEvent {
                uid: "b".to_string(),
                dtstart: NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
                dtend: NaiveDate::from_ymd_opt(2026, 2, 12).unwrap(),
                summary: "建国記念の日".to_string(),
            },
        ];
        let cal = format_calendar(&events);
        assert_eq!(cal.matches("BEGIN:VEVENT").count(), 2);
        assert!(cal.contains("SUMMARY:元日"));
        assert!(cal.contains("SUMMARY:建国記念の日"));
    }

    // Step 7: parse_events round-trip
    #[test]
    fn parse_events_roundtrip() {
        let event = VEvent {
            uid: "rt-1".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 5, 3).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
            summary: "憲法記念日".to_string(),
        };
        let cal = format_calendar(&[event.clone()]);
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], event);
    }

    // Step 8: parse_events empty calendar
    #[test]
    fn parse_events_empty() {
        let cal = format_calendar(&[]);
        let parsed = parse_events(&cal).unwrap();
        assert!(parsed.is_empty());
    }

    // Step 9: insert_event
    #[test]
    fn insert_event_adds_vevent_before_footer() {
        let cal = format_calendar(&[]);
        let event = VEvent {
            uid: "ins-1".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            summary: "春分の日".to_string(),
        };
        let result = insert_event(&cal, &event).unwrap();
        assert!(result.contains("SUMMARY:春分の日"));
        assert!(result.ends_with("END:VCALENDAR\r\n"));
        let parsed = parse_events(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].summary, "春分の日");
    }

    // Step 10: format_event_line
    #[test]
    fn format_event_line_single_day() {
        let event = VEvent {
            uid: "x".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            summary: "元日".to_string(),
        };
        assert_eq!(format_event_line(&event), "2026-01-01 : 元日");
    }

    #[test]
    fn format_event_line_multi_day() {
        let event = VEvent {
            uid: "y".to_string(),
            dtstart: NaiveDate::from_ymd_opt(2026, 12, 29).unwrap(),
            dtend: NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(),
            summary: "年末年始".to_string(),
        };
        assert_eq!(
            format_event_line(&event),
            "2026-12-29 to 2027-01-03 : 年末年始"
        );
    }
}
