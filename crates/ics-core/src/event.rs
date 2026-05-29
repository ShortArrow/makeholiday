use crate::profile::microsoft;
use crate::raw::{RawComponent, RawProperty};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

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

/// RFC 5545 §3.8.2.7 `TRANSP` — time-transparency for free/busy lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transp {
    Transparent,
    Opaque,
}

impl Transp {
    pub fn ics_value(self) -> &'static str {
        match self {
            Transp::Transparent => "TRANSPARENT",
            Transp::Opaque => "OPAQUE",
        }
    }

    pub fn from_ics(s: &str) -> Option<Self> {
        match s {
            "TRANSPARENT" => Some(Transp::Transparent),
            "OPAQUE" => Some(Transp::Opaque),
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
    /// RFC 5545 `TRANSP`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transp: Option<Transp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<EventClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    // ADR-001 Migration: X-MAKEHOLIDAY-ICON moves out of VEvent into a
    // makeholiday-side reader on VEvent.unknown in Step 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Microsoft / Outlook event extension bundle. `X-MICROSOFT-CDO-BUSYSTATUS`
    /// lives in `microsoft.busystatus` after ADR-001 Migration Step 4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microsoft: Option<microsoft::EventExtensions>,

    /// Properties matching no registered vendor prefix (ADR-001 rule 4).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unknown: Vec<RawProperty>,

    /// Nested components the typed model does not understand (e.g.
    /// `VALARM`). Preserved verbatim for ADR-001 / ADR-018 round-trip.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecognized_components: Vec<RawComponent>,
}

fn serialize_date<S: serde::Serializer>(date: &NaiveDate, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.format("%Y-%m-%d").to_string())
}

fn serialize_dtstamp<S: serde::Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
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
    use crate::test_helpers::make_event;

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

    #[test]
    fn format_event_line_with_icon() {
        let mut event = make_event("x", (2026, 6, 15), (2026, 6, 16), "出張");
        event.icon = Some("airplane".to_string());
        assert_eq!(format_event_line(&event), "2026-06-15 : 出張 [airplane]");
    }
}
