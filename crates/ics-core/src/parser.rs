use crate::error::{Error, Result};
use crate::event::{BusyStatus, EventClass, VEvent};
use crate::raw::RawProperty;
use chrono::{NaiveDate, NaiveDateTime};

pub fn parse_events(content: &str) -> Result<Vec<VEvent>> {
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
    let mut unknown: Vec<RawProperty> = Vec::new();
    let mut unknown_index: u32 = 0;

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
            unknown.clear();
            unknown_index = 0;
        } else if line == "END:VEVENT" && in_event {
            let stamp = dtstamp.ok_or_else(|| Error::parse("VEVENT missing DTSTAMP"))?;
            let start = dtstart.ok_or_else(|| Error::parse("VEVENT missing DTSTART"))?;
            let end = dtend.ok_or_else(|| Error::parse("VEVENT missing DTEND"))?;
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
                unknown: std::mem::take(&mut unknown),
            });
            in_event = false;
        } else if in_event {
            if let Some(val) = line.strip_prefix("UID:") {
                uid = val.to_string();
            } else if let Some(val) = line.strip_prefix("DTSTAMP:") {
                dtstamp = Some(
                    NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%SZ")
                        .map_err(|e| Error::parse(format!("Invalid DTSTAMP: {e}")))?,
                );
            } else if let Some(val) = line.strip_prefix("DTSTART;VALUE=DATE:") {
                dtstart = Some(
                    NaiveDate::parse_from_str(val, "%Y%m%d")
                        .map_err(|e| Error::parse(format!("Invalid DTSTART: {e}")))?,
                );
            } else if let Some(val) = line.strip_prefix("DTEND;VALUE=DATE:") {
                dtend = Some(
                    NaiveDate::parse_from_str(val, "%Y%m%d")
                        .map_err(|e| Error::parse(format!("Invalid DTEND: {e}")))?,
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
            } else if line.starts_with("X-") {
                if let Some(prop) = parse_raw_property(line, unknown_index + 1) {
                    unknown.push(prop);
                    unknown_index += 1;
                }
            }
        }
    }
    Ok(events)
}

/// Parse a property line `NAME[;PARAM=VALUE...]:VALUE` into a `RawProperty`.
/// Returns `None` if the line has no `:` separator. Quoted parameter values
/// have their surrounding `"` stripped; other escapes are left intact per
/// ADR-018 (raw value preservation).
pub(crate) fn parse_raw_property(line: &str, source_index: u32) -> Option<RawProperty> {
    let colon = line.find(':')?;
    let prefix = &line[..colon];
    let value = &line[colon + 1..];

    let mut parts = prefix.split(';');
    let name = parts.next()?.to_uppercase();
    let mut params = Vec::new();
    for p in parts {
        if let Some((k, v)) = p.split_once('=') {
            let v = v.trim_matches('"');
            params.push((k.to_uppercase(), v.to_string()));
        }
    }
    Some(RawProperty {
        name,
        params,
        value: value.to_string(),
        source_index,
    })
}

