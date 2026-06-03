//! Key event → [`Intent`] mapping. The rendered user-facing contract
//! lives in `presentation::screens::help` — that screen is the spec for
//! which physical keys produce which intents in which mode.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::screens::ViewKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapMode {
    Browse,
    Form,
}

/// High-level user intent produced by a single key press. See the help
/// overlay for the per-context behavior of each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Soft quit — `q` in Browse mode. Screens interpret it contextually
    /// (view → exit app; overlay → close overlay).
    Quit,
    /// Hard quit — `Ctrl+C`. Always exits.
    ForceQuit,
    /// Back out of the current modal state. Pure "go up one level":
    /// closes overlays, cancels forms, exits Remove mode. Never exits
    /// the app — the help overlay is the authority on that.
    Cancel,
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    NavTop,
    NavBottom,
    CycleView,
    SwitchView(ViewKind),
    CycleGranularity,
    OpenRemove,
    OpenAdd,
    OpenEdit,
    /// Toggle the help overlay. Same key opens and closes.
    OpenHelp,
    ToggleMark,
    Confirm,
    TypeChar(char),
    Backspace,
    NextField,
    PrevField,
    SubmitForm,
}

pub fn map(event: KeyEvent, mode: KeymapMode) -> Option<Intent> {
    // Press only: a quick tap shouldn't fire twice via the Release event.
    if event.kind != KeyEventKind::Press {
        return None;
    }
    match mode {
        KeymapMode::Browse => map_browse(event),
        KeymapMode::Form => map_form(event),
    }
}

fn map_browse(event: KeyEvent) -> Option<Intent> {
    match (event.code, event.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Intent::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Intent::ForceQuit),
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

        (KeyCode::Tab, _) => Some(Intent::CycleView),
        (KeyCode::Char('1'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::List)),
        (KeyCode::Char('2'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::Timeline)),
        (KeyCode::Char('3'), KeyModifiers::NONE) => Some(Intent::SwitchView(ViewKind::Grid)),
        (KeyCode::Char('u'), KeyModifiers::NONE) => Some(Intent::CycleGranularity),
        (KeyCode::Char('a'), KeyModifiers::NONE) => Some(Intent::OpenAdd),
        (KeyCode::Char('e'), KeyModifiers::NONE) => Some(Intent::OpenEdit),
        // `?` arrives as Char('?') with Shift on most layouts; accept any modifiers.
        (KeyCode::Char('?'), _) => Some(Intent::OpenHelp),
        (KeyCode::Char('d'), KeyModifiers::NONE) => Some(Intent::OpenRemove),
        (KeyCode::Char('x'), KeyModifiers::NONE) => Some(Intent::OpenRemove),
        (KeyCode::Char(' '), KeyModifiers::NONE) => Some(Intent::ToggleMark),
        (KeyCode::Enter, _) => Some(Intent::Confirm),
        (KeyCode::Char('D'), KeyModifiers::SHIFT) => Some(Intent::Confirm),

        _ => None,
    }
}

