use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BusyStatus {
    Free,
    Tentative,
    Busy,
    Oof,
    #[serde(rename = "working")]
    WorkingElsewhere,
}

impl BusyStatus {
    pub fn transp(self) -> &'static str {
        match self {
            BusyStatus::Free => "TRANSPARENT",
            _ => "OPAQUE",
        }
    }

    pub fn cdo_value(self) -> &'static str {
        match self {
            BusyStatus::Free => "FREE",
            BusyStatus::Tentative => "TENTATIVE",
            BusyStatus::Busy => "BUSY",
            BusyStatus::Oof => "OOF",
            BusyStatus::WorkingElsewhere => "WORKINGELSEWHERE",
        }
    }

    pub fn from_cdo(s: &str) -> Option<Self> {
        match s {
            "FREE" => Some(BusyStatus::Free),
            "TENTATIVE" => Some(BusyStatus::Tentative),
            "BUSY" => Some(BusyStatus::Busy),
            "OOF" => Some(BusyStatus::Oof),
            "WORKINGELSEWHERE" => Some(BusyStatus::WorkingElsewhere),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EventClass {
    Public,
    Private,
    Confidential,
}

impl EventClass {
    pub fn ics_value(self) -> &'static str {
        match self {
            EventClass::Public => "PUBLIC",
            EventClass::Private => "PRIVATE",
            EventClass::Confidential => "CONFIDENTIAL",
        }
    }

    pub fn from_ics(s: &str) -> Option<Self> {
        match s {
            "PUBLIC" => Some(EventClass::Public),
            "PRIVATE" => Some(EventClass::Private),
            "CONFIDENTIAL" => Some(EventClass::Confidential),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VEvent {
    pub uid: String,
    #[serde(serialize_with = "serialize_dtstamp")]
    pub dtstamp: NaiveDateTime,
    #[serde(serialize_with = "serialize_date")]
    pub dtstart: NaiveDate,
    #[serde(serialize_with = "serialize_date")]
    pub dtend: NaiveDate,
    pub summary: String,
    pub busystatus: BusyStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<EventClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

fn serialize_date<S: serde::Serializer>(date: &NaiveDate, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.format("%Y-%m-%d").to_string())
}

fn serialize_dtstamp<S: serde::Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

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
        format!("X-MICROSOFT-CDO-BUSYSTATUS:{}", event.busystatus.cdo_value()),
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

pub fn parse_events(content: &str) -> Result<Vec<VEvent>, String> {
    let mut events = Vec::new();
    let normalized = content.replace("\r\n", "\n");
    let mut in_event = false;
    let mut uid = String::new();
    let mut dtstamp: Option<NaiveDateTime> = None;
    let mut dtstart: Option<NaiveDate> = None;
    let mut dtend: Option<NaiveDate> = None;
    let mut summary = String::new();
    let mut busystatus = BusyStatus::Free;
    let mut class: Option<EventClass> = None;
    let mut categories: Vec<String> = Vec::new();
    let mut icon: Option<String> = None;

    for line in normalized.lines() {
        let line = line.trim();
        if line == "BEGIN:VEVENT" {
            in_event = true;
            uid.clear();
            dtstamp = None;
            dtstart = None;
            dtend = None;
            summary.clear();
            busystatus = BusyStatus::Free;
            class = None;
            categories.clear();
            icon = None;
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
                busystatus,
                class,
                categories: categories.clone(),
                icon: icon.clone(),
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
            } else if let Some(val) = line.strip_prefix("X-MICROSOFT-CDO-BUSYSTATUS:") {
                if let Some(bs) = BusyStatus::from_cdo(val) {
                    busystatus = bs;
                }
            } else if let Some(val) = line.strip_prefix("CLASS:") {
                class = EventClass::from_ics(val);
            } else if let Some(val) = line.strip_prefix("CATEGORIES:") {
                categories = val.split(',').map(|s| s.trim().to_string()).collect();
            } else if let Some(val) = line.strip_prefix("X-MAKEHOLIDAY-ICON:") {
                icon = Some(val.to_string());
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


/// Parse index specifier: "3", "1,4,6", "3-7", "1,3-5,8"
/// Returns sorted, deduplicated 1-based indices.
pub fn parse_indices(input: &str, max: usize) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if let Some((start, end)) = part.split_once('-') {
            let s: usize = start
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {start}"))?;
            let e: usize = end
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {end}"))?;
            if s == 0 || e == 0 || s > max || e > max {
                return Err(format!("Index out of range (1-{max})"));
            }
            if s > e {
                return Err(format!("Invalid range: {s}-{e}"));
            }
            indices.extend(s..=e);
        } else {
            let idx: usize = part
                .parse()
                .map_err(|_| format!("Invalid number: {part}"))?;
            if idx == 0 || idx > max {
                return Err(format!("Index {idx} out of range (1-{max})"));
            }
            indices.push(idx);
        }
    }
    indices.sort();
    indices.dedup();
    Ok(indices)
}

pub fn remove_events_by_indices(content: &str, indices: &[usize]) -> Result<String, String> {
    let events = parse_events(content)?;
    for &idx in indices {
        if idx == 0 || idx > events.len() {
            return Err(format!(
                "Index {idx} out of range (1-{})",
                events.len()
            ));
        }
    }
    let remaining: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices.contains(&(i + 1)))
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
    let date_part = if start == end {
        format!("{}", start.format("%Y-%m-%d"))
    } else {
        format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"))
    };
    let icon_part = event
        .icon
        .as_ref()
        .map(|i| format!(" [{i}]"))
        .unwrap_or_default();
    format!("{date_part} : {}{icon_part}", event.summary)
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
            busystatus: BusyStatus::Free,
            class: None,
            categories: vec![],
            icon: None,
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
    fn parse_roundtrip_with_busystatus_and_class() {
        let mut event = make_event("rt-bs", (2026, 5, 1), (2026, 5, 2), "出張");
        event.busystatus = BusyStatus::WorkingElsewhere;
        event.class = Some(EventClass::Confidential);
        let cal = format_calendar(&[event.clone()]);
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].busystatus, BusyStatus::WorkingElsewhere);
        assert_eq!(parsed[0].class, Some(EventClass::Confidential));
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
    fn format_event_line_no_categories_no_icon() {
        let event = make_event("x", (2026, 1, 1), (2026, 1, 2), "元日");
        let output = format_vevent(&event);
        assert!(!output.contains("CATEGORIES:"));
        assert!(!output.contains("X-MAKEHOLIDAY-ICON:"));
    }

    #[test]
    fn format_event_line_with_icon() {
        let mut event = make_event("x", (2026, 6, 15), (2026, 6, 16), "出張");
        event.icon = Some("airplane".to_string());
        assert_eq!(format_event_line(&event), "2026-06-15 : 出張 [airplane]");
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

    // parse_indices tests
    #[test]
    fn parse_indices_single() {
        assert_eq!(parse_indices("3", 5).unwrap(), vec![3]);
    }

    #[test]
    fn parse_indices_comma() {
        assert_eq!(parse_indices("4,6", 10).unwrap(), vec![4, 6]);
    }

    #[test]
    fn parse_indices_range() {
        assert_eq!(parse_indices("6-10", 12).unwrap(), vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn parse_indices_mixed() {
        assert_eq!(parse_indices("1,3-5,8", 10).unwrap(), vec![1, 3, 4, 5, 8]);
    }

    #[test]
    fn parse_indices_dedup() {
        assert_eq!(parse_indices("3,3,3", 5).unwrap(), vec![3]);
    }

    #[test]
    fn parse_indices_out_of_range() {
        assert!(parse_indices("0", 5).is_err());
        assert!(parse_indices("6", 5).is_err());
    }

    #[test]
    fn parse_indices_invalid_range() {
        assert!(parse_indices("5-3", 10).is_err());
    }

    // remove_events_by_indices tests
    #[test]
    fn remove_multiple_by_indices() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "A"),
            make_event("b", (2026, 2, 1), (2026, 2, 2), "B"),
            make_event("c", (2026, 3, 1), (2026, 3, 2), "C"),
            make_event("d", (2026, 4, 1), (2026, 4, 2), "D"),
        ];
        let cal = format_calendar(&events);
        let result = remove_events_by_indices(&cal, &[2, 4]).unwrap();
        let parsed = parse_events(&result).unwrap();
        let summaries: Vec<_> = parsed.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["A", "C"]);
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
