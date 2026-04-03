use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

use crate::ics::{BusyStatus, EventClass, SortKey};

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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliBusyStatus {
    Free,
    Tentative,
    Busy,
    Oof,
    Working,
}

impl CliBusyStatus {
    pub fn to_busystatus(self) -> BusyStatus {
        match self {
            CliBusyStatus::Free => BusyStatus::Free,
            CliBusyStatus::Tentative => BusyStatus::Tentative,
            CliBusyStatus::Busy => BusyStatus::Busy,
            CliBusyStatus::Oof => BusyStatus::Oof,
            CliBusyStatus::Working => BusyStatus::WorkingElsewhere,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliEventClass {
    Public,
    Private,
    Confidential,
}

impl CliEventClass {
    pub fn to_event_class(self) -> EventClass {
        match self {
            CliEventClass::Public => EventClass::Public,
            CliEventClass::Private => EventClass::Private,
            CliEventClass::Confidential => EventClass::Confidential,
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
        /// Busy status: free, tentative, busy, oof, working (default: free)
        #[arg(long, value_enum, default_value_t = CliBusyStatus::Free)]
        busystatus: CliBusyStatus,
        /// Event class: public, private, confidential
        #[arg(long, value_enum)]
        class: Option<CliEventClass>,
        /// Category (repeatable, e.g. --category 仕事 --category 出張)
        #[arg(long)]
        category: Vec<String>,
        /// Icon name (e.g. --icon airplane)
        #[arg(long)]
        icon: Option<String>,
    },
    /// List all events in the calendar
    List {
        /// Sort by field (repeatable for multi-key sort, e.g. --sort start --sort summary)
        #[arg(long, value_enum)]
        sort: Vec<SortField>,
        /// Sort in descending order
        #[arg(long, default_value_t = false)]
        desc: bool,
        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// List available preset icon names
    Icons,
    /// Remove an event from the calendar
    Remove {
        /// Index specifier: "4", "4,6", "6-10", "1,3-5,8" (interactive if omitted)
        target: Option<String>,
        /// Remove events matching this summary
        #[arg(long)]
        summary: Option<String>,
    },
}
