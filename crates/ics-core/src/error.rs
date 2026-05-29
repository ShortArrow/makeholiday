//! Typed parser / formatter errors for `ics-core`.
//!
//! The `Parse` variant carries optional `line` and `property` fields.
//! `line` is the 1-based logical line number within the input document
//! (post-unfold). `property` is the offending property name when
//! identifiable.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Parse {
        message: String,
        line: Option<u32>,
        property: Option<String>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                message,
                line,
                property,
            } => {
                f.write_str("parse error")?;
                if let Some(l) = line {
                    write!(f, " at line {l}")?;
                }
                if let Some(p) = property {
                    write!(f, " [{p}]")?;
                }
                write!(f, ": {message}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Construct a `Parse` error from a plain message with no line / property
    /// context. Use `parse_at_line` when a logical line number is available.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            line: None,
            property: None,
        }
    }

    /// Construct a `Parse` error attached to a specific 1-based logical
    /// line number.
    pub fn parse_at_line(line: u32, message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            line: Some(line),
            property: None,
        }
    }

    /// Like `parse_at_line` but also records the offending property name.
    pub fn parse_at(line: u32, property: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            line: Some(line),
            property: Some(property.into()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_without_line_or_property() {
        let err = Error::parse("VEVENT missing DTSTAMP");
        assert_eq!(err.to_string(), "parse error: VEVENT missing DTSTAMP");
    }

    #[test]
    fn display_with_line_only() {
        let err = Error::parse_at_line(42, "VEVENT missing DTSTAMP");
        assert_eq!(
            err.to_string(),
            "parse error at line 42: VEVENT missing DTSTAMP"
        );
    }

    #[test]
    fn display_with_line_and_property() {
        let err = Error::parse_at(42, "DTSTAMP", "Invalid value");
        assert_eq!(
            err.to_string(),
            "parse error at line 42 [DTSTAMP]: Invalid value"
        );
    }
}
