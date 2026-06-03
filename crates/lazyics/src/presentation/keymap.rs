//! Key event → [`Intent`] mapping.
//!
//! Keymapping is **context-aware**: in [`KeymapMode::Browse`] (List /
//! Timeline / Grid views) keys are interpreted as navigation / modal-
//! action intents, while in [`KeymapMode::Form`] (Add / Edit forms)
//! printable characters become [`Intent::TypeChar`] so the focused text
//! field receives them and Tab/Shift+Tab navigate fields instead of
//! cycling views.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::screens::ViewKind;

/// Whether the active screen interprets keypresses as navigation
/// (`Browse`) or as text input + field nav (`Form`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapMode {
    Browse,
    Form,
}

/// High-level user intent produced by a single key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Quit the application — force-exit regardless of modal state.
    Quit,
    /// Back out of the current modal state. The active screen interprets
    /// this contextually: in a top-level browse view it falls through to
    /// [`Intent::Quit`]; in multi-select Remove mode it discards the marks
    /// and returns to browse; in a form it dismisses the form.
    Cancel,
    /// Move selection one row up.
    NavUp,
    /// Move selection one row down.
    NavDown,
    /// Move selection one column left. Used by Grid and by `TextInput`
    /// cursor movement; List/Timeline ignore.
    NavLeft,
    /// Move selection one column right. Used by Grid and by `TextInput`
    /// cursor movement; List/Timeline ignore.
    NavRight,
    /// Move selection to the first row / cell.
    NavTop,
    /// Move selection to the last row / cell.
    NavBottom,
    /// Cycle to the next view (Tab in Browse mode):
    /// List → Timeline → Grid → List. Composition-Root level intent.
    CycleView,
    /// Jump to a specific view (number keys 1/2/3, Browse mode only).
    SwitchView(ViewKind),
    /// Cycle the active view's time granularity. List ignores; Timeline
    /// cycles month ↔ week; Grid cycles month ↔ week.
    CycleGranularity,
    /// Enter multi-select Remove mode (List view, Browse).
    OpenRemove,
    /// Open the Add form (List view, Browse).
    OpenAdd,
    /// Open the Edit form on the currently-selected event
    /// (List view, Browse).
    OpenEdit,
    /// Open / toggle the in-app help overlay. Sent by `?` in Browse mode;
    /// in the help overlay the same key re-emits and closes it.
    OpenHelp,
    /// Toggle the mark on the currently-selected row (List Remove mode).
    ToggleMark,
    /// Confirm the current modal action — in List Remove mode, submit
    /// the marked indices to `icscli::application::use_cases::remove`.
    Confirm,
    /// Form-mode intents (Phase 3b additions).
    /// Insert a typed character into the focused field.
    TypeChar(char),
    /// Delete the character immediately before the cursor.
    Backspace,
    /// Move focus to the next field (Tab in Form mode).
    NextField,
    /// Move focus to the previous field (Shift+Tab in Form mode).
    PrevField,
    /// Submit the entire form (Ctrl+S in Form mode, or Enter on the
    /// last field). AddForm validates and either returns
    /// `ScreenAction::SubmitAdd(...)` or stays put with an error banner.
    SubmitForm,
}

/// Map a single [`KeyEvent`] to an [`Intent`]. The mapping depends on
/// `mode` — see [`KeymapMode`].
pub fn map(event: KeyEvent, mode: KeymapMode) -> Option<Intent> {
    // crossterm emits Press / Release / Repeat. We only act on Press so
    // that a quick `q` tap doesn't quit twice.
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

        // Add form entry (Phase 3b).
        (KeyCode::Char('a'), KeyModifiers::NONE) => Some(Intent::OpenAdd),

        // Edit form entry (Phase 3c).
        (KeyCode::Char('e'), KeyModifiers::NONE) => Some(Intent::OpenEdit),

        // In-app help overlay. `?` is Shift+/ on most layouts, so this
        // arrives as Char('?') with the Shift modifier set.
        (KeyCode::Char('?'), _) => Some(Intent::OpenHelp),

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

fn map_form(event: KeyEvent) -> Option<Intent> {
    match (event.code, event.modifiers) {
        // Hard-exit shortcuts work everywhere.
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Intent::Quit),
        (KeyCode::Esc, _) => Some(Intent::Cancel),

        // Submission shortcuts.
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Intent::SubmitForm),
        (KeyCode::Enter, _) => Some(Intent::SubmitForm),

        // Field navigation.
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Intent::NextField),
        (KeyCode::BackTab, _) => Some(Intent::PrevField),
        (KeyCode::Tab, KeyModifiers::SHIFT) => Some(Intent::PrevField),
        (KeyCode::Down, _) => Some(Intent::NextField),
        (KeyCode::Up, _) => Some(Intent::PrevField),

        // Text-input editing.
        (KeyCode::Backspace, _) => Some(Intent::Backspace),
        (KeyCode::Left, _) => Some(Intent::NavLeft),
        (KeyCode::Right, _) => Some(Intent::NavRight),
        (KeyCode::Home, _) => Some(Intent::NavTop),
        (KeyCode::End, _) => Some(Intent::NavBottom),

        // Any printable character (incl. Shift-letters) becomes TypeChar.
        // We exclude Ctrl-modified keys so Ctrl+C / Ctrl+S land above.
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
    fn ctrl_c_quits_in_both_modes() {
        for mode in [KeymapMode::Browse, KeymapMode::Form] {
            assert_eq!(
                map(press(KeyCode::Char('c'), KeyModifiers::CONTROL), mode),
                Some(Intent::Quit)
            );
        }
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
