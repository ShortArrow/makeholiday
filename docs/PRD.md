[ **English** | [日本語](PRD.jp.md) ]

# Product Requirements Document — makeholiday

> Status: **Draft**. Sections 1–4 and 5.1 are settled; later sections still evolving.

## 1. Background

iCalendar (RFC 5545) is the de-facto interchange format for calendar data, but the ecosystem around it is fragmented. Existing ICS tooling tends to fall into one of two failure modes:

- **Strict RFC-only tools** that drop or refuse vendor-specific extensions (Outlook's `X-MICROSOFT-CDO-*`, Google's `X-GOOGLE-*`, iCloud's variants), causing silent data loss on round-trip.
- **Vendor-bound tools** that handle one vendor's dialect well but cannot describe the others as first-class data.

In practice, users who simply want to assemble or edit `.ics` files — for personal holidays, team calendars, or interoperability glue between calendar services — end up writing one-off scripts each time, or accepting lossy conversions.

`makeholiday` exists to close that gap with a small, deliberate tool: a CLI for everyday ICS authoring on top of a typed core that treats vendor extensions as first-class, not as opaque strings.

## 2. Goals

In priority order:

1. **CLI UX (highest priority).** The CLI must be pleasant to use for everyday calendar authoring — discoverable subcommands, sensible defaults, both scriptable (full flags) and interactive modes. UX considerations override architectural neatness when they conflict.
2. **Round-trip losslessness.** Reading and re-emitting an ICS file preserves order, whitespace where semantically meaningful, and *all* properties — including unknown and vendor-specific ones. A file passed through `makeholiday` is recognizable to its origin tool. See [ADR-001](design/001-vendor-extension-typing.md) for the typing-level commitments; ordering semantics are deferred to a future round-trip strategy ADR.
3. **Typed handling of vendor extensions.** Outlook / Google / iCloud extensions are modeled as distinct, type-safe values, not as raw `X-*` strings. The boundary between RFC 5545 and each vendor profile is explicit in the code and documented. See [ADR-001](design/001-vendor-extension-typing.md) for the model.
4. **Library reusability.** The ICS handling core is consumable as an independent crate, so other tools can depend on it without pulling in CLI machinery.

## 3. Non-Goals

- **Server / service synchronization.** No CalDAV server, no direct integration with Google Calendar API, iCloud Calendar, or Outlook Online. `makeholiday` operates on local `.ics` files.
- **GUI / WebUI.** No desktop application, no web interface. (A TUI is *not* ruled out — see §8.)
- **Non-ICS calendar formats.** Microsoft `.msg`, legacy vCalendar 1.0, proprietary binary calendar formats are out of scope.

## 4. Target Users

Both groups are supported; the CLI persona drives prioritization.

- **Primary — CLI-comfortable individuals** managing personal holiday, vacation, or event calendars from the terminal. They value scriptability, plain-text storage, and minimal ceremony.
- **Secondary — calendar integrators** building tools that generate or consume ICS. They need the library surface and the typed vendor-extension model.

When the two personas conflict, the CLI persona wins.

## 5. Functional Requirements

### 5.1 Currently shipped (v0.1.0)

Implemented and covered by tests in `tests/cli.rs` and unit tests in `src/`:

- **`init`** — create a new `VCALENDAR` file (`PRODID:-//makeholiday//EN`, `VERSION:2.0`).
- **`add`** — append a `VEVENT` (all-day, single or multi-day). Supports:
  - `--summary`, `--start`, `--end` (inclusive on input, converted to RFC-exclusive `DTEND` internally)
  - Date input formats: `YYYY-MM-DD` and `YYYY/M/D`
  - `--busystatus` (`free` / `tentative` / `busy` / `oof` / `working`) emitting `TRANSP` + `X-MICROSOFT-CDO-BUSYSTATUS`
  - `--class` (`public` / `private` / `confidential`)
  - `--category` (repeatable)
  - `--icon` (vendor extension `X-MAKEHOLIDAY-ICON`)
  - Interactive mode when `--summary` / `--start` are omitted
- **`list`** — enumerate events. `--sort` (repeatable: `start` / `end` / `summary`), `--desc`, `--json`.
- **`icons`** — print bundled preset icon names.
- **`remove`** — delete events by 1-based index (`N`, `N-M`, `N,M`, mixed), or `--summary` match, or interactive selection.

### 5.2 Planned

Items are listed in approximate priority. Acceptance criteria to be expanded as work begins.

- **`edit` subcommand** — modify an existing event in place by index. Required to round out CRUD.
- **`search` / `filter` subcommand** — query events by date range, summary substring, category, or busy status.
- **`import` / `export` subcommand variants** — bulk ingestion from other ICS files, optionally with vendor-profile normalization.
- **`convert` subcommand (candidate)** — translate between vendor profiles (e.g., Outlook-flavored ICS → Google-flavored ICS) with explicit loss reporting. Scope to be confirmed.
- **Vendor extension support — Outlook profile.** First-class types for `X-MICROSOFT-CDO-*` family, reminders, categories color, etc. Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **Vendor extension support — Google profile.** First-class types for `X-GOOGLE-*` and Google-specific value handling. Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **Vendor extension support — iCloud profile.** First-class types for Apple-specific extensions (`X-APPLE-*`, `X-CALENDARSERVER-*`). Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **RFC ↔ vendor extension boundary documentation.** A reference document, generated where possible from code, listing which properties live in RFC 5545 and which belong to which vendor profile. Boundary rules captured in [ADR-001](design/001-vendor-extension-typing.md).
- **Reusable ICS handling library (crate split).** Extract `src/ics.rs` (and the typed extension model) into a separately publishable crate. The CLI becomes a thin layer on top. The type shape is fixed by [ADR-001](design/001-vendor-extension-typing.md); the split timing is the subject of a future crate-split ADR.
- **Task management properties (`VTODO`, candidate).** Because `makeholiday` is positioned as a general ICS CLI, `VTODO` support is on the table; scope is not yet committed.
- **TUI front-end (candidate).** An interactive terminal UI may be added if CLI UX hits its ceiling. Not in scope today but not ruled out.

## 6. Non-Functional Requirements

- **Platforms.** First-class support for Windows, macOS, and Linux. CI must cover all three.
- **Performance.** Operations on calendars with up to ~10,000 events complete in well under a second on commodity hardware. Larger calendars are a stretch goal.
- **Memory.** Streaming-friendly parser is preferred over whole-file load when feasible; not a v0.x blocker.
- **Stability.** Public CLI surface follows SemVer once 1.0 ships. Until 1.0, breaking changes are documented in the changelog.
- **Error reporting.** Errors identify the input line and the offending property name when parsing ICS; commands fail closed (non-zero exit) rather than silently dropping data.
- **Internationalization.** Summary, categories, and other free-text fields must round-trip non-ASCII (UTF-8) without escaping or loss. Default examples and help text are English; Japanese translations live in `docs/*.jp.md`.

## 7. Out of Scope

Distinct from Non-Goals: these are explicitly *not* committed for any planned release, though some may move into scope later.

- Cloud sync of calendar state between machines.
- Calendar invitation workflows (iTIP, `REQUEST` / `REPLY` / `CANCEL` method handling).
- Recurring event expansion to discrete instances (RRULE materialization). RRULE *preservation* on round-trip is in scope; expansion is not.
- Time zone database bundling. We rely on the system tz database where time zones come into play.

## 8. Open Questions

- **TUI front-end** — when does it become worth building, and what subset of commands does it cover first?
- **`VTODO` scope** — full read/write parity with `VEVENT`, or read-only preservation for round-trip?
- **`convert` subcommand** — is vendor profile conversion a goal, or is "preserve as input vendor profile" sufficient?
- **Crate split timing** — extract the library before or after the typed vendor-extension model lands?
- **License of preset icon names / descriptions** — the `PRESET_ICONS` table ships under the project license; revisit if we add SVG / image assets later.
