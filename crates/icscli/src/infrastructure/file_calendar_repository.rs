//! `FileCalendarRepository` — disk-backed `CalendarRepository` impl.
//!
//! Writes go through `tempfile::NamedTempFile` colocated in the target's
//! parent directory, then `persist` / `persist_noclobber` for an atomic
//! rename per ADR-011. Half-written calendar files from process aborts
//! are no longer possible.

use std::io::Write;
use std::path::{Path, PathBuf};

use ics_core::{self as ics, VCalendar};
use tempfile::NamedTempFile;

use crate::application::ports::CalendarRepository;
use crate::error::{IcsError, Result};

pub struct FileCalendarRepository {
    path: PathBuf,
}

impl FileCalendarRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write `content` atomically to `self.path`.
    /// When `no_clobber` is true, fail with `AlreadyExists` if the target
    /// already exists.
    fn atomic_write(&self, content: &str, no_clobber: bool) -> Result<()> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let mut tmp = NamedTempFile::new_in(dir).map_err(|e| IcsError::io(&self.path, e))?;
        tmp.write_all(content.as_bytes())
            .map_err(|e| IcsError::io(&self.path, e))?;
        tmp.as_file()
            .sync_all()
            .map_err(|e| IcsError::io(&self.path, e))?;

        if no_clobber {
            tmp.persist_noclobber(&self.path).map_err(|e| {
                if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                    IcsError::already_exists(self.path.clone())
                } else {
                    IcsError::io(&self.path, e.error)
                }
            })?;
        } else {
            tmp.persist(&self.path)
                .map_err(|e| IcsError::io(&self.path, e.error))?;
        }
        Ok(())
    }
}

impl CalendarRepository for FileCalendarRepository {
    fn create(&self) -> Result<()> {
        if self.path.exists() {
            return Err(IcsError::already_exists(self.path.clone()));
        }
        let content = ics::format_calendar(&VCalendar::new("-//icscli//EN"));
        self.atomic_write(&content, true)
    }

    fn create_with(&self, calendar: &VCalendar) -> Result<()> {
        if self.path.exists() {
            return Err(IcsError::already_exists(self.path.clone()));
        }
        let content = ics::format_calendar(calendar);
        self.atomic_write(&content, true)
    }

    fn load(&self) -> Result<VCalendar> {
        let content =
            std::fs::read_to_string(&self.path).map_err(|e| IcsError::io(&self.path, e))?;
        let cal = ics::parse_calendar(&content)?;
        Ok(cal)
    }

    fn save(&self, calendar: &VCalendar) -> Result<()> {
        let content = ics::format_calendar(calendar);
        self.atomic_write(&content, false)
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }
}
