//! In-app help overlay.
//!
//! Renders all keybindings grouped by context. Opened from any view by
//! `?` (Browse mode), closed by `?` again, `Esc`, or `q`. The overlay
//! uses Browse keymap (no text input) but is treated as a non-view by
//! the Composition Root (`Screen::kind()` returns `None`), which is how
//! view-switching shortcuts get inert while help is on screen.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::presentation::keymap::Intent;
use crate::presentation::screens::ScreenAction;

pub struct HelpScreen {
    file_label: String,
    transient_status: Option<String>,
}

impl HelpScreen {
    pub fn new(file_label: impl Into<String>) -> Self {
        Self {
            file_label: file_label.into(),
            transient_status: None,
        }
    }

    pub fn file_label(&self) -> &str {
        &self.file_label
    }

    pub fn set_transient_status(&mut self, msg: impl Into<String>) {
        self.transient_status = Some(msg.into());
    }

    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        self.transient_status = None;
        match intent {
            // Hard exit: same as anywhere else.
            Intent::Quit => ScreenAction::Quit,
            // The three "close help" affordances. `OpenHelp` toggling
            // closes the overlay when it's already open.
            Intent::Cancel | Intent::OpenHelp => ScreenAction::DismissHelp,
            // Everything else: ignore. Listing the variants keeps the
            // match exhaustive and tells future readers each intent was
            // considered.
            Intent::NavUp
            | Intent::NavDown
            | Intent::NavLeft
            | Intent::NavRight
            | Intent::NavTop
            | Intent::NavBottom
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::CycleGranularity
            | Intent::OpenRemove
            | Intent::OpenAdd
            | Intent::OpenEdit
            | Intent::ToggleMark
            | Intent::Confirm
            | Intent::TypeChar(_)
            | Intent::Backspace
            | Intent::NextField
            | Intent::PrevField
            | Intent::SubmitForm => ScreenAction::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [body, status] = layout.areas(frame.area());

        let block = Block::default()
            .title("lazyics — help")
            .borders(Borders::ALL);
        let inner = block.inner(body);
        frame.render_widget(block, body);

        let lines = help_lines();
        frame.render_widget(Paragraph::new(lines), inner);

        let status_text = self
            .transient_status
            .clone()
            .unwrap_or_else(|| "? close help  q quit".to_string());
        frame.render_widget(Paragraph::new(status_text), status);
    }
}

fn header(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn binding(keys: &'static str, desc: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{keys:<22}"), Style::default().fg(Color::Cyan)),
        Span::raw(desc),
    ])
}

fn blank() -> Line<'static> {
    Line::raw("")
}

fn help_lines() -> Vec<Line<'static>> {
    vec![
        header("Global"),
        binding("q | Ctrl+C", "Quit"),
        binding("?", "Open / close this help"),
        binding("Esc", "Cancel modal state / close help / Quit at top level"),
        blank(),
        header("View switching"),
        binding("Tab", "Cycle List → Timeline → Grid → List"),
        binding("1 / 2 / 3", "Jump to List / Timeline / Grid"),
        binding("u", "Cycle current view's time unit (month ↔ week)"),
        blank(),
        header("Movement"),
        binding("j | Down", "Down / next row / next week (Grid)"),
        binding("k | Up", "Up / previous row / previous week (Grid)"),
        binding("h | Left", "Previous day (Grid)"),
        binding("l | Right", "Next day (Grid)"),
        binding("g | Home", "First event / first of period"),
        binding("G | End", "Last event / last of period"),
        blank(),
        header("List view"),
        binding("a", "Open Add form"),
        binding("e", "Open Edit form on selected event"),
        binding("d | x", "Enter multi-select Remove mode"),
        binding("Space", "Toggle mark on selected row (Remove mode)"),
        binding("Enter | Shift+D", "Confirm removal of marked events"),
        blank(),
        header("Add / Edit form"),
        binding("Tab | Shift+Tab", "Next / previous field"),
        binding(
            "Left | Right",
            "Cursor in text fields; cycle prev/next for pickers",
        ),
        binding("Space", "Cycle next on busy-status / class pickers"),
        binding("Ctrl+S | Enter", "Submit (validates required fields)"),
        binding("Esc", "Cancel and return to the previous view"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_dismisses_help() {
        let mut s = HelpScreen::new("h.ics");
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::DismissHelp);
    }

    #[test]
    fn question_mark_intent_toggles_close() {
        let mut s = HelpScreen::new("h.ics");
        assert_eq!(s.handle(Intent::OpenHelp), ScreenAction::DismissHelp);
    }

    #[test]
    fn quit_force_exits() {
        let mut s = HelpScreen::new("h.ics");
        assert_eq!(s.handle(Intent::Quit), ScreenAction::Quit);
    }

    #[test]
    fn nav_intents_are_no_ops() {
        let mut s = HelpScreen::new("h.ics");
        assert_eq!(s.handle(Intent::NavDown), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavUp), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::CycleView), ScreenAction::Continue);
    }
}
