use crate::event::{BusyStatus, EventClass, VEvent};
use chrono::{NaiveDate, NaiveDateTime};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::format_calendar;
    use crate::event::{BusyStatus, EventClass};
    use crate::test_helpers::make_event;

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
    fn parse_events_roundtrip() {
        let event = make_event("rt-1", (2026, 5, 3), (2026, 5, 4), "憲法記念日");
        let cal = format_calendar(std::slice::from_ref(&event));
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
}
