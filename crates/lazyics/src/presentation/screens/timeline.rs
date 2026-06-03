//! Timeline screen — chronological vertical scroll grouped by month or week.
//!
//! Events are sorted by `dtstart` and rendered with one header per group
//! (the group key being the month or ISO-week, controlled by the active
//! [`Granularity`]). The cursor only lands on event rows; pressing
//! [`Intent::CycleGranularity`] (`u`) toggles month ↔ week and preserves
//! the cursor on the previously-selected event when possible.

use chrono::Datelike;
use ics_core::VEvent;
use icscli::application::ports::CalendarRepository;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::error::Result;
use crate::presentation::keymap::Intent;
use crate::presentation::screens::ScreenAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Month,
    Week,
}

impl Granularity {
    pub fn cycle(self) -> Self {
        match self {
            Granularity::Month => Granularity::Week,
            Granularity::Week => Granularity::Month,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Granularity::Month => "month",
            Granularity::Week => "week",
        }
    }
}

#[derive(Debug, Clone)]
enum Row {
    Header(String),
    Event { uid: String, text: String },
}

pub struct TimelineScreen {
    events: Vec<VEvent>,
    rows: Vec<Row>,
    /// Indices into `rows` that point at `Row::Event`.
    event_row_indices: Vec<usize>,
    /// Index into `event_row_indices` (i.e. which event is selected).
    /// `None` when there are no events.
    selected_event: Option<usize>,
    state: ListState,
    file_label: String,
    granularity: Granularity,
    transient_status: Option<String>,
}

impl TimelineScreen {
    pub fn from_events(events: &[VEvent], file_label: impl Into<String>) -> Self {
        let mut events_sorted = events.to_vec();
        events_sorted.sort_by_key(|e| (e.dtstart, e.dtend));
        let granularity = Granularity::Month;
        let (rows, event_row_indices) = build_rows(&events_sorted, granularity);
        let mut state = ListState::default();
        let selected_event = if event_row_indices.is_empty() {
            None
        } else {
            state.select(Some(event_row_indices[0]));
            Some(0)
        };
        Self {
            events: events_sorted,
            rows,
            event_row_indices,
            selected_event,
            state,
            file_label: file_label.into(),
            granularity,
            transient_status: None,
        }
    }

    pub fn from_repo<R: CalendarRepository>(
        repo: &R,
        file_label: impl Into<String>,
    ) -> Result<Self> {
        let cal = repo.load()?;
        Ok(Self::from_events(&cal.events, file_label))
    }

    pub fn event_count(&self) -> usize {
        self.event_row_indices.len()
    }

    /// 0-based index of the currently-selected event (across all groups).
    pub fn selected_event_index(&self) -> Option<usize> {
        self.selected_event
    }