/// Parse index specifier: "3", "1,4,6", "3-7", "1,3-5,8"
/// Returns sorted, deduplicated 1-based indices.
pub fn parse_indices(input: &str, max: usize) -> Result<Vec<usize>> {
    let mut indices = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if let Some((start, end)) = part.split_once('-') {
            let s: usize = start
                .trim()
                .parse()
                .map_err(|_| Error::parse(format!("Invalid number: {start}")))?;
            let e: usize = end
                .trim()
                .parse()
                .map_err(|_| Error::parse(format!("Invalid number: {end}")))?;
            if s == 0 || e == 0 || s > max || e > max {
                return Err(Error::parse(format!("Index out of range (1-{max})")));
            }
            if s > e {
                return Err(Error::parse(format!("Invalid range: {s}-{e}")));
            }
            indices.extend(s..=e);
        } else {
            let idx: usize = part
                .parse()
                .map_err(|_| Error::parse(format!("Invalid number: {part}")))?;
            if idx == 0 || idx > max {
                return Err(Error::parse(format!("Index {idx} out of range (1-{max})")));
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
    use crate::raw::RawProperty;
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

    // ADR-001 Migration Step 1 — unknown property round-trip.

    #[test]
    fn unknown_x_property_round_trips() {
        let mut event = make_event("rt-unk", (2026, 4, 29), (2026, 4, 30), "昭和の日");
        event.unknown.push(RawProperty {
            name: "X-CUSTOM-COLOR".to_string(),
            params: vec![],
            value: "blue".to_string(),
            source_index: 1,
        });
        let cal = format_calendar(std::slice::from_ref(&event));
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].unknown.len(), 1);
        assert_eq!(parsed[0].unknown[0].name, "X-CUSTOM-COLOR");
        assert_eq!(parsed[0].unknown[0].value, "blue");
    }

    #[test]
    fn unknown_x_property_with_params_round_trips() {
        let mut event = make_event("rt-unk-p", (2026, 4, 29), (2026, 4, 30), "昭和の日");
        event.unknown.push(RawProperty {
            name: "X-CUSTOM-FOO".to_string(),
            params: vec![("LANG".to_string(), "en".to_string())],
            value: "hello".to_string(),
            source_index: 1,
        });
        let cal = format_calendar(std::slice::from_ref(&event));
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed[0].unknown.len(), 1);
        assert_eq!(parsed[0].unknown[0].name, "X-CUSTOM-FOO");
        assert_eq!(
            parsed[0].unknown[0].params,
            vec![("LANG".to_string(), "en".to_string())]
        );
        assert_eq!(parsed[0].unknown[0].value, "hello");
    }

    #[test]
    fn unknown_x_property_preserves_source_index_order() {
        // Inputs in order A, B, C should come back A, B, C with source_index 1, 2, 3.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-CUSTOM-A:1\r\n");
        input.push_str("X-CUSTOM-B:2\r\n");
        input.push_str("X-CUSTOM-C:3\r\n");
        input.push_str("END:VEVENT\r\n");
        input.push_str("END:VCALENDAR\r\n");
        let parsed = parse_events(&input).unwrap();
        assert_eq!(parsed[0].unknown.len(), 3);
        assert_eq!(parsed[0].unknown[0].source_index, 1);
        assert_eq!(parsed[0].unknown[0].name, "X-CUSTOM-A");
        assert_eq!(parsed[0].unknown[1].source_index, 2);
        assert_eq!(parsed[0].unknown[1].name, "X-CUSTOM-B");
        assert_eq!(parsed[0].unknown[2].source_index, 3);
        assert_eq!(parsed[0].unknown[2].name, "X-CUSTOM-C");
    }

    #[test]
    fn typed_x_microsoft_and_x_makeholiday_stay_typed_not_in_unknown() {
        // The two pre-existing typed X-* fields must not be duplicated into unknown.
        let mut event = make_event("rt-typed", (2026, 4, 29), (2026, 4, 30), "昭和の日");
        event.busystatus = BusyStatus::Oof;
        event.icon = Some("flag".to_string());
        let cal = format_calendar(std::slice::from_ref(&event));
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed[0].busystatus, BusyStatus::Oof);
        assert_eq!(parsed[0].icon.as_deref(), Some("flag"));
        assert!(parsed[0].unknown.is_empty());
    }

    #[test]
    fn parse_raw_property_uppercases_name_and_keys() {
        let p = parse_raw_property("x-custom-foo;lang=en:hello", 1).unwrap();
        assert_eq!(p.name, "X-CUSTOM-FOO");
        assert_eq!(p.params, vec![("LANG".to_string(), "en".to_string())]);
        assert_eq!(p.value, "hello");
    }

    #[test]
    fn parse_raw_property_strips_quotes_from_param_value() {
        let p = parse_raw_property(r#"X-FOO;LANG="ja-JP":val"#, 1).unwrap();
        assert_eq!(p.params, vec![("LANG".to_string(), "ja-JP".to_string())]);
    }

    #[test]
    fn parse_raw_property_returns_none_when_no_colon() {
        assert!(parse_raw_property("X-NOCOLON", 1).is_none());
    }

    #[test]
    fn class_categories_not_starting_with_x_do_not_fall_to_unknown() {
        // Make sure the X- check is properly scoped — typed-but-non-X fields
        // (CLASS, CATEGORIES) keep working and don't end up in unknown.
        let mut event = make_event("rt-tc", (2026, 4, 29), (2026, 4, 30), "s");
        event.class = Some(EventClass::Private);
        event.categories = vec!["work".to_string()];
        let cal = format_calendar(std::slice::from_ref(&event));
        let parsed = parse_events(&cal).unwrap();
        assert_eq!(parsed[0].class, Some(EventClass::Private));
        assert_eq!(parsed[0].categories, vec!["work".to_string()]);
        assert!(parsed[0].unknown.is_empty());
    }
}
