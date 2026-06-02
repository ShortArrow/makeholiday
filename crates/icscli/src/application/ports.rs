//! Application ports — abstract interfaces use cases depend on.
//!
//! Per ADR-009 / ADR-011, the application layer programs against trait
//! contracts; concrete I/O implementations live in `infrastructure`.

use ics_core::VCalendar;

use crate::error::Result;

/// Read / write contract for a single calendar store (e.g. a file).
///
/// Path / identity is captured by the implementation; methods take no
/// path argument. Use cases that need a different store create a
/// different repository instance.
pub trait CalendarRepository {
    /// Create a new empty calendar at the store. Fails with
    /// `IcsError::AlreadyExists` if the store already has a calendar.
    fn create(&self) -> Result<()>;

    /// Load and parse the stored calendar into a typed `VCalendar`.
    ///
    /// Parse errors surface as `IcsError::Parse` (wrapping `ics_core::Error`).
    fn load(&self) -> Result<VCalendar>;

    /// Atomically replace the stored calendar with `calendar`.
    ///
    /// Implementations are responsible for formatting `calendar` to the
    /// underlying wire form and writing it atomically.
    fn save(&self, calendar: &VCalendar) -> Result<()>;

    /// True if the underlying store has a calendar to load.
    fn exists(&self) -> bool;
}
