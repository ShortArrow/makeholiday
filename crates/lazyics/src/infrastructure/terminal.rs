//! Terminal RAII guard.
//!
//! Enters raw mode + alternate screen on construction, leaves them on drop.
//! Also installs a panic hook on first use so that a panic mid-render still
//! restores the terminal — without it, the user is left looking at a garbled
//! shell prompt with line buffering broken.

use std::io::{self, Stdout};
use std::sync::Once;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::error::{LazyicsError, Result};

pub type Backend = CrosstermBackend<Stdout>;
pub type Tui = Terminal<Backend>;

/// RAII guard that owns the terminal in alternate-screen + raw mode and
/// restores it on `Drop`.
pub struct TerminalGuard {
    inner: Option<Tui>,
}

impl TerminalGuard {
    /// Acquire the terminal. Installs the panic-restore hook on first call.
    pub fn enter() -> Result<Self> {
        install_panic_hook();

        enable_raw_mode().map_err(LazyicsError::Terminal)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| {
            // Best-effort: undo the raw-mode we just took. Ignore secondary errors.
            let _ = disable_raw_mode();
            LazyicsError::Terminal(e)
        })?;

        let backend = CrosstermBackend::new(stdout);
        let inner = Terminal::new(backend).map_err(|e| {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            LazyicsError::Terminal(e)
        })?;

        Ok(Self { inner: Some(inner) })
    }

    /// Borrow the ratatui terminal for rendering.
    pub fn terminal(&mut self) -> &mut Tui {
        self.inner
            .as_mut()
            .expect("TerminalGuard inner taken before drop")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Order: leave alt screen, then disable raw mode. Mirrors `enter`.
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        // ratatui terminal dropped after the backend has been restored.
        let _ = self.inner.take();
    }
}

/// Best-effort terminal restore on panic. Installed once per process.
fn install_panic_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            previous(info);
        }));
    });
}
