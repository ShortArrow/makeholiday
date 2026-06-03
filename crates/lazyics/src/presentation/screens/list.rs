//! Event list screen.
//!
//! Phase 3a introduces a Browse/Remove mode state machine. In Browse mode
//! the screen behaves as before (`j`/`k`/`g`/`G` nav, `q` quit). Pressing
//! `d` or `x` opens Remove mode: space marks/unmarks rows, Enter or `D`
//! confirms (the screen returns a [`ScreenAction::RemoveByIndices`] which
//! the Composition Root submits to `icscli::application::use_cases::remove`),
//! Esc discards marks and returns to Browse.

use std::collections::BTreeSet;

use ics_core::VEvent;
use icscli::application::ports::CalendarRepository;
use icscli::display::format_event_line;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::error::Result;
use crate::presentation::keymap::Intent;
use crate::presentation::screens::ScreenAction;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Browse,
    Remove { marked: BTreeSet<usize> },
}

pub struct ListScreen {
    items: Vec<String>,
    state: ListState,
    file_label: String,
    mode: Mode,
    /// A one-shot message (e.g. "Removed 3 event(s).") rendered on the
    /// status bar until the next user interaction clears it.
    transient_status: Option<String>,
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
            mode: Mode::Browse,
            transient_status: None,
        }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// The path-as-displayed (passed in at construction). Composition Root
    /// reuses this when rebuilding the screen after a successful remove.
    pub fn file_label(&self) -> &str {
        &self.file_label
    }

    /// Set a one-shot status message that the next render will display on
    /// the status bar. Cleared by the next intent.
    pub fn set_transient_status(&mut self, msg: impl Into<String>) {
        self.transient_status = Some(msg.into());
    }

    /// Marked 0-based indices in Remove mode. Empty in Browse mode.
    /// Exposed for unit tests.
    #[cfg(test)]
    fn marked(&self) -> Vec<usize> {
        match &self.mode {
            Mode::Browse => Vec::new(),
            Mode::Remove { marked } => marked.iter().copied().collect(),
        }
    }

    /// Whether the screen is currently in Remove mode. Exposed for tests
    /// and for the status-bar render.
    pub fn is_remove_mode(&self) -> bool {
        matches!(self.mode, Mode::Remove { .. })
    }

    /// Apply an [`Intent`]. Returns whether the app should keep looping,
    /// quit, or submit a remove request to the Composition Root.
    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        // Any user interaction clears the previous status message — except
        // when the new intent itself produces a fresh one (handled by
        // Composition Root, which sets it after `handle` returns).
        self.transient_status = None;

        match intent {
            Intent::Quit | Intent::ForceQuit => ScreenAction::Quit,
            Intent::Cancel => match self.mode {
                // Esc at the top level quits, matching the original
                // Phase 1 behavior (q/Ctrl+C still work too).
                Mode::Browse => ScreenAction::Quit,
                // Esc in Remove mode discards marks and returns to Browse.
                Mode::Remove { .. } => {
                    self.mode = Mode::Browse;
                    ScreenAction::Continue
                }
            },
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
            Intent::OpenRemove => {
                if !self.items.is_empty() && matches!(self.mode, Mode::Browse) {
                    self.mode = Mode::Remove {
                        marked: BTreeSet::new(),
                    };
                }
                ScreenAction::Continue
            }
            Intent::ToggleMark => {
                if let Mode::Remove { marked } = &mut self.mode {
                    if let Some(idx) = self.state.selected() {
                        if !marked.insert(idx) {
                            marked.remove(&idx);
                        }
                    }
                }
                ScreenAction::Continue
            }
            Intent::Confirm => match &self.mode {
                Mode::Remove { marked } if !marked.is_empty() => {
                    // Convert 0-based to 1-based for `icscli`'s index spec.
                    let indices: Vec<usize> = marked.iter().map(|i| i + 1).collect();
                    ScreenAction::RemoveByIndices(indices)
                }
                _ => ScreenAction::Continue,
            },
            // 'a' opens the Add form. List is the only view that hosts
            // forms per ADR-025 §"Multi-view amendment".
            Intent::OpenAdd => {
                if matches!(self.mode, Mode::Browse) {
                    ScreenAction::OpenAdd
                } else {
                    // In Remove mode, swallow 'a' to avoid the user
                    // losing their marks via a misfire.
                    ScreenAction::Continue
                }
            }
            // 'e' opens the Edit form on the selected event. Browse mode
            // only; a no-op when no row is selected.
            Intent::OpenEdit => match (&self.mode, self.state.selected()) {
                (Mode::Browse, Some(idx)) => ScreenAction::OpenEdit {
                    event_index: idx + 1,
                },
                _ => ScreenAction::Continue,
            },
            // '?' opens the help overlay regardless of mode.
            Intent::OpenHelp => ScreenAction::OpenHelp,
            // List view has a single column and no granularity — these
            // intents are meaningful in Grid / Timeline / forms only.
            Intent::NavLeft
            | Intent::NavRight
            | Intent::CycleGranularity
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::TypeChar(_)
            | Intent::Backspace
            | Intent::NextField
            | Intent::PrevField
            | Intent::SubmitForm => ScreenAction::Continue,
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

        let title = match &self.mode {
            Mode::Browse => "lazyics — events".to_string(),
            Mode::Remove { marked } => format!("lazyics — REMOVE ({} marked)", marked.len()),
        };
        let block = Block::default().title(title).borders(Borders::ALL);

        if self.items.is_empty() {
            let inner = block.inner(list_area);
            frame.render_widget(block, list_area);
            let hint = Paragraph::new(
                "No events.\n\nUse `icscli add --summary ... --start ...` to create one.",
            )
            .alignment(Alignment::Center);
            frame.render_widget(hint, inner);
        } else {
            let marked: BTreeSet<usize> = match &self.mode {
                Mode::Remove { marked } => marked.clone(),
                Mode::Browse => BTreeSet::new(),
            };
            let list_items: Vec<ListItem> = self
                .items
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    if self.is_remove_mode() {
                        let prefix = if marked.contains(&i) { "[x] " } else { "[ ] " };
                        let style = if marked.contains(&i) {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default()
                        };
                        ListItem::new(format!("{prefix}{text}")).style(style)
                    } else {
                        ListItem::new(text.as_str())
                    }
                })
                .collect();
            let list = List::new(list_items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, list_area, &mut self.state);
        }

        let status_text = self.transient_status.clone().unwrap_or_else(|| {
            let count = self.items.len();
            match &self.mode {
                Mode::Browse => format!(
                    "{}  |  {} event(s)  |  q quit  j/k move  g/G top/bottom  d remove",
                    self.file_label, count,
                ),
                Mode::Remove { marked } => format!(
                    "REMOVE  |  {} marked / {} total  |  space toggle  Enter confirm  Esc cancel",
                    marked.len(),
                    count,
                ),
            }
        });
        frame.render_widget(Paragraph::new(status_text), status_area);
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

    fn three_events() -> Vec<VEvent> {
        vec![
            make_event((2026, 1, 1), (2026, 1, 2), "a"),
            make_event((2026, 2, 11), (2026, 2, 12), "b"),
            make_event((2026, 5, 3), (2026, 5, 7), "c"),
        ]
    }

    #[test]
    fn from_events_formats_each_row_via_icscli_display() {
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

    // --- Remove mode transitions ---------------------------------------

    #[test]
    fn open_remove_enters_remove_mode_when_non_empty() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        assert!(!s.is_remove_mode());
        s.handle(Intent::OpenRemove);
        assert!(s.is_remove_mode());
    }

    #[test]
    fn open_remove_is_noop_on_empty_screen() {
        let mut s = ListScreen::from_events(&[], "h.ics");
        s.handle(Intent::OpenRemove);
        assert!(!s.is_remove_mode());
    }

    #[test]
    fn cancel_in_browse_quits() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::Quit);
    }

    #[test]
    fn cancel_in_remove_returns_to_browse_and_clears_marks() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.handle(Intent::OpenRemove);
        s.handle(Intent::ToggleMark); // mark row 0
        assert_eq!(s.marked(), vec![0]);
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::Continue);
        assert!(!s.is_remove_mode());
        assert_eq!(s.marked(), Vec::<usize>::new());
    }

    #[test]
    fn toggle_mark_in_browse_is_noop() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.handle(Intent::ToggleMark);
        assert_eq!(s.marked(), Vec::<usize>::new());
    }

    #[test]
    fn toggle_mark_in_remove_adds_then_removes() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.handle(Intent::OpenRemove);
        s.handle(Intent::ToggleMark); // mark 0
        assert_eq!(s.marked(), vec![0]);
        s.handle(Intent::NavDown);
        s.handle(Intent::ToggleMark); // mark 1
        assert_eq!(s.marked(), vec![0, 1]);
        s.handle(Intent::NavUp);
        s.handle(Intent::ToggleMark); // unmark 0
        assert_eq!(s.marked(), vec![1]);
    }

    #[test]
    fn confirm_with_no_marks_is_noop() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.handle(Intent::OpenRemove);
        assert_eq!(s.handle(Intent::Confirm), ScreenAction::Continue);
        assert!(s.is_remove_mode());
    }

    #[test]
    fn confirm_with_marks_returns_remove_by_indices_one_based() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.handle(Intent::OpenRemove);
        s.handle(Intent::ToggleMark); // mark 0
        s.handle(Intent::NavDown);
        s.handle(Intent::NavDown);
        s.handle(Intent::ToggleMark); // mark 2
        assert_eq!(
            s.handle(Intent::Confirm),
            ScreenAction::RemoveByIndices(vec![1, 3])
        );
    }

    #[test]
    fn confirm_in_browse_mode_is_noop() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        assert_eq!(s.handle(Intent::Confirm), ScreenAction::Continue);
    }

    #[test]
    fn transient_status_is_cleared_on_next_intent() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        s.set_transient_status("Removed 1 event(s).");
        s.handle(Intent::NavDown);
        assert_eq!(s.transient_status, None);
    }
}
