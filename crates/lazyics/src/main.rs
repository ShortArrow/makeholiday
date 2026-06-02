//! lazyics — `lazygit`-inspired TUI for iCalendar files.
//!
//! Composition Root. Parses minimal CLI args, refuses to run without a TTY
//! (ADR-025 §"Output and exit codes"), opens the terminal under a RAII
//! guard, and drives the event loop until the active screen asks to quit.
//!
//! Phase 4a wires multi-view switching (List / Timeline / Grid) at the
//! Composition-Root level. Phase 3b layers an `AddForm` modal on top:
//! pressing `a` in List view replaces the active screen with the form;
//! Esc/Ctrl+S/Enter on the form return to the *previously-active* view,
//! reloaded from disk so the just-added event appears in place.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{self, Event};
use ics_core::VEvent;
use icscli::application::ports::CalendarRepository;
use icscli::infrastructure::FileCalendarRepository;

use lazyics::error::{LazyicsError, Result};
use lazyics::infrastructure::terminal::TerminalGuard;
use lazyics::presentation::keymap::{self, Intent, KeymapMode};
use lazyics::presentation::screens::{
    AddForm, AddRequest, GridScreen, ListScreen, Screen, ScreenAction, TimelineScreen, ViewKind,
};

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

    let repo = FileCalendarRepository::new(args.file.clone());
    if !repo.exists() {
        return Err(LazyicsError::InvalidArgs(format!(
            "calendar file not found: {} (run `icscli -f {} init` first, or pass --file <PATH>)",
            args.file.display(),
            args.file.display(),
        )));
    }
    let file_label = args.file.display().to_string();
    let events = repo.load()?.events;
    let mut screen = build_screen(ViewKind::List, &events, &file_label);

    let mut guard = TerminalGuard::enter()?;
    event_loop(&mut guard, &repo, &mut screen, &file_label)
}

fn event_loop(
    guard: &mut TerminalGuard,
    repo: &FileCalendarRepository,
    screen: &mut Screen,
    file_label: &str,
) -> Result<()> {
    // Where to return after a modal dismisses. Initialized to List
    // (the first screen we open with); updated each time a modal opens.
    let mut previous_view = ViewKind::List;

    loop {
        guard
            .terminal()
            .draw(|frame| screen.render(frame))
            .map_err(LazyicsError::Terminal)?;

        if !event::poll(POLL_INTERVAL).map_err(LazyicsError::Terminal)? {
            continue;
        }
        if let Event::Key(key_event) = event::read().map_err(LazyicsError::Terminal)? {
            // KeymapMode follows the active screen: forms get text-input
            // semantics, views get nav/modal semantics. The same `a`
            // press means OpenAdd in a List view and a typed character
            // in an AddForm.
            let mode = if screen.is_modal() {
                KeymapMode::Form
            } else {
                KeymapMode::Browse
            };
            let Some(intent) = keymap::map(key_event, mode) else {
                continue;
            };

            // View-switching intents are intercepted at the Composition
            // Root (Browse mode only). In Form mode the keymap never
            // emits CycleView/SwitchView in the first place, so the
            // is_modal guard is belt-and-suspenders.
            if !screen.is_modal() {
                match intent {
                    Intent::CycleView => {
                        if let Some(current) = screen.kind() {
                            switch_view(repo, screen, current.next(), file_label)?;
                        }
                        continue;
                    }
                    Intent::SwitchView(kind) => {
                        if screen.kind() != Some(kind) {
                            switch_view(repo, screen, kind, file_label)?;
                        }
                        continue;
                    }
                    _ => {}
                }
            }

            match screen.handle(intent) {
                ScreenAction::Continue => {}
                ScreenAction::Quit => return Ok(()),
                ScreenAction::RemoveByIndices(indices) => {
                    apply_remove(repo, screen, file_label, &indices)?;
                }
                ScreenAction::OpenAdd => {
                    // Remember the host view so DismissForm / SubmitAdd
                    // can return the user to it.
                    if let Some(kind) = screen.kind() {
                        previous_view = kind;
                    }
                    *screen = Screen::AddForm(AddForm::new(file_label.to_string()));
                }
                ScreenAction::SubmitAdd(req) => {
                    apply_add(repo, screen, file_label, previous_view, req)?;
                }
                ScreenAction::DismissForm => {
                    switch_view(repo, screen, previous_view, file_label)?;
                }
            }
        }
    }
}

