//! makeholiday — ICS calendar file manager.
//!
//! Library surface for the binary at `src/main.rs`. Per ADR-010 / ADR-017,
//! the bin is a thin Composition Root that wires this library together.

pub mod cli;
pub mod commands;
pub mod error;
pub mod icons;
