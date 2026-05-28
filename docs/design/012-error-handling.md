# 012. Error Handling Strategy

- Status: **Accepted**
- Date: 2026-05-28

## Context

Today `src/commands.rs` and `src/ics.rs` return `Result<T, String>`. Every error site builds an ad-hoc `format!(...)` and the only information that survives to `main.rs` is whatever was baked into the format string. Consequences:

- No structured fields. [PRD §6 NFR](../PRD.md#6-non-functional-requirements) asks for "input line and offending property name when parsing ICS" — currently expressed only inside format strings, not as data.
- No type-level categories. Downstream library consumers (PRD Goal 4) cannot `match` on error kinds.
- No source-error chaining. The root cause behind a wrapped error is lost in the formatted string.

[ADR-009](009-module-layering.md) put the error type at `src/error.rs` as a cross-cutting module. [ADR-010](010-lib-and-main-separation.md) declared it as part of the library's public surface (`pub mod error;`). [ADR-011](011-io-boundary-and-repository.md) declared that the infrastructure layer is the boundary that converts `std::io::Error` and parser errors into our error type. This ADR pins the type itself and the CLI presentation of it.

## Decision

### Error type — `MhError` via `thiserror`

```rust
// src/error.rs
use std::io;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum MhError {
    #[error("I/O error: {}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "parse error at line {line}: {message}{}",
        .property.as_ref().map(|p| format!("\n  Property: {p}")).unwrap_or_default()
    )]
    Parse {
        line: u32,
        message: String,
        property: Option<String>,
    },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("conflicting arguments: {0}")]
    Conflict(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("file already exists: {}", .path.display())]
    AlreadyExists { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, MhError>;
```

- **`thiserror`** for derive-based `std::error::Error` impl. Adds `thiserror` to `[dependencies]` (proc-macro, build-time only at runtime cost).
- **`anyhow` is not used.** The library surface needs `match`-able typed errors (PRD Goal 4); `anyhow::Error` would erase types. The binary layer also does not need `anyhow` because the typed error provides enough context on its own.

### Six categories

| Variant | When |
|---|---|
| `Io { path, source }` | Filesystem failures (read, write, create, rename). `path` is always the user-visible path; `source` is the raw `io::Error`. |
| `Parse { line, message, property }` | ICS parser failures. `line` is the offending input line (1-based). `property` is the property name (UPPERCASE) when identifiable. |
| `InvalidInput(String)` | User-supplied values that fail validation (bad date string, bad index syntax). |
| `Conflict(String)` | Mutually exclusive arguments given simultaneously (e.g., both `--summary` and positional index target on `remove`). |
| `NotFound(String)` | Lookup failures (no event matches the given summary, no event at index N). |
| `AlreadyExists { path }` | `init` attempted on an existing file. |

The six are sufficient for the current command surface and span the categories real `Result<_, String>` sites already implicitly used.

### Boundary rules

- **Infrastructure layer is the conversion point.** `FileCalendarRepository` maps `io::Error` and `tempfile::PersistError` to `MhError::Io { path, source }`. Application and domain code receive only `MhError`.
- **Domain layer** uses `MhError::Parse { line, message, property }` from its parser functions.
- **Application layer** raises `MhError::InvalidInput`, `MhError::Conflict`, `MhError::NotFound`, `MhError::AlreadyExists` — these are the "rules of the use case," not raw I/O.
- **Presentation layer** does not produce `MhError`; it consumes them via `emit_error` (below).

### Result type alias

`crate::error::Result<T>` is the alias used everywhere. Importing via `use crate::error::Result;` shadows `std::result::Result` within the file, which is the Rust ecosystem convention.

### CLI output — source chain expansion

`presentation::output::emit_error`:

```rust
use std::error::Error;
use makeholiday::error::MhError;

pub fn emit_error(e: &MhError) {
    eprintln!("Error: {e}");
    let mut source: Option<&dyn Error> = e.source();
    while let Some(s) = source {
        eprintln!("  Caused by: {s}");
        source = s.source();
    }
}
```

Output example (filesystem permission denied):

```
Error: I/O error: ./calendar.ics
  Caused by: Permission denied (os error 13)
```

Output example (parser failure):

```
Error: parse error at line 42: invalid DTSTART
  Property: DTSTART
```

(The parser variant's `Display` impl folds property name into the top line; there is no additional `Caused by:` line unless the parser wraps a lower-level error in the future.)

### Out of scope (deferred)

- **`--verbose` / `--quiet` flags** for richer or quieter output. Subject to the Diagnostic Output Policy ADR (Task #25, currently ADR-015).
- **Strict mode** that turns "first wins + warning" duplicate-property handling ([ADR-001](001-vendor-extension-typing.md) rule 8) into a hard error. Subject to a future CLI policy ADR.
- **Logging / tracing crates** (`log`, `tracing`). Not introduced here. May be revisited if we need leveled output.

## Consequences

### Positive

- Library consumers can `match err { MhError::Parse { line, .. } => ..., ... }` — satisfies [PRD Goal 4](../PRD.md#2-goals).
- Parser errors carry `line` and `property` as structured fields, satisfying [PRD §6 NFR](../PRD.md#6-non-functional-requirements).
- Source chain expansion gives the user the *root cause* without inventing new fields per error.
- Single error type across all layers — no `From` chains between layer-specific errors. The trade-off is a slightly wider enum but a much smaller code footprint.
- `tempfile::PersistError`'s inner `io::Error` is preserved via `#[source]` on `MhError::Io.source`, so atomicity failures retain the original cause.

### Negative

- `thiserror` added as a runtime dependency (proc-macro at build, zero runtime cost). Acceptable; widely used, MIT/Apache-2.0 licensed. This is the second added runtime dep (after `tempfile`); the formal dependency policy lands in Task #23 / ADR-013.
- One enum for all error categories means contributors must learn the variants. The mapping table above is the contract.
- Adding a new variant is a breaking change for downstream `match` consumers. Pre-1.0 we accept this; post-1.0 we add `#[non_exhaustive]` and document the policy in a future SemVer-stability ADR.

### Migration

Land in this order, each step independently testable:

1. Add `thiserror` to `[dependencies]`. Create `src/error.rs` with the enum and `Result<T>` alias. No call-site changes yet.
2. Switch `domain::parser` (formerly `src/ics.rs`) to return `Result<_, MhError>`. Parser-internal `format!(...)` sites become `MhError::Parse { line, message, property }`.
3. Switch `infrastructure::FileCalendarRepository` (per [ADR-011](011-io-boundary-and-repository.md)) to return `Result<_, MhError>`. All `io::Error` → `MhError::Io { path, source }` mappings live here.
4. Switch `application::use_cases::*` (formerly `src/commands.rs`) to return `Result<_, MhError>`. Validation errors become `MhError::InvalidInput / Conflict / NotFound / AlreadyExists`.
5. Update `src/main.rs` to call `presentation::output::emit_error(&e)` and `std::process::exit(1)`.
6. Update existing tests; add at least one test asserting that `MhError::Parse` carries the expected `line` and `property`.

Lands together with the [ADR-009/010/011](009-module-layering.md) restructure as part of the Tidy First foundation, **before** the [ADR-001](001-vendor-extension-typing.md) typed-model migration begins.
