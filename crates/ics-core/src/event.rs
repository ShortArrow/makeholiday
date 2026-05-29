use crate::raw::RawProperty;
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
    // ADR-001 Migration: X-MAKEHOLIDAY-ICON moves out of VEvent into a
    // makeholiday-side reader on VEvent.unknown. Kept here through Step 3
    // so use sites don't break before the typed-extension restructure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Properties matching no registered vendor prefix (ADR-001 rule 4).
    /// Includes both the property name and its parameters; round-tripped
    /// per ADR-018 in `source_index` order at the tail of the component.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unknown: Vec<RawProperty>,
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
