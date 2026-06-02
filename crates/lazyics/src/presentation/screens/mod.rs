//! Screens — one module per top-level view (List / Timeline / Grid).
//!
//! Phase 4a introduces the multi-view structure. `Screen` is an enum
//! that explicitly dispatches `handle` / `render` to the active variant.
//! View switching is a Composition-Root concern (handled at the top of
//! `main.rs::event_loop`) so each Screen variant stays focused on its
//! own keybindings and render logic.

pub mod grid;
pub mod list;
pub mod timeline;

pub use grid::GridScreen;
pub use list::ListScreen;
pub use timeline::TimelineScreen;

use crate::presentation::keymap::Intent;
use ratatui::Frame;

/// Which top-level view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    List,
    Timeline,
    Grid,
}

impl ViewKind {
    /// Tab-cycle order: List → Timeline → Grid → List.
    pub fn next(self) -> Self {
        match self {
            ViewKind::List => ViewKind::Timeline,
            ViewKind::Timeline => ViewKind::Grid,
            ViewKind::Grid => ViewKind::List,
        }
    }

    /// Number-key mapping: 1 → List, 2 → Timeline, 3 → Grid.
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(ViewKind::List),
            2 => Some(ViewKind::Timeline),
            3 => Some(ViewKind::Grid),
            _ => None,
        }
    }
}

/// Action returned by a screen after handling an [`Intent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenAction {
    /// Stay on this screen; keep looping.
    Continue,
    /// Exit the application normally.
    Quit,
    /// Submit a remove request for the given **1-based** indices into the
    /// active calendar's event list. The Composition Root issues the
    /// actual `use_cases::remove` call so the repository write happens
    /// outside the screen.
    RemoveByIndices(Vec<usize>),
}

/// The active screen. Enum-dispatched rather than `Box<dyn Screen>`
/// because the variant set is small, closed, and explicit dispatch makes
/// the call sites greppable.
pub enum Screen {
    List(ListScreen),
    Timeline(TimelineScreen),
    Grid(GridScreen),
}

impl Screen {
    pub fn kind(&self) -> ViewKind {
        match self {
            Screen::List(_) => ViewKind::List,
            Screen::Timeline(_) => ViewKind::Timeline,
            Screen::Grid(_) => ViewKind::Grid,
        }
    }

    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        match self {
            Screen::List(s) => s.handle(intent),
            Screen::Timeline(s) => s.handle(intent),
            Screen::Grid(s) => s.handle(intent),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self {
            Screen::List(s) => s.render(frame),
            Screen::Timeline(s) => s.render(frame),
            Screen::Grid(s) => s.render(frame),
        }
    }

    pub fn set_transient_status(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        match self {
            Screen::List(s) => s.set_transient_status(msg),
            Screen::Timeline(s) => s.set_transient_status(msg),
            Screen::Grid(s) => s.set_transient_status(msg),
        }
    }

    pub fn file_label(&self) -> &str {
        match self {
            Screen::List(s) => s.file_label(),
            Screen::Timeline(s) => s.file_label(),
            Screen::Grid(s) => s.file_label(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_kind_cycle() {
        assert_eq!(ViewKind::List.next(), ViewKind::Timeline);
        assert_eq!(ViewKind::Timeline.next(), ViewKind::Grid);
        assert_eq!(ViewKind::Grid.next(), ViewKind::List);
    }

    #[test]
    fn view_kind_from_number() {
        assert_eq!(ViewKind::from_number(1), Some(ViewKind::List));
        assert_eq!(ViewKind::from_number(2), Some(ViewKind::Timeline));
        assert_eq!(ViewKind::from_number(3), Some(ViewKind::Grid));
        assert_eq!(ViewKind::from_number(0), None);
        assert_eq!(ViewKind::from_number(4), None);
    }
}
