//! Use cases — free functions orchestrating I/O via `CalendarRepository`
//! and pure domain logic via `ics_core`.

use std::io::{self, BufRead, Write};

use chrono::NaiveDate;
use ics_core::{self as ics, VEvent};

use crate::application::ports::CalendarRepository;
use crate::error::{MhError, Result};

pub fn init<R: CalendarRepository>(repo: &R) -> Result<()> {
    repo.create()
}

fn resolve_add_params(
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<(String, NaiveDate, Option<NaiveDate>)> {
    let interactive = summary.is_none() || start.is_none();
    let summary = match summary {
        Some(s) => s.to_string(),
        None => {
            write!(writer, "Summary: ").map_err(|e| MhError::io("<stdin>", e))?;
            writer.flush().map_err(|e| MhError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| MhError::io("<stdin>", e))?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return Err(MhError::InvalidInput("Summary cannot be empty".to_string()));
            }
            trimmed
        }
    };
    let start = match start {
        Some(s) => s,
        None => {
            write!(writer, "Start date: ").map_err(|e| MhError::io("<stdin>", e))?;
            writer.flush().map_err(|e| MhError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| MhError::io("<stdin>", e))?;
            crate::input::parse_date(line.trim()).map_err(MhError::InvalidInput)?
        }
    };
    let end = match end {
        Some(e) => Some(e),
        None if !interactive => None,
        None => {
            write!(writer, "End date (empty for single day): ")
                .map_err(|e| MhError::io("<stdin>", e))?;
            writer.flush().map_err(|e| MhError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| MhError::io("<stdin>", e))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(crate::input::parse_date(trimmed).map_err(MhError::InvalidInput)?)
            }
        }
    };
    Ok((summary, start, end))
}

#[allow(clippy::too_many_arguments)] // ADR-001 Migration will replace flat args with a request struct
pub fn add<R: CalendarRepository>(
    repo: &R,
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    busystatus: ics::microsoft::MsBusyStatus,
    class: Option<ics::EventClass>,
    categories: Vec<String>,
    icon: Option<String>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut writer = io::stderr();
    let (summary, start, end) = resolve_add_params(summary, start, end, &mut reader, &mut writer)?;

    let dtend = match end {
        Some(e) if e < start => {
            return Err(MhError::InvalidInput(
                "--end must not be before --start".to_string(),
            ));
        }
        Some(e) if e == start => start + chrono::Days::new(1),
        Some(e) => e + chrono::Days::new(1),
        None => start + chrono::Days::new(1),
    };

    let content = repo.load()?;
    let mut cal = ics::parse_calendar(&content)?;

    let event = VEvent {
        uid: uuid::Uuid::new_v4().to_string(),
        dtstamp: chrono::Utc::now().naive_utc(),
        dtstart: start,
        dtend,
        summary,
        transp: None,
        class,
        categories,
        icon,
        microsoft: Some(ics::microsoft::EventExtensions {
            busystatus: Some(busystatus),
        }),
        unknown: vec![],
        unrecognized_components: vec![],
    };

    cal.events.push(event.clone());
    repo.save(&ics::format_calendar(&cal))?;

    let line = ics::format_event_line(&event);
    eprintln!("Added: {line}");
    Ok(())
}

pub fn list<R: CalendarRepository>(
    repo: &R,
    sort_keys: &[ics::SortKey],
    descending: bool,
    json: bool,
) -> Result<String> {
    let content = repo.load()?;
    let cal = ics::parse_calendar(&content)?;
    let events = if sort_keys.is_empty() {
        cal.events
    } else {
        ics::sort_events(&cal.events, sort_keys, descending)
    };
    if json {
        serde_json::to_string_pretty(&events)
            .map_err(|e| MhError::InvalidInput(format!("JSON error: {e}")))
    } else {
        let output = events
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}: {}", i + 1, ics::format_event_line(e)))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(output)
    }
}

