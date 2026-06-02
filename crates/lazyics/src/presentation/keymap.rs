//! Key event → [`Intent`] mapping.
//!
//! Keeping the mapping in one place — separate from the screen render code —
//! lets unit tests pin keybindings without spinning up a terminal, and lets
//! a future config layer override the map per ADR-025 (out-of-scope for
//! v0.2.0 but the seam is here).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::screens::ViewKind;

/// High-level user intent produced by a single key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Quit the application — force-exit regardless of modal state.
    Quit,
    /// Back out of the current modal state. The active screen interprets
    /// this contextually: in a top-level browse view it falls through to
    /// [`Intent::Quit`]; in multi-select Remove mode it discards the marks
    /// and returns to browse.
    Cancel,
    /// Move selection one row up.
    NavUp,
    /// Move selection one row down.
    NavDown,
    /// Move selection one column left. Used by Grid; List/Timeline ignore.
    NavLeft,
    /// Move selection one column right. Used by Grid; List/Timeline ignore.
    NavRight,
    /// Move selection to the first row / cell.
    NavTop,
    /// Move selection to the last row / cell.
    NavBottom,
    /// Cycle to the next view (Tab): List → Timeline → Grid → List.
    /// Composition-Root level intent — screens never see it.
    CycleView,
    /// Jump to a specific view (number keys 1/2/3).
    /// Composition-Root level intent — screens never see it.
    SwitchView(ViewKind),
    /// Cycle the active view's time granularity. List ignores; Timeline
    /// cycles month ↔ week; Grid cycles month ↔ week.
    CycleGranularity,
    /// Enter multi-select Remove mode.
    OpenRemove,
    /// Toggle the mark on the currently-selected row (Remove mode only).
    ToggleMark,
    /// Confirm the current modal action — in Remove mode, submit the
    /// marked indices to `icscli::application::use_cases::remove`.
    Confirm,
}

