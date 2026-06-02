[ **English** | [日本語](USAGE.jp.md) ]

# Usage Reference

Comprehensive command reference for `icscli`. For installation see [SETUP.md](SETUP.md). For the short overview see the [README](../README.md).

## Conventions

- All examples assume `icscli` is on your `PATH`. If running from a checkout, substitute `cargo run --` (e.g., `cargo run -- list`).
- Commands write to **stdout** for successful, user-readable output. Diagnostics (`Added: ...`, `Removed: ...`) and prompts go to **stderr**.
- Exit code is `0` on success, `1` on any user-facing error.

## Global Options

| Option | Default | Description |
|---|---|---|
| `--file <PATH>`, `-f <PATH>` | `calendar.ics` | Path to the ICS file all subcommands operate on. |
| `--help`, `-h` | | Print help. Available on every subcommand. |
| `--version`, `-V` | | Print version. |

## Date Input Formats

`--start` and `--end` accept:

- `YYYY-MM-DD` (e.g., `2026-01-01`)
- `YYYY/M/D` (e.g., `2026/1/1`, single-digit month / day allowed)

Invalid dates are rejected with `invalid date '<input>' (expected YYYY-MM-DD or YYYY/M/D)`.

## Subcommands

### `init`

Create a new ICS calendar file.

```sh
icscli init
icscli --file holidays.ics init
```

- Creates a `VCALENDAR` with `VERSION:2.0` and `PRODID:-//icscli//EN`.
- Fails if the target file already exists. To re-initialize, remove the file first.

### `add`

Append a `VEVENT` to the calendar.

```sh
icscli add [--summary <TEXT>] [--start <DATE>] [--end <DATE>]
                [--busystatus <STATUS>] [--class <CLASS>]
                [--category <NAME> ...] [--icon <NAME>]
```

| Flag | Type | Notes |
|---|---|---|
| `--summary <TEXT>` | string | Event title. Required (interactively prompted if omitted). |
| `--start <DATE>` | date | Start date. Required (prompted if omitted). |
| `--end <DATE>` | date | End date (inclusive). Omit for a single-day event. |
| `--busystatus <STATUS>` | `free` \| `tentative` \| `busy` \| `oof` \| `working` | Default: `free`. Emits both `TRANSP` and `X-MICROSOFT-CDO-BUSYSTATUS`. |
| `--class <CLASS>` | `public` \| `private` \| `confidential` | Optional. Emits `CLASS:`. |
| `--category <NAME>` | string, repeatable | Multiple values join into a single `CATEGORIES:` line, comma-separated. |
| `--icon <NAME>` | string | Emits `X-ICSCLI-ICON:<NAME>`. See [`icons`](#icons) for preset names; arbitrary values also work. |

#### Examples

```sh
# Single-day event with defaults
icscli add --summary "New Year's Day" --start 2026-01-01

# Multi-day range, OOF, private, with categories and icon
icscli add \
    --summary "Business trip" \
    --start 2026-05-10 --end 2026-05-12 \
    --busystatus oof --class private \
    --category work --category travel \
    --icon airplane

# Interactive: prompts on stderr for Summary, Start date, End date
icscli add
```

#### Behavior

- `--end` is inclusive on the CLI. Internally `DTEND` is set to `--end + 1 day` to comply with RFC 5545's exclusive end semantics for `VALUE=DATE`.
- If `--end < --start`, the command fails with `--end must not be before --start`.
- If `--start == --end`, the event spans a single day.
- A fresh UUIDv4 is generated for `UID`. `DTSTAMP` is the current UTC time.
- The new `VEVENT` is inserted immediately before `END:VCALENDAR`, preserving any existing events.

### `list`

Enumerate events from the calendar.

```sh
icscli list [--sort <FIELD> ...] [--desc] [--json]
```

| Flag | Notes |
|---|---|
| `--sort <FIELD>` | Repeatable. `start` \| `end` \| `summary`. Multiple keys define a stable multi-key sort. |
| `--desc` | Reverse the sort. |
| `--json` | Output a JSON array instead of human-readable lines. |

#### Output formats

Human-readable (default):

```
1: 2026-01-01 : New Year's Day
2: 2026-12-29 to 2027-01-03 : Year-end break
```

`<index>: <start>[ to <end>] : <summary>`. The trailing date is shown only for multi-day events. Indices are 1-based and used by [`remove`](#remove).

JSON (`--json`):

```json
[
  {
    "uid": "…",
    "dtstamp": "2026-05-27T00:00:00Z",
    "dtstart": "2026-01-01",
    "dtend": "2026-01-02",
    "summary": "New Year's Day",
    "busystatus": "free"
  }
]
```

`dtend` is the exclusive RFC value (one day after the inclusive end). Optional fields (`class`, `categories`, `icon`) appear only when present.

### `icons`

Print the bundled preset icon names.

```sh
icscli icons
```

The output lists each icon name followed by its Japanese description, e.g. `airplane    出張・旅行`. These are convenience presets; `add --icon` also accepts arbitrary strings.

### `remove`

Delete events from the calendar.

```sh
icscli remove [<INDEX_SPEC>] [--summary <TEXT>]
```

| Argument / Flag | Notes |
|---|---|
| `<INDEX_SPEC>` (positional) | 1-based indices into `list` output. Forms: single (`4`), list (`2,4`), range (`6-10`), mixed (`1,3-5,8`). |
| `--summary <TEXT>` | Remove every event whose summary exactly matches. |

#### Examples

```sh
# By index
icscli remove 1
icscli remove 2,4
icscli remove 1,3-5,8

# By summary (all matching events)
icscli remove --summary "New Year's Day"

# Interactive: lists events and prompts for an index spec on stderr
icscli remove
```

#### Errors

- Specifying both `<INDEX_SPEC>` and `--summary` fails immediately.
- Indices out of range (`< 1` or `> N`) fail with `Index out of range (1-N)`.
- `--summary` with no match fails with `No event found with summary: <text>`.

## File Format

`icscli` reads and writes RFC 5545 iCalendar files with the following conventions:

- Line endings: `CRLF` (`\r\n`) on output, both accepted on input.
- Wrapping: long property lines are not folded (input that uses RFC 5545 line folding is currently *not* unfolded; expanded handling tracked in [PRD §5.2](PRD.md#52-planned)).
- All `VEVENT` entries are all-day: `DTSTART;VALUE=DATE`, `DTEND;VALUE=DATE`.
- Properties emitted, in order: `UID`, `DTSTAMP`, `DTSTART`, `DTEND`, `SUMMARY`, `TRANSP`, `X-MICROSOFT-CDO-BUSYSTATUS`, then optional `CLASS`, `CATEGORIES`, `X-ICSCLI-ICON`.

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | Any user-facing error: invalid arguments, file I/O failure, parse error, no matching event, index out of range. |

## See Also

- [README](../README.md) — high-level overview.
- [SETUP](SETUP.md) — installation and platform setup.
- [PRD](PRD.md) — planned commands and longer-term direction.
- [CONTRIBUTING](CONTRIBUTING.md) — development workflow.