/// Replace the active `Screen` with a fresh instance of `kind`, reloading
/// events from disk so concurrent edits via `icscli` are reflected.
fn switch_view(
    repo: &FileCalendarRepository,
    screen: &mut Screen,
    kind: ViewKind,
    file_label: &str,
) -> Result<()> {
    let events = repo.load()?.events;
    *screen = build_screen(kind, &events, file_label);
    Ok(())
}

fn build_screen(kind: ViewKind, events: &[VEvent], file_label: &str) -> Screen {
    match kind {
        ViewKind::List => Screen::List(ListScreen::from_events(events, file_label.to_string())),
        ViewKind::Timeline => {
            Screen::Timeline(TimelineScreen::from_events(events, file_label.to_string()))
        }
        ViewKind::Grid => Screen::Grid(GridScreen::from_events(events, file_label.to_string())),
    }
}

/// Submit a Remove-mode confirmation to `icscli::application::use_cases::remove`
/// and reload the screen from the repository so the deletion is reflected.
fn apply_remove(
    repo: &FileCalendarRepository,
    screen: &mut Screen,
    file_label: &str,
    indices: &[usize],
) -> Result<()> {
    use lazyics::application::use_cases::{RunContext, remove};

    let count = indices.len();
    let spec = indices
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let ctx = RunContext {
        quiet: true,
        allow_prompts: false,
    };
    match remove(repo, ctx, None, Some(&spec)) {
        Ok(()) => {
            let kind = screen.kind().unwrap_or(ViewKind::List);
            switch_view(repo, screen, kind, file_label)?;
            screen.set_transient_status(format!("Removed {count} event(s)."));
            tracing::info!(count, indices = ?indices, "remove succeeded");
        }
        Err(e) => {
            tracing::error!(error = %e, "remove failed");
            screen.set_transient_status(format!("Remove failed: {e}"));
        }
    }
    Ok(())
}

/// Submit a validated Add request to `icscli::application::use_cases::add`.
/// On success: switch back to `previous_view`, reloaded from disk, with a
/// transient status banner. On failure: stay on the form and surface the
/// error in its status bar so the user can edit and retry.
fn apply_add(
    repo: &FileCalendarRepository,
    screen: &mut Screen,
    file_label: &str,
    previous_view: ViewKind,
    req: AddRequest,
) -> Result<()> {
    use lazyics::application::use_cases::{RunContext, add};

    let ctx = RunContext {
        quiet: true,
        allow_prompts: false,
    };
    let summary_for_msg = req.summary.clone();
    let start_for_msg = req.start;
    match add(
        repo,
        ctx,
        Some(&req.summary),
        Some(req.start),
        req.end,
        req.busystatus,
        req.class,
        req.categories,
        req.icon,
    ) {
        Ok(()) => {
            switch_view(repo, screen, previous_view, file_label)?;
            screen.set_transient_status(format!("Added: {summary_for_msg} on {start_for_msg}"));
            tracing::info!(summary = %summary_for_msg, start = %start_for_msg, "add succeeded");
        }
        Err(e) => {
            tracing::error!(error = %e, "add failed");
            // Form is still the active screen; reuse its status bar.
            screen.set_transient_status(format!("Add failed: {e}"));
        }
    }
    Ok(())
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

Views:
  Tab                 Cycle List → Timeline → Grid → List
  1                   Jump to List view
  2                   Jump to Timeline view
  3                   Jump to Grid view
  u                   Cycle current view's time unit (month ↔ week)

Common keys:
  q | Ctrl+C          Quit
  Esc                 Cancel modal state / Quit at top level
  j | Down            Down / next row / next week (Grid)
  k | Up              Up / previous row / previous week (Grid)
  h | Left            Previous day (Grid)
  l | Right           Next day (Grid)
  g | Home            First event / first of period
  G | End             Last event / last of period

List view:
  a                   Open Add form
  d | x               Enter multi-select Remove mode
  Space               Toggle mark on highlighted row (Remove mode)
  Enter | Shift+D     Confirm removal of marked events

Add form:
  Tab | Shift+Tab     Next / previous field
  Left | Right        Cursor in text fields; cycle prev/next for pickers
  Space               Cycle next on busy-status / class pickers
  Ctrl+S | Enter      Submit (validates required fields)
  Esc                 Cancel and return to the previous view
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
