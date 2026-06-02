//! Add form — captures the 7 fields the CLI `add` subcommand accepts and
//! produces an `AddRequest` for the Composition Root to submit through
//! `icscli::application::use_cases::add`.
//!
//! Field layout (top to bottom):
//!   1. Summary       — TextInput, required
//!   2. Start date    — TextInput, parsed via `icscli::input::parse_date`
//!   3. End date      — TextInput, optional (empty = single-day event)
//!   4. Busy status   — cycle picker (free/tentative/busy/oof/working)
//!   5. Class         — cycle picker (none/public/private/confidential)
//!   6. Categories    — TextInput, comma-separated
//!   7. Icon          — TextInput, optional
//!
//! Keys (Form keymap):
//!   - `Tab` / `Shift+Tab` / `Down` / `Up` — next/prev field
//!   - `Enter` / `Ctrl+S` — submit (validates first)
//!   - `Esc` — dismiss form
//!   - `Left` / `Right` — TextInput cursor, or cycle picker prev/next
//!   - Printable chars — typed into focused TextInput; Space cycles a picker

use ics_core::EventClass;
use ics_core::microsoft::MsBusyStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::presentation::keymap::Intent;
use crate::presentation::screens::{AddRequest, ScreenAction};
use crate::presentation::widgets::TextInput;

const SUMMARY: usize = 0;
const START: usize = 1;
const END: usize = 2;
const BUSY: usize = 3;
const CLASS: usize = 4;
const CATEGORIES: usize = 5;
const ICON: usize = 6;
const FIELD_COUNT: usize = 7;

const BUSY_STATUSES: &[MsBusyStatus] = &[
    MsBusyStatus::Free,
    MsBusyStatus::Tentative,
    MsBusyStatus::Busy,
    MsBusyStatus::Oof,
    MsBusyStatus::WorkingElsewhere,
];

/// Class options. `None` = "(unset)", which omits `CLASS:` from the
/// generated VEVENT.
const CLASSES: &[Option<EventClass>] = &[
    None,
    Some(EventClass::Public),
    Some(EventClass::Private),
    Some(EventClass::Confidential),
];

pub struct AddForm {
    summary: TextInput,
    start: TextInput,
    end: TextInput,
    busystatus: MsBusyStatus,
    class: Option<EventClass>,
    categories: TextInput,
    icon: TextInput,
    focus: usize,
    /// Persistent error banner — cleared by next valid submit or by any
    /// field edit. (Distinct from `transient_status` which is the
    /// composition-root-set one-shot bar.)
    error: Option<String>,
    file_label: String,
    transient_status: Option<String>,
}

impl AddForm {
    pub fn new(file_label: impl Into<String>) -> Self {
        Self {
            summary: TextInput::new(),
            start: TextInput::new(),
            end: TextInput::new(),
            busystatus: MsBusyStatus::Free,
            class: None,
            categories: TextInput::new(),
            icon: TextInput::new(),
            focus: SUMMARY,
            error: None,
            file_label: file_label.into(),
            transient_status: None,
        }
    }

    pub fn focus(&self) -> usize {
        self.focus
    }

    pub fn busystatus(&self) -> MsBusyStatus {
        self.busystatus
    }

