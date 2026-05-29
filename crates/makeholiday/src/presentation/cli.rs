use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

use ics_core::microsoft::MsBusyStatus;
use ics_core::{EventClass, SortKey};

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
    pub fn to_busystatus(self) -> MsBusyStatus {
        match self {
            CliBusyStatus::Free => MsBusyStatus::Free,
            CliBusyStatus::Tentative => MsBusyStatus::Tentative,
            CliBusyStatus::Busy => MsBusyStatus::Busy,
            CliBusyStatus::Oof => MsBusyStatus::Oof,
            CliBusyStatus::Working => MsBusyStatus::WorkingElsewhere,
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

use crate::input::parse_date;

const DEFAULT_FILE: &str = "calendar.ics";

#[derive(Parser)]
#[command(name = "makeholiday", about = "ICS calendar file manager")]
pub struct Cli {
    /// Path to the ICS file (default: calendar.ics)
    #[arg(long, short, global = true, default_value = DEFAULT_FILE)]
    pub file: PathBuf,

    /// Suppress status / warning messages
    #[arg(long, short, global = true, default_value_t = false)]
    pub quiet: bool,

    /// Force interactive prompts even when stdin is not a TTY
    #[arg(long, global = true, conflicts_with = "no_interactive")]
    pub interactive: bool,

    /// Disable interactive prompts; missing required arguments error out
    #[arg(long, global = true)]
    pub no_interactive: bool,

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
    /// Edit an existing event in place by 1-based index
    Edit {
        /// 1-based event index to edit (look up via `makeholiday list`)
        index: usize,
        /// Replace the event title
        #[arg(long)]
        summary: Option<String>,
        /// Replace the start date (inclusive)
        #[arg(long, value_parser = parse_date)]
        start: Option<NaiveDate>,
        /// Replace the end date (inclusive); pass an empty string to drop a
        /// multi-day end and convert to a single-day event
        #[arg(long, value_parser = parse_date)]
        end: Option<NaiveDate>,
        /// Replace the busy status: free, tentative, busy, oof, working
        #[arg(long, value_enum)]
        busystatus: Option<CliBusyStatus>,
        /// Replace the event class: public, private, confidential
        #[arg(long, value_enum)]
        class: Option<CliEventClass>,
        /// Replace the category list (repeatable). Pass --category-clear to
        /// drop categories entirely.
        #[arg(long)]
        category: Vec<String>,
        /// Drop all categories before applying --category. Pass --category-clear
        /// with no --category to leave the event with no categories.
        #[arg(long, default_value_t = false)]
        category_clear: bool,
        /// Replace the makeholiday icon
        #[arg(long)]
        icon: Option<String>,
        /// Drop the icon. Mutually exclusive with --icon.
        #[arg(long, default_value_t = false, conflicts_with = "icon")]
        icon_clear: bool,
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
