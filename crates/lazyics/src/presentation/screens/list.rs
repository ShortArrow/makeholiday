//! Event list screen.
//!
//! Phase 2 introduces `from_events` (typed VEvent slice) and `from_repo`
//! (any `CalendarRepository`) constructors so the screen renders real
//! calendar data via `icscli::display::format_event_line`. `placeholder`
//! is retained for the keymap/render unit tests, which would otherwise
//! need to fabricate VEvents to exercise navigation.

use ics_core::VEvent;
use icscli::application::ports::CalendarRepository;
use icscli::display::format_event_line;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::error::Result;
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
    /// Build a screen from already-typed events. Uses `icscli`'s
    /// `format_event_line` so the row format matches the CLI's `list`
    /// output exactly.
    pub fn from_events(events: &[VEvent], file_label: impl Into<String>) -> Self {
        let items: Vec<String> = events.iter().map(format_event_line).collect();
        Self::with_items(items, file_label.into())
    }

    /// Build a screen by loading from a `CalendarRepository` (typically
    /// `FileCalendarRepository`). Parse / I/O failures surface as
    /// `LazyicsError::UseCase`.
    pub fn from_repo<R: CalendarRepository>(
        repo: &R,
        file_label: impl Into<String>,
    ) -> Result<Self> {
        let cal = repo.load()?;
        Ok(Self::from_events(&cal.events, file_label))
    }

    /// Phase 1 dummy-data constructor, retained so keymap/render unit tests
    /// don't need to fabricate VEvents. Not exercised by the binary.
    pub fn placeholder(file_label: impl Into<String>) -> Self {
        let items = vec![
            "(placeholder) 2026-01-01 : 元日".to_string(),
            "(placeholder) 2026-02-11 : 建国記念の日".to_string(),
            "(placeholder) 2026-05-03 to 2026-05-06 : 連休".to_string(),
        ];
        Self::with_items(items, file_label.into())
    }

    fn with_items(items: Vec<String>, file_label: String) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            file_label,
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

        let block = Block::default()
            .title("lazyics — events")
            .borders(Borders::ALL);

        if self.items.is_empty() {
            // Render an empty-state hint inside the bordered block. The
            // List widget would render empty rows; this is a friendlier
            // landing pad for newly-`init`-ed calendars.
            let inner = block.inner(list_area);
            frame.render_widget(block, list_area);
            let hint = Paragraph::new(
                "No events.\n\nUse `icscli add --summary ... --start ...` to create one.",
            )
            .alignment(Alignment::Center);
            frame.render_widget(hint, inner);
        } else {
            let list_items: Vec<ListItem> = self
                .items
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();
            let list = List::new(list_items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, list_area, &mut self.state);
        }

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
    use chrono::NaiveDate;
    use ics_core::VEvent;

    fn make_event(start: (i32, u32, u32), end: (i32, u32, u32), summary: &str) -> VEvent {
        let dtstamp = NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        VEvent {
            uid: format!("uid-{summary}"),
            dtstamp,
            dtstart: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            dtend: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            summary: summary.to_string(),
            transp: None,
            class: None,
            categories: vec![],
            microsoft: None,
            google: None,
            icloud: None,
            unknown: vec![],
            unrecognized_components: vec![],
        }
    }

    #[test]
    fn from_events_formats_each_row_via_icscli_display() {
        // dtend is RFC-exclusive (+1 day from inclusive end), per ics-core.
        let events = vec![
            make_event((2026, 1, 1), (2026, 1, 2), "元日"),
            make_event((2026, 12, 29), (2027, 1, 4), "年末年始"),
        ];
        let s = ListScreen::from_events(&events, "h.ics");
        assert_eq!(s.item_count(), 2);
        assert_eq!(s.selected(), Some(0));
    }

    #[test]
    fn from_events_empty_has_no_selection() {
        let s = ListScreen::from_events(&[], "empty.ics");
        assert_eq!(s.item_count(), 0);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn nav_intents_are_no_ops_on_empty_screen() {
        let mut s = ListScreen::from_events(&[], "empty.ics");
        assert_eq!(s.handle(Intent::NavDown), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavTop), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavBottom), ScreenAction::Continue);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn from_repo_loads_events_through_calendar_repository() {
        use icscli::application::use_cases::{RunContext, add, init};
        use icscli::infrastructure::FileCalendarRepository;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("h.ics");
        let repo = FileCalendarRepository::new(path.clone());
        init(&repo).unwrap();
        add(
            &repo,
            RunContext::default(),
            Some("元日"),
            Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            None,
            ics_core::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            None,
        )
        .unwrap();

        let s = ListScreen::from_repo(&repo, path.display().to_string()).unwrap();
        assert_eq!(s.item_count(), 1);
        assert_eq!(s.selected(), Some(0));
    }

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
