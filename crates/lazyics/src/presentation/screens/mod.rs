//! Screens — one module per top-level view (List / Timeline / Grid).
//!
//! Phase 4a introduces the multi-view structure. `Screen` is an enum
//! that explicitly dispatches `handle` / `render` to the active variant.
//! View switching is a Composition-Root concern (handled at the top of
//! `main.rs::event_loop`) so each Screen variant stays focused on its
//! own keybindings and render logic.

pub mod add_form;
pub mod grid;
pub mod list;
pub mod timeline;

pub use add_form::AddForm;
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
#[derive(Debug, Clone, PartialEq)]
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
    /// Open the Add form. The Composition Root swaps the active screen
    /// for `Screen::AddForm`, remembering which view to return to.
    OpenAdd,
    /// Submit a validated Add request. AddForm produces this; the
    /// Composition Root drives `icscli::application::use_cases::add` and
    /// returns to the previously-active view on success.
    SubmitAdd(AddRequest),
    /// Dismiss the active modal (e.g. AddForm cancelled with Esc).
    /// Composition Root returns to the previously-active view.
    DismissForm,
}

/// Validated Add request handed from `AddForm` to the Composition Root.
///
/// Fields mirror `icscli::application::use_cases::add`'s parameters.
/// Only `PartialEq` is derived because `ics_core::EventClass` does not
/// implement `Eq`; tests get all they need from `PartialEq`.
#[derive(Debug, Clone, PartialEq)]
pub struct AddRequest {
    pub summary: String,
    pub start: chrono::NaiveDate,
    pub end: Option<chrono::NaiveDate>,
    pub busystatus: ics_core::microsoft::MsBusyStatus,
    pub class: Option<ics_core::EventClass>,
    pub categories: Vec<String>,
    pub icon: Option<String>,
}

/// The active screen. Enum-dispatched rather than `Box<dyn Screen>`
/// because the variant set is small, closed, and explicit dispatch makes
/// the call sites greppable.
pub enum Screen {
    List(ListScreen),
    Timeline(TimelineScreen),
    Grid(GridScreen),
    AddForm(AddForm),
}

impl Screen {
    /// `Some(_)` for top-level views, `None` for modal surfaces (forms).
    /// Composition Root uses this to remember which view to return to
    /// after a modal dismisses.
    pub fn kind(&self) -> Option<ViewKind> {
        match self {
            Screen::List(_) => Some(ViewKind::List),
            Screen::Timeline(_) => Some(ViewKind::Timeline),
            Screen::Grid(_) => Some(ViewKind::Grid),
            Screen::AddForm(_) => None,
        }
    }

    /// True for modal surfaces (forms / confirmations). The keymap is
    /// switched to `KeymapMode::Form` for these so printable characters
    /// reach the focused text field instead of triggering view-level
    /// shortcuts.
    pub fn is_modal(&self) -> bool {
        matches!(self, Screen::AddForm(_))
    }

    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        match self {
            Screen::List(s) => s.handle(intent),
            Screen::Timeline(s) => s.handle(intent),
            Screen::Grid(s) => s.handle(intent),
            Screen::AddForm(s) => s.handle(intent),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self {
            Screen::List(s) => s.render(frame),
            Screen::Timeline(s) => s.render(frame),
            Screen::Grid(s) => s.render(frame),
            Screen::AddForm(s) => s.render(frame),
        }
    }

    pub fn set_transient_status(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        match self {
            Screen::List(s) => s.set_transient_status(msg),
            Screen::Timeline(s) => s.set_transient_status(msg),
            Screen::Grid(s) => s.set_transient_status(msg),
            Screen::AddForm(s) => s.set_transient_status(msg),
        }
    }

    pub fn file_label(&self) -> &str {
        match self {
            Screen::List(s) => s.file_label(),
            Screen::Timeline(s) => s.file_label(),
            Screen::Grid(s) => s.file_label(),
            Screen::AddForm(s) => s.file_label(),
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