/// Map a single [`KeyEvent`] to an [`Intent`]. Returns `None` for keys that
/// don't bind to anything in the current screen.
///
/// Phase 1 binds only navigation + quit; richer intents (Add / Edit /
/// Remove / Search) arrive with their respective phases.
pub fn map(event: KeyEvent) -> Option<Intent> {
    // crossterm emits Press / Release / Repeat. We only act on Press so that
    // a quick `q` tap doesn't quit twice.
    if event.kind != KeyEventKind::Press {
        return None;
    }

    match (event.code, event.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Intent::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Intent::Quit),
        (KeyCode::Esc, _) => Some(Intent::Cancel),

        (KeyCode::Char('j'), KeyModifiers::NONE) => Some(Intent::NavDown),
        (KeyCode::Down, _) => Some(Intent::NavDown),

        (KeyCode::Char('k'), KeyModifiers::NONE) => Some(Intent::NavUp),
        (KeyCode::Up, _) => Some(Intent::NavUp),

        (KeyCode::Char('h'), KeyModifiers::NONE) => Some(Intent::NavLeft),
        (KeyCode::Left, _) => Some(Intent::NavLeft),

        (KeyCode::Char('l'), KeyModifiers::NONE) => Some(Intent::NavRight),
        (KeyCode::Right, _) => Some(Intent::NavRight),

        (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Intent::NavTop),
        (KeyCode::Home, _) => Some(Intent::NavTop),

        (KeyCode::Char('G'), KeyModifiers::SHIFT) => Some(Intent::NavBottom),
        (KeyCode::End, _) => Some(Intent::NavBottom),

        // View switching (Phase 4a).
        (KeyCode::Tab, _) => Some(Intent::CycleView),
        (KeyCode::Char('1'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::List)),
        (KeyCode::Char('2'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::Timeline)),
        (KeyCode::Char('3'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::Grid)),

        // Granularity cycle ("u" for "unit").
        (KeyCode::Char('u'), KeyModifiers::NONE) => Some(Intent::CycleGranularity),

        // Remove-mode entry: ADR-025 §"Initial scope" binds d and x.
        (KeyCode::Char('d'), KeyModifiers::NONE) => Some(Intent::OpenRemove),
        (KeyCode::Char('x'), KeyModifiers::NONE) => Some(Intent::OpenRemove),

        // Mark toggle for multi-select.
        (KeyCode::Char(' '), KeyModifiers::NONE) => Some(Intent::ToggleMark),

        // Confirm: Enter or Shift+D (ADR-025 §"Initial scope": "D / Enter").
        (KeyCode::Enter, _) => Some(Intent::Confirm),
        (KeyCode::Char('D'), KeyModifiers::SHIFT) => Some(Intent::Confirm),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn release(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_lowercase_quits() {
        assert_eq!(
            map(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Intent::Quit)
        );
    }

    #[test]
    fn esc_emits_cancel_not_quit() {
        // ADR-025 §"Initial scope" — Esc backs out of modal state;
        // ListScreen falls through to Quit at the top level.
        assert_eq!(
            map(press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Intent::Cancel)
        );
    }

    #[test]
    fn d_and_x_open_remove_mode() {
        assert_eq!(
            map(press(KeyCode::Char('d'), KeyModifiers::NONE)),
            Some(Intent::OpenRemove)
        );
        assert_eq!(
            map(press(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Intent::OpenRemove)
        );
    }

    #[test]
    fn space_toggles_mark() {
        assert_eq!(
            map(press(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(Intent::ToggleMark)
        );
    }

    #[test]
    fn tab_cycles_view() {
        assert_eq!(
            map(press(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Intent::CycleView)
        );
    }

    #[test]
    fn number_keys_switch_view() {
        assert_eq!(
            map(press(KeyCode::Char('1'), KeyModifiers::NONE)),
            Some(Intent::SwitchView(ViewKind::List))
        );
        assert_eq!(
            map(press(KeyCode::Char('2'), KeyModifiers::NONE)),
            Some(Intent::SwitchView(ViewKind::Timeline))
        );
        assert_eq!(
            map(press(KeyCode::Char('3'), KeyModifiers::NONE)),
            Some(Intent::SwitchView(ViewKind::Grid))
        );
    }

    #[test]
    fn u_cycles_granularity() {
        assert_eq!(
            map(press(KeyCode::Char('u'), KeyModifiers::NONE)),
            Some(Intent::CycleGranularity)
        );
    }

    #[test]
    fn h_and_l_nav_horizontal() {
        assert_eq!(
            map(press(KeyCode::Char('h'), KeyModifiers::NONE)),
            Some(Intent::NavLeft)
        );
        assert_eq!(
            map(press(KeyCode::Char('l'), KeyModifiers::NONE)),
            Some(Intent::NavRight)
        );
        assert_eq!(
            map(press(KeyCode::Left, KeyModifiers::NONE)),
            Some(Intent::NavLeft)
        );
        assert_eq!(
            map(press(KeyCode::Right, KeyModifiers::NONE)),
            Some(Intent::NavRight)
        );
    }

    #[test]
    fn enter_and_shift_d_both_confirm() {
        assert_eq!(
            map(press(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Intent::Confirm)
        );
        assert_eq!(
            map(press(KeyCode::Char('D'), KeyModifiers::SHIFT)),
            Some(Intent::Confirm)
        );
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(
            map(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Intent::Quit)
        );
    }

    #[test]
    fn j_and_down_arrow_both_nav_down() {
        assert_eq!(
            map(press(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Intent::NavDown)
        );
        assert_eq!(
            map(press(KeyCode::Down, KeyModifiers::NONE)),
            Some(Intent::NavDown)
        );
    }

    #[test]
    fn k_and_up_arrow_both_nav_up() {
        assert_eq!(
            map(press(KeyCode::Char('k'), KeyModifiers::NONE)),
            Some(Intent::NavUp)
        );
        assert_eq!(
            map(press(KeyCode::Up, KeyModifiers::NONE)),
            Some(Intent::NavUp)
        );
    }

    #[test]
    fn g_to_top_shift_g_to_bottom() {
        assert_eq!(
            map(press(KeyCode::Char('g'), KeyModifiers::NONE)),
            Some(Intent::NavTop)
        );
        assert_eq!(
            map(press(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Some(Intent::NavBottom)
        );
    }

    #[test]
    fn release_events_ignored() {
        assert_eq!(map(release(KeyCode::Char('q'))), None);
        assert_eq!(map(release(KeyCode::Down)), None);
    }

    #[test]
    fn unbound_keys_return_none() {
        assert_eq!(map(press(KeyCode::Char('z'), KeyModifiers::NONE)), None);
        assert_eq!(map(press(KeyCode::F(1), KeyModifiers::NONE)), None);
    }
}
