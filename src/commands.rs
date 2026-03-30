use std::io::{self, BufRead, Write};
use std::path::Path;

use chrono::NaiveDate;

use crate::ics::{self, VEvent};

pub fn init(file: &Path) -> Result<(), String> {
    if file.exists() {
        return Err(format!("File already exists: {}", file.display()));
    }
    let content = ics::format_calendar(&[]);
    std::fs::write(file, content.as_bytes()).map_err(|e| format!("Failed to write: {e}"))
}

fn resolve_add_params(
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<(String, NaiveDate, Option<NaiveDate>), String> {
    let summary = match summary {
        Some(s) => s.to_string(),
        None => {
            write!(writer, "Summary: ").map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return Err("Summary cannot be empty".to_string());
            }
            trimmed
        }
    };
    let start = match start {
        Some(s) => s,
        None => {
            write!(writer, "Start date: ").map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            crate::cli::parse_date(line.trim())?
        }
    };
    let end = match end {
        Some(e) => Some(e),
        None if summary.is_empty() => None,
        None => {
            write!(writer, "End date (empty for single day): ")
                .map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(crate::cli::parse_date(trimmed)?)
            }
        }
    };
    Ok((summary, start, end))
}

pub fn add(
    file: &Path,
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
) -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut writer = io::stderr();
    let (summary, start, end) = resolve_add_params(summary, start, end, &mut reader, &mut writer)?;

    let dtend = match end {
        Some(e) if e < start => return Err("--end must not be before --start".to_string()),
        Some(e) if e == start => start + chrono::Days::new(1),
        Some(e) => e + chrono::Days::new(1),
        None => start + chrono::Days::new(1),
    };

    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {}: {e}", file.display()))?;

    let event = VEvent {
        uid: uuid::Uuid::new_v4().to_string(),
        dtstamp: chrono::Utc::now().naive_utc(),
        dtstart: start,
        dtend,
        summary,
    };

    let new_content = ics::insert_event(&content, &event)?;
    std::fs::write(file, new_content.as_bytes()).map_err(|e| format!("Failed to write: {e}"))?;

    let line = ics::format_event_line(&event);
    eprintln!("Added: {line}");
    Ok(())
}

pub fn list(file: &Path) -> Result<String, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {}: {e}", file.display()))?;
    let events = ics::parse_events(&content)?;
    let output = events
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}: {}", i + 1, ics::format_event_line(e)))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(output)
}

pub fn remove(
    file: &Path,
    summary: Option<&str>,
    index: Option<usize>,
) -> Result<(), String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {}: {e}", file.display()))?;

    let (new_content, removed_desc) = match (summary, index) {
        (Some(s), None) => {
            let events = ics::parse_events(&content)?;
            let removed: Vec<_> = events.iter().filter(|e| e.summary == s).collect();
            if removed.is_empty() {
                return Err(format!("No event found with summary: {s}"));
            }
            let desc = removed
                .iter()
                .map(|e| ics::format_event_line(e))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_event_by_summary(&content, s)?, desc)
        }
        (None, Some(idx)) => {
            let events = ics::parse_events(&content)?;
            if idx == 0 || idx > events.len() {
                return Err(format!("Index {idx} out of range (1-{})", events.len()));
            }
            let desc = ics::format_event_line(&events[idx - 1]);
            (ics::remove_event_by_index(&content, idx)?, desc)
        }
        (Some(_), Some(_)) => {
            return Err("Cannot specify both --summary and --index".to_string());
        }
        (None, None) => {
            // Interactive mode
            let events = ics::parse_events(&content)?;
            if events.is_empty() {
                return Err("No events to remove".to_string());
            }
            for (i, e) in events.iter().enumerate() {
                eprintln!("{}: {}", i + 1, ics::format_event_line(e));
            }
            eprint!("Remove # (or 'q' to cancel): ");
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            let trimmed = line.trim();
            if trimmed == "q" || trimmed.is_empty() {
                return Ok(());
            }
            let idx: usize = trimmed
                .parse()
                .map_err(|_| format!("Invalid number: {trimmed}"))?;
            if idx == 0 || idx > events.len() {
                return Err(format!("Index {idx} out of range (1-{})", events.len()));
            }
            let desc = ics::format_event_line(&events[idx - 1]);
            (ics::remove_event_by_index(&content, idx)?, desc)
        }
    };

    std::fs::write(file, new_content.as_bytes()).map_err(|e| format!("Failed to write: {e}"))?;
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
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn add_one_event() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, Some("元日"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None).unwrap();
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
        add(&path, Some("元日"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None).unwrap();
        add(
            &path,
            Some("建国記念の日"),
            Some(NaiveDate::from_ymd_opt(2026, 2, 11).unwrap()),
            None,
        )
        .unwrap();
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
        add(&path, Some("元日"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None).unwrap();
        add(
            &path,
            Some("年末年始"),
            Some(NaiveDate::from_ymd_opt(2026, 12, 29).unwrap()),
            Some(NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()),
        )
        .unwrap();
        let output = list(&path).unwrap();
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
        );
        assert!(result.is_err());
    }

    #[test]
    fn remove_by_summary() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, Some("元日"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None).unwrap();
        add(&path, Some("建国記念の日"), Some(NaiveDate::from_ymd_opt(2026, 2, 11).unwrap()), None).unwrap();
        remove(&path, Some("元日"), None).unwrap();
        let output = list(&path).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }

    #[test]
    fn remove_by_index() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, Some("元日"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None).unwrap();
        add(&path, Some("建国記念の日"), Some(NaiveDate::from_ymd_opt(2026, 2, 11).unwrap()), None).unwrap();
        remove(&path, None, Some(1)).unwrap();
        let output = list(&path).unwrap();
        assert!(!output.contains("元日"));
        assert!(output.contains("建国記念の日"));
    }
}
