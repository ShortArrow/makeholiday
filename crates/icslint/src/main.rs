//! `icslint` — composition root.
//!
//! Reads each input path, runs [`icslint::lint`] on the source, hands
//! the resulting diagnostics off to the chosen [`reporter::Reporter`],
//! and exits with the code dictated by ADR-026 §"Exit codes":
//!
//! - `0` — no diagnostics, or only info-level diagnostics.
//! - `1` — at least one warning emitted (and not promoted to error).
//! - `2` — at least one error emitted, or any warning when `-W`.
//! - `3` — internal: file unreadable, parse failure that the tolerant
//!   parser could not recover from.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use icslint::reporter::Format;
use icslint::{Diagnostic, Severity, exit_code_for, lint};

#[derive(Parser, Debug)]
#[command(
    name = "icslint",
    about = "Lint .ics calendar files for RFC 5545 issues, vendor extension hygiene, and structure problems"
)]
struct Cli {
    /// Input `.ics` files. Use `-` to read from stdin.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Treat warnings as errors (exit 2 on any warning).
    #[arg(short = 'W', long = "warnings-as-errors", default_value_t = false)]
    warnings_as_errors: bool,

    /// Suppress info-level diagnostics from the output.
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

    /// Output format. `human` (default) prints compiler-style messages
    /// to stderr; `json` and `github` print machine-readable output to
    /// stdout.
    #[arg(short = 'f', long = "format", value_enum, default_value_t = Format::Human)]
    format: Format,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let mut all_diags: Vec<(PathBuf, Diagnostic)> = Vec::new();
    let mut internal_error = false;

    for path in &cli.paths {
        let source = match read_source(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("icslint: cannot read {}: {}", path.display(), e);
                internal_error = true;
                continue;
            }
        };
        for diag in lint(&source) {
            if cli.quiet && diag.severity == Severity::Info {
                continue;
            }
            all_diags.push((path.clone(), diag));
        }
    }

    let reporter = cli.format.reporter();
    let mut stream = cli.format.stream();
    // Reporter write errors (a closed pipe, full disk) cannot meaningfully
    // change the exit code we owe the user; the diagnostics' own severity
    // tier still drives the contract. Swallow IO errors here so a `| head`
    // consumer does not flip a clean run to exit 3.
    let _ = reporter.write(&mut *stream, &all_diags);

    if internal_error {
        return ExitCode::from(3);
    }
    let raw: Vec<Diagnostic> = all_diags.into_iter().map(|(_, d)| d).collect();
    ExitCode::from(exit_code_for(&raw, cli.warnings_as_errors) as u8)
}

fn read_source(path: &Path) -> std::io::Result<String> {
    if path.to_str() == Some("-") {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}
