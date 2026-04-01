use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

use crate::ics::SortKey;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortField {
    Start,
    End,
    Summary,
}

impl SortField {
    pub fn to_sort_key(self) -> SortKey {
        match self {
            SortField::Start => SortKey::Start,
            SortField::End => SortKey::End,
            SortField::Summary => SortKey::Summary,
        }
    }
}

const DEFAULT_FILE: &str = "calendar.ics";

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d);
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                return Ok(date);
            }
        }
    }
    Err(format!("invalid date '{s}' (expected YYYY-MM-DD or YYYY/M/D)"))
}

#[derive(Parser)]
#[command(name = "makeholiday", about = "ICS calendar file manager")]
pub struct Cli {
    /// Path to the ICS file (default: calendar.ics)
    #[arg(long, short, global = true, default_value = DEFAULT_FILE)]
    pub file: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new ICS calendar file
    Init,
    /// Add an all-day event to the calendar
    Add {
        /// Event summary/title (interactive if omitted)
        #[arg(long)]
        summary: Option<String>,
        /// Start date (YYYY-MM-DD or YYYY/M/D, interactive if omitted)
        #[arg(long, value_parser = parse_date)]
        start: Option<NaiveDate>,
        /// End date (YYYY-MM-DD or YYYY/M/D, inclusive). Omit for single-day event
        #[arg(long, value_parser = parse_date)]
        end: Option<NaiveDate>,
    },
    /// List all events in the calendar
    List {
        /// Sort by field (repeatable for multi-key sort, e.g. --sort start --sort summary)
        #[arg(long, value_enum)]
        sort: Vec<SortField>,
        /// Sort in descending order
        #[arg(long, default_value_t = false)]
        desc: bool,
    },
    /// Remove an event from the calendar
    Remove {
        /// Index specifier: "4", "4,6", "6-10", "1,3-5,8" (interactive if omitted)
        target: Option<String>,
        /// Remove events matching this summary
        #[arg(long)]
        summary: Option<String>,
    },
}
