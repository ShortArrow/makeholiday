//! Use cases — free functions orchestrating I/O via `CalendarRepository`
//! and pure domain logic via `ics_core`.

use std::io::{self, BufRead, Write};

use chrono::NaiveDate;
use ics_core::{self as ics, VEvent};

use crate::application::ports::CalendarRepository;
use crate::display::format_event_line;
use crate::error::{IcsError, Result};
use crate::icons;

/// Runtime context for status output and interactive-prompt policy.
/// Per ADR-015.
#[derive(Debug, Clone, Copy)]
pub struct RunContext {
    /// Suppress status / warning output.
    pub quiet: bool,
    /// Whether interactive prompts are allowed. Resolved at the
    /// composition root from the explicit override flags + TTY detection.
    pub allow_prompts: bool,
}

impl RunContext {
    pub fn status(&self, msg: &str) {
        if !self.quiet {
            eprintln!("{msg}");
        }
    }
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            quiet: false,
            allow_prompts: true,
        }
    }
}

pub fn init<R: CalendarRepository>(repo: &R) -> Result<()> {
    repo.create()
}

fn resolve_add_params(
    summary: Option<&str>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    allow_prompts: bool,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<(String, NaiveDate, Option<NaiveDate>)> {
    let interactive = summary.is_none() || start.is_none();
    if interactive && !allow_prompts {
        return Err(IcsError::InvalidInput(
            "missing required arguments: pass --summary and --start (no TTY for interactive prompts)"
                .to_string(),
        ));
    }
    let summary = match summary {
        Some(s) => s.to_string(),
        None => {
            write!(writer, "Summary: ").map_err(|e| IcsError::io("<stdin>", e))?;
            writer.flush().map_err(|e| IcsError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| IcsError::io("<stdin>", e))?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return Err(IcsError::InvalidInput(
                    "Summary cannot be empty".to_string(),
                ));
            }
            trimmed
        }
    };
    let start = match start {
        Some(s) => s,
        None => {
            write!(writer, "Start date: ").map_err(|e| IcsError::io("<stdin>", e))?;
            writer.flush().map_err(|e| IcsError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| IcsError::io("<stdin>", e))?;
            crate::input::parse_date(line.trim()).map_err(IcsError::InvalidInput)?
        }
    };
    let end = match end {
        Some(e) => Some(e),
        None if !interactive => None,
        None => {
            write!(writer, "End date (empty for single day): ")
                .map_err(|e| IcsError::io("<stdin>", e))?;
            writer.flush().map_err(|e| IcsError::io("<stdin>", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| IcsError::io("<stdin>", e))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(crate::input::parse_date(trimmed).map_err(IcsError::InvalidInput)?)
            }
        }
    };
    Ok((summary, start, end))
}

#[allow(clippy::too_many_arguments)] // ADR-001 Migration will replace flat args with a request struct
pub fn add<R: CalendarRepository>(
    repo: &R,
    ctx: RunContext,
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
    let (summary, start, end) = resolve_add_params(
        summary,
        start,
        end,
        ctx.allow_prompts,
        &mut reader,
        &mut writer,
    )?;

    let dtend = match end {
        Some(e) if e < start => {
            return Err(IcsError::InvalidInput(
                "--end must not be before --start".to_string(),
            ));
        }
        Some(e) if e == start => start + chrono::Days::new(1),
        Some(e) => e + chrono::Days::new(1),
        None => start + chrono::Days::new(1),
    };

    let mut cal = repo.load()?;

    let mut event = VEvent {
        uid: uuid::Uuid::new_v4().to_string(),
        dtstamp: chrono::Utc::now().naive_utc(),
        dtstart: start,
        dtend,
        summary,
        transp: None,
        class,
        categories,
        microsoft: Some(ics::microsoft::EventExtensions {
            busystatus: Some(busystatus),
            unrecognized: vec![],
        }),
        google: None,
        icloud: None,
        unknown: vec![],
        unrecognized_components: vec![],
    };
    if let Some(icon_name) = icon {
        icons::write_icon(&mut event, icon_name);
    }

    cal.events.push(event.clone());
    repo.save(&cal)?;

    let line = format_event_line(&event);
    ctx.status(&format!("Added: {line}"));
    Ok(())
}

/// Patch describing which fields of an existing `VEvent` should be
/// replaced by `edit`. `Some(_)` means "set this field"; `None` means
/// "leave it alone". Toggles (`clear_categories`, `clear_icon`) are
/// independent of the same-name fields and trigger removal even when
/// no replacement is provided.
#[derive(Debug, Default)]
pub struct EditPatch {
    pub summary: Option<String>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub busystatus: Option<ics::microsoft::MsBusyStatus>,
    pub class: Option<ics::EventClass>,
    pub categories: Option<Vec<String>>,
    pub clear_categories: bool,
    pub icon: Option<String>,
    pub clear_icon: bool,
}