    pub fn class(&self) -> Option<EventClass> {
        self.class
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
            Intent::Quit => ScreenAction::Quit,
            Intent::Cancel => ScreenAction::DismissForm,
            Intent::NextField => {
                self.focus = (self.focus + 1) % FIELD_COUNT;
                self.error = None;
                ScreenAction::Continue
            }
            Intent::PrevField => {
                self.focus = (self.focus + FIELD_COUNT - 1) % FIELD_COUNT;
                self.error = None;
                ScreenAction::Continue
            }
            Intent::SubmitForm => match self.validate() {
                Ok(req) => ScreenAction::SubmitAdd(req),
                Err(e) => {
                    self.error = Some(e);
                    ScreenAction::Continue
                }
            },
            Intent::TypeChar(c) => {
                self.type_char(c);
                ScreenAction::Continue
            }
            Intent::Backspace => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.backspace();
                    self.error = None;
                }
                ScreenAction::Continue
            }
            Intent::NavLeft => {
                self.nav_left();
                ScreenAction::Continue
            }
            Intent::NavRight => {
                self.nav_right();
                ScreenAction::Continue
            }
            Intent::NavTop => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.move_home();
                }
                ScreenAction::Continue
            }
            Intent::NavBottom => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.move_end();
                }
                ScreenAction::Continue
            }
            // Browse-mode intents are unreachable while the AddForm is
            // the active screen (keymap is in Form mode), but listing
            // them keeps the match exhaustive and protects against
            // future intent additions silently dropping through.
            Intent::NavUp
            | Intent::NavDown
            | Intent::CycleView
            | Intent::SwitchView(_)
            | Intent::CycleGranularity
            | Intent::OpenRemove
            | Intent::OpenAdd
            | Intent::ToggleMark
            | Intent::Confirm => ScreenAction::Continue,
        }
    }

    fn type_char(&mut self, c: char) {
        match self.focus {
            BUSY | CLASS if c == ' ' => self.cycle_picker(1),
            BUSY | CLASS => {} // ignore other printable chars on pickers
            _ => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.insert_char(c);
                    self.error = None;
                }
            }
        }
    }

    fn nav_left(&mut self) {
        match self.focus {
            BUSY | CLASS => self.cycle_picker(-1),
            _ => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.move_left();
                }
            }
        }
    }

    fn nav_right(&mut self) {
        match self.focus {
            BUSY | CLASS => self.cycle_picker(1),
            _ => {
                if let Some(field) = self.focused_text_input_mut() {
                    field.move_right();
                }
            }
        }
    }

    fn cycle_picker(&mut self, delta: i32) {
        match self.focus {
            BUSY => {
                let idx = BUSY_STATUSES
                    .iter()
                    .position(|b| *b == self.busystatus)
                    .unwrap_or(0);
                let len = BUSY_STATUSES.len() as i32;
                let new_idx = (idx as i32 + delta).rem_euclid(len) as usize;
                self.busystatus = BUSY_STATUSES[new_idx];
            }
            CLASS => {
                let idx = CLASSES.iter().position(|c| *c == self.class).unwrap_or(0);
                let len = CLASSES.len() as i32;
                let new_idx = (idx as i32 + delta).rem_euclid(len) as usize;
                self.class = CLASSES[new_idx];
            }
            _ => {}
        }
    }

    fn focused_text_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.focus {
            SUMMARY => Some(&mut self.summary),
            START => Some(&mut self.start),
            END => Some(&mut self.end),
            CATEGORIES => Some(&mut self.categories),
            ICON => Some(&mut self.icon),
            _ => None,
        }
    }

    /// Validate the form. On success, the `error` field is left untouched
    /// (the caller is about to replace the screen); on failure the caller
    /// stores the returned message in `error`.
    pub fn validate(&self) -> Result<AddRequest, String> {
        let summary = self.summary.value().trim();
        if summary.is_empty() {
            return Err("Summary is required".into());
        }
        let start = icscli::input::parse_date(self.start.value().trim())
            .map_err(|e| format!("Start date: {e}"))?;
        let end_raw = self.end.value().trim();
        let end = if end_raw.is_empty() {
            None
        } else {
            let parsed =
                icscli::input::parse_date(end_raw).map_err(|e| format!("End date: {e}"))?;
            if parsed < start {
                return Err("End date must not be before start".into());
            }
            Some(parsed)
        };
        let categories: Vec<String> = self
            .categories
            .value()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let icon_raw = self.icon.value().trim();
        let icon = if icon_raw.is_empty() {
            None
        } else {
            Some(icon_raw.to_string())
        };
        Ok(AddRequest {
            summary: summary.to_string(),
            start,
            end,
            busystatus: self.busystatus,
            class: self.class,
            categories,
            icon,
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(3), // summary
            Constraint::Length(3), // start
            Constraint::Length(3), // end
            Constraint::Length(3), // busystatus
            Constraint::Length(3), // class
            Constraint::Length(3), // categories
            Constraint::Length(3), // icon
            Constraint::Length(1), // error
            Constraint::Min(0),    // filler
            Constraint::Length(1), // status
        ]);
        let areas: [Rect; 10] = layout.areas(frame.area());

        self.render_text_field(
            frame,
            areas[SUMMARY],
            "Summary (required)",
            &self.summary,
            self.focus == SUMMARY,
        );
        self.render_text_field(
            frame,
            areas[START],
            "Start date YYYY-MM-DD (required)",
            &self.start,
            self.focus == START,
        );
        self.render_text_field(
            frame,
            areas[END],
            "End date YYYY-MM-DD (optional)",
            &self.end,
            self.focus == END,
        );
        self.render_picker(
            frame,
            areas[BUSY],
            "Busy status",
            busystatus_label(self.busystatus),
            self.focus == BUSY,
        );
        self.render_picker(
            frame,
            areas[CLASS],
            "Class",
            class_label(self.class),
            self.focus == CLASS,
        );
        self.render_text_field(
            frame,
            areas[CATEGORIES],
            "Categories (comma-separated)",
            &self.categories,
            self.focus == CATEGORIES,
        );
        self.render_text_field(
            frame,
            areas[ICON],
            "Icon (optional)",
            &self.icon,
            self.focus == ICON,
        );

        // Error banner.
        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("⚠ {err}"),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))),
                areas[7],
            );
        }

        // Status bar.
        let status_text = self.transient_status.clone().unwrap_or_else(|| {
            format!(
                "{}  |  Add event  |  Tab/Shift+Tab field  ←/→ cursor/cycle  Ctrl+S or Enter submit  Esc cancel",
                self.file_label,
            )
        });
        frame.render_widget(Paragraph::new(status_text), areas[9]);
    }

    fn render_text_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        input: &TextInput,
        focused: bool,
    ) {
        let block_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title(label)
            .borders(Borders::ALL)
            .border_style(block_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        input.render(frame, inner, focused);
    }

    fn render_picker(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        focused: bool,
    ) {
        let block_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title(label)
            .borders(Borders::ALL)
            .border_style(block_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let text = if focused {
            format!("◀ {value} ▶")
        } else {
            format!("  {value}  ")
        };
        let style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(text).style(style), inner);
    }
}

