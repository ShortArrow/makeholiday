//! `LazyicsError`: the lazyics application-layer error type.
//!
//! Wraps `icscli::IcsError` (the CLI's use-case error) and `std::io::Error`
//! (terminal / file ops). Exit code mapping per ADR-025 §"Output and exit
//! codes": 1 = terminal/I/O, 2 = parse failure.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LazyicsError {
    #[error("terminal error: {0}")]
    Terminal(#[source] std::io::Error),

    #[error("TTY required: stdin is not a terminal")]
    NotATty,

    #[error(transparent)]
    UseCase(#[from] icscli::error::IcsError),

    #[error("invalid argument: {0}")]
    InvalidArgs(String),
}

pub type Result<T> = std::result::Result<T, LazyicsError>;

impl LazyicsError {
    /// Map the error to the lazyics exit code (ADR-025 §Output and exit codes).
    pub fn exit_code(&self) -> i32 {
        match self {
            // Parse errors surface as IcsError::Parse — exit 2.
            LazyicsError::UseCase(icscli::error::IcsError::Parse(_)) => 2,
            // Everything else is I/O / terminal / argument level — exit 1.
            _ => 1,
        }
    }
}
