//! Event display helpers shared between use cases (status messages) and
//! the future presentation-layer list output.

use ics_core::VEvent;

use crate::icons::read_icon;

/// Format `event` as a single human-readable line used by `list` and
/// the `add` / `remove` confirmation messages.
///
/// `2026-01-01 : 元日`
/// `2026-12-29 to 2027-01-03 : 年末年始`
/// `2026-06-15 : 出張 [airplane]`
pub fn format_event_line(event: &VEvent) -> String {
    let start = event.dtstart;
    let end = event.dtend - chrono::Days::new(1);
    let date_part = if start == end {
        format!("{}", start.format("%Y-%m-%d"))
    } else {
        format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"))
    };
    let icon_part = read_icon(event)
        .map(|i| format!(" [{i}]"))
        .unwrap_or_default();
    format!("{date_part} : {}{icon_part}", event.summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::write_icon;
    use chrono::NaiveDate;
    use ics_core::VEvent;

    fn make_event(
        uid: &str,
        start: (i32, u32, u32),
        end: (i32, u32, u32),
        summary: &str,
    ) -> VEvent {
        let dtstamp = NaiveDate::from_ymd_opt(2026, 3, 27)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        VEvent {
            uid: uid.to_string(),
            dtstamp,
            dtstart: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            dtend: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            summary: summary.to_string(),
            transp: None,
            class: None,
            categories: vec![],
            microsoft: None,
            unknown: vec![],
            unrecognized_components: vec![],
        }
    }

    #[test]
    fn single_day() {
        let event = make_event("x", (2026, 1, 1), (2026, 1, 2), "元日");
        assert_eq!(format_event_line(&event), "2026-01-01 : 元日");
    }

    #[test]
    fn multi_day() {
        let event = make_event("y", (2026, 12, 29), (2027, 1, 4), "年末年始");
        assert_eq!(
            format_event_line(&event),
            "2026-12-29 to 2027-01-03 : 年末年始"
        );
    }

    #[test]
    fn with_icon() {
        let mut event = make_event("x", (2026, 6, 15), (2026, 6, 16), "出張");
        write_icon(&mut event, "airplane");
        assert_eq!(format_event_line(&event), "2026-06-15 : 出張 [airplane]");
    }
}
