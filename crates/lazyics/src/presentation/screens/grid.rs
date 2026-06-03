//! Grid screen — calendar grid with a date-cursor.
//!
//! Month granularity (default): a 7×6 grid showing the cursor's month
//! with day-of-week headers. Out-of-month cells are dimmed; cells with
//! at least one event show a "•" marker. The cursor cell is reverse-
//! highlighted.
//!
//! Week granularity: a single 7-cell row of the cursor's week. `j`/`k`
//! step through weeks; `h`/`l` step through days; `u` cycles back to
//! month granularity.
//!
//! Cursor `today` defaults via `from_events` (which calls
//! [`chrono::Local::now`]) but the testable [`from_events_with_today`]
//! accepts an explicit anchor so unit tests stay deterministic.

use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use ics_core::VEvent;
use icscli::application::ports::CalendarRepository;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

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

pub struct GridScreen {
    events: Vec<VEvent>,
    /// All dates each event covers — start..=end_inclusive — pre-indexed
    /// so cell-event lookup during render is O(log N) per cell.
    events_by_date: BTreeMap<NaiveDate, Vec<usize>>,
    cursor: NaiveDate,
    granularity: Granularity,
    file_label: String,
    transient_status: Option<String>,
}

impl GridScreen {
    /// Build the screen using the system's local date as `today`. Use
    /// [`from_events_with_today`] in tests for determinism.
    pub fn from_events(events: &[VEvent], file_label: impl Into<String>) -> Self {
        let today = chrono::Local::now().date_naive();
        Self::from_events_with_today(events, file_label, today)
    }

    pub fn from_events_with_today(
        events: &[VEvent],
        file_label: impl Into<String>,
        today: NaiveDate,
    ) -> Self {
        let events: Vec<VEvent> = events.to_vec();
        let events_by_date = index_events_by_date(&events);
        // Cursor preference: today if any event covers today, else today
        // verbatim, else (no events) today. Either way the user can navigate.
        let cursor = today;
        Self {
            events,
            events_by_date,
            cursor,
            granularity: Granularity::Month,
            file_label: file_label.into(),
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

    pub fn cursor(&self) -> NaiveDate {
        self.cursor
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
            Intent::Quit | Intent::Cancel => ScreenAction::Quit,
            Intent::NavLeft => {
                self.cursor = self.cursor - Days::new(1);
                ScreenAction::Continue
            }
            Intent::NavRight => {
                self.cursor = self.cursor + Days::new(1);
                ScreenAction::Continue
            }
            Intent::NavUp => {
                self.cursor = self.cursor - Days::new(7);
                ScreenAction::Continue
            }
            Intent::NavDown => {
                self.cursor = self.cursor + Days::new(7);
                ScreenAction::Continue
            }
            Intent::NavTop => {
                self.cursor = match self.granularity {
                    Granularity::Month => first_of_month(self.cursor),
                    Granularity::Week => monday_of(self.cursor),
                };
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                self.cursor = match self.granularity {
                    Granularity::Month => last_of_month(self.cursor),
                    Granularity::Week => sunday_of(self.cursor),
                };
                ScreenAction::Continue
            }
            Intent::CycleGranularity => {
                self.granularity = self.granularity.cycle();
                ScreenAction::Continue
            }
            // Phase 4a: Remove / Add / mark / confirm belong to List view
            // only. Form-mode intents are unreachable here (Grid is never
            // the active screen in Form keymap mode) but listing them
            // keeps the match exhaustive.
            Intent::OpenRemove
            | Intent::OpenAdd
            | Intent::OpenEdit
            | Intent::ToggleMark
            | Intent::Confirm
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::TypeChar(_)
            | Intent::Backspace
            | Intent::NextField
            | Intent::PrevField
            | Intent::SubmitForm => ScreenAction::Continue,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ]);
        let [grid_area, detail_area, status_area] = layout.areas(frame.area());

        let title = match self.granularity {
            Granularity::Month => format!("lazyics — grid ({})", self.cursor.format("%Y-%m")),
            Granularity::Week => format!(
                "lazyics — grid (week of {})",
                monday_of(self.cursor).format("%Y-%m-%d")
            ),
        };
        let block = Block::default().title(title).borders(Borders::ALL);
        let grid_inner = block.inner(grid_area);
        frame.render_widget(block, grid_area);

        let lines = self.render_grid_lines();
        frame.render_widget(Paragraph::new(lines), grid_inner);

        // Detail panel: events on cursor date.
        let detail_block = Block::default()
            .title(format!("Events on {}", self.cursor.format("%Y-%m-%d")))
            .borders(Borders::ALL);
        let detail_inner = detail_block.inner(detail_area);
        frame.render_widget(detail_block, detail_area);

        let detail_text = self
            .events_by_date
            .get(&self.cursor)
            .map(|indices| {
                indices
                    .iter()
                    .map(|&i| format!("• {}", self.events[i].summary))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "(no events)".to_string());
        frame.render_widget(Paragraph::new(detail_text), detail_inner);

        let status_text = self.transient_status.clone().unwrap_or_else(|| {
            format!(
                "{}  |  {} unit  |  Tab/1/2/3 view  h/l day  j/k week  u unit  q quit",
                self.file_label,
                self.granularity.label(),
            )
        });
        frame.render_widget(Paragraph::new(status_text), status_area);
    }

    fn render_grid_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        lines.push(Line::raw(" Mon  Tue  Wed  Thu  Fri  Sat  Sun"));

        let weeks = match self.granularity {
            Granularity::Month => 6,
            Granularity::Week => 1,
        };

        let start = match self.granularity {
            Granularity::Month => monday_of(first_of_month(self.cursor)),
            Granularity::Week => monday_of(self.cursor),
        };
        let anchor_month = self.cursor.month();

        for week in 0..weeks {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for d in 0..7 {
                let date = start + Days::new(week * 7 + d);
                let cell = format_cell(date, &self.events_by_date);
                let mut style = Style::default();
                if self.granularity == Granularity::Month && date.month() != anchor_month {
                    style = style.add_modifier(Modifier::DIM);
                }
                if date == self.cursor {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                if date == self.cursor {
                    // Add a foreground color to make the reversed cell pop
                    // distinctly from a row-highlight.
                    style = style.fg(Color::Yellow);
                }
                spans.push(Span::styled(cell, style));
            }
            lines.push(Line::from(spans));
        }
        lines
    }
}

fn format_cell(date: NaiveDate, events_by_date: &BTreeMap<NaiveDate, Vec<usize>>) -> String {
    let marker = if events_by_date.contains_key(&date) {
        '•'
    } else {
        ' '
    };
    format!(" {:>2}{} ", date.day(), marker)
}

fn index_events_by_date(events: &[VEvent]) -> BTreeMap<NaiveDate, Vec<usize>> {
    let mut map: BTreeMap<NaiveDate, Vec<usize>> = BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        let start = event.dtstart;
        let end_inclusive = event.dtend - Days::new(1);
        let mut date = start;
        while date <= end_inclusive {
            map.entry(date).or_default().push(i);
            date = date + Days::new(1);
        }
    }
    map
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    let offset = date.weekday().num_days_from_monday() as u64;
    date - Days::new(offset)
}