pub fn edit<R: CalendarRepository>(
    repo: &R,
    ctx: RunContext,
    index: usize,
    patch: EditPatch,
) -> Result<()> {
    let mut cal = repo.load()?;
    if index == 0 || index > cal.events.len() {
        return Err(IcsError::NotFound(format!(
            "Index {index} out of range (1-{})",
            cal.events.len()
        )));
    }
    let event = &mut cal.events[index - 1];

    if let Some(s) = patch.summary {
        event.summary = s;
    }
    match (patch.start, patch.end) {
        (Some(new_start), Some(new_end_incl)) => {
            if new_end_incl < new_start {
                return Err(IcsError::InvalidInput(
                    "--end must not be before --start".to_string(),
                ));
            }
            event.dtstart = new_start;
            event.dtend = new_end_incl + chrono::Days::new(1);
        }
        (Some(new_start), None) => {
            // Move the event preserving its current duration.
            let duration = event.dtend - event.dtstart;
            event.dtstart = new_start;
            event.dtend = new_start + duration;
        }
        (None, Some(new_end_incl)) => {
            if new_end_incl < event.dtstart {
                return Err(IcsError::InvalidInput(
                    "--end must not be before --start".to_string(),
                ));
            }
            event.dtend = new_end_incl + chrono::Days::new(1);
        }
        (None, None) => {}
    }

    if let Some(bs) = patch.busystatus {
        let ms = event
            .microsoft
            .get_or_insert_with(ics::microsoft::EventExtensions::default);
        ms.busystatus = Some(bs);
    }
    if let Some(c) = patch.class {
        event.class = Some(c);
    }
    if patch.clear_categories {
        event.categories.clear();
    }
    if let Some(cats) = patch.categories {
        if !cats.is_empty() {
            event.categories = cats;
        }
    }
    if patch.clear_icon {
        event.unknown.retain(|p| p.name != icons::ICON_PROPERTY);
    }
    if let Some(icon_name) = patch.icon {
        icons::write_icon(event, icon_name);
    }

    let line = format_event_line(event);
    repo.save(&cal)?;
    ctx.status(&format!("Edited: {line}"));
    Ok(())
}

pub fn list<R: CalendarRepository>(
    repo: &R,
    sort_keys: &[ics::SortKey],
    descending: bool,
    json: bool,
) -> Result<String> {
    let cal = repo.load()?;
    let events = if sort_keys.is_empty() {
        cal.events
    } else {
        ics::sort_events(&cal.events, sort_keys, descending)
    };
    if json {
        serde_json::to_string_pretty(&events)
            .map_err(|e| IcsError::InvalidInput(format!("JSON error: {e}")))
    } else {
        let output = events
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}: {}", i + 1, format_event_line(e)))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(output)
    }
}

