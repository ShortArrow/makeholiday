//! `MhError`: the makeholiday application-layer error type.
//!
//! Per ADR-012 and ADR-017's error type relationship. `ics_core::Error`
//! wraps into `MhError::Parse` via `#[from]`. `std::io::Error` is wrapped
//! by hand because the I/O variant carries the offending file path
//! alongside the underlying error.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MhError {
    #[error("I/O error on {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    Parse(#[from] ics_core::Error),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("conflicting arguments: {0}")]
    Conflict(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("file already exists: {}", path.display())]
    AlreadyExists { path: PathBuf },
}

impl MhError {
    /// Wrap an `io::Error` with the offending path.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Construct an `AlreadyExists` from the conflicting path.
    pub fn already_exists(path: impl Into<PathBuf>) -> Self {
        Self::AlreadyExists { path: path.into() }
    }
}

pub type Result<T> = std::result::Result<T, MhError>;
