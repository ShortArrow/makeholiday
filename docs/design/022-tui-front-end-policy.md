# 022. TUI Front-end Policy

- Status: **Superseded by [ADR-025](025-lazyics-project-definition.md)** (2026-05-29)
- Date: 2026-05-28

> **Note (2026-05-29)** — This ADR's "future plan, no launch date" stance has
> been superseded. [PRD §9](../PRD.md#9-roadmap) now stages the TUI as a
> v0.2.0 deliverable under the brand name **lazyics** (not `makeholiday-tui`),
> and [ADR-025](025-lazyics-project-definition.md) records the project
> definition — separate binary, ratatui, in-tree workspace member until the
> v0.2.0 repo split. Read this ADR only for historical context; treat
> ADR-025 as authoritative.

## Context

[PRD §3 Non-Goals](../PRD.md#3-non-goals) explicitly does **not** rule out a TUI ("A TUI is *not* ruled out — see §8"). [PRD §8 Open Question](../PRD.md#8-open-questions) asked: "when does it become worth building, and what subset of commands does it cover first?"

The workspace structure from [ADR-017](017-workspace-and-ics-core-crate.md) makes a TUI tractable as a third workspace member that consumes `ics-core`, alongside the existing `makeholiday` binary and the future external `icslint` consumer. Today the CLI already has interactive prompts for `add` and `remove` when required arguments are omitted ([ADR-015](015-diagnostic-output.md)) — these cover the immediate ergonomic gaps; the TUI would address richer interaction patterns the CLI cannot reach (multi-event selection, in-place editing of a calendar view, fuzzy filtering across thousands of events).

This ADR records the **intent and the initial scope** so a future implementer is not starting from a blank page, without committing to a launch date.

## Decision

### Future plan, no launch date

A TUI front-end **is on the roadmap** but **not in active development**. It will be added as a separate workspace member (e.g. `crates/makeholiday-tui/`) that depends on `ics-core` (and, if it wants to reuse use case logic, on the `makeholiday` library crate per [ADR-010](010-lib-and-main-separation.md)).

- The CLI binary (`makeholiday`) and the TUI binary (`makeholiday-tui`) are **sibling consumers** of `ics-core`. Neither depends on the other for runtime.
- The TUI is **not** a replacement for the CLI; both ship side-by-side. The CLI remains the primary surface; the TUI is an opt-in alternative for users who prefer interactive navigation.

### Launch trigger — maintainer judgment

No numeric or feature-based trigger is committed. Launch happens when the maintainer judges that TUI investment beats the next-best alternative use of their time. This is recorded explicitly so future contributors do not interpret "no trigger" as "blocked on something specific."

If a TUI starts taking shape (PR with skeleton code, RFC issue, etc.), a follow-up ADR records the launch decision, the chosen TUI library (see below), and the public roadmap.

### Initial scope (when launched)

The first releasable TUI iteration covers:

1. **Interactive list view** — render all events (and optionally todos per [ADR-021](021-vtodo-scope.md)) with scrolling, keyboard navigation, search-as-you-type filter, and a date-range jump.
2. **Add via form** — a form that captures `--summary`, `--start`, `--end`, `--busystatus`, `--class`, `--category` (multi-entry), `--icon`. Submit writes through `application::use_cases::add_event` so the typed model and round-trip rules apply uniformly.
3. **Select-and-remove** — multi-select events from the list view, confirm, and delete via `application::use_cases::remove_event`.

Out of scope for the initial release:

- 1:1 port of every CLI subcommand. The CLI's `icons` listing, `--json` output, `init`, etc. have no TUI equivalent because the CLI surface is sufficient for them.
- VTODO editing (per [ADR-021](021-vtodo-scope.md)): the TUI may display todos in the list view but does not edit them. Same scope discipline as the CLI.
- Calendar-level metadata editing (X-WR-CALNAME etc.). Future addition; not required for the first release.
- Plugin / theming systems. The TUI is one cohesive UI, not a framework.

### TUI library choice — deferred to launch ADR

The implementation language (Rust per [ADR-002](002-language-and-edition.md)) constrains the candidate libraries. Likely candidates at launch time:

- `ratatui` — the de facto choice in modern Rust CLIs; active, broad widget library, well-documented.
- `cursive` — alternative declarative TUI; smaller widget surface but simpler programming model.

Selection happens in the launch ADR following [ADR-013](013-dependency-policy.md) (license, MSRV, maintenance, alternatives considered). This ADR commits only to "a typical Rust TUI library is acceptable"; no library is pre-blessed.

### Integration with existing layers

When the TUI is built:

- It is a **presentation-layer consumer** ([ADR-009](009-module-layering.md)) sitting where today's CLI `presentation` sits — but in a different binary crate, not in `makeholiday`'s binary. The CLI's `presentation` module stays untouched.
- It uses **`application::use_cases::*`** for every state-mutating action. Use case functions take `&CalendarRepository`, so the TUI passes the same `FileCalendarRepository` the CLI uses.
- Errors (`MhError` per [ADR-012](012-error-handling.md)) surface as TUI dialogs / status bars instead of stderr lines. Formatting is the TUI's choice; the error type is shared.

### Distribution

- Released alongside the CLI as a separate binary (`makeholiday-tui`). Both share the same release pipeline ([ADR-014](014-ci-cd-platform.md)), tagged `v*` triggers builds of both, with two artifact sets per target.
- Users who want only the CLI install only the CLI (`cargo install makeholiday`); users who want the TUI install separately (`cargo install makeholiday-tui`). No bundled install path is provided.
- Pre-1.0 the TUI is not published to crates.io alongside the CLI. Publication timing matches whatever the launch ADR decides.

## Consequences

### Positive

- The "TUI is not ruled out" pledge in [PRD §3](../PRD.md#3-non-goals) gains structure: a future contributor (or returning maintainer) sees the intended scope and shape, not a blank slate.
- The TUI being a separate binary crate keeps the CLI clean and lean — no clap-vs-ratatui dependency entanglement, no opt-in feature flag, no conditional compilation.
- Reusing `application::use_cases::*` means the TUI cannot diverge in behavior from the CLI: both go through the same typed-model, the same I/O boundary, the same errors.
- Sibling-binary architecture leaves room for additional front-ends (web, GUI) without restructuring, even though those are out of scope per [PRD §3](../PRD.md#3-non-goals).

### Negative

- The "no trigger" decision means the TUI may never ship if no one prioritizes it. Acceptable: the CLI is the contract; the TUI is a nice-to-have.
- Pre-blessing `ratatui` / `cursive` would make the launch ADR shorter but might prematurely lock in a choice that turns out to be wrong by the time launch happens. We accept the future cost of choosing then.
- Users may expect the TUI as soon as they read the PRD; they have to live with "planned, no date."

### PRD update

This decision partially closes [PRD §8 Open Question](../PRD.md#8-open-questions) on TUI. The open question is replaced with the pointer to this ADR; the launch trigger is left under "maintainer judgment" rather than dropped from §8.

The PRD update is a small documentation commit, independent of this ADR's acceptance.

### Migration

No code work follows this ADR. The TUI launch — whenever it happens — is its own ADR + implementation effort. The migration step is purely documentary:

1. Update [PRD §8 Open Questions](../PRD.md#8-open-questions) to point to this ADR for the TUI question (without removing the entry, since the launch trigger remains open).
2. No code changes.