fn map_form(event: KeyEvent) -> Option<Intent> {
    match (event.code, event.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Intent::ForceQuit),
        (KeyCode::Esc, _) => Some(Intent::Cancel),
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Intent::SubmitForm),
        (KeyCode::Enter, _) => Some(Intent::SubmitForm),

        (KeyCode::Tab, KeyModifiers::NONE) => Some(Intent::NextField),
        (KeyCode::BackTab, _) => Some(Intent::PrevField),
        (KeyCode::Tab, KeyModifiers::SHIFT) => Some(Intent::PrevField),
        (KeyCode::Down, _) => Some(Intent::NextField),
        (KeyCode::Up, _) => Some(Intent::PrevField),

        (KeyCode::Backspace, _) => Some(Intent::Backspace),
        (KeyCode::Left, _) => Some(Intent::NavLeft),
        (KeyCode::Right, _) => Some(Intent::NavRight),
        (KeyCode::Home, _) => Some(Intent::NavTop),
        (KeyCode::End, _) => Some(Intent::NavBottom),

        // Printable char fallthrough — Ctrl-modified keys excluded so
        // Ctrl+C / Ctrl+S above still win.
        (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => Some(Intent::TypeChar(c)),

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
    fn q_lowercase_quits_in_browse() {
        assert_eq!(
            map(
                press(KeyCode::Char('q'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::Quit)
        );
    }

    #[test]
    fn q_in_form_is_a_typed_character() {
        assert_eq!(
            map(
                press(KeyCode::Char('q'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('q'))
        );
    }

    #[test]
    fn esc_emits_cancel_in_both_modes() {
        for mode in [KeymapMode::Browse, KeymapMode::Form] {
            assert_eq!(
                map(press(KeyCode::Esc, KeyModifiers::NONE), mode),
                Some(Intent::Cancel)
            );
        }
    }

    #[test]
    fn ctrl_c_force_quits_in_both_modes() {
        for mode in [KeymapMode::Browse, KeymapMode::Form] {
            assert_eq!(
                map(press(KeyCode::Char('c'), KeyModifiers::CONTROL), mode),
                Some(Intent::ForceQuit)
            );
        }
    }

    #[test]
    fn q_in_browse_emits_soft_quit_not_force_quit() {
        assert_eq!(
            map(
                press(KeyCode::Char('q'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::Quit)
        );
    }

    #[test]
    fn tab_cycles_view_in_browse_navigates_fields_in_form() {
        assert_eq!(
            map(press(KeyCode::Tab, KeyModifiers::NONE), KeymapMode::Browse),
            Some(Intent::CycleView)
        );
        assert_eq!(
            map(press(KeyCode::Tab, KeyModifiers::NONE), KeymapMode::Form),
            Some(Intent::NextField)
        );
    }

    #[test]
    fn backtab_navigates_back_in_form() {
        assert_eq!(
            map(
                press(KeyCode::BackTab, KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::PrevField)
        );
    }

    #[test]
    fn enter_in_form_submits() {
        assert_eq!(
            map(press(KeyCode::Enter, KeyModifiers::NONE), KeymapMode::Form),
            Some(Intent::SubmitForm)
        );
    }

    #[test]
    fn ctrl_s_in_form_submits() {
        assert_eq!(
            map(
                press(KeyCode::Char('s'), KeyModifiers::CONTROL),
                KeymapMode::Form
            ),
            Some(Intent::SubmitForm)
        );
    }

    #[test]
    fn backspace_in_form_deletes() {
        assert_eq!(
            map(
                press(KeyCode::Backspace, KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::Backspace)
        );
    }

    #[test]
    fn arrows_in_form_move_cursor_or_field() {
        assert_eq!(
            map(press(KeyCode::Left, KeyModifiers::NONE), KeymapMode::Form),
            Some(Intent::NavLeft)
        );
        assert_eq!(
            map(press(KeyCode::Down, KeyModifiers::NONE), KeymapMode::Form),
            Some(Intent::NextField)
        );
        assert_eq!(
            map(press(KeyCode::Up, KeyModifiers::NONE), KeymapMode::Form),
            Some(Intent::PrevField)
        );
    }

    #[test]
    fn question_mark_opens_help_in_browse_types_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('?'), KeyModifiers::SHIFT),
                KeymapMode::Browse
            ),
            Some(Intent::OpenHelp)
        );
        // In Form mode '?' is just a typed character (the modifier is
        // present but not Control, so map_form takes the TypeChar branch).
        assert_eq!(
            map(
                press(KeyCode::Char('?'), KeyModifiers::SHIFT),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('?'))
        );
    }

    #[test]
    fn e_opens_edit_in_browse_types_e_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('e'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::OpenEdit)
        );
        assert_eq!(
            map(
                press(KeyCode::Char('e'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('e'))
        );
    }

    #[test]
    fn a_opens_add_in_browse_types_a_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('a'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::OpenAdd)
        );
        assert_eq!(
            map(
                press(KeyCode::Char('a'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('a'))
        );
    }

    #[test]
    fn release_events_ignored_in_both_modes() {
        for mode in [KeymapMode::Browse, KeymapMode::Form] {
            assert_eq!(map(release(KeyCode::Char('q')), mode), None);
            assert_eq!(map(release(KeyCode::Tab), mode), None);
        }
    }

    #[test]
    fn d_and_x_open_remove_in_browse_type_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('d'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::OpenRemove)
        );
        assert_eq!(
            map(
                press(KeyCode::Char('d'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('d'))
        );
    }

    #[test]
    fn space_in_browse_marks_in_form_types() {
        assert_eq!(
            map(
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::ToggleMark)
        );
        assert_eq!(
            map(
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar(' '))
        );
    }

    #[test]
    fn number_keys_switch_view_in_browse() {
        assert_eq!(
            map(
                press(KeyCode::Char('1'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::SwitchView(ViewKind::List))
        );
        assert_eq!(
            map(
                press(KeyCode::Char('2'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::SwitchView(ViewKind::Timeline))
        );
        assert_eq!(
            map(
                press(KeyCode::Char('3'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::SwitchView(ViewKind::Grid))
        );
    }

    #[test]
    fn number_keys_type_digits_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('1'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('1'))
        );
        assert_eq!(
            map(
                press(KeyCode::Char('5'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('5'))
        );
    }

    #[test]
    fn u_cycles_granularity_in_browse_types_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('u'), KeyModifiers::NONE),
                KeymapMode::Browse
            ),
            Some(Intent::CycleGranularity)
        );
        assert_eq!(
            map(
                press(KeyCode::Char('u'), KeyModifiers::NONE),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('u'))
        );
    }

    #[test]
    fn shift_d_confirms_in_browse() {
        assert_eq!(
            map(
                press(KeyCode::Char('D'), KeyModifiers::SHIFT),
                KeymapMode::Browse
            ),
            Some(Intent::Confirm)
        );
    }

    #[test]
    fn shift_uppercase_letter_types_in_form() {
        assert_eq!(
            map(
                press(KeyCode::Char('A'), KeyModifiers::SHIFT),
                KeymapMode::Form
            ),
            Some(Intent::TypeChar('A'))
        );
    }
}
