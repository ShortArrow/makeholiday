# 017. Workspace Structure and `ics-core` Crate

- Status: **Accepted**
- Date: 2026-05-28
- Amended: 2026-06-02 — [ADR-027](027-makeholiday-to-icscli-rename.md) renamed the CLI crate from `makeholiday` to `icscli`. The architectural decisions in this ADR (workspace shape, `ics-core` scope, error type wrapping, split timing) are unaffected. References below to the `makeholiday` crate / binary should be read as the `icscli` crate / binary from v0.2.0 onward; original prose preserved for historical accuracy.

## Context

[PRD §2 Goal 4](../PRD.md#2-goals) commits to a reusable ICS handling library. [ADR-010](010-lib-and-main-separation.md) introduced `src/lib.rs` as the library boundary inside a single Cargo package, deferring the actual library/binary separation to "a future crate-split ADR." That future has arrived under specific pressure:

- A sibling project, **icslint** (an ICS linter), is being planned. It needs to consume the same ICS handling logic that makeholiday uses.
- A survey of crates.io shows the existing `ics` crate (last updated 2022-09, creator-only) and `icalendar` (active, builder/parser, 93k downloads/month) both fall short of [ADR-001](001-vendor-extension-typing.md)'s typed vendor extension model — `icalendar` stores properties in `BTreeMap<String, Property>` with no vendor profiles, no order preservation, and no unknown-property round-trip.
- Reusing an existing crate would require abandoning ADR-001's design. Writing our own is the lesser cost.

The maintainer prefers to land the library as an **internal workspace crate first**, then split into a separate repository when icslint launches. The workspace approach gives boundary discipline today without requiring crates.io publication or repository-split logistics yet.

## Decision

### Workspace structure

The repository becomes a Cargo workspace with two member crates:

```
makeholiday/                  # repo root, workspace manifest
├── Cargo.toml                # [workspace] members = ["crates/*"]
├── Cargo.lock
└── crates/
    ├── ics-core/             # shared library: ICS types, parser, formatter, vendor profiles
    │   ├── Cargo.toml        # name = "ics-core"
    │   └── src/
    │       ├── lib.rs
    │       ├── event.rs
    │       ├── calendar.rs
    │       ├── parser.rs
    │       ├── error.rs      # parse-time errors
    │       └── profile/
    │           ├── mod.rs
    │           ├── microsoft.rs
    │           ├── google.rs
    │           └── icloud.rs
    └── makeholiday/          # CLI binary
        ├── Cargo.toml        # name = "makeholiday"
        └── src/
            ├── main.rs       # Composition Root
            ├── lib.rs        # makeholiday's library surface (re-exports + use case API)
            ├── error.rs      # MhError wrapping ics_core::Error
            ├── presentation/
            ├── application/
            └── infrastructure/
```

The workspace root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
authors = ["ShortArrow"]
repository = "https://github.com/ShortArrow/makeholiday"

[workspace.dependencies]
chrono = { version = "0.4", features = ["clock"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }
clap = { version = "4", features = ["derive"] }
tempfile = "3"
```

Each crate's `Cargo.toml` uses `package.edition.workspace = true` etc. and `dependencies` declares only what that crate needs from `workspace.dependencies`.

### `ics-core` crate scope

- **Owns:** `VEvent`, `VCalendar`, `RawProperty`, `RawComponent`, vendor-specific event/calendar extension types (`microsoft::EventExtensions`, etc.), the parser, the formatter, and `ics_core::Error` (parse errors with `{ line, message, property }`).
- **Does not own:** CLI concerns, file I/O, makeholiday-specific X-MAKEHOLIDAY-* logic, repository abstractions.
- **Dependencies:** `chrono`, `serde`, `uuid`, `thiserror`. No `clap`, no `tempfile`.
- **Public surface:** intentionally generous within the crate (consumers will need `VEvent` fields, profile types, parser entry points). Specifics evolve during ADR-001 Migration.

### `makeholiday` crate scope

- **Owns:** `application` (use cases + ports), `infrastructure` (`FileCalendarRepository`), `presentation` (CLI binary side), `error::MhError`.
- **Depends on:** `ics-core` via `ics-core = { path = "../ics-core" }`. All workspace deps via `.workspace = true`.
- The CLI binary continues to be named `makeholiday`.

### Supersedes from earlier ADRs

- [ADR-009](009-module-layering.md) — the `domain` layer in its original definition becomes the contents of `ics-core`. The dependency direction rules continue to apply, now crossing a crate boundary instead of a module boundary. `domain` → `crate::error` becomes `ics-core` → `ics_core::error`. `application` → `domain` becomes `makeholiday::application` → `ics_core::{event, calendar, profile}`.
- [ADR-010](010-lib-and-main-separation.md) — the single-crate dual-target shape (one `Cargo.toml` with both `src/lib.rs` and `src/main.rs`) is replaced by the workspace shape. `makeholiday` still has its own `lib.rs` and `main.rs` (so use cases remain testable via library API), but `domain` is no longer in `makeholiday`'s `lib.rs`; it lives in `ics-core`.
- [ADR-001](001-vendor-extension-typing.md) Rule 6 — the `makeholiday` namespace within vendor profile bundles changes scope. `ics-core` ships built-in `microsoft` / `google` / `icloud` profiles only; the `makeholiday` namespace (originally containing `X-MAKEHOLIDAY-ICON`) becomes a thin reader/writer in the `makeholiday` crate that operates on `VEvent.unknown`. Effect: `ics-core` stays generic; makeholiday-specific `X-MAKEHOLIDAY-*` handling lives where it belongs.

### Error type relationship

```rust
// crates/ics-core/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("parse error at line {line}: {message}{}", ...)]
    Parse { line: u32, message: String, property: Option<String> },
}
pub type Result<T> = std::result::Result<T, Error>;
```

```rust
// crates/makeholiday/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum MhError {
    #[error("I/O error: {}", .path.display())]
    Io { path: PathBuf, #[source] source: io::Error },

    #[error(transparent)]
    Parse(#[from] ics_core::Error),       // wraps ics_core's parse error

    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("conflicting arguments: {0}")]
    Conflict(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("file already exists: {}", .path.display())]
    AlreadyExists { path: PathBuf },
}
```

`ics_core::Error` is automatically convertible into `MhError::Parse` via `#[from]`. This refines [ADR-012](012-error-handling.md): `Parse` is no longer a struct variant in `MhError`; it is a wrapper around `ics_core::Error`.

### Publishing strategy

- `ics-core` is **not published to crates.io yet**. It lives only as a workspace path dependency.
- Publication happens when one of these triggers fires:
  1. icslint reaches a state where it needs to depend on `ics-core` from a separate repository.
  2. `ics-core`'s public API stabilizes enough to commit to a versioned release (judged by the maintainer).
- At publication time, a follow-up ADR records the publish trigger, the chosen `ics-core` version, and any final renames / API curation needed before going public.
- `makeholiday` likewise stays unpublished until 1.0 (per [ADR-010](010-lib-and-main-separation.md)).

### Repository split strategy

- The workspace remains in this repository until `ics-core` is published.
- When `ics-core` is published and the workspace path dep is replaced with a crates.io version dep, that is the moment to consider moving `ics-core` to its own repository (`ShortArrow/ics-core` or similar).
- The split is mechanical: copy `crates/ics-core/` to the new repository, set up its own CI / release workflows, replace makeholiday's path dep with a crates.io version dep, archive the contents of `crates/ics-core/` in the makeholiday repo.
- A future ADR documents the split itself when it happens.

### CI/CD implications

- [ADR-014](014-ci-cd-platform.md) workflow files run `cargo test --workspace --locked` instead of single-crate `cargo test --locked`. Other matrix shape unchanged.
- `cargo clippy --workspace --all-targets`, `cargo fmt --check`, `cargo deny check` all gain `--workspace` where relevant.
- Release pipeline (`release.yml`) builds **the `makeholiday` binary only** from the `crates/makeholiday` directory. `ics-core` produces no binary.
- No `publish-crates` job until the publishing trigger above fires.

### Naming

- Crate name **`ics-core`** (kebab-case in `Cargo.toml`, `ics_core` in Rust source).
- "core" suffix signals "the ICS data core" — not a complete iCalendar ecosystem replacement, but the typed-model foundation.
- The existing crates.io `ics` crate (creator-only, stale since 2022) is not in our way for the `ics-core` name. If a publishing-time check finds `ics-core` is taken, the follow-up ADR picks a fallback (`ics-typed`, `icrs`, etc.).

## Consequences

### Positive

- icslint can consume `ics-core` cleanly via a path dep today (workspace) and via a crates.io version dep tomorrow (after split). The transition is mechanical.
- The boundary between "ICS data" and "ICS CLI" is enforced by the Rust compiler from day one. `make` and `cargo check --workspace` will fail if makeholiday-specific concepts leak into `ics-core`.
- The library surface in `ics-core` is the *only* public-API discussion forum from now on; the `makeholiday` crate stays free to evolve its CLI without breaking external consumers.
- Repo-split is delayed until there is concrete pressure (icslint), avoiding speculative repo proliferation.
- [PRD §2 Goal 4](../PRD.md#2-goals) gains concrete steps and a concrete consumer (icslint), not just a stated intent.

### Negative

- Workspace setup adds Cargo.toml plumbing — root manifest, per-crate manifests, workspace deps. One-time cost.
- ADR-001's `MakeholidayExtensions` namespace decision is partially superseded; makeholiday-specific X-* handling becomes a thin reader on `VEvent.unknown` instead of a first-class typed bundle. Small loss of expressiveness; acceptable because the same data is preserved, just less ergonomic.
- Refactoring the existing flat code (`src/{main,cli,commands,ics}.rs`) into the workspace structure is a larger Tidy First step than originally scoped under [ADR-009](009-module-layering.md) alone. Still tractable; lands as its own commits before [ADR-001](001-vendor-extension-typing.md) Migration Step 1.
- Path dependencies obscure the eventual published-version reality. We accept this until publication.

### Migration

Updates the Tidy First plan accumulated across [ADR-009](009-module-layering.md), [ADR-010](010-lib-and-main-separation.md), [ADR-011](011-io-boundary-and-repository.md), [ADR-012](012-error-handling.md):

1. **Create workspace shell.** Move existing `Cargo.toml` to root as workspace manifest. Create `crates/makeholiday/` with `Cargo.toml` and move all `src/*` files into `crates/makeholiday/src/`. `cargo test` passes (no behavior change).
2. **Create `crates/ics-core/` crate** with empty `lib.rs`. Add `ics-core` as a path dep in `crates/makeholiday/Cargo.toml`. Still no behavior change.
3. **Move ICS types and parser** from `crates/makeholiday/src/ics.rs` to `crates/ics-core/src/{event,calendar,parser,error}.rs`. Update use sites in makeholiday to `use ics_core::*;`. Tests pass.
4. **Restructure makeholiday** into the [ADR-009](009-module-layering.md) layers (`presentation`, `application`, `infrastructure`). Lift use cases out of the former `commands.rs`. Introduce `CalendarRepository` and `FileCalendarRepository` per [ADR-011](011-io-boundary-and-repository.md). Introduce `MhError` per [ADR-012](012-error-handling.md), with `Parse(#[from] ics_core::Error)`.
5. **Begin [ADR-001](001-vendor-extension-typing.md) Migration Steps 1–7** entirely within `crates/ics-core/`. The crate boundary now actively prevents leakage.

Each step lands as its own commit (or small PR series) with `cargo test --workspace` passing.

[ADR-014](014-ci-cd-platform.md) workflow files are updated to use `--workspace` flags in the same commit that introduces the workspace structure (step 1).
