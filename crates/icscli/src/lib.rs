//! icscli — general-purpose iCalendar (RFC 5545) CLI.
//!
//! Library surface for the binary at `src/main.rs`. Per ADR-010 / ADR-017,
//! the bin is a thin Composition Root that wires this library together.

pub mod application;
pub mod display;
pub mod error;
pub mod icons;
pub mod infrastructure;
pub mod input;
pub mod presentation;
