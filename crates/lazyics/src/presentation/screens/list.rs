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
use crate::presentation::widgets::TextInput;

#[derive(Debug, Clone)]
enum Mode {
    Browse,
    Remove {
        /// 0-based indices into the *original* `items` vec — stable
        /// across filter changes.
        marked: BTreeSet<usize>,
    },
    /// Search-as-you-type input. `draft` is the in-progress filter;
    /// `previous_filter` is the committed filter at the moment Search
    /// opened, restored on Cancel.
    Search {
        draft: TextInput,
        previous_filter: Option<String>,
    },
}

pub struct ListScreen {
    items: Vec<String>,
    state: ListState,
    file_label: String,
    mode: Mode,
    /// Committed filter (None = no filter). Lowercased substring match
    /// against `items` lines. Persists across Browse / Remove modes;
    /// modified by entering and committing Search mode.
    filter: Option<String>,
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
            filter: None,
            transient_status: None,
        }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Currently-selected position **within the filtered list** (not the
    /// raw `items` index). Use `selected_event_index()` for the 1-based
    /// original-domain index that the use cases expect.
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Committed filter (lowercased substring), or `None` if no filter.
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    /// 0-based indices into `items` that pass the active filter (the
    /// live `draft` while in Search mode, or the committed `filter`
    /// otherwise). With no filter, returns every index.
    fn filtered_indices(&self) -> Vec<usize> {
        let needle: String = match &self.mode {
            Mode::Search { draft, .. } => draft.value().to_lowercase(),
            _ => match &self.filter {
                Some(f) => f.clone(),
                None => return (0..self.items.len()).collect(),
            },
        };
        if needle.is_empty() {
            return (0..self.items.len()).collect();
        }
        self.items
            .iter()
            .enumerate()
            .filter(|(_, s)| s.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
    }

    /// 1-based original index of the currently-selected event, ready
    /// for `icscli::application::use_cases::{edit,remove}`. `None` if
    /// the filtered list is empty.
    fn selected_original_index(&self) -> Option<usize> {
        let filtered = self.filtered_indices();
        let sel = self.state.selected()?;
        filtered.get(sel).copied()
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

    /// Marked 0-based original indices. Empty outside Remove mode.
    /// Exposed for unit tests.
    #[cfg(test)]
    fn marked(&self) -> Vec<usize> {
        match &self.mode {
            Mode::Remove { marked } => marked.iter().copied().collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the screen is currently in Remove mode. Exposed for tests
    /// and for the status-bar render.
    pub fn is_remove_mode(&self) -> bool {
        matches!(self.mode, Mode::Remove { .. })
    }

    /// Whether the screen is currently in Search input mode. The Screen
    /// enum forwards this to `is_modal()` so the Composition Root knows
    /// to switch the keymap into Form mode (typed chars → TypeChar).
    pub fn is_search_mode(&self) -> bool {
        matches!(self.mode, Mode::Search { .. })
    }

    pub fn handle(&mut self, intent: Intent) -> ScreenAction {
        self.transient_status = None;
        if matches!(self.mode, Mode::Search { .. }) {
            return self.handle_search(intent);
        }
        self.handle_browse_or_remove(intent)
    }

    fn handle_search(&mut self, intent: Intent) -> ScreenAction {
        match intent {
            Intent::ForceQuit => ScreenAction::Quit,
            Intent::Cancel => {
                if let Mode::Search {
                    previous_filter, ..
                } = &self.mode
                {
                    self.filter = previous_filter.clone();
                }
                self.mode = Mode::Browse;
                self.clamp_selection();
                ScreenAction::Continue
            }
            Intent::SubmitForm => {
                if let Mode::Search { draft, .. } = &self.mode {
                    let v = draft.value().trim().to_lowercase();
                    self.filter = if v.is_empty() { None } else { Some(v) };
                }
                self.mode = Mode::Browse;
                self.clamp_selection();
                ScreenAction::Continue
            }
            Intent::TypeChar(c) => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.insert_char(c);
                }
                self.clamp_selection();
                ScreenAction::Continue
            }
            Intent::Backspace => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.backspace();
                }
                self.clamp_selection();
                ScreenAction::Continue
            }
            Intent::NavLeft => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.move_left();
                }
                ScreenAction::Continue
            }
            Intent::NavRight => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.move_right();
                }
                ScreenAction::Continue
            }
            Intent::NavTop => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.move_home();
                }
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                if let Mode::Search { draft, .. } = &mut self.mode {
                    draft.move_end();
                }
                ScreenAction::Continue
            }
            Intent::OpenHelp => ScreenAction::OpenHelp,
            // Everything else (Quit, NavUp/Down, NextField/PrevField,
            // OpenAdd/Edit/Remove/Search, CycleView, etc.) is a no-op
            // while typing the filter — the user is committing or
            // cancelling, not navigating views.
            _ => ScreenAction::Continue,
        }
    }

    fn handle_browse_or_remove(&mut self, intent: Intent) -> ScreenAction {
        match intent {
            Intent::Quit | Intent::ForceQuit => ScreenAction::Quit,
            Intent::Cancel => match self.mode {
                Mode::Browse => ScreenAction::Continue,
                Mode::Remove { .. } => {
                    self.mode = Mode::Browse;
                    ScreenAction::Continue
                }
                Mode::Search { .. } => unreachable!("handled by handle_search"),
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
                let count = self.filtered_indices().len();
                if count > 0 {
                    self.state.select(Some(0));
                }
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                let count = self.filtered_indices().len();
                if count > 0 {
                    self.state.select(Some(count - 1));
                }
                ScreenAction::Continue
            }
            Intent::OpenRemove => {
                if !self.filtered_indices().is_empty() && matches!(self.mode, Mode::Browse) {
                    self.mode = Mode::Remove {
                        marked: BTreeSet::new(),
                    };
                }
                ScreenAction::Continue
            }
            Intent::ToggleMark => {
                let Some(original) = self.selected_original_index() else {
                    return ScreenAction::Continue;
                };
                if let Mode::Remove { marked } = &mut self.mode {
                    if !marked.insert(original) {
                        marked.remove(&original);
                    }
                }
                ScreenAction::Continue
            }
            Intent::Confirm => match &self.mode {
                Mode::Remove { marked } if !marked.is_empty() => {
                    let indices: Vec<usize> = marked.iter().map(|i| i + 1).collect();
                    ScreenAction::RemoveByIndices(indices)
                }
                _ => ScreenAction::Continue,
            },
            Intent::OpenAdd => match self.mode {
                // Browse mode opens the form; Remove mode swallows `a`
                // so the user doesn't lose marks via a misfire.
                Mode::Browse => ScreenAction::OpenAdd {
                    start_hint: None,
                    end_hint: None,
                },
                Mode::Remove { .. } => ScreenAction::Continue,
                Mode::Search { .. } => unreachable!(),
            },
            Intent::OpenEdit => match self.mode {
                Mode::Browse => match self.selected_original_index() {
                    Some(original) => ScreenAction::OpenEdit {
                        event_index: original + 1,
                    },
                    None => ScreenAction::Continue,
                },
                _ => ScreenAction::Continue,
            },
            Intent::OpenSearch => match self.mode {
                Mode::Browse => {
                    let initial = self.filter.clone().unwrap_or_default();
                    self.mode = Mode::Search {
                        draft: TextInput::with_value(initial),
                        previous_filter: self.filter.clone(),
                    };
                    ScreenAction::Continue
                }
                // Remove mode swallows `/` so the user doesn't lose
                // marks via a misfire. They can Esc out first.
                _ => ScreenAction::Continue,
            },
            Intent::OpenHelp => ScreenAction::OpenHelp,
            Intent::NavLeft
            | Intent::NavRight
            | Intent::CycleGranularity
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::TypeChar(_)
            | Intent::Backspace
            | Intent::NextField
            | Intent::PrevField
            | Intent::SubmitForm
            | Intent::OpenMonthPicker
            | Intent::OpenYearPicker
            | Intent::ToggleVisualRange => ScreenAction::Continue,
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.state.select(None);
            return;
        }
        let last = (count - 1) as i32;
        let current = self.state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, last) as usize;
        self.state.select(Some(next));
    }

    /// After a filter change, the previous selected position may be out
    /// of range. Clamp to last valid filtered index, or drop selection
    /// entirely when the filter has no matches.
    fn clamp_selection(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.state.select(None);
        } else {
            let sel = self.state.selected().unwrap_or(0).min(count - 1);
            self.state.select(Some(sel));
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        // Search mode reserves the top row for the filter input. Browse
        // and Remove use the whole pane for the list.
        let in_search = matches!(self.mode, Mode::Search { .. });
        let layout = if in_search {
            Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
        } else {
            Layout::vertical([
                Constraint::Length(0),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
        };
        let [search_area, list_area, status_area] = layout.areas(frame.area());

        if let Mode::Search { draft, .. } = &self.mode {
            let block = Block::default()
                .title("Search (substring, case-insensitive)")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(search_area);
            frame.render_widget(block, search_area);
            draft.render(frame, inner, true);
        }

        let filtered = self.filtered_indices();
        let total = self.items.len();
        let visible = filtered.len();

        let title = match &self.mode {
            Mode::Browse => {
                if self.filter.is_some() {
                    format!("lazyics — events  ({visible} of {total})")
                } else {
                    "lazyics — events".to_string()
                }
            }
            Mode::Remove { marked } => format!("lazyics — REMOVE ({} marked)", marked.len()),
            Mode::Search { .. } => format!("lazyics — events  (live: {visible} of {total})"),
        };
        let block = Block::default().title(title).borders(Borders::ALL);

        if filtered.is_empty() {
            let inner = block.inner(list_area);
            frame.render_widget(block, list_area);
            let hint = if total == 0 {
                "No events.\n\nUse `icscli add --summary ... --start ...` to create one."
                    .to_string()
            } else {
                format!("No events match the filter ({total} total — Esc to clear).")
            };
            frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), inner);
        } else {
            let marked: BTreeSet<usize> = match &self.mode {
                Mode::Remove { marked } => marked.clone(),
                _ => BTreeSet::new(),
            };
            let in_remove = self.is_remove_mode();
            let list_items: Vec<ListItem> = filtered
                .iter()
                .map(|&original_idx| {
                    let text = &self.items[original_idx];
                    if in_remove {
                        let is_marked = marked.contains(&original_idx);
                        let prefix = if is_marked { "[x] " } else { "[ ] " };
                        let style = if is_marked {
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
            match &self.mode {
                Mode::Browse => match &self.filter {
                    Some(f) => format!(
                        "{}  |  [filter: {f}] {visible} of {total}  |  / search  Esc-via-/ clear",
                        self.file_label,
                    ),
                    None => format!(
                        "{}  |  {total} event(s)  |  / search  a add  e edit  d remove  ? help",
                        self.file_label,
                    ),
                },
                Mode::Remove { marked } => format!(
                    "REMOVE  |  {} marked / {visible} of {total}  |  space toggle  Enter confirm  Esc cancel",
                    marked.len(),
                ),
                Mode::Search { .. } => {
                    "Search  |  type to filter  Enter commit  Esc cancel".to_string()
                }
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
    fn cancel_in_browse_is_noop() {
        let mut s = ListScreen::from_events(&three_events(), "h.ics");
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::Continue);
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

    // --- Search mode ----------------------------------------------------

    fn distinct_summaries() -> Vec<VEvent> {
        vec![
            make_event((2026, 1, 1), (2026, 1, 2), "元日"),
            make_event((2026, 2, 11), (2026, 2, 12), "建国記念の日"),
            make_event((2026, 5, 3), (2026, 5, 7), "連休"),
            make_event((2026, 5, 10), (2026, 5, 11), "Travel"),
        ]
    }

    #[test]
    fn open_search_enters_search_mode_and_is_modal() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        assert!(!s.is_search_mode());
        s.handle(Intent::OpenSearch);
        assert!(s.is_search_mode());
    }

    #[test]
    fn open_search_is_noop_in_remove_mode() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenRemove);
        s.handle(Intent::OpenSearch);
        assert!(s.is_remove_mode());
        assert!(!s.is_search_mode());
    }

    #[test]
    fn typing_in_search_live_filters() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        for c in "trav".chars() {
            s.handle(Intent::TypeChar(c));
        }
        // 4 events, only "Travel" matches "trav" (case-insensitive).
        assert_eq!(s.filtered_indices().len(), 1);
    }

    #[test]
    fn submit_commits_filter_and_returns_to_browse() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        for c in "連".chars() {
            s.handle(Intent::TypeChar(c));
        }
        s.handle(Intent::SubmitForm);
        assert!(!s.is_search_mode());
        assert_eq!(s.filter(), Some("連"));
        assert_eq!(s.filtered_indices().len(), 1);
    }

    #[test]
    fn cancel_restores_previous_filter() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        // First commit "連" as the baseline filter.
        s.handle(Intent::OpenSearch);
        s.handle(Intent::TypeChar('連'));
        s.handle(Intent::SubmitForm);
        assert_eq!(s.filter(), Some("連"));
        // Re-enter Search, change draft, then Cancel — original sticks.
        s.handle(Intent::OpenSearch);
        for c in "trav".chars() {
            s.handle(Intent::TypeChar(c));
        }
        s.handle(Intent::Cancel);
        assert!(!s.is_search_mode());
        assert_eq!(s.filter(), Some("連"));
    }

    #[test]
    fn empty_submit_clears_filter() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        s.handle(Intent::TypeChar('連'));
        s.handle(Intent::SubmitForm);
        assert_eq!(s.filter(), Some("連"));
        // Re-enter search; the prior filter pre-populates the draft.
        // Backspace-clear it then commit empty — filter is cleared.
        s.handle(Intent::OpenSearch);
        s.handle(Intent::Backspace);
        s.handle(Intent::SubmitForm);
        assert!(s.filter().is_none());
    }

    #[test]
    fn nav_in_filtered_browse_stays_within_filtered_count() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        s.handle(Intent::TypeChar('日'));
        s.handle(Intent::SubmitForm);
        // 元日 and 建国記念の日 both contain 日.
        assert_eq!(s.filtered_indices().len(), 2);
        s.handle(Intent::NavBottom);
        assert_eq!(s.selected(), Some(1));
        s.handle(Intent::NavDown); // saturates
        assert_eq!(s.selected(), Some(1));
    }

    #[test]
    fn open_edit_reports_original_index_not_filtered() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        s.handle(Intent::TypeChar('連')); // matches index 2 only ("連休")
        s.handle(Intent::SubmitForm);
        assert_eq!(s.filtered_indices(), vec![2]);
        match s.handle(Intent::OpenEdit) {
            ScreenAction::OpenEdit { event_index } => assert_eq!(event_index, 3), // 1-based original
            other => panic!("expected OpenEdit{{3}}, got {other:?}"),
        }
    }

    #[test]
    fn toggle_mark_uses_original_index_under_filter() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        s.handle(Intent::TypeChar('日'));
        s.handle(Intent::SubmitForm);
        // Filtered: [0, 1] (元日 at 0, 建国記念の日 at 1).
        s.handle(Intent::OpenRemove);
        s.handle(Intent::ToggleMark); // selected = filtered[0] = original 0
        assert_eq!(s.marked(), vec![0]);
        s.handle(Intent::NavDown);
        s.handle(Intent::ToggleMark); // selected = filtered[1] = original 1
        assert_eq!(s.marked(), vec![0, 1]);
        // Confirm produces 1-based original indices.
        match s.handle(Intent::Confirm) {
            ScreenAction::RemoveByIndices(idx) => assert_eq!(idx, vec![1, 2]),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn empty_filter_match_clamps_selection_to_none() {
        let mut s = ListScreen::from_events(&distinct_summaries(), "h.ics");
        s.handle(Intent::OpenSearch);
        for c in "zzzzz".chars() {
            s.handle(Intent::TypeChar(c));
        }
        assert_eq!(s.filtered_indices().len(), 0);
        assert_eq!(s.selected(), None);
    }
}
