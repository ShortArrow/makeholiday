//! Event list screen.
//!
//! Phase 1 ships a placeholder backing store so the keymap + render path can
//! exercise without a calendar file. Phase 2 swaps `placeholder` for a
//! `from_repo` constructor that loads events through `icscli`'s
//! `FileCalendarRepository`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::presentation::keymap::Intent;

/// The outcome of handling an [`Intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenAction {
    /// Stay on this screen; keep looping.
    Continue,
    /// Exit the application normally.
    Quit,
}

pub struct ListScreen {
    items: Vec<String>,
    state: ListState,
    /// Filesystem path the screen will eventually load from. Stored for the
    /// status bar; not yet used for I/O in Phase 1.
    file_label: String,
}

impl ListScreen {
    /// Phase 1 placeholder: a few dummy rows so navigation is observable.
    pub fn placeholder(file_label: impl Into<String>) -> Self {
        let items = vec![
            "(placeholder) 2026-01-01 : 元日".to_string(),
            "(placeholder) 2026-02-11 : 建国記念の日".to_string(),
            "(placeholder) 2026-05-03 to 2026-05-06 : 連休".to_string(),
        ];
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            file_label: file_label.into(),
        }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Apply a navigation [`Intent`]. Returns whether the app should keep
    /// looping or quit.
    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        match intent {
            Intent::Quit => ScreenAction::Quit,
            Intent::NavDown => {
                self.move_cursor(1);
                ScreenAction::Continue
            }
            Intent::NavUp => {
                self.move_cursor(-1);
                ScreenAction::Continue
            }
            Intent::NavTop => {
                if !self.items.is_empty() {
                    self.state.select(Some(0));
                }
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                if !self.items.is_empty() {
                    self.state.select(Some(self.items.len() - 1));
                }
                ScreenAction::Continue
            }
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        if self.items.is_empty() {
            return;
        }
        let last = self.items.len() - 1;
        let current = self.state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, last as i32) as usize;
        self.state.select(Some(next));
    }

    /// Render the screen into `frame`.
    pub fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, status_area] = layout.areas(frame.area());

        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|s| ListItem::new(s.as_str()))
            .collect();
        let list = List::new(list_items)
            .block(
                Block::default()
                    .title("lazyics — events")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, list_area, &mut self.state);

        let status = format!(
            "{}  |  {} event(s)  |  q quit  j/k move  g/G top/bottom",
            self.file_label,
            self.items.len()
        );
        frame.render_widget(Paragraph::new(status), status_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_selects_first_item() {
        let s = ListScreen::placeholder("test.ics");
        assert_eq!(s.selected(), Some(0));
        assert_eq!(s.item_count(), 3);
    }

    #[test]
    fn nav_down_advances_until_last() {
        let mut s = ListScreen::placeholder("test.ics");
        s.handle(Intent::NavDown);
        assert_eq!(s.selected(), Some(1));
        s.handle(Intent::NavDown);
        assert_eq!(s.selected(), Some(2));
        s.handle(Intent::NavDown); // saturates at last
        assert_eq!(s.selected(), Some(2));
    }

    #[test]
    fn nav_up_stops_at_zero() {
        let mut s = ListScreen::placeholder("test.ics");
        s.handle(Intent::NavUp);
        assert_eq!(s.selected(), Some(0));
    }

    #[test]
    fn nav_top_and_bottom_jump_to_ends() {
        let mut s = ListScreen::placeholder("test.ics");
        s.handle(Intent::NavBottom);
        assert_eq!(s.selected(), Some(s.item_count() - 1));
        s.handle(Intent::NavTop);
        assert_eq!(s.selected(), Some(0));
    }

    #[test]
    fn quit_intent_returns_quit_action() {
        let mut s = ListScreen::placeholder("test.ics");
        assert_eq!(s.handle(Intent::Quit), ScreenAction::Quit);
    }

    #[test]
    fn non_quit_intent_returns_continue() {
        let mut s = ListScreen::placeholder("test.ics");
        assert_eq!(s.handle(Intent::NavDown), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavTop), ScreenAction::Continue);
    }
}
