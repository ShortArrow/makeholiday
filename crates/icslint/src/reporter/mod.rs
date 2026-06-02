//! Diagnostic reporters — one per output format.
//!
//! ADR-026 §"Output formats" lists three: `human` (default), `json`,
//! and `github` (GitHub Actions workflow commands). Each format owns
//! its own module; this file exposes the trait every reporter
//! implements and the `Format` enum the CLI uses to pick one.

use std::io::{self, Write};
use std::path::PathBuf;

use crate::Diagnostic;

pub mod github;
pub mod human;
pub mod json;

pub use github::GithubReporter;
pub use human::HumanReporter;
pub use json::JsonReporter;

/// Write the per-file diagnostic batch in this format's wire shape.
///
/// Reporters do not decide their own output stream — the caller picks
/// stderr or stdout depending on whether the format is human-facing
/// (stderr, alongside compiler-style messages) or machine-facing
/// (stdout, where redirection and pipelining go).
pub trait Reporter {
    fn write(&self, w: &mut dyn Write, diagnostics: &[(PathBuf, Diagnostic)]) -> io::Result<()>;
}

/// Output format choice exposed to the CLI as `--format <FORMAT>`.
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[clap(rename_all = "lowercase")]
pub enum Format {
    /// Compiler-style messages on stderr. The default.
    #[default]
    Human,
    /// JSON array of diagnostics on stdout. Stable schema across
    /// patch releases per ADR-026.
    Json,
    /// GitHub Actions workflow command annotations on stdout.
    Github,
}

impl Format {
    /// Pick the reporter implementation for this format.
    pub fn reporter(self) -> Box<dyn Reporter> {
        match self {
            Format::Human => Box::new(HumanReporter),
            Format::Json => Box::new(JsonReporter),
            Format::Github => Box::new(GithubReporter),
        }
    }

    /// Pick which stream this format writes to.
    ///
    /// Human goes to stderr so it composes with the standard "stdout =
    /// data, stderr = messages" pipeline contract. Machine-facing
    /// formats go to stdout where consumers redirect into files or
    /// pipe through `jq` / GitHub Actions log parsers.
    pub fn stream(self) -> Box<dyn Write> {
        match self {
            Format::Human => Box::new(io::stderr()),
            Format::Json | Format::Github => Box::new(io::stdout()),
        }
    }
}