fn busystatus_label(b: MsBusyStatus) -> &'static str {
    match b {
        MsBusyStatus::Free => "free",
        MsBusyStatus::Tentative => "tentative",
        MsBusyStatus::Busy => "busy",
        MsBusyStatus::Oof => "oof",
        MsBusyStatus::WorkingElsewhere => "working",
    }
}

fn class_label(c: Option<EventClass>) -> &'static str {
    match c {
        None => "(unset)",
        Some(EventClass::Public) => "public",
        Some(EventClass::Private) => "private",
        Some(EventClass::Confidential) => "confidential",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn type_string(form: &mut AddForm, s: &str) {
        for c in s.chars() {
            form.handle(Intent::TypeChar(c));
        }
    }

    #[test]
    fn new_starts_on_summary_with_defaults() {
        let f = AddForm::new("h.ics");
        assert_eq!(f.focus(), SUMMARY);
        assert_eq!(f.busystatus(), MsBusyStatus::Free);
        assert_eq!(f.class(), None);
        assert!(f.error().is_none());
    }

    #[test]
    fn next_field_advances_and_wraps() {
        let mut f = AddForm::new("h.ics");
        for i in 1..FIELD_COUNT {
            f.handle(Intent::NextField);
            assert_eq!(f.focus(), i);
        }
        f.handle(Intent::NextField);
        assert_eq!(f.focus(), 0); // wrapped
    }

    #[test]
    fn prev_field_wraps_backward() {
        let mut f = AddForm::new("h.ics");
        f.handle(Intent::PrevField);
        assert_eq!(f.focus(), FIELD_COUNT - 1);
    }

    #[test]
    fn type_char_inserts_into_focused_text_field() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "元日");
        assert_eq!(f.summary.value(), "元日");
        // Tab to start date; new chars go there.
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-01-01");
        assert_eq!(f.start.value(), "2026-01-01");
        assert_eq!(f.summary.value(), "元日");
    }

    #[test]
    fn space_cycles_busystatus_when_focused() {
        let mut f = AddForm::new("h.ics");
        for _ in 0..BUSY {
            f.handle(Intent::NextField);
        }
        assert_eq!(f.focus(), BUSY);
        assert_eq!(f.busystatus(), MsBusyStatus::Free);
        f.handle(Intent::TypeChar(' '));
        assert_eq!(f.busystatus(), MsBusyStatus::Tentative);
        f.handle(Intent::NavRight);
        assert_eq!(f.busystatus(), MsBusyStatus::Busy);
        f.handle(Intent::NavLeft);
        assert_eq!(f.busystatus(), MsBusyStatus::Tentative);
    }

    #[test]
    fn class_cycles_through_options_including_unset() {
        let mut f = AddForm::new("h.ics");
        for _ in 0..CLASS {
            f.handle(Intent::NextField);
        }
        assert_eq!(f.class(), None);
        f.handle(Intent::NavRight);
        assert_eq!(f.class(), Some(EventClass::Public));
        f.handle(Intent::NavRight);
        assert_eq!(f.class(), Some(EventClass::Private));
        f.handle(Intent::NavRight);
        assert_eq!(f.class(), Some(EventClass::Confidential));
        f.handle(Intent::NavRight);
        assert_eq!(f.class(), None); // wraps back to unset
    }

    #[test]
    fn cancel_dismisses_form() {
        let mut f = AddForm::new("h.ics");
        assert_eq!(f.handle(Intent::Cancel), ScreenAction::DismissForm);
    }

    #[test]
    fn quit_force_exits() {
        let mut f = AddForm::new("h.ics");
        assert_eq!(f.handle(Intent::Quit), ScreenAction::Quit);
    }

    #[test]
    fn submit_with_empty_summary_records_error() {
        let mut f = AddForm::new("h.ics");
        let action = f.handle(Intent::SubmitForm);
        assert_eq!(action, ScreenAction::Continue);
        assert!(f.error().unwrap().contains("Summary"));
    }

    #[test]
    fn submit_with_invalid_start_records_error() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "元日");
        f.handle(Intent::NextField);
        type_string(&mut f, "nonsense");
        let action = f.handle(Intent::SubmitForm);
        assert_eq!(action, ScreenAction::Continue);
        assert!(f.error().unwrap().to_lowercase().contains("start"));
    }

    #[test]
    fn submit_minimal_required_fields_produces_request() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "元日");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-01-01");
        match f.handle(Intent::SubmitForm) {
            ScreenAction::SubmitAdd(req) => {
                assert_eq!(req.summary, "元日");
                assert_eq!(req.start, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
                assert_eq!(req.end, None);
                assert_eq!(req.busystatus, MsBusyStatus::Free);
                assert_eq!(req.class, None);
                assert!(req.categories.is_empty());
                assert_eq!(req.icon, None);
            }
            other => panic!("expected SubmitAdd, got {other:?}"),
        }
    }

    #[test]
    fn submit_full_form_populates_all_fields() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "Travel");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-05-10");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-05-12");
        f.handle(Intent::NextField); // BUSY
        f.handle(Intent::TypeChar(' ')); // Free -> Tentative
        f.handle(Intent::TypeChar(' ')); // -> Busy
        f.handle(Intent::TypeChar(' ')); // -> Oof
        f.handle(Intent::NextField); // CLASS
        f.handle(Intent::NavRight); // None -> Public
        f.handle(Intent::NavRight); // -> Private
        f.handle(Intent::NextField);
        type_string(&mut f, "work, travel");
        f.handle(Intent::NextField);
        type_string(&mut f, "airplane");

        match f.handle(Intent::SubmitForm) {
            ScreenAction::SubmitAdd(req) => {
                assert_eq!(req.summary, "Travel");
                assert_eq!(req.start, NaiveDate::from_ymd_opt(2026, 5, 10).unwrap());
                assert_eq!(req.end, Some(NaiveDate::from_ymd_opt(2026, 5, 12).unwrap()));
                assert_eq!(req.busystatus, MsBusyStatus::Oof);
                assert_eq!(req.class, Some(EventClass::Private));
                assert_eq!(
                    req.categories,
                    vec!["work".to_string(), "travel".to_string()]
                );
                assert_eq!(req.icon, Some("airplane".to_string()));
            }
            other => panic!("expected SubmitAdd, got {other:?}"),
        }
    }

    #[test]
    fn submit_with_end_before_start_records_error() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "x");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-05-10");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-05-01");
        let action = f.handle(Intent::SubmitForm);
        assert_eq!(action, ScreenAction::Continue);
        assert!(f.error().unwrap().to_lowercase().contains("end"));
    }

    #[test]
    fn categories_split_and_trim() {
        let mut f = AddForm::new("h.ics");
        type_string(&mut f, "x");
        f.handle(Intent::NextField);
        type_string(&mut f, "2026-01-01");
        for _ in 0..(CATEGORIES - START) {
            f.handle(Intent::NextField);
        }
        type_string(&mut f, "  work , , travel  ,  ");
        match f.handle(Intent::SubmitForm) {
            ScreenAction::SubmitAdd(req) => {
                assert_eq!(
                    req.categories,
                    vec!["work".to_string(), "travel".to_string()]
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    fn editing_clears_error() {
        let mut f = AddForm::new("h.ics");
        f.handle(Intent::SubmitForm); // error: Summary required
        assert!(f.error().is_some());
        f.handle(Intent::TypeChar('a'));
        assert!(f.error().is_none());
    }
}