pub fn remove<R: CalendarRepository>(
    repo: &R,
    ctx: RunContext,
    summary: Option<&str>,
    target: Option<&str>,
) -> Result<()> {
    let cal = repo.load()?;
    let events = &cal.events;

    let (new_cal, removed_desc) = match (summary, target) {
        (Some(_), Some(_)) => {
            return Err(IcsError::Conflict(
                "Cannot specify both --summary and index target".to_string(),
            ));
        }
        (Some(s), None) => {
            let removed: Vec<_> = events.iter().filter(|e| e.summary == s).collect();
            if removed.is_empty() {
                return Err(IcsError::NotFound(format!(
                    "No event found with summary: {s}"
                )));
            }
            let desc = removed
                .iter()
                .map(|e| format_event_line(e))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_event_by_summary(&cal, s)?, desc)
        }
        (None, Some(spec)) => {
            let indices = ics::parse_indices(spec, events.len())?;
            let desc = indices
                .iter()
                .map(|&i| format_event_line(&events[i - 1]))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_events_by_indices(&cal, &indices)?, desc)
        }
        (None, None) => {
            if !ctx.allow_prompts {
                return Err(IcsError::InvalidInput(
                    "missing required arguments: pass <INDEX> or --summary (no TTY for interactive prompts)"
                        .to_string(),
                ));
            }
            if events.is_empty() {
                return Err(IcsError::NotFound("No events to remove".to_string()));
            }
            for (i, e) in events.iter().enumerate() {
                eprintln!("{}: {}", i + 1, format_event_line(e));
            }
            eprint!("Remove # (or 'q' to cancel): ");
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .map_err(|e| IcsError::io("<stdin>", e))?;
            let trimmed = line.trim();
            if trimmed == "q" || trimmed.is_empty() {
                return Ok(());
            }
            let indices = ics::parse_indices(trimmed, events.len())?;
            let desc = indices
                .iter()
                .map(|&i| format_event_line(&events[i - 1]))
                .collect::<Vec<_>>()
                .join(", ");
            (ics::remove_events_by_indices(&cal, &indices)?, desc)
        }
    };

    repo.save(&new_cal)?;
    ctx.status(&format!("Removed: {removed_desc}"));
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
        let cal = repo.load().unwrap();
        assert_eq!(cal.prodid, "-//icscli//EN");
        assert_eq!(cal.version, "2.0");
        assert!(cal.events.is_empty());
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        let result = init(&repo);
        assert!(matches!(result, Err(IcsError::AlreadyExists { .. })));
    }

    fn add_free(repo: &FileCalendarRepository, summary: &str, start: NaiveDate) {
        add(
            repo,
            RunContext::default(),
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
            RunContext::default(),
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
        let cal = repo.load().unwrap();
        assert_eq!(cal.events.len(), 1);
        assert_eq!(cal.events[0].summary, "元日");
        assert!(!cal.events[0].uid.is_empty());
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
        let cal = repo.load().unwrap();
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
            RunContext::default(),
            Some("invalid"),
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            None,
        );
        assert!(matches!(result, Err(IcsError::InvalidInput(_))));
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
        remove(&repo, RunContext::default(), Some("元日"), None).unwrap();
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
        remove(&repo, RunContext::default(), None, Some("1")).unwrap();
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
            RunContext::default(),
            Some("不在"),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            None,
            ics::microsoft::MsBusyStatus::Oof,
            Some(ics::EventClass::Private),
            vec![],
            None,
        )
        .unwrap();
        let cal = repo.load().unwrap();
        let event = &cal.events[0];
        assert_eq!(
            event.microsoft.as_ref().and_then(|m| m.busystatus),
            Some(ics::microsoft::MsBusyStatus::Oof)
        );
        assert_eq!(event.class, Some(ics::EventClass::Private));
    }

    // ADR-019 ship-blocker #4: `edit` subcommand.

    #[test]
    fn edit_replaces_summary() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

        let patch = EditPatch {
            summary: Some("New Year".to_string()),
            ..EditPatch::default()
        };
        edit(&repo, RunContext::default(), 1, patch).unwrap();

        let cal = repo.load().unwrap();
        assert_eq!(cal.events[0].summary, "New Year");
    }

    #[test]
    fn edit_replaces_start_date() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

        let patch = EditPatch {
            start: Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            ..EditPatch::default()
        };
        edit(&repo, RunContext::default(), 1, patch).unwrap();

        let cal = repo.load().unwrap();
        assert_eq!(
            cal.events[0].dtstart,
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()
        );
    }

    #[test]
    fn edit_replaces_busystatus() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(
            &repo,
            "Travel",
            NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        );

        let patch = EditPatch {
            busystatus: Some(ics::microsoft::MsBusyStatus::Oof),
            ..EditPatch::default()
        };
        edit(&repo, RunContext::default(), 1, patch).unwrap();

        let cal = repo.load().unwrap();
        assert_eq!(
            cal.events[0].microsoft.as_ref().and_then(|m| m.busystatus),
            Some(ics::microsoft::MsBusyStatus::Oof)
        );
    }

    #[test]
    fn edit_clears_icon() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        // Add an event with an icon via the add() use case.
        add(
            &repo,
            RunContext::default(),
            Some("Travel"),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            None,
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec![],
            Some("airplane".to_string()),
        )
        .unwrap();
        assert_eq!(
            icons::read_icon(&repo.load().unwrap().events[0]),
            Some("airplane")
        );

        let patch = EditPatch {
            clear_icon: true,
            ..EditPatch::default()
        };
        edit(&repo, RunContext::default(), 1, patch).unwrap();

        let cal = repo.load().unwrap();
        assert_eq!(icons::read_icon(&cal.events[0]), None);
    }

    #[test]
    fn edit_replaces_categories() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add(
            &repo,
            RunContext::default(),
            Some("Mtg"),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            None,
            ics::microsoft::MsBusyStatus::Free,
            None,
            vec!["old".to_string()],
            None,
        )
        .unwrap();

        let patch = EditPatch {
            categories: Some(vec!["work".to_string(), "important".to_string()]),
            clear_categories: true,
            ..EditPatch::default()
        };
        edit(&repo, RunContext::default(), 1, patch).unwrap();

        let cal = repo.load().unwrap();
        assert_eq!(cal.events[0].categories, vec!["work", "important"]);
    }

    #[test]
    fn edit_out_of_range_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "元日", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

        let patch = EditPatch {
            summary: Some("ignored".to_string()),
            ..EditPatch::default()
        };
        let result = edit(&repo, RunContext::default(), 99, patch);
        assert!(matches!(result, Err(IcsError::NotFound(_))));
    }

    #[test]
    fn edit_end_before_start_errors() {
        let dir = TempDir::new().unwrap();
        let repo = temp_repo(&dir, "test.ics");
        init(&repo).unwrap();
        add_free(&repo, "Trip", NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());

        let patch = EditPatch {
            start: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
            end: Some(NaiveDate::from_ymd_opt(2026, 6, 5).unwrap()),
            ..EditPatch::default()
        };
        let result = edit(&repo, RunContext::default(), 1, patch);
        assert!(matches!(result, Err(IcsError::InvalidInput(_))));
    }
}
