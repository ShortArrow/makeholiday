//! In-app help overlay — also the **canonical behavior spec** for
//! lazyics. The rendered content describes every keybinding in every
//! context it's reachable; the implementation must match the help text
//! exactly. (See `feedback-help-text-is-a-contract` memory.) Code
//! comments that re-state what the help text already covers are
//! redundant by design; only "why" / non-obvious invariants are
//! commented in the screen modules.

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
            Intent::ForceQuit => ScreenAction::Quit,
            Intent::Quit | Intent::Cancel | Intent::OpenHelp => ScreenAction::DismissHelp,
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
            | Intent::OpenSearch
            | Intent::OpenMonthPicker
            | Intent::OpenYearPicker
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
            .unwrap_or_else(|| "q / ? / Esc close help  |  Ctrl+C quit app".to_string());
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
        Span::styled(format!("{keys:<30}"), Style::default().fg(Color::Cyan)),
        Span::raw(desc),
    ])
}

fn blank() -> Line<'static> {
    Line::raw("")
}

fn help_lines() -> Vec<Line<'static>> {
    vec![
        header("Quit / dismiss (scope is precise)"),
        binding("Ctrl+C", "Quit the app (anywhere — overlays, forms, views)"),
        binding("q (in a view)", "Quit the app"),
        binding("q (in help)", "Close help"),
        binding("q (in a form)", "Typed into the focused text field"),
        binding("Esc (in a view)", "No-op (use q or Ctrl+C to quit)"),
        binding("Esc (in help)", "Close help"),
        binding("Esc (in a form)", "Cancel form (discard changes)"),
        binding("Esc (in Remove mode)", "Exit Remove mode (discard marks)"),
        binding("Esc (in Search)", "Cancel search (restore previous filter)"),
        blank(),
        header("Always available"),
        binding("?", "Open / close this help"),
        blank(),
        header("View switching (Browse only)"),
        binding("Tab", "Cycle List → Timeline → Grid → List"),
        binding("1 / 2 / 3", "Jump to List / Timeline / Grid"),
        binding("u", "Cycle current view's time unit (week → month → year)"),
        blank(),
        header("Movement (Browse only)"),
        binding("j | Down", "Down / next row / next week (Grid)"),
        binding("k | Up", "Up / previous row / previous week (Grid)"),
        binding("h | Left", "Previous day (Grid only — no-op elsewhere)"),
        binding("l | Right", "Next day (Grid only — no-op elsewhere)"),
        binding("g | Home", "First event / first of period"),
        binding("G | End", "Last event / last of period"),
        blank(),
        header("CRUD (where each affordance applies)"),
        binding("a (List)", "Open Add form (blank)"),
        binding("a (Timeline)", "Open Add form (blank)"),
        binding(
            "a (Grid)",
            "Open Add form with Start pre-filled to cursor date",
        ),
        binding("e (List)", "Edit selected event"),
        binding("e (Timeline)", "Edit selected event"),
        binding(
            "e (Grid)",
            "Edit first event on cursor date (no-op if none)",
        ),
        binding("d | x (List only)", "Enter multi-select Remove mode"),
        binding("Space (in Remove)", "Toggle mark on selected row"),
        binding(
            "Enter | Shift+D (in Remove)",
            "Confirm removal of marked events",
        ),
        binding("/ (List only)", "Open search-as-you-type filter"),
        binding("m (Grid only)", "Open month-jump picker"),
        binding("Y (Grid only)", "Open year-jump picker"),
        blank(),
        header("Jump pickers (Grid)"),
        binding("h | j | k | l", "Move picker selection"),
        binding("Enter", "Jump cursor to selected month / year and close"),
        binding("q | Esc", "Cancel and close picker (cursor unchanged)"),
        binding("l at right edge (Year)", "Scroll year window +1 year"),
        binding("h at left edge (Year)", "Scroll year window -1 year"),
        blank(),
        header("Search (List view)"),
        binding("Any printable key", "Append to filter (live-updates list)"),
        binding("Backspace", "Delete last filter character"),
        binding("Left | Right | Home | End", "Cursor within filter input"),
        binding("Enter", "Commit filter and return to Browse mode"),
        binding("Esc", "Cancel search; restore previous filter"),
        blank(),
        header("Add / Edit form"),
        binding("Tab | Shift+Tab", "Next / previous field"),
        binding("Down | Up", "Same as Tab / Shift+Tab"),
        binding("Ctrl+N | Ctrl+P", "Same as Tab / Shift+Tab (emacs-style)"),
        binding(
            "Left | Right",
            "Cursor in text fields; cycle prev/next on pickers",
        ),
        binding(
            "h | l (on pickers)",
            "Cycle prev/next on busy-status / class",
        ),
        binding("Home | End", "Start / end of focused text field"),
        binding("Space (on pickers)", "Cycle next on busy-status / class"),
        binding("Ctrl+S | Enter", "Submit (validates required fields)"),
        binding("Backspace", "Delete character before cursor (text fields)"),
        binding("Any other printable key", "Typed into focused text field"),
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
    fn soft_quit_dismisses_help_not_app() {
        let mut s = HelpScreen::new("h.ics");
        // `q` (Intent::Quit) closes the overlay rather than the app,
        // matching less / man / vim help conventions.
        assert_eq!(s.handle(Intent::Quit), ScreenAction::DismissHelp);
    }

    #[test]
    fn force_quit_still_exits_the_app() {
        let mut s = HelpScreen::new("h.ics");
        // Ctrl+C is the explicit hard-exit affordance.
        assert_eq!(s.handle(Intent::ForceQuit), ScreenAction::Quit);
    }

    #[test]
    fn nav_intents_are_no_ops() {
        let mut s = HelpScreen::new("h.ics");
        assert_eq!(s.handle(Intent::NavDown), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::NavUp), ScreenAction::Continue);
        assert_eq!(s.handle(Intent::CycleView), ScreenAction::Continue);
    }
}
