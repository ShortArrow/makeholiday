use chrono::NaiveDate;

use crate::error::{Error, Result};
use crate::event::VEvent;
use crate::vcalendar::VCalendar;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKey {
    Start,
    End,
    Summary,
}

pub fn sort_events(events: &[VEvent], keys: &[SortKey], descending: bool) -> Vec<VEvent> {
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        let ord = keys
            .iter()
            .map(|key| match key {
                SortKey::Start => a.dtstart.cmp(&b.dtstart),
                SortKey::End => a.dtend.cmp(&b.dtend),
                SortKey::Summary => a.summary.cmp(&b.summary),
            })
            .find(|o| o.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal);
        if descending { ord.reverse() } else { ord }
    });
    sorted
}

/// Drop events whose `summary` matches the given string. Returns a new
/// `VCalendar` with the same calendar-level fields and any unrecognized
/// components preserved. Fails with a `Parse` error if no matching event
/// is found.
pub fn remove_event_by_summary(cal: &VCalendar, summary: &str) -> Result<VCalendar> {
    let remaining: Vec<VEvent> = cal
        .events
        .iter()
        .filter(|e| e.summary != summary)
        .cloned()
        .collect();
    if remaining.len() == cal.events.len() {
        return Err(Error::parse(format!(
            "No event found with summary: {summary}"
        )));
    }
    Ok(VCalendar {
        events: remaining,
        ..cal.clone()
    })
}

/// Drop events whose 1-based index appears in `indices`. Returns a new
/// `VCalendar`.
pub fn remove_events_by_indices(cal: &VCalendar, indices: &[usize]) -> Result<VCalendar> {
    for &idx in indices {
        if idx == 0 || idx > cal.events.len() {
            return Err(Error::parse(format!(
                "Index {idx} out of range (1-{})",
                cal.events.len()
            )));
        }
    }
    let remaining: Vec<VEvent> = cal
        .events
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices.contains(&(i + 1)))
        .map(|(_, e)| e.clone())
        .collect();
    Ok(VCalendar {
        events: remaining,
        ..cal.clone()
    })
}

