[ **English** | [日本語](PRD.jp.md) ]

# Product Requirements Document — icscli

> Status: **Draft**. Sections 1–4 and 5.1 are settled; later sections still evolving.

## 1. Background

iCalendar (RFC 5545) is the de-facto interchange format for calendar data, but the ecosystem around it is fragmented. Existing ICS tooling tends to fall into one of two failure modes:

- **Strict RFC-only tools** that drop or refuse vendor-specific extensions (Outlook's `X-MICROSOFT-CDO-*`, Google's `X-GOOGLE-*`, iCloud's variants), causing silent data loss on round-trip.
- **Vendor-bound tools** that handle one vendor's dialect well but cannot describe the others as first-class data.

In practice, users who simply want to assemble or edit `.ics` files — for personal holidays, team calendars, or interoperability glue between calendar services — end up writing one-off scripts each time, or accepting lossy conversions.

`icscli` exists to close that gap with a small, deliberate tool: a CLI for everyday ICS authoring on top of a typed core that treats vendor extensions as first-class, not as opaque strings. (The v0.1.x series shipped under the name `makeholiday`; renamed at v0.2.0 per [ADR-027](design/027-makeholiday-to-icscli-rename.md).)

## 2. Goals

In priority order:

1. **CLI UX (highest priority).** The CLI must be pleasant to use for everyday calendar authoring — discoverable subcommands, sensible defaults, both scriptable (full flags) and interactive modes. UX considerations override architectural neatness when they conflict.
2. **Round-trip losslessness.** Reading and re-emitting an ICS file preserves order, whitespace where semantically meaningful, and *all* properties — including unknown and vendor-specific ones. A file passed through `icscli` is recognizable to its origin tool. See [ADR-001](design/001-vendor-extension-typing.md) for the typing-level commitments; ordering semantics are deferred to a future round-trip strategy ADR.
3. **Typed handling of vendor extensions.** Outlook / Google / iCloud extensions are modeled as distinct, type-safe values, not as raw `X-*` strings. The boundary between RFC 5545 and each vendor profile is explicit in the code and documented. See [ADR-001](design/001-vendor-extension-typing.md) for the model.
4. **Library reusability.** The ICS handling core is consumable as an independent crate, so other tools can depend on it without pulling in CLI machinery.

## 3. Non-Goals

Things `icscli` will not do, full stop. CalDAV / cloud-service synchronization is *not* on this list — it is staged for v0.3.0 per [§9 Roadmap](#9-roadmap).

- **GUI / WebUI.** No desktop application, no web interface. (A TUI is planned as a sibling binary — see [ADR-022](design/022-tui-front-end-policy.md).)
- **Non-ICS calendar formats.** Microsoft `.msg`, legacy vCalendar 1.0, proprietary binary calendar formats are out of scope.
- **Vendor profile conversion.** Translating ICS from one vendor's flavor (Outlook / Google / iCloud) to another's is out of scope. Round-trip preserves the source profile unchanged. See [ADR-023](design/023-no-convert-subcommand.md).

## 4. Target Users

Both groups are supported; the CLI persona drives prioritization.

- **Primary — CLI-comfortable individuals** managing personal holiday, vacation, or event calendars from the terminal. They value scriptability, plain-text storage, and minimal ceremony.
- **Secondary — calendar integrators** building tools that generate or consume ICS. They need the library surface and the typed vendor-extension model.

When the two personas conflict, the CLI persona wins.

## 5. Functional Requirements

### 5.1 Currently shipped (v0.1.0)

Implemented and covered by tests in `tests/cli.rs` and unit tests in `src/`:

- **`init`** — create a new `VCALENDAR` file (`PRODID:-//icscli//EN`, `VERSION:2.0`).
- **`add`** — append a `VEVENT` (all-day, single or multi-day). Supports:
  - `--summary`, `--start`, `--end` (inclusive on input, converted to RFC-exclusive `DTEND` internally)
  - Date input formats: `YYYY-MM-DD` and `YYYY/M/D`
  - `--busystatus` (`free` / `tentative` / `busy` / `oof` / `working`) emitting `TRANSP` + `X-MICROSOFT-CDO-BUSYSTATUS`
  - `--class` (`public` / `private` / `confidential`)
  - `--category` (repeatable)
  - `--icon` (vendor extension `X-ICSCLI-ICON`)
  - Interactive mode when `--summary` / `--start` are omitted
- **`list`** — enumerate events. `--sort` (repeatable: `start` / `end` / `summary`), `--desc`, `--json`.
- **`icons`** — print bundled preset icon names.
- **`remove`** — delete events by 1-based index (`N`, `N-M`, `N,M`, mixed), or `--summary` match, or interactive selection.

### 5.2 Planned

Items are listed in approximate priority. Acceptance criteria to be expanded as work begins.

- **`edit` subcommand** — modify an existing event in place by index. Required to round out CRUD.
- **`search` / `filter` subcommand** — query events by date range, summary substring, category, or busy status.
- **`import` / `export` subcommand variants** — bulk ingestion from other ICS files. Vendor-profile preservation only; no normalization or conversion (see [ADR-023](design/023-no-convert-subcommand.md)).
- **Vendor extension support — Outlook profile.** First-class types for `X-MICROSOFT-CDO-*` family, reminders, categories color, etc. Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **Vendor extension support — Google profile.** First-class types for `X-GOOGLE-*` and Google-specific value handling. Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **Vendor extension support — iCloud profile.** First-class types for Apple-specific extensions (`X-APPLE-*`, `X-CALENDARSERVER-*`). Typing model defined in [ADR-001](design/001-vendor-extension-typing.md).
- **RFC ↔ vendor extension boundary documentation.** A reference document, generated where possible from code, listing which properties live in RFC 5545 and which belong to which vendor profile. Boundary rules captured in [ADR-001](design/001-vendor-extension-typing.md).
- **Reusable ICS handling library (`ics-core` crate).** The shared core lives in `crates/ics-core/` as an in-tree workspace member; external publication timing is settled by [ADR-017](design/017-workspace-and-ics-core-crate.md). Type shape per [ADR-001](design/001-vendor-extension-typing.md).
- **Task management properties (`VTODO`).** Typed `VTodo` in `ics-core`; the `icscli` CLI exposes read-only display via `list --include-todos` (no editing subcommands). See [ADR-021](design/021-vtodo-scope.md).
- **TUI front-end (`lazyics`).** Separate `lazyics` binary consuming `ics-core` and the `icscli` library's use cases, planned for v0.2.0. See [ADR-025](design/025-lazyics-project-definition.md). [ADR-022](design/022-tui-front-end-policy.md) is the predecessor (TUI policy, no launch date) and is superseded.

## 6. Non-Functional Requirements

- **Platforms.** First-class support for Windows, macOS, and Linux. CI must cover all three.
- **Performance.** Operations on calendars with up to ~10,000 events complete in well under a second on commodity hardware. Larger calendars are a stretch goal.
- **Memory.** Streaming-friendly parser is preferred over whole-file load when feasible; not a v0.x blocker.
- **Stability.** Public CLI surface follows SemVer once 1.0 ships. Until 1.0, breaking changes are documented in the changelog.
- **Error reporting.** Errors identify the input line and the offending property name when parsing ICS; commands fail closed (non-zero exit) rather than silently dropping data.
- **Internationalization.** Summary, categories, and other free-text fields must round-trip non-ASCII (UTF-8) without escaping or loss. Default examples and help text are English; Japanese translations live in `docs/*.jp.md`.

## 7. Out of Scope

Distinct from Non-Goals: these are explicitly *not* committed for any planned release, though some may move into scope later.

- Cloud sync of calendar state between machines. → graduates into v0.3.0 scope alongside CalDAV; see [§9 Roadmap](#9-roadmap).
- Calendar invitation workflows (iTIP, `REQUEST` / `REPLY` / `CANCEL` method handling).
- Recurring event expansion to discrete instances (RRULE materialization). RRULE *preservation* on round-trip is in scope; expansion is not.
- Time zone database bundling. We rely on the system tz database where time zones come into play.

## 8. Open Questions

- *(Resolved 2026-05-29)* **TUI front-end launch trigger** — launching in the v0.2.0 ICS Ecosystem milestone under the `lazyics` brand; project definition in [ADR-025](design/025-lazyics-project-definition.md).
- **License of preset icon names / descriptions** — the `PRESET_ICONS` table ships under the project license; revisit if we add SVG / image assets later.

## 9. Roadmap

`icscli` evolves in versioned milestones. Each milestone has a clear scope and is delivered as a series of minor releases.

### v0.1.x — ICS Text Operations (current)

The v0.1.x series (shipped under the name `makeholiday`) scopes the CLI as a high-fidelity local ICS file manager. The `ics-core` library aim is to be a typed lingua franca for RFC 5545 plus the major vendor extension dialects.

- Lossless round-trip with typed vendor extensions ([ADR-001](design/001-vendor-extension-typing.md) Migration complete).
- Parser correctness — RFC 5545 line folding, UTF-8 BOM handling, TEXT escape decode/encode (ADR-019, in progress).
- Calendar-level extension surface — `X-WR-*` typed promotion, `VCalendar.unknown` bucket.
- CLI subcommand completeness: `edit`, `search` / `filter`, `import` / `export`.
- CLI UX polish ([ADR-015](design/015-diagnostic-output.md) `--quiet` / `--interactive`, [ADR-020](design/020-cli-subcommand-policy.md) help-text examples).
- v0.1.0 freezes the CLI surface contract for SemVer purposes ([ADR-004](design/004-trunk-based-and-semver.md)).

### v0.2.0 — ICS Ecosystem (next)

The v0.2.0 series shifts the project from a single CLI to a small ecosystem of tools all consuming the same `ics-core` library. The library graduates from in-tree workspace member to a published crate with its own repository.

- **`ics-core` extracted to its own repository** and published to crates.io. The version contract for `ics-core` begins here, derived from the v0.1.x experience inside this repo. See [ADR-017](design/017-workspace-and-ics-core-crate.md) for the split trigger and lifecycle.
- **`icscli` — the renamed CLI** (was `makeholiday` in v0.1.x). The brand rename ([ADR-027](design/027-makeholiday-to-icscli-rename.md)) aligns the CLI with the rest of the `ics*` ecosystem. Functional surface preserved.
- **`lazyics` — interactive TUI editor** for `.ics` files, inspired by `lazygit`. Naming convention: lazy-prefixed TUI tools. Ships as a **separate binary** (`cargo install lazyics`), built on `ratatui`, depending on the `icscli` library's use cases to mechanically prevent CLI/TUI divergence. See [ADR-025](design/025-lazyics-project-definition.md) (supersedes [ADR-022](design/022-tui-front-end-policy.md)).
- **`icslint` — ICS lint tool** consuming `ics-core`. Surfaces vendor-prefix warnings ("this property is Microsoft-specific and will be ignored by Google clients") and RFC compliance hints. Four rule families ship at v0.2.0 — RFC 5545 cardinality/required, vendor hygiene, text encoding, structure. See [ADR-026](design/026-icslint-project-definition.md).
- `icscli` itself continues evolving on the v0.1.x feature trajectory — additive features like `search` / `filter`, `import` / `export`, calendar-level extensions land here too.

The three-tool launch is the "ecosystem" theme. The release-train discipline of [ADR-024](design/024-solo-phase-branching-carve-out.md) reactivates the moment `ics-core` lands in its own repository (the carve-out's first trigger).

### v0.3.0 — CalDAV / Cloud Backend

The v0.3.0 series extends the ecosystem into a multi-backend story. The `ics-core` parser and typed model carry over unchanged because every CalDAV response is a syntactically valid `VCALENDAR` blob — the work concentrates on the I/O boundary, on event identity, and on time-of-day typing.

- CalDAV client integration with a per-event `Repository` abstraction (`fetch_by_uid`, `put_event`, `delete_by_uid`) alongside the bulk file-level API.
- ETag-based optimistic locking on event resources.
- Timed `VEvent` typing — revises [ADR-001](design/001-vendor-extension-typing.md) Rule 9 so that `DTSTART;VALUE=DATE-TIME` events stop falling back to `RawComponent`.
- `VTimezone` typing alongside the timed-event work.
- Authentication scaffolding for cloud calendars (CalDAV servers, future provider-specific APIs).

This unblocks the "Cloud sync of calendar state between machines" item currently in [§7 Out of Scope](#7-out-of-scope).

### Beyond v0.3.0

Open. Candidates currently on the watch list:

- VTODO full editing (currently planned as read-only via `list --include-todos`; see [ADR-021](design/021-vtodo-scope.md)).
- Additional vendor profile typed fields beyond the current Microsoft `busystatus`.
- RRULE materialization (recurring-event expansion), per `§7` still out of scope today.
- Provider-specific cloud APIs (Google Calendar API, Microsoft Graph) layered on top of the CalDAV-shaped Repository abstraction.
