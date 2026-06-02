//! lazyics — `lazygit`-inspired TUI for iCalendar files.
//!
//! Composition Root. Parses minimal CLI args, refuses to run without a TTY
//! (ADR-025 §"Output and exit codes"), opens the terminal under a RAII
//! guard, and drives the event loop until the active screen asks to quit.
//!
//! Phase 1 ships only the [`ListScreen`] with placeholder data. Add / Edit
//! / Remove forms and live calendar loading land in later phases.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{self, Event};
use icscli::application::ports::CalendarRepository;
use icscli::infrastructure::FileCalendarRepository;

use lazyics::error::{LazyicsError, Result};
use lazyics::infrastructure::terminal::TerminalGuard;
use lazyics::presentation::keymap;
use lazyics::presentation::screens::list::{ListScreen, ScreenAction};

const DEFAULT_FILE: &str = "calendar.ics";
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
struct Args {
    file: PathBuf,
    log: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            eprintln!("lazyics: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run() -> Result<()> {
    let args = parse_args(std::env::args().skip(1))?;
    init_logging(args.log.as_deref())?;

    if !std::io::stdin().is_terminal() {
        return Err(LazyicsError::NotATty);
    }

    // Resolve I/O failures (missing file, unreadable file, parse errors)
    // *before* taking the terminal so the user sees the error on their
    // shell prompt instead of inside a half-rendered alternate screen.
    let repo = FileCalendarRepository::new(args.file.clone());
    if !repo.exists() {
        return Err(LazyicsError::InvalidArgs(format!(
            "calendar file not found: {} (run `icscli -f {} init` first, or pass --file <PATH>)",
            args.file.display(),
            args.file.display(),
        )));
    }
    let file_label = args.file.display().to_string();
    let mut screen = ListScreen::from_repo(&repo, file_label)?;

    let mut guard = TerminalGuard::enter()?;
    event_loop(&mut guard, &mut screen)
}

fn event_loop(guard: &mut TerminalGuard, screen: &mut ListScreen) -> Result<()> {
    loop {
        guard
            .terminal()
            .draw(|frame| screen.render(frame))
            .map_err(LazyicsError::Terminal)?;

        // Block-with-timeout so terminal resizes etc. eventually redraw even
        // without input. Phase 1 just polls; richer event types (mouse,
        // resize) get handled as their phases land.
        if !event::poll(POLL_INTERVAL).map_err(LazyicsError::Terminal)? {
            continue;
        }
        // Non-key events (Resize / Mouse / Paste / FocusGained / FocusLost)
        // are no-ops in Phase 1; the next draw picks up any resize naturally.
        if let Event::Key(key_event) = event::read().map_err(LazyicsError::Terminal)? {
            let Some(intent) = keymap::map(key_event) else {
                continue;
            };
            match screen.handle(intent) {
                ScreenAction::Continue => {}
                ScreenAction::Quit => return Ok(()),
            }
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args> {
    let mut file: Option<PathBuf> = None;
    let mut log: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" | "-f" => {
                let value = args
                    .next()
                    .ok_or_else(|| LazyicsError::InvalidArgs(format!("{arg} requires a value")))?;
                file = Some(PathBuf::from(value));
            }
            "--log" => {
                let value = args
                    .next()
                    .ok_or_else(|| LazyicsError::InvalidArgs("--log requires a path".into()))?;
                log = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("{}", help_text());
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("lazyics {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => {
                return Err(LazyicsError::InvalidArgs(format!(
                    "unknown argument: {other}"
                )));
            }
        }
    }

    Ok(Args {
        file: file.unwrap_or_else(|| PathBuf::from(DEFAULT_FILE)),
        log,
    })
}

fn init_logging(path: Option<&std::path::Path>) -> Result<()> {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt::writer::BoxMakeWriter;

    let Some(path) = path else {
        return Ok(());
    };
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(LazyicsError::Terminal)?;
    let writer = BoxMakeWriter::new(std::sync::Arc::new(file));
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .map_err(|e| LazyicsError::InvalidArgs(format!("logging init failed: {e}")))?;
    Ok(())
}

fn help_text() -> &'static str {
    "lazyics — lazygit-inspired TUI for iCalendar files

Usage:
  lazyics [--file <PATH>] [--log <PATH>]
  lazyics -h | --help
  lazyics -V | --version

Options:
  -f, --file <PATH>   Path to the ICS file (default: calendar.ics)
      --log <PATH>    Append tracing output to PATH (filtered via RUST_LOG)
  -h, --help          Show this help and exit
  -V, --version       Show version and exit

Keys (Phase 1):
  q | Esc | Ctrl+C    Quit
  j | Down            Move selection down
  k | Up              Move selection up
  g | Home            Jump to first event
  G | End             Jump to last event
"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_when_empty() {
        let args = parse_args(std::iter::empty()).unwrap();
        assert_eq!(args.file, PathBuf::from(DEFAULT_FILE));
        assert!(args.log.is_none());
    }

    #[test]
    fn parse_args_reads_file_flag() {
        let args = parse_args(["--file", "holidays.ics"].into_iter().map(String::from)).unwrap();
        assert_eq!(args.file, PathBuf::from("holidays.ics"));
    }

    #[test]
    fn parse_args_short_file_flag() {
        let args = parse_args(["-f", "h.ics"].into_iter().map(String::from)).unwrap();
        assert_eq!(args.file, PathBuf::from("h.ics"));
    }

    #[test]
    fn parse_args_reads_log_flag() {
        let args = parse_args(["--log", "/tmp/lazyics.log"].into_iter().map(String::from)).unwrap();
        assert_eq!(args.log, Some(PathBuf::from("/tmp/lazyics.log")));
    }

    #[test]
    fn parse_args_combined_flags() {
        let args = parse_args(
            ["--file", "h.ics", "--log", "h.log"]
                .into_iter()
                .map(String::from),
        )
        .unwrap();
        assert_eq!(args.file, PathBuf::from("h.ics"));
        assert_eq!(args.log, Some(PathBuf::from("h.log")));
    }

    #[test]
    fn parse_args_missing_value_errors() {
        let err = parse_args(["--file"].into_iter().map(String::from)).unwrap_err();
        assert!(matches!(err, LazyicsError::InvalidArgs(_)));
    }

    #[test]
    fn parse_args_unknown_flag_errors() {
        let err = parse_args(["--nope"].into_iter().map(String::from)).unwrap_err();
        assert!(matches!(err, LazyicsError::InvalidArgs(_)));
    }
}
