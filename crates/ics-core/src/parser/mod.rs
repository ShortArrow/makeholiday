pub mod escape;
pub mod line;
pub mod unfold;

use crate::error::{Error, Result};
use crate::event::{EventClass, Transp, VEvent};
use crate::parser::line::parse_logical_line;
use crate::profile::microsoft::MsBusyStatus;
use crate::profile::{google, icloud, microsoft};
use crate::raw::{RawComponent, RawProperty};
use crate::vcalendar::VCalendar;
use chrono::{NaiveDate, NaiveDateTime};

/// Parse the full ICS document into a typed `VCalendar`.
///
/// The input flows through `unfold::unfold` first, which strips a leading
/// UTF-8 BOM and joins RFC 5545 folded continuation lines into logical
/// lines. Calendar-level non-`VEVENT` components (`VTIMEZONE`,
/// `VJOURNAL`, etc.) are preserved into `VCalendar.unrecognized_components`.
/// Non-typed nested components inside a `VEVENT` (e.g. `VALARM`) flow
/// into `VEvent.unrecognized_components`.
pub fn parse_calendar(content: &str) -> Result<VCalendar> {
    let logical = unfold::unfold(content);
    let lines: Vec<&str> = logical.iter().map(|s| s.trim()).collect();
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
    let mut unknown: Vec<RawProperty> = Vec::new();
    let mut ms_unrecognized: Vec<RawProperty> = Vec::new();
    let mut google_unrecognized: Vec<RawProperty> = Vec::new();
    let mut icloud_unrecognized: Vec<RawProperty> = Vec::new();
    // Monotonic across all X-* properties in this VEVENT regardless of
    // which bucket they land in — preserves source-arrival order for the
    // per-bucket sort the formatter does (ADR-018).
    let mut x_index: u32 = 0;
    let mut unrecognized_components: Vec<RawComponent> = Vec::new();

    let mut idx = start;
    while idx < lines.len() {
        let line = lines[idx];
        let line_no = (idx + 1) as u32;
        if line == "END:VEVENT" {
            let stamp =
                dtstamp.ok_or_else(|| Error::parse_at_line(line_no, "VEVENT missing DTSTAMP"))?;
            let s =
                dtstart.ok_or_else(|| Error::parse_at_line(line_no, "VEVENT missing DTSTART"))?;
            let e = dtend.ok_or_else(|| Error::parse_at_line(line_no, "VEVENT missing DTEND"))?;
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
                    microsoft: if ms_busystatus.is_some() || !ms_unrecognized.is_empty() {
                        Some(microsoft::EventExtensions {
                            busystatus: ms_busystatus,
                            unrecognized: ms_unrecognized,
                        })
                    } else {
                        None
                    },
                    google: if !google_unrecognized.is_empty() {
                        Some(google::EventExtensions {
                            unrecognized: google_unrecognized,
                        })
                    } else {
                        None
                    },
                    icloud: if !icloud_unrecognized.is_empty() {
                        Some(icloud::EventExtensions {
                            unrecognized: icloud_unrecognized,
                        })
                    } else {
                        None
                    },
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
        if let Some(ll) = parse_logical_line(line) {
            match ll.name.as_str() {
                "UID" => uid = ll.value.to_string(),
                "DTSTAMP" => {
                    dtstamp = Some(
                        NaiveDateTime::parse_from_str(ll.value, "%Y%m%dT%H%M%SZ").map_err(|e| {
                            Error::parse_at(line_no, "DTSTAMP", format!("Invalid DTSTAMP: {e}"))
                        })?,
                    );
                }
                "DTSTART" => {
                    if has_value_date_param(&ll.params) {
                        dtstart =
                            Some(NaiveDate::parse_from_str(ll.value, "%Y%m%d").map_err(|e| {
                                Error::parse_at(line_no, "DTSTART", format!("Invalid DTSTART: {e}"))
                            })?);
                    }
                    // Timed events (DTSTART;VALUE=DATE-TIME or no VALUE) currently
                    // fall through silently per ADR-001 Rule 9; v0.3.0 lifts this.
                }
                "DTEND" => {
                    if has_value_date_param(&ll.params) {
                        dtend =
                            Some(NaiveDate::parse_from_str(ll.value, "%Y%m%d").map_err(|e| {
                                Error::parse_at(line_no, "DTEND", format!("Invalid DTEND: {e}"))
                            })?);
                    }
                }
                "SUMMARY" => summary = escape::decode_text(ll.value),
                "TRANSP" => transp = Transp::from_ics(ll.value),
                "X-MICROSOFT-CDO-BUSYSTATUS" => {
                    if let Some(bs) = MsBusyStatus::from_cdo(ll.value) {
                        ms_busystatus = Some(bs);
                    }
                }
                "CLASS" => class = EventClass::from_ics(ll.value),
                "CATEGORIES" => {
                    categories = escape::split_text_list(ll.value)
                        .into_iter()
                        .map(|s| s.trim().to_string())
                        .collect();
                }
                name if name.starts_with("X-") => {
                    x_index += 1;
                    let prop = ll.to_raw_property(x_index);
                    if microsoft::owns_property(&prop.name) {
                        ms_unrecognized.push(prop);
                    } else if google::owns_property(&prop.name) {
                        google_unrecognized.push(prop);
                    } else if icloud::owns_property(&prop.name) {
                        icloud_unrecognized.push(prop);
                    } else {
                        unknown.push(prop);
                    }
                }
                _ => {
                    // Unknown non-X property — ignored for now. Future work:
                    // promote to VEvent.unknown for full round-trip preservation.
                }
            }
        }
        idx += 1;
    }
    Err(Error::parse("VEVENT missing END:VEVENT"))
}

/// True if the parameter list contains `VALUE=DATE` (date-only typing).
fn has_value_date_param(params: &[(String, String)]) -> bool {
    params.iter().any(|(k, v)| k == "VALUE" && v == "DATE")
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
///
/// Thin wrapper around `line::parse_logical_line` + `LogicalLine::to_raw_property`
/// kept for the unrecognized-component path and for existing tests. Quoted
/// parameter values have their surrounding `"` stripped; TEXT-value
/// escapes are left intact per ADR-018 (raw value preservation).
pub(crate) fn parse_raw_property(line: &str, source_index: u32) -> Option<RawProperty> {
    parse_logical_line(line).map(|ll| ll.to_raw_property(source_index))
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
            unrecognized: vec![],
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
    fn x_microsoft_stays_typed_x_makeholiday_lands_in_unknown() {
        // Post-Step-5: X-MAKEHOLIDAY-ICON is no longer specially handled
        // in ics-core; it round-trips through VEvent.unknown like any
        // other X-* property. Read/write is the makeholiday crate's job.
        let mut event = make_event("rt-typed", (2026, 4, 29), (2026, 4, 30), "昭和の日");
        event.microsoft = Some(MsExtensions {
            busystatus: Some(MsBusyStatus::Oof),
            unrecognized: vec![],
        });
        event.unknown.push(RawProperty {
            name: "X-MAKEHOLIDAY-ICON".to_string(),
            params: vec![],
            value: "flag".to_string(),
            source_index: 1,
        });
        let cal = format_calendar(&vcal(vec![event.clone()]));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(
            parsed.events[0]
                .microsoft
                .as_ref()
                .and_then(|m| m.busystatus),
            Some(MsBusyStatus::Oof)
        );
        let icon = parsed.events[0]
            .unknown
            .iter()
            .find(|p| p.name == "X-MAKEHOLIDAY-ICON")
            .map(|p| p.value.as_str());
        assert_eq!(icon, Some("flag"));
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
            busystatus: Some(MsBusyStatus::Oof),
            unrecognized: vec![], // derives OPAQUE
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
            busystatus: Some(MsBusyStatus::Oof),
            unrecognized: vec![], // derives OPAQUE
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

    // ADR-001 Migration Step 6 — per-vendor unrecognized fallback.

    #[test]
    fn x_microsoft_prefix_routes_to_microsoft_unrecognized_not_unknown() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-MICROSOFT-CDO-ALLDAYEVENT:TRUE\r\n");
        input.push_str("X-MICROSOFT-IMPORTANCE:1\r\n");
        input.push_str("X-CUSTOM-COLOR:blue\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let event = &parsed.events[0];

        // Microsoft prefix properties land in microsoft.unrecognized.
        let ms = event.microsoft.as_ref().unwrap();
        assert_eq!(ms.busystatus, None); // typed slot still empty
        assert_eq!(ms.unrecognized.len(), 2);
        let ms_names: Vec<_> = ms.unrecognized.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            ms_names,
            vec!["X-MICROSOFT-CDO-ALLDAYEVENT", "X-MICROSOFT-IMPORTANCE"]
        );

        // Non-Microsoft X-* stays in VEvent.unknown.
        assert_eq!(event.unknown.len(), 1);
        assert_eq!(event.unknown[0].name, "X-CUSTOM-COLOR");
    }

    #[test]
    fn x_microsoft_cdo_busystatus_still_promotes_to_typed_field_not_unrecognized() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-MICROSOFT-CDO-BUSYSTATUS:OOF\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let ms = parsed.events[0].microsoft.as_ref().unwrap();
        assert_eq!(ms.busystatus, Some(MsBusyStatus::Oof));
        assert!(ms.unrecognized.is_empty());
    }

    #[test]
    fn microsoft_unrecognized_round_trips_through_format() {
        let mut event = make_event("rt-ms-unrec", (2026, 4, 29), (2026, 4, 30), "s");
        event.microsoft = Some(MsExtensions {
            busystatus: None,
            unrecognized: vec![RawProperty {
                name: "X-MICROSOFT-CDO-ALLDAYEVENT".to_string(),
                params: vec![],
                value: "TRUE".to_string(),
                source_index: 1,
            }],
        });
        let cal = format_calendar(&vcal(vec![event.clone()]));
        assert!(cal.contains("X-MICROSOFT-CDO-ALLDAYEVENT:TRUE\r\n"));
        let parsed = parse_calendar(&cal).unwrap();
        let ms = parsed.events[0].microsoft.as_ref().unwrap();
        assert_eq!(ms.unrecognized.len(), 1);
        assert_eq!(ms.unrecognized[0].name, "X-MICROSOFT-CDO-ALLDAYEVENT");
        assert_eq!(ms.unrecognized[0].value, "TRUE");
    }

    #[test]
    fn source_index_is_monotonic_across_buckets() {
        // Input order A, MS, B should yield X-CUSTOM-A at index 1,
        // X-MICROSOFT-FOO at index 2, X-CUSTOM-B at index 3.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-CUSTOM-A:1\r\n");
        input.push_str("X-MICROSOFT-FOO:2\r\n");
        input.push_str("X-CUSTOM-B:3\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let event = &parsed.events[0];
        assert_eq!(event.unknown[0].name, "X-CUSTOM-A");
        assert_eq!(event.unknown[0].source_index, 1);
        assert_eq!(event.unknown[1].name, "X-CUSTOM-B");
        assert_eq!(event.unknown[1].source_index, 3);
        let ms = event.microsoft.as_ref().unwrap();
        assert_eq!(ms.unrecognized[0].name, "X-MICROSOFT-FOO");
        assert_eq!(ms.unrecognized[0].source_index, 2);
    }

    #[test]
    fn empty_microsoft_bundle_stays_none() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-CUSTOM-A:1\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        // No X-MICROSOFT-* input means microsoft bundle is None entirely.
        assert!(parsed.events[0].microsoft.is_none());
    }

    // ADR-001 Migration Step 7 — google / icloud skeleton routing.

    #[test]
    fn x_google_prefix_routes_to_google_unrecognized() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-GOOGLE-CONFERENCEPROPERTIES:foo\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let g = parsed.events[0].google.as_ref().unwrap();
        assert_eq!(g.unrecognized.len(), 1);
        assert_eq!(g.unrecognized[0].name, "X-GOOGLE-CONFERENCEPROPERTIES");
        assert!(parsed.events[0].microsoft.is_none());
        assert!(parsed.events[0].icloud.is_none());
        assert!(parsed.events[0].unknown.is_empty());
    }

    #[test]
    fn x_apple_and_x_calendarserver_prefixes_route_to_icloud_unrecognized() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-APPLE-CALENDAR-COLOR:#FF0000\r\n");
        input.push_str("X-CALENDARSERVER-ACCESS:CONFIDENTIAL\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let ic = parsed.events[0].icloud.as_ref().unwrap();
        assert_eq!(ic.unrecognized.len(), 2);
        let names: Vec<_> = ic.unrecognized.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["X-APPLE-CALENDAR-COLOR", "X-CALENDARSERVER-ACCESS"]
        );
        assert!(parsed.events[0].google.is_none());
        assert!(parsed.events[0].unknown.is_empty());
    }

    #[test]
    fn all_three_vendor_buckets_round_trip_together() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str("X-MICROSOFT-CDO-ALLDAYEVENT:TRUE\r\n");
        input.push_str("X-GOOGLE-X:1\r\n");
        input.push_str("X-APPLE-Y:2\r\n");
        input.push_str("X-CUSTOM-Z:3\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let event = &parsed.events[0];
        assert_eq!(event.microsoft.as_ref().unwrap().unrecognized.len(), 1);
        assert_eq!(event.google.as_ref().unwrap().unrecognized.len(), 1);
        assert_eq!(event.icloud.as_ref().unwrap().unrecognized.len(), 1);
        assert_eq!(event.unknown.len(), 1);

        let cal = format_calendar(&vcal(vec![event.clone()]));
        let reparsed = parse_calendar(&cal).unwrap();
        assert_eq!(reparsed.events[0], *event);
    }

    #[test]
    fn vendor_bundles_stay_none_when_no_matching_prefix_seen() {
        let event = make_event("rt-none", (2026, 4, 29), (2026, 4, 30), "s");
        let cal = format_calendar(&vcal(vec![event]));
        let parsed = parse_calendar(&cal).unwrap();
        assert!(parsed.events[0].microsoft.is_none());
        assert!(parsed.events[0].google.is_none());
        assert!(parsed.events[0].icloud.is_none());
    }

    // ADR-019 Step 0 — folding + BOM acceptance at the parse_calendar boundary.

    #[test]
    fn parse_calendar_accepts_leading_utf8_bom() {
        // Outlook etc. emit a UTF-8 BOM. parse_calendar must tolerate it.
        let mut input =
            String::from("\u{FEFF}BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.version, "2.0");
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].summary, "s");
    }

    #[test]
    fn parse_calendar_reassembles_folded_summary() {
        // A long SUMMARY split across multiple physical lines per RFC 5545 §3.1.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:This is a very long event title that has been\r\n");
        input.push_str(" folded across multiple physical lines per RFC 5545\r\n");
        input.push_str(" section 3.1 line folding rules.\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(
            parsed.events[0].summary,
            "This is a very long event title that has been\
             folded across multiple physical lines per RFC 5545\
             section 3.1 line folding rules."
        );
    }

    #[test]
    fn parse_calendar_handles_tab_continuation_too() {
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:long\r\n\tvalue\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events[0].summary, "longvalue");
    }

    #[test]
    fn parse_calendar_accepts_lf_only_line_terminators() {
        // Some tools emit Unix line endings. The unfolder accepts both.
        let mut input = String::from("BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//mh//EN\n");
        input.push_str("BEGIN:VEVENT\n");
        input.push_str("UID:e1\nDTSTAMP:20260101T000000Z\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\nDTEND;VALUE=DATE:20260430\n");
        input.push_str("SUMMARY:s\nEND:VEVENT\nEND:VCALENDAR\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].summary, "s");
    }

    #[test]
    fn parse_calendar_preserves_japanese_utf8_across_fold() {
        // Multi-byte UTF-8 split across a fold boundary must reassemble
        // correctly. The boundary lands between bytes, not between chars,
        // but since the folding-marker whitespace is single-byte ASCII
        // and we drop only that single byte, surrounding multi-byte
        // sequences survive intact.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:憲法\r\n 記念日\r\n");
        input.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events[0].summary, "憲法記念日");
    }

    // ADR-019 Step 1 — LogicalLine dispatch + parse error line numbers.

    #[test]
    fn invalid_dtstamp_error_message_carries_line_number() {
        // The bogus DTSTAMP is on logical line 6 (post-unfold, 1-based).
        let mut input = String::from("BEGIN:VCALENDAR\r\n"); // line 1
        input.push_str("VERSION:2.0\r\n"); // line 2
        input.push_str("PRODID:-//mh//EN\r\n"); // line 3
        input.push_str("BEGIN:VEVENT\r\n"); // line 4
        input.push_str("UID:e1\r\n"); // line 5
        input.push_str("DTSTAMP:NOT-A-DATE\r\n"); // line 6 — error here
        input.push_str("DTSTART;VALUE=DATE:20260429\r\n");
        input.push_str("DTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let err = parse_calendar(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("at line 6"),
            "expected 'at line 6' in error: {msg}"
        );
        assert!(
            msg.contains("DTSTAMP"),
            "expected DTSTAMP property name in error: {msg}"
        );
    }

    #[test]
    fn missing_required_field_error_carries_end_vevent_line() {
        // VEVENT body has no DTSTAMP; the END:VEVENT line is where we
        // discover the missing required field.
        let mut input = String::from("BEGIN:VCALENDAR\r\n"); // line 1
        input.push_str("VERSION:2.0\r\n"); // line 2
        input.push_str("PRODID:-//mh//EN\r\n"); // line 3
        input.push_str("BEGIN:VEVENT\r\n"); // line 4
        input.push_str("UID:e1\r\n"); // line 5
        input.push_str("DTSTART;VALUE=DATE:20260429\r\n"); // line 6
        input.push_str("DTEND;VALUE=DATE:20260430\r\n"); // line 7
        input.push_str("SUMMARY:s\r\n"); // line 8
        input.push_str("END:VEVENT\r\n"); // line 9 — END:VEVENT
        input.push_str("END:VCALENDAR\r\n");
        let err = parse_calendar(&input).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("at line 9"), "expected 'at line 9': {msg}");
        assert!(msg.contains("missing DTSTAMP"));
    }

    #[test]
    fn dispatch_handles_property_with_extra_params() {
        // UID;X-FOO=bar:abc-123 must still set uid; the old strip_prefix
        // dispatcher would have missed this because of the inline param.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID;X-FOO=bar:event-uid-with-param\r\n");
        input.push_str("DTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(parsed.events[0].uid, "event-uid-with-param");
    }

    #[test]
    fn dispatch_handles_value_date_param_in_any_position() {
        // DTSTART;TZID=Asia/Tokyo;VALUE=DATE:20260429 — VALUE=DATE is the
        // second param, not the first. With LogicalLine the param scan is
        // order-independent.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;TZID=Asia/Tokyo;VALUE=DATE:20260429\r\n");
        input.push_str("DTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        assert_eq!(
            parsed.events[0].dtstart,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 29).unwrap()
        );
    }

    // ADR-019 Step 2 — TEXT escape decode/encode applied to typed fields.

    #[test]
    fn summary_with_comma_round_trips_via_escape() {
        let mut event = make_event("rt-esc-comma", (2026, 4, 29), (2026, 4, 30), "");
        event.summary = "Lunch, dinner, snack".to_string();
        let cal = format_calendar(&vcal(vec![event.clone()]));
        // Wire form has escaped commas.
        assert!(cal.contains(r"SUMMARY:Lunch\, dinner\, snack"));
        let parsed = parse_calendar(&cal).unwrap();
        // Parsed summary has them decoded back.
        assert_eq!(parsed.events[0].summary, "Lunch, dinner, snack");
    }

    #[test]
    fn summary_with_semicolon_round_trips_via_escape() {
        let mut event = make_event("rt-esc-semi", (2026, 4, 29), (2026, 4, 30), "");
        event.summary = "Q1; Q2".to_string();
        let cal = format_calendar(&vcal(vec![event.clone()]));
        assert!(cal.contains(r"SUMMARY:Q1\; Q2"));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].summary, "Q1; Q2");
    }

    #[test]
    fn summary_with_newline_round_trips_via_escape() {
        let mut event = make_event("rt-esc-nl", (2026, 4, 29), (2026, 4, 30), "");
        event.summary = "Line1\nLine2".to_string();
        let cal = format_calendar(&vcal(vec![event.clone()]));
        assert!(cal.contains(r"SUMMARY:Line1\nLine2"));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].summary, "Line1\nLine2");
    }

    #[test]
    fn summary_with_backslash_round_trips_via_escape() {
        let mut event = make_event("rt-esc-bs", (2026, 4, 29), (2026, 4, 30), "");
        event.summary = r"path\to\file".to_string();
        let cal = format_calendar(&vcal(vec![event.clone()]));
        assert!(cal.contains(r"SUMMARY:path\\to\\file"));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].summary, r"path\to\file");
    }

    #[test]
    fn categories_with_commas_in_items_round_trip() {
        // An item with a literal comma must survive split + decode.
        let mut event = make_event("rt-cat-comma", (2026, 4, 29), (2026, 4, 30), "x");
        event.categories = vec!["work, project A".to_string(), "personal".to_string()];
        let cal = format_calendar(&vcal(vec![event.clone()]));
        assert!(cal.contains(r"CATEGORIES:work\, project A,personal"));
        let parsed = parse_calendar(&cal).unwrap();
        assert_eq!(parsed.events[0].categories.len(), 2);
        assert_eq!(parsed.events[0].categories[0], "work, project A");
        assert_eq!(parsed.events[0].categories[1], "personal");
    }

    #[test]
    fn raw_property_value_is_not_escape_decoded() {
        // RawProperty.value stays raw per ADR-018 — escape interpretation
        // only applies to typed TEXT fields. An X- property's value
        // preserves the backslash.
        let mut input = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mh//EN\r\n");
        input.push_str("BEGIN:VEVENT\r\n");
        input.push_str("UID:e1\r\nDTSTAMP:20260101T000000Z\r\n");
        input.push_str("DTSTART;VALUE=DATE:20260429\r\nDTEND;VALUE=DATE:20260430\r\n");
        input.push_str("SUMMARY:s\r\n");
        input.push_str(r"X-CUSTOM-FOO:value with \,comma");
        input.push_str("\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let parsed = parse_calendar(&input).unwrap();
        let rp = &parsed.events[0].unknown[0];
        assert_eq!(rp.name, "X-CUSTOM-FOO");
        assert_eq!(rp.value, r"value with \,comma"); // raw, not decoded
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
