//! Key event → [`Intent`] mapping.
//!
//! Keeping the mapping in one place — separate from the screen render code —
//! lets unit tests pin keybindings without spinning up a terminal, and lets
//! a future config layer override the map per ADR-025 (out-of-scope for
//! v0.2.0 but the seam is here).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// High-level user intent produced by a single key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Quit the application.
    Quit,
    /// Move selection one row up.
    NavUp,
    /// Move selection one row down.
    NavDown,
    /// Move selection to the first row.
    NavTop,
    /// Move selection to the last row.
    NavBottom,
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
        (KeyCode::Esc, _) => Some(Intent::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Intent::Quit),

        (KeyCode::Char('j'), KeyModifiers::NONE) => Some(Intent::NavDown),
        (KeyCode::Down, _) => Some(Intent::NavDown),

        (KeyCode::Char('k'), KeyModifiers::NONE) => Some(Intent::NavUp),
        (KeyCode::Up, _) => Some(Intent::NavUp),

        (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Intent::NavTop),
        (KeyCode::Home, _) => Some(Intent::NavTop),

        (KeyCode::Char('G'), KeyModifiers::SHIFT) => Some(Intent::NavBottom),
        (KeyCode::End, _) => Some(Intent::NavBottom),

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
    fn esc_quits() {
        assert_eq!(
            map(press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Intent::Quit)
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
