[ **English** | [日本語](docs/README.jp.md) ]

# makeholiday

A small command-line tool for building and editing iCalendar (`.ics`) files. Designed for managing personal holiday and event calendars from the terminal.

## Features

- `init` — create a new ICS calendar file
- `add` — add an all-day event (single day or multi-day range)
- `list` — list events, optionally sorted, optionally as JSON
- `icons` — show the bundled preset icon names
- `remove` — delete events by index, range, or summary
- Microsoft-style busy status (`FREE` / `TENTATIVE` / `BUSY` / `OOF` / `WORKINGELSEWHERE`)
- Event classification (`PUBLIC` / `PRIVATE` / `CONFIDENTIAL`)
- Categories and an `X-MAKEHOLIDAY-ICON` vendor extension

## Installation

```sh
cargo install --path .
```

Or run locally without installing:

```sh
cargo run -- <subcommand> [options]
```

## Usage

All commands accept a global `--file` / `-f` flag (default: `calendar.ics`).

### Initialize a calendar

```sh
makeholiday init
makeholiday --file holidays.ics init
```

### Add an event

```sh
# Single-day event
makeholiday add --summary "New Year's Day" --start 2026-01-01

# Multi-day range (inclusive)
makeholiday add --summary "Year-end break" --start 2026-12-29 --end 2027-01-03

# With busy status, class, categories and icon
makeholiday add \
    --summary "Business trip" \
    --start 2026-05-10 --end 2026-05-12 \
    --busystatus oof \
    --class private \
    --category work --category travel \
    --icon airplane

# Interactive (prompts for summary, start, end)
makeholiday add
```

Date formats accepted: `YYYY-MM-DD` and `YYYY/M/D`.

### List events

```sh
makeholiday list
makeholiday list --sort start
makeholiday list --sort start --sort summary --desc
makeholiday list --json
```

### Show preset icons

```sh
makeholiday icons
```

### Remove events

```sh
# By 1-based index, range, or mixed list
makeholiday remove 1
makeholiday remove 2,4
makeholiday remove 1,3-5,8

# By summary
makeholiday remove --summary "New Year's Day"

# Interactive (lists events and prompts)
makeholiday remove
```

## File format

- iCalendar (RFC 5545) `VCALENDAR` with `VEVENT` entries
- All-day events (`DTSTART;VALUE=DATE`, `DTEND;VALUE=DATE`)
- `DTEND` is exclusive per RFC 5545; CLI inputs treat `--end` as inclusive and adjust automatically

## Roadmap

See [docs/PRD.md](docs/PRD.md) for product direction (CRUD enhancement, Outlook / Google / iCloud extension support, RFC compliance vs vendor extension boundary, and a reusable ICS handling library).

## Documentation

- [SETUP](docs/SETUP.md) — installation and platform setup
- [USAGE](docs/USAGE.md) — comprehensive command reference
- [PRD](docs/PRD.md) — product requirements
- [ADR policy](docs/design/000-ADR-policy.md) — how architectural decisions are recorded
- [CONTRIBUTING](docs/CONTRIBUTING.md) — development guidelines
- [Japanese README](docs/README.jp.md)

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
