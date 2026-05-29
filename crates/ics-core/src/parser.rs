use crate::error::{Error, Result};
use crate::event::{EventClass, Transp, VEvent};
use crate::profile::microsoft::{self, MsBusyStatus};
use crate::raw::{RawComponent, RawProperty};
use crate::vcalendar::VCalendar;
use chrono::{NaiveDate, NaiveDateTime};

/// Parse the full ICS document into a typed `VCalendar`.
///
/// Calendar-level non-`VEVENT` components (`VTIMEZONE`, `VJOURNAL`,
/// etc.) are preserved into `VCalendar.unrecognized_components`.
/// Non-typed nested components inside a `VEVENT` (e.g. `VALARM`) flow
/// into `VEvent.unrecognized_components`.
pub fn parse_calendar(content: &str) -> Result<VCalendar> {
    let normalized = content.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().map(str::trim).collect();
    let mut idx = 0;

    // Skip until BEGIN:VCALENDAR. Be lenient about leading whitespace /
    // comments / BOM that may have slipped through.
    while idx < lines.len() && lines[idx] != "BEGIN:VCALENDAR" {
        idx += 1;
    }
    if idx == lines.len() {
        return Err(Error::parse("missing BEGIN:VCALENDAR"));
    }
    idx += 1; // step past BEGIN:VCALENDAR

    let mut version = String::new();
    let mut prodid = String::new();
    let mut calscale: Option<String> = None;
    let mut method: Option<String> = None;
    let mut events: Vec<VEvent> = Vec::new();
    let mut unrecognized_components: Vec<RawComponent> = Vec::new();

    while idx < lines.len() {
        let line = lines[idx];
        if line == "END:VCALENDAR" {
            break;
        }
        if let Some(name) = strip_begin(line) {
            if name == "VEVENT" {
                let (event, next) = parse_vevent_block(&lines, idx + 1)?;
                events.push(event);
                idx = next;
                continue;
            }
            let (comp, next) = parse_raw_component_block(name, &lines, idx + 1);
            unrecognized_components.push(comp);
            idx = next;
            continue;
        }
        // Otherwise it's a calendar-level property line.
        if let Some(v) = line.strip_prefix("VERSION:") {
            version = v.to_string();
        } else if let Some(v) = line.strip_prefix("PRODID:") {
            prodid = v.to_string();
        } else if let Some(v) = line.strip_prefix("CALSCALE:") {
            calscale = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("METHOD:") {
            method = Some(v.to_string());
        }
        // Calendar-level X-WR-*, unknown X-*, and other properties are
        // not yet captured at this layer — landing alongside ADR-018
        // round-trip work for the calendar shell.
        idx += 1;
    }

    Ok(VCalendar {
        version,
        prodid,
        calscale,
        method,
        events,
        unrecognized_components,
    })
}

/// Thin compatibility wrapper returning only the events.
pub fn parse_events(content: &str) -> Result<Vec<VEvent>> {
    parse_calendar(content).map(|c| c.events)
}

fn strip_begin(line: &str) -> Option<&str> {
    line.strip_prefix("BEGIN:")
}

fn strip_end(line: &str) -> Option<&str> {
    line.strip_prefix("END:")
}

/// Parse a `VEVENT` body starting at `start` (immediately after
/// `BEGIN:VEVENT`). Returns the parsed event and the index of the line
/// immediately after `END:VEVENT`.
fn parse_vevent_block(lines: &[&str], start: usize) -> Result<(VEvent, usize)> {
    let mut uid = String::new();
    let mut dtstamp: Option<NaiveDateTime> = None;
    let mut dtstart: Option<NaiveDate> = None;
    let mut dtend: Option<NaiveDate> = None;
    let mut summary = String::new();
    let mut transp: Option<Transp> = None;
    let mut ms_busystatus: Option<MsBusyStatus> = None;
    let mut class: Option<EventClass> = None;
    let mut categories: Vec<String> = Vec::new();
    let mut icon: Option<String> = None;
    let mut unknown: Vec<RawProperty> = Vec::new();
    let mut unknown_index: u32 = 0;
    let mut unrecognized_components: Vec<RawComponent> = Vec::new();

    let mut idx = start;
    while idx < lines.len() {
        let line = lines[idx];
        if line == "END:VEVENT" {
            let stamp = dtstamp.ok_or_else(|| Error::parse("VEVENT missing DTSTAMP"))?;
            let s = dtstart.ok_or_else(|| Error::parse("VEVENT missing DTSTART"))?;
            let e = dtend.ok_or_else(|| Error::parse("VEVENT missing DTEND"))?;
            return Ok((
                VEvent {
                    uid,
                    dtstamp: stamp,
                    dtstart: s,
                    dtend: e,
                    summary,
                    transp,
                    class,
                    categories,
                    icon,
                    microsoft: ms_busystatus.map(|b| microsoft::EventExtensions {
                        busystatus: Some(b),
                    }),
                    unknown,
                    unrecognized_components,
                },
                idx + 1,
            ));
        }
        if let Some(name) = strip_begin(line) {
            let (comp, next) = parse_raw_component_block(name, lines, idx + 1);
            unrecognized_components.push(comp);
            idx = next;
            continue;
        }
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
        } else if let Some(val) = line.strip_prefix("TRANSP:") {
            transp = Transp::from_ics(val);
        } else if let Some(val) = line.strip_prefix("X-MICROSOFT-CDO-BUSYSTATUS:") {
            if let Some(bs) = MsBusyStatus::from_cdo(val) {
                ms_busystatus = Some(bs);
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
        idx += 1;
    }
    Err(Error::parse("VEVENT missing END:VEVENT"))
}

/// Recursively capture a `BEGIN:<name>...END:<name>` block as a
/// `RawComponent`. Nested `BEGIN:`/`END:` blocks become entries in
/// `sub_components`. Returns the component and the line index immediately
/// after the matching `END:<name>`.
fn parse_raw_component_block(name: &str, lines: &[&str], start: usize) -> (RawComponent, usize) {
    let name = name.to_uppercase();
    let mut properties: Vec<RawProperty> = Vec::new();
    let mut sub_components: Vec<RawComponent> = Vec::new();
    let mut prop_index: u32 = 0;
    let mut idx = start;
    while idx < lines.len() {
        let line = lines[idx];
        if let Some(end_name) = strip_end(line) {
            if end_name.eq_ignore_ascii_case(&name) {
                return (
                    RawComponent {
                        name,
                        properties,
                        sub_components,
                    },
                    idx + 1,
                );
            }
            // Mismatched END (or END for an outer scope) — bail out;
            // upstream component will consume it.
            return (
                RawComponent {
                    name,
                    properties,
                    sub_components,
                },
                idx,
            );
        }
        if let Some(sub_name) = strip_begin(line) {
            let (sub, next) = parse_raw_component_block(sub_name, lines, idx + 1);
            sub_components.push(sub);
            idx = next;
            continue;
        }
        if let Some(prop) = parse_raw_property(line, prop_index + 1) {
            properties.push(prop);
            prop_index += 1;
        }
        idx += 1;
    }
    // Reached EOF before END — best-effort return.
    (
        RawComponent {
            name,
            properties,
            sub_components,
        },
        idx,
    )
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
    use crate::event::EventClass;
    use crate::profile::microsoft::{EventExtensions as MsExtensions, MsBusyStatus};
    use crate::raw::{RawComponent, RawProperty};
    use crate::test_helpers::make_event;
    use crate::vcalendar::VCalendar;

    fn vcal(events: Vec<VEvent>) -> VCalendar {
        VCalendar {
            events,
            ..VCalendar::new("-//makeholiday//EN")
        }
    }

    #[test]
    fn parse_roundtrip_with_busystatus_and_class() {
        let mut event = make_event("rt-bs", (2026, 5, 1), (2026, 5, 2), "出張");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::WorkingElsewhere),
        });
        event.class = Some(EventClass::Confidential);
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(
            parsed.events[0]
                .microsoft
                .as_ref()
                .and_then(|m| m.busystatus),
            Some(MsBusyStatus::WorkingElsewhere)
        );
        assert_eq!(parsed.events[0].class, Some(EventClass::Confidential));
    }

    #[test]
    fn parse_events_roundtrip() {
        // make_event leaves microsoft = None and transp = None; the formatter
        // omits both TRANSP and X-MICROSOFT-CDO-BUSYSTATUS in that case so
        // the round-trip is exact (no inferred fields appearing in the
        // re-parsed value).
        let event = make_event("rt-1", (2026, 5, 3), (2026, 5, 4), "憲法記念日");
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0], event);
    }

    #[test]
    fn parse_events_empty() {
        let cal = format_calendar(&vcal(vec![]));
        let parsed = parse_calendar(&cal).unwrap();
        assert!(parsed.events.is_empty());
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
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].unknown.len(), 1);
        assert_eq!(parsed.events[0].unknown[0].name, "X-CUSTOM-COLOR");
        assert_eq!(parsed.events[0].unknown[0].value, "blue");
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
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].unknown.len(), 1);
        assert_eq!(parsed.events[0].unknown[0].name, "X-CUSTOM-FOO");
        assert_eq!(
            parsed.events[0].unknown[0].params,
            vec![("LANG".to_string(), "en".to_string())]
        );
        assert_eq!(parsed.events[0].unknown[0].value, "hello");
    }

    #[test]
    fn unknown_x_property_preserves_source_index_order() {
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
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events[0].unknown.len(), 3);
        assert_eq!(parsed.events[0].unknown[0].source_index, 1);
        assert_eq!(parsed.events[0].unknown[0].name, "X-CUSTOM-A");
        assert_eq!(parsed.events[0].unknown[2].source_index, 3);
        assert_eq!(parsed.events[0].unknown[2].name, "X-CUSTOM-C");
    }

    #[test]
    fn typed_x_microsoft_and_x_makeholiday_stay_typed_not_in_unknown() {
        let mut event = make_event("rt-typed", (2026, 4, 29), (2026, 4, 30), "昭和の日");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Oof),
        });
        event.icon = Some("flag".to_string());
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(
            parsed.events[0]
                .microsoft
                .as_ref()
                .and_then(|m| m.busystatus),
            Some(MsBusyStatus::Oof)
        );
        assert_eq!(parsed.events[0].icon.as_deref(), Some("flag"));
        assert!(parsed.events[0].unknown.is_empty());
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
        let mut event = make_event("rt-tc", (2026, 4, 29), (2026, 4, 30), "s");
        event.class = Some(EventClass::Private);
        event.categories = vec!["work".to_string()];
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].class, Some(EventClass::Private));
        assert_eq!(parsed.events[0].categories, vec!["work".to_string()]);
        assert!(parsed.events[0].unknown.is_empty());
    }

    // ADR-001 Migration Step 2 — RawComponent + unrecognized_components.

    #[test]
    fn vtimezone_round_trips_into_calendar_unrecognized_components() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VTIMEZONE\r\n");
        input.push_str("TZID:Asia/Tokyo\r\n");
        input.push_str("BEGIN:STANDARD\r\n");
        input.push_str("DTSTART:19700101T000000\r\n");
        input.push_str("TZOFFSETFROM:+0900\r\n");
        input.push_str("TZOFFSETTO:+0900\r\n");
        input.push_str("TZNAME:JST\r\n");
        input.push_str("END:STANDARD\r\n");
        input.push_str("END:VTIMEZONE\r\n");
        input.push_str("END:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.unrecognized_components.len(), 1);
        let tz = &parsed.unrecognized_components[0];
        assert_eq!(tz.name, "VTIMEZONE");
        assert_eq!(tz.properties.len(), 1);
        assert_eq!(tz.properties[0].name, "TZID");
        assert_eq!(tz.properties[0].value, "Asia/Tokyo");
        assert_eq!(tz.sub_components.len(), 1);
        assert_eq!(tz.sub_components[0].name, "STANDARD");
        assert_eq!(tz.sub_components[0].properties.len(), 4);
    }

    #[test]
    fn valarm_round_trips_into_event_unrecognized_components() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("BEGIN:VALARM\r\n");
        input.push_str("ACTION:DISPLAY\r\n");
        input.push_str("TRIGGER:-PT15M\r\n");
        input.push_str("DESCRIPTION:reminder\r\n");
        input.push_str("END:VALARM\r\n");
        input.push_str("END:VEVENT\r\n");
        input.push_str("END:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events.len(), 1);
        let event = &parsed.events[0];
        assert_eq!(event.unrecognized_components.len(), 1);
        let alarm = &event.unrecognized_components[0];
        assert_eq!(alarm.name, "VALARM");
        assert_eq!(alarm.properties.len(), 3);
        let names: Vec<_> = alarm.properties.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["ACTION", "TRIGGER", "DESCRIPTION"]);
    }

    // ADR-001 Migration Step 3 — TRANSP typed field.

    #[test]
    fn transp_field_parses_from_input() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("TRANSP:OPAQUE\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events[0].transp, Some(crate::Transp::Opaque));
    }

    #[test]
    fn transp_field_overrides_busystatus_derived_transp_on_output() {
        // If transp is explicitly set, the formatter must honor it even
        // when microsoft.busystatus would derive a different value.
        let mut event = make_event("transp-override", (2026, 4, 29), (2026, 4, 30), "s");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Oof), // derives OPAQUE
        });
        event.transp = Some(crate::Transp::Transparent); // typed override
        let cal = format_calendar(&vcal(vec![event]));
        assert!(cal.contains("TRANSP:TRANSPARENT\r\n"));
        assert!(cal.contains("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n"));
    }

    #[test]
    fn transp_none_falls_back_to_microsoft_busystatus_derived_value() {
        let mut event = make_event("transp-fallback", (2026, 4, 29), (2026, 4, 30), "s");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Oof), // derives OPAQUE
        });
        event.transp = None;
        let cal = format_calendar(&vcal(vec![event]));
        assert!(cal.contains("TRANSP:OPAQUE\r\n"));
    }

    #[test]
    fn no_microsoft_and_no_transp_omits_both_lines() {
        let event = make_event("transp-nothing", (2026, 4, 29), (2026, 4, 30), "s");
        let cal = format_calendar(&vcal(vec![event]));
        assert!(!cal.contains("TRANSP:"));
        assert!(!cal.contains("X-MICROSOFT-CDO-BUSYSTATUS:"));
    }

    #[test]
    fn transp_round_trip_preserves_typed_value() {
        let mut event = make_event("transp-rt", (2026, 4, 29), (2026, 4, 30), "s");
        event.transp = Some(crate::Transp::Opaque);
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].transp, Some(crate::Transp::Opaque));
    }

    #[test]
    fn vtimezone_format_round_trip_yields_same_structure() {
        let cal = VCalendar {
            version: "2.0".to_string(),
            prodid: "-//mh//EN".to_string(),
            calscale: None,
            method: None,
            events: vec![],
            unrecognized_components: vec![RawComponent {
                name: "VTIMEZONE".to_string(),
                properties: vec![RawProperty {
                    name: "TZID".to_string(),
                    params: vec![],
                    value: "Asia/Tokyo".to_string(),
                    source_index: 1,
                }],
                sub_components: vec![],
            }],
        };
        let s = format_calendar(&cal);
        let reparsed = parse_calendar(&s).unwrap();
        assert_eq!(reparsed.unrecognized_components.len(), 1);
        assert_eq!(reparsed.unrecognized_components[0].name, "VTIMEZONE");
        assert_eq!(
            reparsed.unrecognized_components[0].properties[0].value,
            "Asia/Tokyo"
        );
    }
}
