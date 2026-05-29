//! Raw property and component preservation types for ADR-001 round-trip.
//!
//! `RawProperty` stores prefix-unmatched `X-*` properties (and, after later
//! migration steps, vendor-prefix-matched but not-yet-typed properties).
//! Values are kept verbatim — no escape decoding — because we don't know
//! what value type rules apply to an unknown property.
//!
//! `source_index` is the monotonic input order; ADR-018 specifies that
//! the formatter emits `unknown` properties at the end of their component,
//! sorted by `source_index`, so the round-trip preserves the relative
//! ordering of unknowns even if their absolute position drifts past the
//! canonical-order typed fields.

use serde::Serialize;

/// A component (`BEGIN:NAME ... END:NAME`) that the typed model does not
/// understand, preserved verbatim for ADR-001 / ADR-018 round-trip.
///
/// Examples: `VTIMEZONE` at the calendar level, `VALARM` nested inside a
/// `VEVENT`. Nested unknown components are stored recursively in
/// `sub_components`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawComponent {
    /// Component name, UPPERCASE-normalized (e.g. `VTIMEZONE`, `VALARM`).
    pub name: String,

    /// All properties of this component as `RawProperty` instances.
    pub properties: Vec<RawProperty>,

    /// Nested unknown sub-components (e.g. `STANDARD` / `DAYLIGHT` inside
    /// a `VTIMEZONE`).
    pub sub_components: Vec<RawComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawProperty {
    /// Property name, normalized to UPPERCASE.
    pub name: String,

    /// Parameter list as `(KEY, value)` pairs. Keys are UPPERCASE-normalized;
    /// values keep their original casing. Order is preserved from the input.
    pub params: Vec<(String, String)>,

    /// Raw property value, escapes intact.
    pub value: String,

    /// 1-based monotonic input order within the enclosing component. ADR-018
    /// uses this for canonical output ordering.
    pub source_index: u32,
}
