//! RFC 5545 §3.1 content line unfolding.
//!
//! Physical lines longer than 75 octets are folded by inserting a CRLF
//! followed by a single SPACE or HTAB. The reader must reverse this
//! transformation before per-line parsing — the leading folding-marker
//! whitespace is part of the *fold*, not the value.
//!
//! Also handles a leading UTF-8 BOM (`U+FEFF`) tolerantly: many tools
//! (notably Outlook) emit one, and the rest of the parser assumes the
//! input starts at the first wire character.

/// Strip a leading UTF-8 BOM if present.
pub fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{FEFF}').unwrap_or(content)
}

/// Split `content` into RFC 5545 logical lines.
///
/// Steps:
/// 1. Drop a leading UTF-8 BOM if present.
/// 2. Accept either `CRLF` or `LF` as the physical line terminator.
/// 3. A line whose first byte is `SPACE` (0x20) or `HTAB` (0x09) is a
///    continuation of the previous logical line; the folding-marker
///    whitespace is dropped, and the remainder appends to the previous
///    logical line.
///
/// Returns owned `String`s because folded continuation joining means we
/// can no longer borrow slices of the input.
pub fn unfold(content: &str) -> Vec<String> {
    let content = strip_bom(content);
    let normalized = content.replace("\r\n", "\n");
    let mut logical: Vec<String> = Vec::new();
    let mut current: Option<String> = None;

    for line in normalized.split('\n') {
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t')) {
            match current.as_mut() {
                Some(c) => c.push_str(rest),
                None => current = Some(rest.to_string()),
            }
        } else {
            if let Some(c) = current.take() {
                logical.push(c);
            }
            current = Some(line.to_string());
        }
    }
    if let Some(c) = current.take() {
        logical.push(c);
    }
    // The final split element from a trailing newline is an empty string;
    // drop trailing empties so consumers don't see phantom blank logical
    // lines.
    while matches!(logical.last(), Some(s) if s.is_empty()) {
        logical.pop();
    }
    logical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_bom_removes_leading_bom_only() {
        assert_eq!(strip_bom("\u{FEFF}HELLO"), "HELLO");
        assert_eq!(strip_bom("HELLO"), "HELLO");
        // BOM in the middle is left alone — it is not a fold marker.
        assert_eq!(strip_bom("HEL\u{FEFF}LO"), "HEL\u{FEFF}LO");
    }

    #[test]
    fn unfold_passes_through_single_lines() {
        let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n";
        let logical = unfold(input);
        assert_eq!(
            logical,
            vec!["BEGIN:VCALENDAR", "VERSION:2.0", "END:VCALENDAR"]
        );
    }

    #[test]
    fn unfold_joins_space_continuation() {
        // A:long-value-foldedhere -> A:long-value-foldedhere on one logical line
        let input = "A:long-value-folded\r\n here\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["A:long-value-foldedhere"]);
    }

    #[test]
    fn unfold_joins_tab_continuation() {
        let input = "A:long-value-folded\r\n\there\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["A:long-value-foldedhere"]);
    }

    #[test]
    fn unfold_joins_multiple_continuations() {
        let input = "A:part1\r\n part2\r\n part3\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["A:part1part2part3"]);
    }

    #[test]
    fn unfold_accepts_lf_only_line_terminators() {
        let input = "A:foo\nB:bar\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["A:foo", "B:bar"]);
    }

    #[test]
    fn unfold_strips_leading_bom() {
        let input = "\u{FEFF}BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["BEGIN:VCALENDAR", "END:VCALENDAR"]);
    }

    #[test]
    fn unfold_preserves_utf8_in_continuation() {
        // Multi-byte UTF-8 content in continuation lines must survive intact.
        let input = "SUMMARY:憲法\r\n 記念日\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["SUMMARY:憲法記念日"]);
    }

    #[test]
    fn unfold_drops_trailing_empty_logical_line() {
        // A trailing CRLF yields a phantom empty split element; we must
        // not surface it as a logical line.
        let input = "A:foo\r\n";
        let logical = unfold(input);
        assert_eq!(logical, vec!["A:foo"]);
    }
}
