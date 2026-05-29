use crate::calendar::format_calendar;
use crate::event::VEvent;
use crate::parser::parse_events;

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

pub fn remove_event_by_summary(content: &str, summary: &str) -> Result<String, String> {
    let events = parse_events(content)?;
    let remaining: Vec<_> = events.iter().filter(|e| e.summary != summary).collect();
    if remaining.len() == events.len() {
        return Err(format!("No event found with summary: {summary}"));
    }
    Ok(format_calendar(
        &remaining.into_iter().cloned().collect::<Vec<_>>(),
    ))
}

pub fn remove_events_by_indices(content: &str, indices: &[usize]) -> Result<String, String> {
    let events = parse_events(content)?;
    for &idx in indices {
        if idx == 0 || idx > events.len() {
            return Err(format!("Index {idx} out of range (1-{})", events.len()));
        }
    }
    let remaining: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices.contains(&(i + 1)))
        .map(|(_, e)| e.clone())
        .collect();
    Ok(format_calendar(&remaining))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_event;

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
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "元日"),
            make_event("b", (2026, 2, 11), (2026, 2, 12), "建国記念の日"),
        ];
        let cal = format_calendar(&events);
        let result = remove_event_by_summary(&cal, "元日").unwrap();
        let parsed = parse_events(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].summary, "建国記念の日");
    }

    #[test]
    fn remove_by_summary_not_found() {
        let cal = format_calendar(&[make_event("a", (2026, 1, 1), (2026, 1, 2), "元日")]);
        let result = remove_event_by_summary(&cal, "存在しない");
        assert!(result.is_err());
    }

    #[test]
    fn remove_multiple_by_indices() {
        let events = vec![
            make_event("a", (2026, 1, 1), (2026, 1, 2), "A"),
            make_event("b", (2026, 2, 1), (2026, 2, 2), "B"),
            make_event("c", (2026, 3, 1), (2026, 3, 2), "C"),
            make_event("d", (2026, 4, 1), (2026, 4, 2), "D"),
        ];
        let cal = format_calendar(&events);
        let result = remove_events_by_indices(&cal, &[2, 4]).unwrap();
        let parsed = parse_events(&result).unwrap();
        let summaries: Vec<_> = parsed.iter().map(|e| e.summary.as_str()).collect();
        assert_eq!(summaries, vec!["A", "C"]);
    }
}
