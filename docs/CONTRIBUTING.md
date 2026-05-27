[ **English** | [日本語](CONTRIBUTING.jp.md) ]

# Contributing to makeholiday

Thanks for taking the time to contribute. This document covers the development workflow and conventions used in this repository.

## Code of Conduct

Be respectful and constructive. A dedicated `CODE_OF_CONDUCT.md` may be added later; until then, the spirit of the [Contributor Covenant](https://www.contributor-covenant.org/) applies.

## Development Environment

- **Toolchain:** Rust, edition `2024` (see [Cargo.toml](../Cargo.toml))
- **Build:** `cargo build`
- **Test:** `cargo test`
- **Run locally:** `cargo run -- <subcommand> [options]`

No platform-specific setup is required; the project builds on Windows, macOS, and Linux.

## Project Layout

```
src/
  main.rs       # Entry point, dispatches subcommands
  cli.rs        # Clap definitions, date parsing
  commands.rs   # init / add / list / remove implementations
  ics.rs        # ICS parsing, formatting, sorting
tests/
  cli.rs        # Integration tests via assert_cmd
docs/
  README.jp.md
  PRD.md, PRD.jp.md
  CONTRIBUTING.md, CONTRIBUTING.jp.md
  design/       # Architectural Decision Records (ADRs)
```

## Workflow

- **Trunk-based development.** Short-lived branches off `main`, merged via small PRs. Long-lived feature branches are discouraged.
- **Branch naming.** `<type>/<short-slug>`, e.g. `feat/add-rrule`, `fix/parse-date`, `docs/contributing`.
- **One concern per PR.** Mix-ups between refactors and behavior changes make review harder.

## Commit Messages

Follow the existing Conventional Commits-flavored style visible in `git log`:

- `feat: ...` — new user-facing capability
- `fix: ...` — bug fix
- `chore: ...` — tooling, build, gitignore, dependency bumps
- `refactor: ...` — internal restructuring with no behavior change
- `docs: ...` — documentation only
- `test: ...` — tests only

Keep the subject line under ~72 characters. Use the body for the *why*.

## Coding Principles

- **TDD (Red → Green → Refactor).** New behavior arrives with a failing test first. When existing tests are absent, add a minimal characterization test that captures current behavior before changing it.
- **Tidy First, non-ad-hoc.** Minimize the surface of change. Prefer reorganizing related code before introducing new code rather than after.
- **Separation of concerns.** Respect the boundaries between `cli` (parsing user input), `commands` (orchestration), and `ics` (domain serialization). Dependencies flow toward higher-level policy; do not let lower layers reach upward.
- **Express intent through names and structure.** Comments inside functions should be minimal; if intent is unclear, prefer extracting or renaming over adding comments. Interface contracts belong in doc comments.
- **State-centric design.** Reason about Given / When / Then. When state semantics are ambiguous, agree on them before writing the algorithm.

## Documentation Changes

- User-facing changes update both `README.md` (English, primary) and `docs/README.jp.md` (Japanese translation) in the same PR.
- Product direction changes update `docs/PRD.md` (and the JP mirror).
- Architectural decisions are recorded as ADRs under `docs/design/`, following [`000-ADR-policy.md`](design/000-ADR-policy.md).
- Respect existing documentation; do not silently rewrite history of decisions.

## Testing

- `cargo test` must pass before submitting a PR.
- New features ship with tests. Integration coverage lives in `tests/cli.rs`; unit-level coverage lives alongside the module under test (`#[cfg(test)] mod tests`).
- For bug fixes, add a regression test that fails without the fix.

## Issue / PR Templates

For now, follow the structure in this document directly when filing issues or PRs. Dedicated templates under `.github/` may be introduced later.

## Licensing of Contributions

Contributions are dual-licensed under **MIT OR Apache-2.0**, matching the project license. By submitting a contribution you agree that it may be distributed under those terms.