/// Return a new `VCalendar` containing only the events that overlap the
/// closed range `[from, to]`. Either bound may be `None` to leave that
/// side of the range open.
///
/// Overlap means the event's date span intersects the range. Because
/// RFC 5545 DTEND for `DATE` values is exclusive, an event with
/// `dtstart..dtend` overlaps `[from, to]` iff `dtend > from` and
/// `dtstart <= to` (each bound check is skipped when the bound is `None`).
///
/// Edge cases are total — no error is returned:
/// - both bounds `None` → range is `(-∞, +∞)` → all events match.
/// - `from > to` → the range is empty → no events match.
///
/// Calendar-level fields (prodid, version, X-WR-*) and unrecognized
/// components (incl. VTODOs per ADR-021) are preserved verbatim from
/// `cal`. Events appear in the input order. Policy decisions about
/// which bound combinations are user-facing inputs live in the caller
/// (e.g., `icscli` use case validates that at least one bound is given).
pub fn split_by_date_range(
    cal: &VCalendar,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> VCalendar {
    let matched: Vec<VEvent> = cal
        .events
        .iter()
        .filter(|e| from.is_none_or(|f| e.dtend > f) && to.is_none_or(|t| e.dtstart <= t))
        .cloned()
        .collect();
    VCalendar {
        events: matched,
        ..cal.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_event;

    fn vcal(events: Vec<VEvent>) -> VCalendar {
        VCalendar {
            events,
            ..VCalendar::new("-//icscli//EN")
        }
    }

    fn unsorted_events() -> Vec<VEvent> {
        vec![
            make_event("c", (2026, 5, 3), (2026, 5, 6), "憲法記念日"),
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ]
    }

    #[test]
    fn sort_by_start_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Start], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_start_desc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Start], true);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["憲法記念日", "建国記念の日", "元日"]);
    }

    #[test]
    fn sort_by_end_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::End], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_summary_asc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Summary], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["元日", "建国記念の日", "憲法記念日"]);
    }

    #[test]
    fn sort_by_summary_desc() {
        let sorted = sort_events(&unsorted_events(), &[SortKey::Summary], true);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["憲法記念日", "建国記念の日", "元日"]);
    }

    #[test]
    fn sort_multi_key() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "B休日"),
            make_event("b", (2026, 1, 1), (2026, 1, 2), "A休日"),
            make_event("c", (2026, 2, 1), (2026, 2, 2), "C休日"),
        ];
        let sorted = sort_events(&events, &[SortKey::Start, SortKey::Summary], false);
        let summaries: Vec<_> = sorted.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["A休日", "B休日", "C休日"]);
    }

    #[test]
    fn remove_by_summary() {
        let cal = vcal(vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ]);
        let updated = remove_event_by_summary(&cal, "元日").unwrap();
        assert_eq!(updated.events.len(), 1);
        assert_eq!(updated.events[0].summary, "建国記念の日");
    }

    #[test]
    fn remove_by_summary_not_found() {
        let cal = vcal(vec![make_event("a", (2026, 1, 1), (2026, 1, 2), "元日")]);
        let result = remove_event_by_summary(&cal, "存在しない");
        assert!(result.is_err());
    }

    #[test]
    fn remove_multiple_by_indices() {
        let cal = vcal(vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "A"),
            make_event("b", (2026, 2, 1), (2026, 2, 2), "B"),
            make_event("c", (2026, 3, 1), (2026, 3, 2), "C"),
            make_event("d", (2026, 4, 1), (2026, 4, 2), "D"),
        ]);
        let updated = remove_events_by_indices(&cal, &[2, 4]).unwrap();
        let summaries: Vec<_> = updated.events.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["A", "C"]);
    }

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn split_fixture() -> VCalendar {
        // Events for split tests. End dates are RFC-style exclusive (one past last day).
        vcal(vec![
            // before Q1: 2025-12-30..2025-12-31 (inclusive: 2025-12-30)
            make_event("a", (2025, 12, 30), (2025, 12, 31), "前年末"),
            // straddles the 'from' boundary: 2025-12-31..2026-01-03 (inclusive: 12-31, 01-01, 01-02)
            make_event("b", (2025, 12, 31), (2026, 1, 3), "年末年始"),
            // fully inside Q1: 2026-02-11..2026-02-12 (inclusive: 02-11)
            make_event("c", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
            // straddles the 'to' boundary: 2026-03-30..2026-04-02 (inclusive: 03-30, 03-31, 04-01)
            make_event("d", (2026, 3, 30), (2026, 4, 2), "春休み"),
            // after Q1: 2026-04-29..2026-04-30 (inclusive: 2026-04-29)
            make_event("e", (2026, 4, 29), (2026, 4, 30), "昭和の日"),
        ])
    }

    #[test]
    fn split_by_range_includes_straddlers_and_fully_inside() {
        let cal = split_fixture();
        let q1 = split_by_date_range(&cal, Some(ymd(2026, 1, 1)), Some(ymd(2026, 3, 31)));
        let uids: Vec<_> = q1.events.iter().map(|e| e.uid.as_str()).collect();
        assert_eq!(uids, vec!["b", "c", "d"]);
    }

    #[test]
    fn split_by_range_preserves_input_order() {
        let cal = split_fixture();
        let result = split_by_date_range(&cal, Some(ymd(2025, 1, 1)), Some(ymd(2027, 12, 31)));
        let uids: Vec<_> = result.events.iter().map(|e| e.uid.as_str()).collect();
        assert_eq!(uids, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn split_with_only_from_acts_as_lower_half_open() {
        let cal = split_fixture();
        let result = split_by_date_range(&cal, Some(ymd(2026, 4, 1)), None);
        let uids: Vec<_> = result.events.iter().map(|e| e.uid.as_str()).collect();
        // 'd' straddles 2026-04-01 (inclusive day 04-01), 'e' is after.
        assert_eq!(uids, vec!["d", "e"]);
    }

    #[test]
    fn split_with_only_to_acts_as_upper_half_open() {
        let cal = split_fixture();
        let result = split_by_date_range(&cal, None, Some(ymd(2025, 12, 31)));
        let uids: Vec<_> = result.events.iter().map(|e| e.uid.as_str()).collect();
        // 'a' fully before, 'b' starts on 12-31 (matches dtstart <= to).
        assert_eq!(uids, vec!["a", "b"]);
    }

    #[test]
    fn split_with_no_match_returns_empty_events() {
        let cal = split_fixture();
        let result = split_by_date_range(&cal, Some(ymd(2030, 1, 1)), Some(ymd(2030, 12, 31)));
        assert!(result.events.is_empty());
    }

    #[test]
    fn split_preserves_calendar_level_fields() {
        let cal = VCalendar {
            prodid: "-//custom//PRODID".to_string(),
            ..split_fixture()
        };
        let result = split_by_date_range(&cal, Some(ymd(2026, 1, 1)), Some(ymd(2026, 3, 31)));
        assert_eq!(result.prodid, "-//custom//PRODID");
        assert_eq!(result.version, cal.version);
    }

    #[test]
    fn split_with_both_bounds_none_returns_all_events() {
        // Total semantics: (-∞, +∞) matches everything. Caller is
        // responsible for rejecting "no bounds" if its UX requires it.
        let cal = split_fixture();
        let result = split_by_date_range(&cal, None, None);
        assert_eq!(result.events.len(), cal.events.len());
    }

    #[test]
    fn split_with_from_after_to_returns_empty() {
        // Total semantics: an empty range matches nothing.
        let cal = split_fixture();
        let result = split_by_date_range(&cal, Some(ymd(2026, 6, 1)), Some(ymd(2026, 3, 1)));
        assert!(result.events.is_empty());
    }

    #[test]
    fn split_does_not_mutate_input() {
        let cal = split_fixture();
        let before_len = cal.events.len();
        let _ = split_by_date_range(&cal, Some(ymd(2026, 1, 1)), Some(ymd(2026, 3, 31)));
        assert_eq!(cal.events.len(), before_len);
    }
}
