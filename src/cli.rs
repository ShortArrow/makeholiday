use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand};

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
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new ICS calendar file
    Init {
        /// Path to the ICS file (default: calendar.ics)
        #[arg(default_value = DEFAULT_FILE)]
        file: PathBuf,
    },
    /// Add an all-day event to the calendar
    Add {
        /// Path to the ICS file (default: calendar.ics)
        #[arg(default_value = DEFAULT_FILE)]
        file: PathBuf,
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
        /// Path to the ICS file (default: calendar.ics)
        #[arg(default_value = DEFAULT_FILE)]
        file: PathBuf,
    },
    /// Remove an event from the calendar
    Remove {
        /// Path to the ICS file (default: calendar.ics)
        #[arg(default_value = DEFAULT_FILE)]
        file: PathBuf,
        /// Remove events matching this summary
        #[arg(long)]
        summary: Option<String>,
        /// Remove event at this index (1-based, from list output)
        #[arg(long)]
        index: Option<usize>,
    },
}
