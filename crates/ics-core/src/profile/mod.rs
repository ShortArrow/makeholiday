//! Per-vendor profile bundles per ADR-001.
//!
//! Each vendor module owns its `PREFIXES` const, its `EventExtensions`
//! and (in later steps) `CalendarExtensions` types, plus `owns_property`
//! and parse/format helpers. Properties that match a vendor's prefix but
//! aren't yet typed land in the bundle's `unrecognized` slot — promoting
//! them to typed fields later is intra-bundle and never crosses through
//! `VEvent.unknown`.

pub mod google;
pub mod icloud;
pub mod microsoft;

/// Shared prefix-match helper. A property name is owned by a profile if
/// it starts with any of the profile's registered prefixes.
pub(crate) fn matches_prefixes(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| name.starts_with(p))
}
