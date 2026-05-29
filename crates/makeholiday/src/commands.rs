use std::io::{self, BufRead, Write};
use std::path::Path;

use chrono::NaiveDate;

use ics_core::{self as ics, VEvent};

use crate::error::{MhError, Result};

pub fn init(file: &Path) -> Result<()> {
    if file.exists() {
        return Err(MhError::already_exists(file));
    }
    let content = ics::format_calendar(&[]);
    std::fs::write(file, content.as_bytes()).map_err(|e| MhError::io(file, e))
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
            crate::cli::parse_date(line.trim()).map_err(MhError::InvalidInput)?
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
                Some(crate::cli::parse_date(trimmed).map_err(MhError::InvalidInput)?)
            }
        }
    };
    Ok((summary, start, end))
}

#[allow(clippy::too_many_arguments)] // ADR-009/010 restructure will replace this with a request struct
pub fn add(
    file: &Path,
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    busystatus: ics::BusyStatus,
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

    let content = std::fs::read_to_string(file).map_err(|e| MhError::io(file, e))?;

    let event = VEvent {
        uid: uuid::Uuid::new_v4().to_string(),
        dtstamp: chrono::Utc::now().naive_utc(),
        dtstart: start,
        dtend,
        summary,
        busystatus,
        class,
        categories,
        icon,
    };

    let new_content = ics::insert_event(&content, &event)?;
    std::fs::write(file, new_content.as_bytes()).map_err(|e| MhError::io(file, e))?;

    let line = ics::format_event_line(&event);
    eprintln!("Added: {line}");
    Ok(())
}

pub fn list(
    file: &Path,
    sort_keys: &[ics::SortKey],
    descending: bool,
    json: bool,
) -> Result<String> {
    let content = std::fs::read_to_string(file).map_err(|e| MhError::io(file, e))?;
    let events = ics::parse_events(&content)?;
    let events = if sort_keys.is_empty() {
        events
    } else {
        ics::sort_events(&events, sort_keys, descending)
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

pub fn remove(file: &Path, summary: Option<&str>, target: Option<&str>) -> Result<()> {
    let content = std::fs::read_to_string(file).map_err(|e| MhError::io(file, e))?;
    let events = ics::parse_events(&content)?;

    let (new_content, removed_desc) = match (summary, target) {
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
            (ics::remove_event_by_summary(&content, s)?, desc)
        }
        (None, Some(spec)) => {
            let indices = ics::parse_indices(spec, events.len())?;
            let desc = indices
                .iter()
                .map(|&i| ics::format_event_line(&events[i - 1]))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_events_by_indices(&content, &indices)?, desc)
        }
        (None, None) => {
            // Interactive mode
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
            (ics::remove_events_by_indices(&content, &indices)?, desc)
        }
    };

    std::fs::write(file, new_content.as_bytes()).map_err(|e| MhError::io(file, e))?;
    eprintln!("Removed: {removed_desc}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_file(dir: &TempDir, name: &str) -> std::path::PathBuf {
        dir.path().join(name)
    }

    #[test]
    fn init_creates_valid_ics() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, ics::format_calendar(&[]));
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let result = init(&path);
        assert!(matches!(result, Err(MhError::AlreadyExists { .. })));
    }

    fn add_free(path: &std::path::Path, summary: &str, start: NaiveDate, end: Option<NaiveDate>) {
        add(
            path,
            Some(summary),
            Some(start),
            end,
            ics::BusyStatus::Free,
            None,
            vec![],
            None,
        )
        .unwrap();
    }

    #[test]
    fn add_one_event() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add_free(
            &path,
            "元日",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("SUMMARY:元日"));
        assert!(content.contains("BEGIN:VEVENT"));
        assert!(content.contains("DTSTAMP:"));
    }

    #[test]
    fn add_two_events_distinct_uids() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add_free(
            &path,
            "元日",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
        );
        add_free(
            &path,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
            None,
        );
        let content = std::fs::read_to_string(&path).unwrap();
        let events = ics::parse_events(&content).unwrap();
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].uid, events[1].uid);
    }

    #[test]
    fn list_returns_numbered_lines() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add_free(
            &path,
            "元日",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
        );
        add_free(
            &path,
            "年末年始",
            NaiveDate::from_ymd_opt(2026, 12, 29).unwrap(),
            Some(NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()),
        );
        let output = list(&path, &[], false, false).unwrap();
        assert!(output.contains("1: 2026-01-01 : 元日"));
        assert!(output.contains("2: 2026-12-29 to 2027-01-03 : 年末年始"));
    }

    #[test]
    fn add_end_before_start_errors() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let result = add(
            &path,
            Some("invalid"),
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            ics::BusyStatus::Free,
            None,
            vec![],
            None,
        );
        assert!(matches!(result, Err(MhError::InvalidInput(_))));
    }

    #[test]
    fn remove_by_summary() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add_free(
            &path,
            "元日",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
        );
        add_free(
            &path,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
            None,
        );
        remove(&path, Some("元日"), None).unwrap();
        let output = list(&path, &[], false, false).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }

    #[test]
    fn remove_by_index() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add_free(
            &path,
            "元日",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
        );
        add_free(
            &path,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
            None,
        );
        remove(&path, None, Some("1")).unwrap();
        let output = list(&path, &[], false, false).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }

    #[test]
    fn add_with_busystatus_and_class() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(
            &path,
            Some("不在"),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            None,
            ics::BusyStatus::Oof,
            Some(ics::EventClass::Private),
            vec![],
            None,
        )
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("X-MICROSOFT-CDO-BUSYSTATUS:OOF"));
        assert!(content.contains("TRANSP:OPAQUE"));
        assert!(content.contains("CLASS:PRIVATE"));
    }
}
