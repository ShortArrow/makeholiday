//! Microsoft / Outlook profile bundle (`X-MICROSOFT-CDO-*`, `X-MICROSOFT-*`).
//!
//! Houses the typed `MsBusyStatus` (RFC `TRANSP` has 2 values; Microsoft's
//! busy state has 5 — they are not interchangeable, which is why this
//! sits in a vendor bundle, not on `VEvent` directly).

use serde::Serialize;

use crate::raw::RawProperty;

/// Property name prefixes owned by this profile. Longest match wins per
/// ADR-001 rule 3.
pub const PREFIXES: &[&str] = &["X-MICROSOFT-CDO-", "X-MICROSOFT-"];

/// True if `name` starts with any of this profile's registered prefixes.
pub fn owns_property(name: &str) -> bool {
    PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Microsoft's `X-MICROSOFT-CDO-BUSYSTATUS` value space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsBusyStatus {
    Free,
    Tentative,
    Busy,
    Oof,
    #[serde(rename = "working")]
    WorkingElsewhere,
}

impl MsBusyStatus {
    /// Derive the matching RFC 5545 `TRANSP` value. Used as a fallback
    /// by the formatter when `VEvent.transp` is `None`.
    pub fn transp(self) -> &'static str {
        match self {
            MsBusyStatus::Free => "TRANSPARENT",
            _ => "OPAQUE",
        }
    }

    pub fn cdo_value(self) -> &'static str {
        match self {
            MsBusyStatus::Free => "FREE",
            MsBusyStatus::Tentative => "TENTATIVE",
            MsBusyStatus::Busy => "BUSY",
            MsBusyStatus::Oof => "OOF",
            MsBusyStatus::WorkingElsewhere => "WORKINGELSEWHERE",
        }
    }

    pub fn from_cdo(s: &str) -> Option<Self> {
        match s {
            "FREE" => Some(MsBusyStatus::Free),
            "TENTATIVE" => Some(MsBusyStatus::Tentative),
            "BUSY" => Some(MsBusyStatus::Busy),
            "OOF" => Some(MsBusyStatus::Oof),
            "WORKINGELSEWHERE" => Some(MsBusyStatus::WorkingElsewhere),
            _ => None,
        }
    }
}

/// Microsoft event-level extension bundle (ADR-001 Option B).
///
/// `unrecognized` holds `X-MICROSOFT-*` properties whose specific name is
/// not yet typed. Promoting one of them into a typed field (in a future
/// step) is intra-bundle and never changes the `VEvent.unknown` slot.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct EventExtensions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub busystatus: Option<MsBusyStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecognized: Vec<RawProperty>,
}
