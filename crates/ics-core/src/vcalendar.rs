//! `VCalendar` — the typed top-level container.
//!
//! Today (ADR-001 Migration Step 2) the typed surface covers RFC required
//! and common fields plus the `events: Vec<VEvent>` collection.
//! Calendar-level vendor extension bundles (`microsoft`, `google`,
//! `icloud`) land in Steps 4 onward. `X-WR-*` and `unknown` at the
//! calendar level land alongside the round-trip strategy refinements
//! (ADR-018).
//!
//! `unrecognized_components` captures `VTIMEZONE`, `VJOURNAL`, etc. for
//! round-trip preservation.

use serde::Serialize;

use crate::event::VEvent;
use crate::raw::RawComponent;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VCalendar {
    /// Required RFC field. Typically `"2.0"`.
    pub version: String,

    /// Required RFC field. Producer ID, e.g. `-//makeholiday//EN`.
    pub prodid: String,

    /// Optional RFC field. Calendar scale; almost always `GREGORIAN`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calscale: Option<String>,

    /// Optional RFC field. Method (e.g. `PUBLISH`, `REQUEST`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Typed events in this calendar.
    pub events: Vec<VEvent>,

    /// Calendar-level components the typed model does not understand
    /// (e.g. `VTIMEZONE`, `VJOURNAL`). Preserved verbatim per ADR-018.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecognized_components: Vec<RawComponent>,
}

impl VCalendar {
    /// Construct an empty calendar with the given producer ID and
    /// `VERSION:2.0`.
    pub fn new(prodid: impl Into<String>) -> Self {
        Self {
            version: "2.0".to_string(),
            prodid: prodid.into(),
            calscale: None,
            method: None,
            events: Vec::new(),
            unrecognized_components: Vec::new(),
        }
    }
}
