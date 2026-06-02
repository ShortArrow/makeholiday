//! `lazyics` — `lazygit`-inspired terminal UI for iCalendar (RFC 5545) files.
//!
//! Library surface for the binary at `src/main.rs`. Per ADR-025, lazyics
//! reuses `icscli`'s `application::use_cases` so the TUI cannot drift away
//! from the CLI behaviorally. `ratatui` is contained to the presentation
//! layer per the same ADR.

pub mod application;
pub mod error;
pub mod infrastructure;
pub mod presentation;
