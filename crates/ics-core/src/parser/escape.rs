//! RFC 5545 §3.3.11 TEXT value escape handling.
//!
//! Typed TEXT fields (`SUMMARY`, `DESCRIPTION`, individual `CATEGORIES`
//! items, etc.) carry the following escape sequences on the wire:
//!
//! | wire | meaning |
//! |---|---|
//! | `\\` | backslash |
//! | `\;` | semicolon |
//! | `\,` | comma |
//! | `\n` or `\N` | newline (LF) |
//!
//! On parse we decode; on format we re-encode. Per ADR-018, `RawProperty.value`
//! stays raw — escapes are only interpreted for fields the typed model
//! understands and re-emits.

/// Decode a single TEXT value into a Rust `String`, interpreting the
/// escape sequences above. Unknown escape sequences (`\X` for `X` not in
/// the recognized set) pass through with the backslash preserved, so a
/// future spec evolution can decide their meaning without a corrupting
/// round-trip in the meantime.
pub fn decode_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some(';') => out.push(';'),
                Some(',') => out.push(','),
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Encode a Rust string for the wire, escaping the four reserved
/// characters that would otherwise be interpreted as structure.
pub fn encode_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            ';' => out.push_str(r"\;"),
            ',' => out.push_str(r"\,"),
            '\n' => out.push_str(r"\n"),
            _ => out.push(c),
        }
    }
    out
}

/// Split a multi-value TEXT property's raw wire value into individual
/// decoded items, respecting `\,` as a literal comma inside an item.
pub fn split_text_list(s: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            current.push('\\');
            if let Some(next) = chars.next() {
                current.push(next);
            }
        } else if c == ',' {
            items.push(decode_text(&current));
            current.clear();
        } else {
            current.push(c);
        }
    }
    items.push(decode_text(&current));
    items
}

/// Encode an item list into a single TEXT value, comma-joined with each
/// item internally escaped.
pub fn join_text_list(items: &[String]) -> String {
    items
        .iter()
        .map(|s| encode_text(s))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_passes_unescaped_text_through() {
        assert_eq!(decode_text("hello world"), "hello world");
    }

    #[test]
    fn decode_handles_each_recognized_escape() {
        assert_eq!(decode_text(r"a\\b"), r"a\b");
        assert_eq!(decode_text(r"a\;b"), "a;b");
        assert_eq!(decode_text(r"a\,b"), "a,b");
        assert_eq!(decode_text(r"a\nb"), "a\nb");
        assert_eq!(decode_text(r"a\Nb"), "a\nb");
    }

    #[test]
    fn decode_preserves_unknown_escape_sequences() {
        // Future-proofing: an unknown \X comes through as the literal
        // two characters so a re-encode produces the same wire form.
        assert_eq!(decode_text(r"a\Xb"), r"a\Xb");
    }

    #[test]
    fn decode_handles_trailing_lone_backslash() {
        assert_eq!(decode_text(r"abc\"), r"abc\");
    }

    #[test]
    fn encode_escapes_each_reserved_character() {
        assert_eq!(encode_text(r"a\b"), r"a\\b");
        assert_eq!(encode_text("a;b"), r"a\;b");
        assert_eq!(encode_text("a,b"), r"a\,b");
        assert_eq!(encode_text("a\nb"), r"a\nb");
    }

    #[test]
    fn encode_passes_plain_text_through_unchanged() {
        assert_eq!(encode_text("hello 憲法記念日"), "hello 憲法記念日");
    }

    #[test]
    fn decode_then_encode_is_stable_round_trip() {
        let original = r"Meeting, with \;semicolon and \,comma";
        let once = decode_text(original);
        let twice = encode_text(&once);
        let thrice = decode_text(&twice);
        // Once-decoded and twice-decoded values agree — the round-trip
        // through encode is semantically stable.
        assert_eq!(once, thrice);
    }

    #[test]
    fn split_text_list_basic() {
        assert_eq!(
            split_text_list("WORK,PERSONAL,HOLIDAY"),
            vec!["WORK", "PERSONAL", "HOLIDAY"]
        );
    }

    #[test]
    fn split_text_list_respects_escaped_comma() {
        // "a\,b,c" -> ["a,b", "c"]
        assert_eq!(split_text_list(r"a\,b,c"), vec!["a,b", "c"]);
    }

    #[test]
    fn split_text_list_handles_single_item() {
        assert_eq!(split_text_list("solo"), vec!["solo"]);
    }

    #[test]
    fn join_text_list_escapes_per_item() {
        let items = vec!["a,b".to_string(), "c;d".to_string()];
        assert_eq!(join_text_list(&items), r"a\,b,c\;d");
    }

    #[test]
    fn list_round_trip_preserves_items_with_special_chars() {
        let items = vec!["work, project A".to_string(), "personal".to_string()];
        let encoded = join_text_list(&items);
        let decoded = split_text_list(&encoded);
        assert_eq!(decoded, items);
    }
}
