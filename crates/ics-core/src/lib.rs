//! Typed iCalendar (RFC 5545) model with vendor-extension support.
//!
//! Scope today (ADR-017 Migration Step 3): minimal `VEvent` typed model,
//! flat parser, formatter, and helpers. The vendor-extension model from
//! ADR-001 (per-vendor profile bundles + `RawProperty` fallback) and the
//! typed `Error` from ADR-019 land in subsequent migration steps.

pub mod calendar;
pub mod error;
pub mod event;
pub mod parser;
pub mod profile;
pub mod query;
pub mod raw;
pub mod vcalendar;

pub use calendar::{format_calendar, format_vevent};
pub use error::{Error, Result};
pub use event::{EventClass, Transp, VEvent};
pub use parser::{parse_calendar, parse_indices};
pub use profile::{google, icloud, microsoft};
pub use query::{SortKey, remove_event_by_summary, remove_events_by_indices, sort_events};
pub use raw::{RawComponent, RawProperty};
pub use vcalendar::VCalendar;

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::event::VEvent;
    use chrono::{NaiveDate, NaiveDateTime};

    pub(crate) fn test_dtstamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 27)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    pub(crate) fn make_event(
        uid: &str,
        start: (i32, u32, u32),
        end: (i32, u32, u32),
        summary: &str,
    ) -> VEvent {
        VEvent {
            uid: uid.to_string(),
            dtstamp: test_dtstamp(),
            dtstart: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            dtend: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            summary: summary.to_string(),
            transp: None,
            class: None,
            categories: vec![],
            microsoft: None,
            google: None,
            icloud: None,
            unknown: vec![],
            unrecognized_components: vec![],
        }
    }
}
