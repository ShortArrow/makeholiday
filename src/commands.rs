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

pub fn add(
    file: &Path,
    summary: &str,
    start: NaiveDate,
    end: Option<NaiveDate>,
) -> Result<(), String> {
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
        dtstart: start,
        dtend,
        summary: summary.to_string(),
    };

    let new_content = ics::insert_event(&content, &event)?;
    std::fs::write(file, new_content.as_bytes()).map_err(|e| format!("Failed to write: {e}"))
}

pub fn list(file: &Path) -> Result<String, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {}: {e}", file.display()))?;
    let events = ics::parse_events(&content)?;
    let output = events.iter().map(ics::format_event_line).collect::<Vec<_>>().join("\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_file(dir: &TempDir, name: &str) -> std::path::PathBuf {
        dir.path().join(name)
    }

    // Step 11: init creates valid ICS
    #[test]
    fn init_creates_valid_ics() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, ics::format_calendar(&[]));
    }

    // Step 12: init fails on existing file
    #[test]
    fn init_fails_if_exists() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let result = init(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    // Step 13: add one event
    #[test]
    fn add_one_event() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), None).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("SUMMARY:元日"));
        assert!(content.contains("BEGIN:VEVENT"));
    }

    // Step 14: add two events with distinct UIDs
    #[test]
    fn add_two_events_distinct_uids() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), None).unwrap();
        add(
            &path,
            "建国記念の日",
            NaiveDate::from_ymd_opt(2026, 2, 11).unwrap(),
            None,
        )
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let events = ics::parse_events(&content).unwrap();
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].uid, events[1].uid);
    }

    // Step 15: list returns formatted lines
    #[test]
    fn list_returns_formatted_lines() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        add(&path, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), None).unwrap();
        add(
            &path,
            "年末年始",
            NaiveDate::from_ymd_opt(2026, 12, 29).unwrap(),
            Some(NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()),
        )
        .unwrap();
        let output = list(&path).unwrap();
        assert!(output.contains("2026-01-01 : 元日"));
        assert!(output.contains("2026-12-29 to 2027-01-03 : 年末年始"));
    }

    // Validation: end before start
    #[test]
    fn add_end_before_start_errors() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "test.ics");
        init(&path).unwrap();
        let result = add(
            &path,
            "invalid",
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
        );
        assert!(result.is_err());
    }
}
