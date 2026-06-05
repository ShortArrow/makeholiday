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
#[command(name = "icscli", about = "General-purpose iCalendar (RFC 5545) CLI")]
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
    #[command(long_about = "\
Initialize a new VCALENDAR file at the path given by --file (default: calendar.ics).
Fails if the file already exists.

Example:
  icscli init
  icscli -f holidays.ics init
")]
    Init,
    /// Add an all-day event to the calendar
    #[command(long_about = "\
Append a single-day or multi-day all-day VEVENT to the calendar.
--end is inclusive; omit it for a single-day event.

Examples:
  icscli add --summary 元日 --start 2026-01-01
  icscli add --summary 年末年始 --start 2026-12-29 --end 2027-01-03
  icscli add --summary 出張 --start 2026/5/10 --end 2026/5/12 \\
      --busystatus oof --category 仕事 --icon airplane
")]
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
    #[command(long_about = "\
Print every event in the calendar, one per line, numbered for use with `remove <INDEX>`.
--sort is repeatable for multi-key sort. --json switches to JSON output (useful for scripts).

Examples:
  icscli list
  icscli list --sort start
  icscli list --sort start --sort summary --desc
  icscli list --json
")]
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
    #[command(long_about = "\
Patch a single event identified by its 1-based index (look it up with `icscli list`).
Only the flags you pass are changed; everything else stays. Moving --start without
--end preserves the event's duration. Use --category-clear / --icon-clear to drop
those fields without setting a new value.

Examples:
  icscli edit 1 --summary 元日（新名称）
  icscli edit 2 --start 2027-12-29
  icscli edit 3 --busystatus oof --class private
  icscli edit 4 --category-clear --category 仕事 --category 出張
  icscli edit 5 --icon-clear
")]
    Edit {
        /// 1-based event index to edit (look up via `icscli list`)
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
        /// Replace the icon
        #[arg(long)]
        icon: Option<String>,
        /// Drop the icon. Mutually exclusive with --icon.
        #[arg(long, default_value_t = false, conflicts_with = "icon")]
        icon_clear: bool,
    },
    /// List available preset icon names
    #[command(long_about = "\
Print the names of the bundled preset icons that can be passed to --icon
on `add` or `edit`. The icon name is recorded as the X-ICSCLI-ICON
property on the event and appears in `list` output as `[name]`.

Example:
  icscli icons
")]
    Icons,
    /// Remove an event from the calendar
    #[command(long_about = "\
Delete one or more events from the calendar. Provide a 1-based index expression,
a --summary match, or run without arguments for an interactive picker (requires a TTY
or --interactive).

Examples:
  icscli remove 4
  icscli remove 1,3-5,8
  icscli remove --summary 元日
  icscli --interactive remove
")]
    Remove {
        /// Index specifier: "4", "4,6", "6-10", "1,3-5,8" (interactive if omitted)
        target: Option<String>,
        /// Remove events matching this summary
        #[arg(long)]
        summary: Option<String>,
    },
    /// Extract events into a new ICS file (date range and / or UID list)
    #[command(long_about = "\
Write a subset of events into a new calendar at --out. The input file
(--file / -f) is not modified — `split` is non-destructive extraction;
use `remove` afterwards if you want to prune the input.

Predicates (at least one of --from, --to, --uid is required):
  --from / --to   Inclusive date-range bounds. An event matches when its
                  date span intersects the range. Either bound may be
                  omitted for a half-open range.
  --uid           Match events with this UID. Repeatable; the union of
                  the listed UIDs forms the candidate set.

When multiple predicates are given they AND together (intersection):
only events satisfying every predicate are written. --uid values that
no event matches are silently skipped (the output may be empty).

--out must not already exist (atomic create).

Examples:
  icscli -f all.ics split --from 2026-01-01 --to 2026-03-31 --out q1.ics
  icscli -f all.ics split --to 2025-12-31 --out archive-2025.ics
  icscli -f all.ics split --from 2027-01-01 --out future.ics
  icscli -f all.ics split --uid <UID-A> --uid <UID-B> --out picked.ics
  icscli -f all.ics split --from 2026-04-01 --to 2026-06-30 \\
      --uid <UID-A> --out q2-just-A.ics
")]
    Split {
        /// Inclusive lower bound (YYYY-MM-DD or YYYY/M/D)
        #[arg(long, value_parser = parse_date)]
        from: Option<NaiveDate>,
        /// Inclusive upper bound (YYYY-MM-DD or YYYY/M/D)
        #[arg(long, value_parser = parse_date)]
        to: Option<NaiveDate>,
        /// Event UID to match (repeatable, union)
        #[arg(long)]
        uid: Vec<String>,
        /// Destination ICS file (must not already exist)
        #[arg(long)]
        out: PathBuf,
    },
}
