//! Screens — one module per top-level view (List / Timeline / Grid).
//!
//! Phase 4a introduces the multi-view structure. `Screen` is an enum
//! that explicitly dispatches `handle` / `render` to the active variant.
//! View switching is a Composition-Root concern (handled at the top of
//! `main.rs::event_loop`) so each Screen variant stays focused on its
//! own keybindings and render logic.

pub mod event_form;
pub mod grid;
pub mod help;
pub mod list;
pub mod timeline;

pub use event_form::{EventForm, FormMode};
pub use grid::GridScreen;
pub use help::HelpScreen;
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
    /// for `Screen::EventForm` (Add mode), remembering which view to
    /// return to. `start_hint` pre-populates the Start field — Grid
    /// sets it to the cursor date so `a` on a calendar cell creates an
    /// event on that day. List and Timeline pass `None`.
    OpenAdd {
        start_hint: Option<chrono::NaiveDate>,
    },
    /// Open the Edit form on the event at the given 1-based index. The
    /// Composition Root pulls the event from the loaded list and seeds
    /// `Screen::EventForm` (Edit mode) with its values.
    OpenEdit { event_index: usize },
    /// Like `OpenEdit` but the screen doesn't know the canonical
    /// 1-based index — only the event's stable UID. Timeline emits this
    /// because it carries an internally-sorted event list; the
    /// Composition Root resolves the UID against the on-disk events.
    OpenEditByUid { uid: String },
    /// Submit a validated Add request. The Composition Root drives
    /// `icscli::application::use_cases::add` and returns to the
    /// previously-active view on success.
    SubmitAdd(AddRequest),
    /// Submit a validated Edit request. The Composition Root drives
    /// `icscli::application::use_cases::edit` with `event_index` (1-based)
    /// and `patch`, then returns to the previously-active view.
    SubmitEdit {
        event_index: usize,
        patch: icscli::application::use_cases::EditPatch,
    },
    /// Dismiss the active modal (form cancelled with Esc). Composition
    /// Root returns to the previously-active view.
    DismissForm,
    /// Open the in-app help overlay. Composition Root replaces the
    /// active screen with `Screen::Help`, remembering which view to
    /// return to.
    OpenHelp,
    /// Dismiss the help overlay. Composition Root returns to the
    /// previously-active view.
    DismissHelp,
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
    /// EventForm covers both Add and Edit modes — see [`FormMode`].
    EventForm(EventForm),
    /// In-app help overlay. Uses Browse keymap (no text input), but is
    /// non-view so view-switching shortcuts get inert while open.
    Help(HelpScreen),
}

impl Screen {
    /// `Some(_)` for top-level views, `None` for modal / overlay
    /// surfaces. Composition Root uses this to gate view-switching
    /// intents and to remember which view to return to after a modal
    /// dismisses.
    pub fn kind(&self) -> Option<ViewKind> {
        match self {
            Screen::List(_) => Some(ViewKind::List),
            Screen::Timeline(_) => Some(ViewKind::Timeline),
            // Grid in picker mode acts as a non-view: view-switching
            // shortcuts (Tab / 1 / 2 / 3) get inert until the user
            // commits or cancels the jump.
            Screen::Grid(s) if !s.is_picker_mode() => Some(ViewKind::Grid),
            _ => None,
        }
    }

    /// True for surfaces that take text input (`KeymapMode::Form`).
    /// Help uses Browse keymap. List is normally Browse but flips to
    /// Form-style while the user is in Search input mode.
    pub fn is_modal(&self) -> bool {
        match self {
            Screen::EventForm(_) => true,
            Screen::List(s) => s.is_search_mode(),
            _ => false,
        }
    }

    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        match self {
            Screen::List(s) => s.handle(intent),
            Screen::Timeline(s) => s.handle(intent),
            Screen::Grid(s) => s.handle(intent),
            Screen::EventForm(s) => s.handle(intent),
            Screen::Help(s) => s.handle(intent),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self {
            Screen::List(s) => s.render(frame),
            Screen::Timeline(s) => s.render(frame),
            Screen::Grid(s) => s.render(frame),
            Screen::EventForm(s) => s.render(frame),
            Screen::Help(s) => s.render(frame),
        }
    }

    pub fn set_transient_status(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        match self {
            Screen::List(s) => s.set_transient_status(msg),
            Screen::Timeline(s) => s.set_transient_status(msg),
            Screen::Grid(s) => s.set_transient_status(msg),
            Screen::EventForm(s) => s.set_transient_status(msg),
            Screen::Help(s) => s.set_transient_status(msg),
        }
    }

    pub fn file_label(&self) -> &str {
        match self {
            Screen::List(s) => s.file_label(),
            Screen::Timeline(s) => s.file_label(),
            Screen::Grid(s) => s.file_label(),
            Screen::EventForm(s) => s.file_label(),
            Screen::Help(s) => s.file_label(),
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
