# 006. Testing Strategy — TDD + Integration via assert_cmd

- Status: **Accepted** (retroactive)
- Date: 2026-05-27

## Context

The project's user-level CLAUDE.md and [CONTRIBUTING.md](../CONTRIBUTING.md) both mandate TDD ("Red → Green → Refactor"). [PRD §6 NFR](../PRD.md#6-non-functional-requirements) commits to "errors fail closed (non-zero exit) rather than silently dropping data" — a testable contract.

Current setup (recorded here for the record):

- `cargo test` is the single test command.
- Integration coverage lives in `tests/cli.rs`, driving the binary through `assert_cmd` and `predicates`.
- Unit coverage lives alongside each module, e.g. `src/commands.rs` ends with `#[cfg(test)] mod tests`.
- `tempfile` creates real on-disk scratch directories; nothing mocks the filesystem.

Without an explicit ADR, a contributor could reasonably assume "tests are nice but optional" or "we'll mock more later when integration tests get slow." This ADR pins the practice so the team does not drift.

## Decision

### Process

- **TDD: Red → Green → Refactor.** New behavior arrives with a failing test in the same PR as the implementation.
- **Bug fixes ship with a regression test** that fails without the fix.
- **Refactors with no behavior change require no new test;** existing tests passing is sufficient.

### Test layout

- **Unit tests** live in `#[cfg(test)] mod tests` at the bottom of the module they exercise. Visible to crate-private items.
- **Integration tests** live in `tests/*.rs`, exercising the public surface (the CLI binary or, post-crate-split, the library's public API).
- **Test data** lives in `tests/data/` when files are required; otherwise tests construct fixtures inline via `tempfile::TempDir`.

### Test infrastructure

- **Binary integration** uses `assert_cmd` to invoke the built `makeholiday` binary as a subprocess. Assertions on stdout/stderr/exit-code use `predicates`.
- **Filesystem fixtures** use `tempfile` for `TempDir` per test; no shared state between tests.
- **No filesystem mocking.** Tests touch real temp files. The cost of slow tests is paid in exchange for catching bugs that only appear in real I/O paths.

### Test naming

- `snake_case`, descriptive, present tense, no `test_` prefix (Rust convention): `list_returns_numbered_lines`, `add_end_before_start_errors`.
- Names describe the *behavior under test*, not the *function under test*.

### What we test

- **CLI contract:** exit codes, stdout/stderr separation, flag parsing, prompt flow for interactive modes.
- **File format invariants:** round-trip through `parse_events` / `format_calendar`, RFC-required properties present, vendor-specific properties preserved (ADR-001).
- **Error paths:** malformed input, missing required fields, conflicting flags.

### What we do not require tests for

- Tooling and build configuration changes (`chore:` commits) when no code path moves.
- Pure documentation changes (`docs:` commits).

## Consequences

### Positive

- Single test command (`cargo test`) covers everything; CI is trivially configurable.
- Real-filesystem integration catches a class of bugs that mocks miss (path separators, atomic-write behavior, file encoding).
- TDD discipline forces requirements clarification before implementation, aligning with the PRD-first principle in user CLAUDE.md.
- Test names double as a behavior catalog when reading `cargo test --list`.

### Negative

- Test runtime grows linearly with feature count; eventually we may need to split slow integration tests behind a feature flag or feature category. Acceptable until felt.
- Real-filesystem tests are slightly slower than mocked tests. Acceptable; the speed gap is small for a CLI that mostly does I/O.
- Strict TDD adds friction to "quick experiments." We accept the friction; experimentation belongs on a branch, not in `main`.
- No property-based or fuzz testing today; if vendor extension typing (ADR-001) widens, a future ADR may introduce property testing for ICS round-trip.
