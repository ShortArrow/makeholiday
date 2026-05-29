//! Per-vendor profile bundles per ADR-001.
//!
//! Each vendor module owns its `PREFIXES` const, its `EventExtensions`
//! and (in later steps) `CalendarExtensions` types, plus parse/format
//! helpers that route prefix-matched-but-not-yet-typed properties into
//! the bundle's `unrecognized` slot (added in Migration Step 6).
//!
//! Today only `microsoft` is populated. `google` and `icloud` skeletons
//! land in Step 7.

pub mod microsoft;
