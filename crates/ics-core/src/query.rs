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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_event;

    fn vcal(events: Vec<VEvent>) -> VCalendar {
        VCalendar {
            events,
            ..VCalendar::new("-//makeholiday//EN")
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
}
