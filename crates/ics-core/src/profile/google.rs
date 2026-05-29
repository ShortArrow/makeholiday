//! Google / Calendar profile bundle (`X-GOOGLE-*`).
//!
//! Skeleton for ADR-001 Migration Step 7: prefix registration and an
//! empty `EventExtensions` whose `unrecognized` slot captures everything
//! the parser routes here. Typed fields land in subsequent steps when a
//! concrete use case demands them.

use serde::Serialize;

use crate::raw::RawProperty;

/// Property name prefixes owned by this profile. Longest match wins per
/// ADR-001 rule 3.
pub const PREFIXES: &[&str] = &["X-GOOGLE-"];

/// True if `name` starts with any of this profile's registered prefixes.
pub fn owns_property(name: &str) -> bool {
    super::matches_prefixes(name, PREFIXES)
}

/// Google event-level extension bundle (ADR-001 Option B).
///
/// No typed fields yet — every prefix-matched property lands in
/// `unrecognized` until a typed field is introduced.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct EventExtensions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecognized: Vec<RawProperty>,
}
