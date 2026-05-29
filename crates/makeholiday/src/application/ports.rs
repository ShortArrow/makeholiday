//! Application ports — abstract interfaces use cases depend on.
//!
//! Per ADR-009 / ADR-011, the application layer programs against trait
//! contracts; concrete I/O implementations live in `infrastructure`.

use crate::error::Result;

/// Read / write contract for a single calendar store (e.g. a file).
///
/// Path / identity is captured by the implementation; methods take no
/// path argument. Use cases that need a different store create a
/// different repository instance.
pub trait CalendarRepository {
    /// Create a new empty calendar at the store. Fails with
    /// `MhError::AlreadyExists` if the store already has a calendar.
    fn create(&self) -> Result<()>;

    /// Load the calendar's raw ICS content.
    ///
    /// Returns the on-wire string. ADR-001 Migration will lift this to
    /// `ics_core::VCalendar` once the typed top-level shell exists.
    fn load(&self) -> Result<String>;

    /// Atomically replace the stored calendar with `content`.
    fn save(&self, content: &str) -> Result<()>;

    /// True if the underlying store has a calendar to load.
    fn exists(&self) -> bool;
}
