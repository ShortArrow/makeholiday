//! Logical line tokenization per RFC 5545.
//!
//! After unfolding, each logical line has the shape
//!
//! ```text
//! NAME[;PARAM=VALUE...]:VALUE
//! ```
//!
//! This module turns one such text line into a `LogicalLine` token: the
//! property name (UPPERCASE-normalized), the list of parameters (keys
//! UPPERCASE, surrounding `"` stripped from values, order preserved),
//! and the raw text after the colon (escapes intact — TEXT escape
//! handling lives in ADR-019 Step 2).
//!
//! Dispatch sites in `parser/mod.rs` switch on `LogicalLine.name`
//! instead of `strip_prefix("NAME:")`, which also makes them tolerant of
//! properties that arrive with parameters (e.g. `UID;X-FOO=bar:abc`).

use crate::raw::RawProperty;

/// Tokenized property line.
///
/// Pub-promoted from `pub(crate)` to support out-of-tree consumers like
/// `icslint` that need to walk the source at the logical-line level
/// without committing to the typed `VCalendar` view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalLine<'a> {
    /// Property name, UPPERCASE-normalized.
    pub name: String,
    /// `(KEY, value)` pairs. Keys UPPERCASE; values keep their original
    /// casing with surrounding `"` stripped if present.
    pub params: Vec<(String, String)>,
    /// Text after the first `:`. Escapes intact; multi-byte UTF-8 intact.
    pub value: &'a str,
}

impl<'a> LogicalLine<'a> {
    /// Build a `RawProperty` for storage in `VEvent.unknown` or a vendor
    /// bundle's `unrecognized` slot. Allocates because `RawProperty`
    /// owns its `value`.
    pub fn to_raw_property(&self, source_index: u32) -> RawProperty {
        RawProperty {
            name: self.name.clone(),
            params: self.params.clone(),
            value: self.value.to_string(),
            source_index,
        }
    }
}

/// Parse one logical line into a `LogicalLine`. Returns `None` when the
/// line is malformed (no colon, no name).
///
/// Today this is a minimal parser — it splits on the first `:` and on
/// `;` boundaries in the prefix, without handling quoted-string param
/// values that span semicolons. ADR-019 Step 1 brings property routing
/// onto this primitive; a richer quoted-param parser arrives if/when a
/// real-world file demands it.
pub fn parse_logical_line(line: &str) -> Option<LogicalLine<'_>> {
    let colon = line.find(':')?;
    let prefix = &line[..colon];
    let value = &line[colon + 1..];

    let mut parts = prefix.split(';');
    let raw_name = parts.next()?;
    if raw_name.is_empty() {
        return None;
    }
    let name = raw_name.to_uppercase();
    let mut params = Vec::new();
    for p in parts {
        if let Some((k, v)) = p.split_once('=') {
            let v = v.trim_matches('"');
            params.push((k.to_uppercase(), v.to_string()));
        }
    }
    Some(LogicalLine {
        name,
        params,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_name_value() {
        let ll = parse_logical_line("UID:abc-123").unwrap();
        assert_eq!(ll.name, "UID");
        assert!(ll.params.is_empty());
        assert_eq!(ll.value, "abc-123");
    }

    #[test]
    fn name_uppercase_normalization() {
        let ll = parse_logical_line("uid:abc").unwrap();
        assert_eq!(ll.name, "UID");
    }

    #[test]
    fn single_param() {
        let ll = parse_logical_line("DTSTART;VALUE=DATE:20260101").unwrap();
        assert_eq!(ll.name, "DTSTART");
        assert_eq!(ll.params, vec![("VALUE".to_string(), "DATE".to_string())]);
        assert_eq!(ll.value, "20260101");
    }

    #[test]
    fn multiple_params_preserve_order() {
        let ll =
            parse_logical_line("DTSTART;TZID=Asia/Tokyo;VALUE=DATE-TIME:20260101T090000").unwrap();
        assert_eq!(
            ll.params,
            vec![
                ("TZID".to_string(), "Asia/Tokyo".to_string()),
                ("VALUE".to_string(), "DATE-TIME".to_string()),
            ]
        );
        assert_eq!(ll.value, "20260101T090000");
    }

    #[test]
    fn param_keys_uppercase_values_keep_case() {
        let ll = parse_logical_line("X-FOO;lang=ja-JP:hello").unwrap();
        assert_eq!(ll.params, vec![("LANG".to_string(), "ja-JP".to_string())]);
    }

    #[test]
    fn quoted_param_value_strips_quotes() {
        let ll = parse_logical_line(r#"X-FOO;LANG="ja-JP":hello"#).unwrap();
        assert_eq!(ll.params, vec![("LANG".to_string(), "ja-JP".to_string())]);
    }

    #[test]
    fn missing_colon_yields_none() {
        assert!(parse_logical_line("UIDabc").is_none());
    }

    #[test]
    fn empty_name_yields_none() {
        assert!(parse_logical_line(":value").is_none());
    }

    #[test]
    fn empty_value_is_ok() {
        let ll = parse_logical_line("UID:").unwrap();
        assert_eq!(ll.value, "");
    }

    #[test]
    fn to_raw_property_copies_name_params_and_assigns_index() {
        let ll = parse_logical_line("X-CUSTOM-FOO;LANG=en:hello").unwrap();
        let rp = ll.to_raw_property(7);
        assert_eq!(rp.name, "X-CUSTOM-FOO");
        assert_eq!(rp.params, vec![("LANG".to_string(), "en".to_string())]);
        assert_eq!(rp.value, "hello");
        assert_eq!(rp.source_index, 7);
    }

    #[test]
    fn value_can_contain_colon() {
        // Only the first colon splits name/params from value.
        let ll = parse_logical_line("DESCRIPTION:Meeting at 10:00").unwrap();
        assert_eq!(ll.value, "Meeting at 10:00");
    }

    #[test]
    fn multibyte_utf8_in_value() {
        let ll = parse_logical_line("SUMMARY:憲法記念日").unwrap();
        assert_eq!(ll.value, "憲法記念日");
    }
}
