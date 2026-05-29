//! Typed parser / formatter errors for `ics-core`.
//!
//! The `Parse` variant carries optional `line` and `property` fields that
//! the current flat parser leaves as `None`. ADR-019's lexer-based parser
//! populates them with the 1-based logical line number and the offending
//! property name.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("parse error: {message}")]
    Parse {
        message: String,
        /// 1-based logical line number. `None` until ADR-019 lexer-based parser lands.
        line: Option<u32>,
        /// Offending property name when identifiable.
        property: Option<String>,
    },
}

impl Error {
    /// Construct a `Parse` error from a plain message. `line` and
    /// `property` default to `None`; the ADR-019 parser will use the full
    /// struct form to attach context.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            line: None,
            property: None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
