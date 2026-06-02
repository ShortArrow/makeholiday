//! Single-line text input widget for AddForm / EditForm.
//!
//! Cursor positions are tracked in *character* offsets (not byte offsets)
//! so multi-byte UTF-8 sequences — Japanese summary text, accented Latin,
//! emoji — edit predictably. Rendering inserts a "▏" indicator at the
//! cursor when the field is focused.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

const CURSOR_GLYPH: &str = "▏";

#[derive(Debug, Clone, Default)]
pub struct TextInput {
    buffer: String,
    /// Cursor position in characters (0 = before the first char).
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(value: impl Into<String>) -> Self {
        let buffer: String = value.into();
        let cursor = buffer.chars().count();
        Self { buffer, cursor }
    }

    pub fn value(&self) -> &str {
        &self.buffer
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len_chars(&self) -> usize {
        self.buffer.chars().count()
    }

    /// Replace the buffer wholesale and reset the cursor to end.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.buffer = value.into();
        self.cursor = self.len_chars();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_offset = char_offset_to_byte_offset(&self.buffer, self.cursor);
        self.buffer.insert(byte_offset, c);
        self.cursor += 1;
    }

    /// Delete the character immediately before the cursor (DEL-backwards).
    /// No-op when the cursor is at the start.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let new_cursor = self.cursor - 1;
        let start = char_offset_to_byte_offset(&self.buffer, new_cursor);
        let end = char_offset_to_byte_offset(&self.buffer, self.cursor);
        self.buffer.replace_range(start..end, "");
        self.cursor = new_cursor;
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.len_chars() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.len_chars();
    }

    /// Render the input into `area`. When `focused`, a `"▏"` cursor
    /// indicator is inserted at the cursor position and the surrounding
    /// text gains a yellow accent.
    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let pre: String = self.buffer.chars().take(self.cursor).collect();
        let post: String = self.buffer.chars().skip(self.cursor).collect();

        let line = if focused {
            Line::from(vec![
                Span::raw(pre),
                Span::styled(
                    CURSOR_GLYPH,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
                Span::raw(post),
            ])
        } else {
            Line::from(self.buffer.as_str())
        };
        let style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(line).style(style), area);
    }
}

fn char_offset_to_byte_offset(s: &str, char_offset: usize) -> usize {
    s.char_indices()
        .nth(char_offset)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty_with_cursor_zero() {
        let t = TextInput::new();
        assert!(t.is_empty());
        assert_eq!(t.cursor(), 0);
        assert_eq!(t.value(), "");
    }

    #[test]
    fn with_value_places_cursor_at_end() {
        let t = TextInput::with_value("hello");
        assert_eq!(t.value(), "hello");
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn insert_char_appends_and_advances() {
        let mut t = TextInput::new();
        t.insert_char('a');
        t.insert_char('b');
        t.insert_char('c');
        assert_eq!(t.value(), "abc");
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn insert_char_in_middle() {
        let mut t = TextInput::with_value("ac");
        t.move_left();
        assert_eq!(t.cursor(), 1);
        t.insert_char('b');
        assert_eq!(t.value(), "abc");
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn backspace_removes_previous_char() {
        let mut t = TextInput::with_value("abc");
        t.backspace();
        assert_eq!(t.value(), "ab");
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut t = TextInput::with_value("abc");
        t.move_home();
        t.backspace();
        assert_eq!(t.value(), "abc");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn move_left_clamps_at_zero() {
        let mut t = TextInput::with_value("a");
        t.move_left();
        t.move_left();
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn move_right_clamps_at_end() {
        let mut t = TextInput::with_value("ab");
        t.move_right();
        t.move_right();
        t.move_right();
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn home_and_end_jump_to_bounds() {
        let mut t = TextInput::with_value("hello");
        t.move_home();
        assert_eq!(t.cursor(), 0);
        t.move_end();
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn unicode_chars_use_char_offsets_not_bytes() {
        let mut t = TextInput::new();
        // 元 is 3 bytes in UTF-8 but 1 char.
        t.insert_char('元');
        t.insert_char('日');
        assert_eq!(t.value(), "元日");
        assert_eq!(t.cursor(), 2);
        assert_eq!(t.len_chars(), 2);
        // Backspace removes the multi-byte char as a single unit.
        t.backspace();
        assert_eq!(t.value(), "元");
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn unicode_insert_in_middle() {
        let mut t = TextInput::with_value("ac");
        t.move_left();
        t.insert_char('元');
        assert_eq!(t.value(), "a元c");
    }

    #[test]
    fn clear_resets_everything() {
        let mut t = TextInput::with_value("hello");
        t.clear();
        assert!(t.is_empty());
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn set_value_resets_cursor_to_end() {
        let mut t = TextInput::with_value("a");
        t.set_value("hello");
        assert_eq!(t.value(), "hello");
        assert_eq!(t.cursor(), 5);
    }
}