    pub fn granularity(&self) -> Granularity {
        self.granularity
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
            Intent::Quit | Intent::ForceQuit => ScreenAction::Quit,
            Intent::Cancel => ScreenAction::Continue,
            Intent::NavDown => {
                self.move_event_cursor(1);
                ScreenAction::Continue
            }
            Intent::NavUp => {
                self.move_event_cursor(-1);
                ScreenAction::Continue
            }
            Intent::NavTop => {
                self.select_event(0);
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                if !self.event_row_indices.is_empty() {
                    self.select_event(self.event_row_indices.len() - 1);
                }
                ScreenAction::Continue
            }
            Intent::CycleGranularity => {
                self.set_granularity(self.granularity.cycle());
                ScreenAction::Continue
            }
            Intent::OpenHelp => ScreenAction::OpenHelp,
            Intent::OpenRemove
            | Intent::OpenAdd
            | Intent::OpenEdit
            | Intent::ToggleMark
            | Intent::Confirm
            | Intent::NavLeft
            | Intent::NavRight
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::TypeChar(_)
            | Intent::Backspace
            | Intent::NextField
            | Intent::PrevField
            | Intent::SubmitForm => ScreenAction::Continue,
        }
    }

    fn move_event_cursor(&mut self, delta: i32) {
        let Some(cur) = self.selected_event else {
            return;
        };
        let last = self.event_row_indices.len() as i32 - 1;
        let next = (cur as i32 + delta).clamp(0, last) as usize;
        self.select_event(next);
    }

    fn select_event(&mut self, idx: usize) {
        if let Some(row_idx) = self.event_row_indices.get(idx).copied() {
            self.selected_event = Some(idx);
            self.state.select(Some(row_idx));
        }
    }

    fn set_granularity(&mut self, granularity: Granularity) {
        let previously_selected_uid: Option<String> = self
            .selected_event
            .and_then(|i| self.event_row_indices.get(i).copied())
            .and_then(|row_idx| {
                if let Row::Event { uid, .. } = &self.rows[row_idx] {
                    Some(uid.clone())
                } else {
                    None
                }
            });

        self.granularity = granularity;
        let (rows, event_row_indices) = build_rows(&self.events, granularity);
        self.rows = rows;
        self.event_row_indices = event_row_indices;

        // Re-anchor selection on the same event UID after regrouping.
        let new_selected = previously_selected_uid.and_then(|uid| {
            self.event_row_indices.iter().position(
                |&row_idx| matches!(&self.rows[row_idx], Row::Event { uid: u, .. } if u == &uid),
            )
        });
        match new_selected {
            Some(idx) => self.select_event(idx),
            None if !self.event_row_indices.is_empty() => self.select_event(0),
            None => {
                self.selected_event = None;
                self.state.select(None);
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [body, status] = layout.areas(frame.area());

        let block = Block::default()
            .title(format!("lazyics — timeline ({})", self.granularity.label()))
            .borders(Borders::ALL);

        if self.events.is_empty() {
            let inner = block.inner(body);
            frame.render_widget(block, body);
            let hint = Paragraph::new("No events.\n\nPress 1 for List, 2 Timeline, 3 Grid.")
                .alignment(Alignment::Center);
            frame.render_widget(hint, inner);
        } else {
            let items: Vec<ListItem> = self
                .rows
                .iter()
                .map(|r| match r {
                    Row::Header(label) => ListItem::new(Line::raw(label.clone()))
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                    Row::Event { text, .. } => ListItem::new(Line::raw(text.clone())),
                })
                .collect();
            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body, &mut self.state);
        }

        let status_text = self.transient_status.clone().unwrap_or_else(|| {
            format!(
                "{}  |  {} event(s)  |  {} unit  |  Tab/1/2/3 view  u unit  q quit",
                self.file_label,
                self.event_count(),
                self.granularity.label(),
            )
        });
        frame.render_widget(Paragraph::new(status_text), status);
    }
}

fn build_rows(events: &[VEvent], granularity: Granularity) -> (Vec<Row>, Vec<usize>) {
    let mut rows = Vec::new();
    let mut event_row_indices = Vec::new();
    let mut current_group: Option<String> = None;

    for event in events {
        let group = group_label(event.dtstart, granularity);
        if current_group.as_deref() != Some(&group) {
            rows.push(Row::Header(group.clone()));
            current_group = Some(group);
        }
        let text = format_event_inline(event);
        event_row_indices.push(rows.len());
        rows.push(Row::Event {
            uid: event.uid.clone(),
            text,
        });
    }

    (rows, event_row_indices)
}

fn group_label(date: chrono::NaiveDate, gran: Granularity) -> String {
    match gran {
        Granularity::Month => date.format("%Y-%m  %B %Y").to_string(),
        Granularity::Week => {
            // ISO weeks start on Monday. Compute the Monday of the date's
            // week so weeks are stable across granularity flips.
            let weekday = date.weekday().num_days_from_monday();
            let monday = date - chrono::Duration::days(weekday as i64);
            format!("Week of {}", monday.format("%Y-%m-%d"))
        }
    }
}

