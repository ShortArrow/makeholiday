[ **English** | [日本語](docs/CHANGELOG.jp.md) ]

# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) from `1.0.0` onward. Pre-1.0 releases may include breaking changes; see [ADR-004](docs/design/004-trunk-based-and-semver.md).

## [Unreleased]

### Changed
- Repository restructured into a Cargo workspace per [ADR-017](docs/design/017-workspace-and-ics-core-crate.md). `Cargo.toml` is now the workspace manifest; the `makeholiday` binary crate lives under `crates/makeholiday/`. No behavior change.
- Added empty `crates/ics-core/` workspace member (Step 2 of ADR-017 Migration). Wired as a path dependency from `makeholiday`. No public surface yet; types and parser move in Step 3.
- Moved typed model (`VEvent`, `BusyStatus`, `EventClass`, `SortKey`) and the parser / formatter / query helpers from `crates/makeholiday/src/ics.rs` into `crates/ics-core/src/{event,calendar,parser,query}.rs` (Step 3 of ADR-017 Migration). makeholiday now consumes the model via `ics_core`. Makeholiday-namespace preset icons (`PRESET_ICONS`, `format_icons_list`) relocate to a new `crates/makeholiday/src/icons.rs` rather than into `ics-core`. No behavior change.
- Introduced typed `ics_core::Error` (ADR-017 §error type relationship). `parse_events`, `parse_indices`, `insert_event`, `remove_event_by_summary`, `remove_events_by_indices` now return `Result<T, ics_core::Error>` instead of `Result<T, String>`. The `Parse` variant carries `message`, optional `line`, and optional `property` fields; current flat parser leaves the optional fields `None` and ADR-019's lexer-based parser will populate them.
- Introduced `makeholiday::error::MhError` per ADR-012 / ADR-017 §error type relationship. Six variants: `Io { path, source }`, `Parse(#[from] ics_core::Error)`, `InvalidInput(String)`, `Conflict(String)`, `NotFound(String)`, `AlreadyExists { path }`. `commands::*` returns `Result<_, MhError>`; `ics_core::Error` propagates via `?` thanks to `#[from]`. Tests assert variant matches (`matches!(err, MhError::InvalidInput(_))`) instead of inspecting error strings.
- Introduced `crates/makeholiday/src/lib.rs` (per ADR-010 / ADR-017) declaring `pub mod cli; pub mod commands; pub mod error; pub mod icons;`. `main.rs` becomes a slim Composition Root that imports via `use makeholiday::*`. The library surface lets future use-case unit tests (and a future TUI per ADR-022) reuse this code without spawning the binary.

### Added
- Initial documentation scaffold: `README`, `PRD`, `CONTRIBUTING`, `SETUP`, `USAGE` (English + Japanese mirrors).
- ADRs 000–023 covering ADR policy, vendor extension typing model, language/edition, dual licensing, trunk-based development + SemVer, Conventional Commits, testing strategy, documentation language policy, MSRV, module layering, lib/main separation, I/O boundary + repository pattern, error handling, dependency policy, CI/CD platform, diagnostic output, configuration policy, workspace + `ics-core` crate, round-trip strategy, parser implementation, CLI subcommand policy, VTODO scope, TUI front-end policy, and the explicit rejection of a `convert` subcommand.
- [ADR-024](docs/design/024-solo-phase-branching-carve-out.md) — solo-phase carve-out that suspends the ADR-004 feature-branch + PR ceremony until `ics-core` is split, an external contributor opens a PR, or `v1.0.0` is tagged.

## [0.1.0]

### Added
- `init` subcommand — create a new `VCALENDAR` file.
- `add` subcommand — append all-day `VEVENT` with `--summary` / `--start` / `--end`, optional `--busystatus`, `--class`, `--category` (repeatable), `--icon`; interactive prompts when required args are omitted.
- `list` subcommand — enumerate events with `--sort` (repeatable: `start` / `end` / `summary`), `--desc`, `--json`.
- `icons` subcommand — print bundled preset icon names.
- `remove` subcommand — delete events by 1-based index expression (`N`, `N-M`, `N,M`, mixed), `--summary` match, or interactive selection.
- Dual licensing: MIT OR Apache-2.0.