pub fn remove<R: CalendarRepository>(
    repo: &R,
    summary: Option<&str>,
    target: Option<&str>,
) -> Result<()> {
    let content = repo.load()?;
    let cal = ics::parse_calendar(&content)?;
    let events = &cal.events;

    let (new_cal, removed_desc) = match (summary, target) {
        (Some(_), Some(_)) => {
            return Err(MhError::Conflict(
                "Cannot specify both --summary and index target".to_string(),
            ));
        }
        (Some(s), None) => {
            let removed: Vec<_> = events.iter().filter(|e| e.summary == s).collect();
            if removed.is_empty() {
                return Err(MhError::NotFound(format!(
                    "No event found with summary: {s}"
                )));
            }
            let desc = removed
                .iter()
                .map(|e| ics::format_event_line(e))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_event_by_summary(&cal, s)?, desc)
        }
        (None, Some(spec)) => {
            let indices = ics::parse_indices(spec, events.len())?;
            let desc = indices
                .iter()
                .map(|&i| ics::format_event_line(&events[i - 1]))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_events_by_indices(&cal, &indices)?, desc)
        }
        (None, None) => {
            if events.is_empty() {
                return Err(MhError::NotFound("No events to remove".to_string()));
            }
            for (i, e) in events.iter().enumerate() {
                eprintln!("{}: {}", i + 1, ics::format_event_line(e));
            }
            eprint!("Remove # (or 'q' to cancel): ");
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .map_err(|e| MhError::io("<stdin>", e))?;
            let trimmed = line.trim();
            if trimmed == "q" || trimmed.is_empty() {
                return Ok(());
            }
            let indices = ics::parse_indices(trimmed, events.len())?;
            let desc = indices
                .iter()
                .map(|&i| ics::format_event_line(&events[i - 1]))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_events_by_indices(&cal, &indices)?, desc)
        }
    };

    repo.save(&ics::format_calendar(&new_cal))?;
    eprintln!("Removed: {removed_desc}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::FileCalendarRepository;
    use tempfile::TempDir;

    fn temp_repo(dir: &TempDir, name: &str) -> FileCalendarRepository {
        FileCalendarRepository::new(dir.path().join(name))
    }

    #[test]
    fn init_creates_valid_ics() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        let content = repo.load().unwrap();
        let expected = ics::format_calendar(&ics::VCalendar::new("-//makeholiday//EN"));
        assert_eq!(content, expected);
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        let result = init(&repo);
        assert!(matches!(result, Err(MhError::AlreadyExists { .. })));
    }

    fn add_free(repo: &FileCalendarRepository, summary: &str, start: NaiveDate) {
        add(
            repo,
            Some(summary),
            Some(start),
            None,
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            None,
        )
        .unwrap();
    }

    fn add_free_with_end(
        repo: &FileCalendarRepository,
        summary: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) {
        add(
            repo,
            Some(summary),
            Some(start),
            Some(end),
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            None,
        )
        .unwrap();
    }

    #[test]
    fn add_one_event() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let content = repo.load().unwrap();
        assert!(content.contains("SUMMARY:元日"));
        assert!(content.contains("BEGIN:VEVENT"));
        assert!(content.contains("DTSTAMP:"));
    }

    #[test]
    fn add_two_events_distinct_uids() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        add_free(
            &repo,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
        );
        let content = repo.load().unwrap();
        let cal = ics::parse_calendar(&content).unwrap();
        assert_eq!(cal.events.len(), 2);
        assert_ne!(cal.events[0].uid, cal.events[1].uid);
    }

    #[test]
    fn list_returns_numbered_lines() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        add_free_with_end(
            &repo,
            "年末年始",
            NaiveDate::from_ymd_opt(2026, 12, 29).unwrap(),
            NaiveDate::from_ymd_opt(2027, 1, 3).unwrap(),
        );
        let output = list(&repo, &[], false, false).unwrap();
        assert!(output.contains("1: 2026-01-01 : 元日"));
        assert!(output.contains("2: 2026-12-29 to 2027-01-03 : 年末年始"));
    }

    #[test]
    fn add_end_before_start_errors() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        let result = add(
            &repo,
            Some("invalid"),
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            None,
        );
        assert!(matches!(result, Err(MhError::InvalidInput(_))));
    }

    #[test]
    fn remove_by_summary() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        add_free(
            &repo,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
        );
        remove(&repo, Some("元日"), None).unwrap();
        let output = list(&repo, &[], false, false).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }

    #[test]
    fn remove_by_index() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        add_free(
            &repo,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
        );
        remove(&repo, None, Some("1")).unwrap();
        let output = list(&repo, &[], false, false).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }

    #[test]
    fn add_with_busystatus_and_class() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add(
            &repo,
            Some("不在"),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            None,
            ics::microsoft::MsBusyStatus::Oof,
            Some(ics::EventClass::Private),
            vec![],
            None,
        )
        .unwrap();
        let content = repo.load().unwrap();
        assert!(content.contains("X-MICROSOFT-CDO-BUSYSTATUS:OOF"));
        assert!(content.contains("TRANSP:OPAQUE"));
        assert!(content.contains("CLASS:PRIVATE"));
    }
}