fn format_event_inline(event: &VEvent) -> String {
    let start = event.dtstart;
    // dtend is RFC-exclusive (+1 day from inclusive end), per ics-core.
    let end_inclusive = event.dtend - chrono::Days::new(1);
    if start == end_inclusive {
        format!("  {}  {}", start.format("%m-%d"), event.summary)
    } else {
        format!(
            "  {} ─ {}  {}",
            start.format("%m-%d"),
            end_inclusive.format("%m-%d"),
            event.summary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_event(start: (i32, u32, u32), end: (i32, u32, u32), summary: &str) -> VEvent {
        let dtstamp = NaiveDate::from_ymd_opt(2026, 6, 3)
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

    fn three_events_in_three_months() -> Vec<VEvent> {
        vec![
            make_event((2026, 1, 1), (2026, 1, 2), "元日"),
            make_event((2026, 2, 11), (2026, 2, 12), "建国"),
            make_event((2026, 5, 3), (2026, 5, 7), "連休"),
        ]
    }

    #[test]
    fn empty_events_yields_no_selection() {
        let s = TimelineScreen::from_events(&[], "h.ics");
        assert_eq!(s.event_count(), 0);
        assert_eq!(s.selected_event_index(), None);
    }

    #[test]
    fn from_events_starts_on_first_event() {
        let s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        assert_eq!(s.event_count(), 3);
        assert_eq!(s.selected_event_index(), Some(0));
        assert_eq!(s.granularity(), Granularity::Month);
    }

    #[test]
    fn nav_down_advances_across_groups() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        s.handle(Intent::NavDown);
        assert_eq!(s.selected_event_index(), Some(1));
        s.handle(Intent::NavDown);
        assert_eq!(s.selected_event_index(), Some(2));
        s.handle(Intent::NavDown); // saturates at last
        assert_eq!(s.selected_event_index(), Some(2));
    }

    #[test]
    fn nav_top_and_bottom_jump_to_ends() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        s.handle(Intent::NavBottom);
        assert_eq!(s.selected_event_index(), Some(2));
        s.handle(Intent::NavTop);
        assert_eq!(s.selected_event_index(), Some(0));
    }

    #[test]
    fn cycle_granularity_toggles_month_and_week() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        assert_eq!(s.granularity(), Granularity::Month);
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.granularity(), Granularity::Week);
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.granularity(), Granularity::Month);
    }

    #[test]
    fn cycle_granularity_preserves_selection_by_uid() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        s.handle(Intent::NavDown); // select event index 1 (建国)
        assert_eq!(s.selected_event_index(), Some(1));
        s.handle(Intent::CycleGranularity);
        // Same event remains selected even though groups differ.
        assert_eq!(s.selected_event_index(), Some(1));
    }

    #[test]
    fn quit_intent_returns_quit_cancel_is_noop() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        assert_eq!(s.handle(Intent::Quit), ScreenAction::Quit);
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::Continue);
    }

    #[test]
    fn list_specific_intents_are_no_ops() {
        let mut s = TimelineScreen::from_events(&three_events_in_three_months(), "h.ics");
        assert_eq!(s.handle(Intent::OpenRemove), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::ToggleMark), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::Confirm), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavLeft), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavRight), ScreenAction::Continue);
    }

    #[test]
    fn events_in_same_month_share_one_header() {
        let events = vec![
            make_event((2026, 1, 1), (2026, 1, 2), "a"),
            make_event((2026, 1, 15), (2026, 1, 16), "b"),
            make_event((2026, 2, 1), (2026, 2, 2), "c"),
        ];
        let s = TimelineScreen::from_events(&events, "h.ics");
        let header_count = s
            .rows
            .iter()
            .filter(|r| matches!(r, Row::Header(_)))
            .count();
        assert_eq!(header_count, 2); // January, February
    }
}
