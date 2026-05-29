//! Presentation layer — clap-driven CLI parsing.
//!
//! Per ADR-009, this layer parses user input and formats output. It
//! depends on `application` for use cases and on `input` for shared
//! parsing helpers, but is never depended on by either of them.

pub mod cli;

pub use cli::{Cli, Commands};