fn sunday_of(date: NaiveDate) -> NaiveDate {
    monday_of(date) + Days::new(6)
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("first of month always valid")
}

fn last_of_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = (date.year(), date.month());
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("next month first always valid");
    next_month_first - Days::new(1)
}

// Suppress unused warning — kept available for future "jump to weekday" logic.
#[allow(dead_code)]
fn weekday_index(date: NaiveDate) -> u32 {
    match date.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn cursor_starts_on_today() {
        let today = day(2026, 5, 15);
        let s = GridScreen::from_events_with_today(&[], "h.ics", today);
        assert_eq!(s.cursor(), today);
        assert_eq!(s.granularity(), Granularity::Month);
    }

    #[test]
    fn nav_left_right_moves_cursor_one_day() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::NavLeft);
        assert_eq!(s.cursor(), day(2026, 5, 14));
        s.handle(Intent::NavRight);
        s.handle(Intent::NavRight);
        assert_eq!(s.cursor(), day(2026, 5, 16));
    }

    #[test]
    fn nav_up_down_moves_cursor_one_week() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::NavUp);
        assert_eq!(s.cursor(), day(2026, 5, 8));
        s.handle(Intent::NavDown);
        s.handle(Intent::NavDown);
        assert_eq!(s.cursor(), day(2026, 5, 22));
    }

    #[test]
    fn cursor_can_cross_month_boundary() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 1));
        s.handle(Intent::NavLeft);
        assert_eq!(s.cursor(), day(2026, 4, 30));
    }

    #[test]
    fn nav_top_in_month_jumps_to_first_of_month() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::NavTop);
        assert_eq!(s.cursor(), day(2026, 5, 1));
    }

    #[test]
    fn nav_bottom_in_month_jumps_to_last_of_month() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::NavBottom);
        assert_eq!(s.cursor(), day(2026, 5, 31));
    }

    #[test]
    fn nav_top_in_week_jumps_to_monday() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15)); // Friday
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.granularity(), Granularity::Week);
        s.handle(Intent::NavTop);
        assert_eq!(s.cursor(), day(2026, 5, 11)); // Monday
    }

    #[test]
    fn nav_bottom_in_week_jumps_to_sunday() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::CycleGranularity);
        s.handle(Intent::NavBottom);
        assert_eq!(s.cursor(), day(2026, 5, 17)); // Sunday
    }

    #[test]
    fn cycle_granularity_toggles_between_month_and_week() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.granularity(), Granularity::Week);
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.granularity(), Granularity::Month);
    }

    #[test]
    fn cycle_granularity_preserves_cursor_date() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        s.handle(Intent::CycleGranularity);
        assert_eq!(s.cursor(), day(2026, 5, 15));
    }

    #[test]
    fn events_index_covers_every_date_in_span() {
        let events = vec![make_event((2026, 5, 3), (2026, 5, 7), "連休")]; // dtend=5/7 is exclusive, so inclusive end is 5/6
        let s = GridScreen::from_events_with_today(&events, "h.ics", day(2026, 5, 4));
        assert!(s.events_by_date.contains_key(&day(2026, 5, 3)));
        assert!(s.events_by_date.contains_key(&day(2026, 5, 4)));
        assert!(s.events_by_date.contains_key(&day(2026, 5, 5)));
        assert!(s.events_by_date.contains_key(&day(2026, 5, 6)));
        assert!(!s.events_by_date.contains_key(&day(2026, 5, 7)));
        assert!(!s.events_by_date.contains_key(&day(2026, 5, 2)));
    }

    #[test]
    fn quit_and_cancel_return_quit_action() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        assert_eq!(s.handle(Intent::Quit), ScreenAction::Quit);
        assert_eq!(s.handle(Intent::Cancel), ScreenAction::Quit);
    }

    #[test]
    fn list_specific_intents_are_no_ops() {
        let mut s = GridScreen::from_events_with_today(&[], "h.ics", day(2026, 5, 15));
        assert_eq!(s.handle(Intent::OpenRemove), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::ToggleMark), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::Confirm), ScreenAction::Continue);
    }
}
