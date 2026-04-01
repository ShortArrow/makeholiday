use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, Clone, PartialEq)]
pub struct VEvent {
    pub uid: String,
    pub dtstamp: NaiveDateTime,
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
        "BEGIN:VEVENT\r\nUID:{}\r\nDTSTAMP:{}\r\nDTSTART;VALUE=DATE:{}\r\nDTEND;VALUE=DATE:{}\r\nSUMMARY:{}\r\nTRANSP:TRANSPARENT\r\nEND:VEVENT\r\n",
        event.uid,
        event.dtstamp.format("%Y%m%dT%H%M%SZ"),
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
    let mut dtstamp: Option<NaiveDateTime> = None;
    let mut dtstart: Option<NaiveDate> = None;
    let mut dtend: Option<NaiveDate> = None;
    let mut summary = String::new();

    for line in normalized.lines() {
        let line = line.trim();
        if line == "BEGIN:VEVENT" {
            in_event = true;
            uid.clear();
            dtstamp = None;
            dtstart = None;
            dtend = None;
            summary.clear();
        } else if line == "END:VEVENT" && in_event {
            let stamp = dtstamp.ok_or("VEVENT missing DTSTAMP")?;
            let start = dtstart.ok_or("VEVENT missing DTSTART")?;
            let end = dtend.ok_or("VEVENT missing DTEND")?;
            events.push(VEvent {
                uid: uid.clone(),
                dtstamp: stamp,
                dtstart: start,
                dtend: end,
                summary: summary.clone(),
            });
            in_event = false;
        } else if in_event {
            if let Some(val) = line.strip_prefix("UID:") {
                uid = val.to_string();
            } else if let Some(val) = line.strip_prefix("DTSTAMP:") {
                dtstamp = Some(
                    NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%SZ")
                        .map_err(|e| format!("Invalid DTSTAMP: {e}"))?,
                );
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

pub fn remove_event_by_summary(content: &str, summary: &str) -> Result<String, String> {
    let events = parse_events(content)?;
    let remaining: Vec<_> = events.iter().filter(|e| e.summary != summary).collect();
    if remaining.len() == events.len() {
        return Err(format!("No event found with summary: {summary}"));
    }
    Ok(format_calendar(
        &remaining.into_iter().cloned().collect::<Vec<_>>(),
    ))
}

pub fn remove_event_by_index(content: &str, index: usize) -> Result<String, String> {
    let events = parse_events(content)?;
    if index == 0 || index > events.len() {
        return Err(format!(
            "Index {index} out of range (1-{})",
            events.len()
        ));
    }
    let remaining: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index - 1)
        .map(|(_, e)| e.clone())
        .collect();
    Ok(format_calendar(&remaining))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKey {
    Start,
    End,
    Summary,
}

pub fn sort_events(events: &[VEvent], keys: &[SortKey], descending: bool) -> Vec<VEvent> {
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        let ord = keys
            .iter()
            .map(|key| match key {
                SortKey::Start => a.dtstart.cmp(&b.dtstart),
                SortKey::End => a.dtend.cmp(&b.dtend),
                SortKey::Summary => a.summary.cmp(&b.summary),
            })
            .find(|o| o.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal);
        if descending { ord.reverse() } else { ord }
    });
    sorted
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
    use chrono::{NaiveDate, NaiveDateTime};

    fn test_dtstamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 27)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn make_event(uid: &str, start: (i32, u32, u32), end: (i32, u32, u32), summary: &str) -> VEvent {
        VEvent {
            uid: uid.to_string(),
            dtstamp: test_dtstamp(),
            dtstart: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            dtend: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            summary: summary.to_string(),
        }
    }

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
        assert!(output.contains("UID:test-uid-1\r\n"));
        assert!(output.contains("END:VEVENT\r\n"));
    }

    #[test]
    fn format_vevent_multi_day() {
        let event = make_event("test-uid-2", (2026, 12, 29), (2027, 1, 4), "年末年始");
        let output = format_vevent(&event);
        assert!(output.contains("DTSTART;VALUE=DATE:20261229"));
        assert!(output.contains("DTEND;VALUE=DATE:20270104"));
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
    fn parse_events_roundtrip() {
        let event = make_event("rt-1", (2026, 5, 3), (2026, 5, 4), "憲法記念日");
        let cal = format_calendar(&[event.clone()]);
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], event);
    }

    #[test]
    fn parse_events_empty() {
        let cal = format_calendar(&[]);
        let parsed = parse_events(&cal).unwrap();
        assert!(parsed.is_empty());
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

    #[test]
    fn format_event_line_single_day() {
        let event = make_event("x", (2026, 1, 1), (2026, 1, 2), "元日");
        assert_eq!(format_event_line(&event), "2026-01-01 : 元日");
    }

    #[test]
    fn format_event_line_multi_day() {
        let event = make_event("y", (2026, 12, 29), (2027, 1, 4), "年末年始");
        assert_eq!(
            format_event_line(&event),
            "2026-12-29 to 2027-01-03 : 年末年始"
        );
    }

    // remove tests
    #[test]
    fn remove_by_summary() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ];
        let cal = format_calendar(&events);
        let result = remove_event_by_summary(&cal, "元日").unwrap();
        let parsed = parse_events(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].summary, "建国記念の日");
    }

    #[test]
    fn remove_by_summary_not_found() {
        let cal = format_calendar(&[make_event("a", (2026, 1, 1), (2026, 1, 2), "元日")]);
        let result = remove_event_by_summary(&cal, "存在しない");
        assert!(result.is_err());
    }

    #[test]
    fn remove_by_index() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ];
        let cal = format_calendar(&events);
        let result = remove_event_by_index(&cal, 1).unwrap();
        let parsed = parse_events(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].summary, "建国記念の日");
    }

    #[test]
    fn remove_by_index_out_of_range() {
        let cal = format_calendar(&[make_event("a", (2026, 1, 1), (2026, 1, 2), "元日")]);
        assert!(remove_event_by_index(&cal, 0).is_err());
        assert!(remove_event_by_index(&cal, 2).is_err());
    }

    // sort tests
    fn unsorted_events() -> Vec<VEvent> {
        vec![
            make_event("c", (2026, 5, 3), (2026, 5, 6), "憲法記念日"),
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ]
    }

    #[test]
    fn sort_by_start_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Start], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_start_desc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Start], true);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["憲法記念日", "建国記念の日", "元日"]);
    }

    #[test]
    fn sort_by_end_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::End], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_summary_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Summary], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_summary_desc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Summary], true);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["憲法記念日", "建国記念の日", "元日"]);
    }

    #[test]
    fn sort_multi_key() {
        // Two events with same start, different summaries
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "B休日"),
            make_event("b", (2026, 1, 1), (2026, 1, 2), "A休日"),
            make_event("c", (2026, 2, 1), (2026, 2, 2), "C休日"),
        ];
        let sorted = sort_events(&events, &[SortKey::Start, SortKey::Summary], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["A休日", "B休日", "C休日"]);
    }
}
