# 009. Module Layering and Dependency Direction

- Status: **Accepted**
- Date: 2026-05-28

## Context

The project is moving from a small flat layout (`main.rs`, `cli.rs`, `commands.rs`, `ics.rs`) toward substantially more code: vendor extension typing ([ADR-001](001-vendor-extension-typing.md)), round-trip handling, and eventually a reusable library crate (per [PRD §2 Goal 4](../PRD.md#2-goals)).

The user-level CLAUDE.md `第4原則` requires "separation of control flow and dependency direction; dependencies point toward higher-level policy." [PRD §2 Goal 4](../PRD.md#2-goals) requires the ICS handling core to be consumable as an independent crate.

Today the dependency graph already has weak signs of drift: `commands.rs` calls `cli::parse_date` (application reaches into presentation), and `cli.rs` knows about domain `BusyStatus`/`EventClass` enums. Without a declared layering rule, this drift will accelerate as code grows, and the future library-crate split will be expensive.

## Decision

Adopt a **Clean Architecture 4-layer** structure with verbatim layer naming and the dependency rules stated below.

### Layers and module paths

```
src/
  presentation/      // CLI args parsing, prompts, output formatting
    mod.rs
    cli.rs           // clap Cli/Commands definitions
    prompt.rs        // interactive stdin/stderr prompts
    output.rs        // human-readable and JSON output formatters
  application/       // use cases (orchestration, no I/O)
    mod.rs
    ports/           // traits that adapters must implement
      mod.rs
      calendar_repository.rs
    use_cases/
      mod.rs
      init_calendar.rs
      add_event.rs
      list_events.rs
      remove_event.rs
  domain/            // pure types and ICS rules
    mod.rs
    event.rs         // VEvent, BusyStatus, EventClass, Transp
    calendar.rs      // VCalendar
    profile/         // vendor extension types (ADR-001)
      mod.rs
      microsoft.rs
      google.rs
      icloud.rs
      makeholiday.rs
    parser.rs        // parse_events, format_calendar (pure functions over text)
  infrastructure/    // adapters: implementations of application::ports
    mod.rs
    file_calendar_repository.rs
  error.rs           // cross-cutting: MhError, Result<T> alias
  main.rs            // Composition Root: wires concrete adapters into use cases
```

### Layer responsibilities

| Layer | What lives here | What does *not* |
|---|---|---|
| `presentation` | clap definitions, prompt I/O, output formatters (text/JSON), CLI-flavored enum conversions | file I/O, business rules, ICS parsing |
| `application` | Use case functions; ports (traits) that adapters must satisfy; orchestration logic that combines domain calls and repository calls | concrete `std::fs` calls, clap, formatting |
| `domain` | Pure data types (`VEvent`, `VCalendar`, vendor profile types), ICS text parsers/formatters as pure functions, invariants and validation | `std::fs`, clap, prompts |
| `infrastructure` | Concrete implementations of `application::ports` traits (e.g., `FileCalendarRepository`) | use case orchestration, presentation concerns |
| `error` | `MhError` enum, `Result<T>` alias, error constructors | anything domain-specific that would create circular dependency |
| `main` | Composition Root: parse args, instantiate concrete adapters, inject into use cases, format/emit result | business logic, formatting decisions |

### Dependency direction

**Allowed:**

```
main          → presentation, application, infrastructure, domain, error
presentation  → application::use_cases (call), domain (read-only types), error
application   → application::ports (define traits), domain (use rich types), error
infrastructure→ application::ports (impl), domain, error
domain        → error
error         → (nothing internal)
```

**Forbidden:**

- `domain` → anything other than `error`
- `application` → `infrastructure` (concrete types)
- `application` → `presentation`
- `presentation` → `infrastructure`
- Anything → `main`
- Cycles of any kind

`presentation` may depend on `domain` for read-only enum conversions (e.g., `CliBusyStatus::to_busystatus()`), but must not import `infrastructure` or call `std::fs`.

### Dependency injection

- **Generic monomorphization** for repository injection:
  ```rust
  pub fn add_event<R: CalendarRepository>(repo: &R, args: AddArgs) -> Result<VEvent>
  ```
- No `dyn Trait` for repository injection at this stage. Trait objects may be introduced later if the use case set grows to require runtime polymorphism (e.g., a registry of use cases). That move would require its own follow-up ADR.

### Composition Root

- `main.rs` is the **Composition Root**. It is the only place that simultaneously knows about `presentation`, `application`, `infrastructure`, and `domain`.
- `main.rs` responsibilities, in order:
  1. Parse CLI arguments via `presentation::cli`.
  2. Construct concrete adapters from `infrastructure` (e.g., `FileCalendarRepository::new(&args.file)`).
  3. Dispatch to the matching `application::use_cases::*` function, passing the adapter.
  4. Emit the result via `presentation::output`, mapping `MhError` to exit code 1.

- `main.rs` contains no business logic; replacing the file with a test harness or a library-API consumer is the dependency-inversion test.

### Enforcement

- **Convention + code review.** Layering rules are declared here; reviewers verify in PRs.
- **No CI-level enforcement tool** at this stage (`cargo-modules` / `cargo-deny` etc.). Adding one is a future tooling decision.
- **No workspace split** at this stage. The single-crate layout is retained because `pub(crate)` visibility across layers is currently more useful than physical separation, and the [ADR-001](001-vendor-extension-typing.md) migration steps will be easier within one crate. A future crate-split ADR will revisit this and decide when the workspace split happens.

### Error type placement

- `MhError` lives at `src/error.rs` (crate root). Every layer may `use crate::error::{MhError, Result};` because errors are pure data with no behavior that creates a back-edge.
- The exact `MhError` shape is decided by a separate error-handling ADR (currently deferred — see Task #22).

## Consequences

### Positive

- Layer boundaries are visible in the directory tree; new contributors and reviewers can determine "which layer am I touching?" by file path alone.
- The Composition Root pattern makes the library-crate split (PRD Goal 4) mechanical: lift `domain`, `application`, `error`, and `infrastructure` into a library crate; keep `presentation` and `main` in the binary crate.
- Repository abstraction via `application::ports::CalendarRepository` enables unit testing of use cases without touching the filesystem.
- Pure `domain` (no I/O) supports the future library-crate consumer who wants to use ICS types without inheriting our file I/O conventions.
- Generic DI is zero-cost at runtime and idiomatic Rust.

### Negative

- More directories and files than the previous flat layout. Acceptable: file count grows linearly with feature count anyway; layering makes the growth navigable.
- Refactoring the existing `cli.rs` / `commands.rs` / `ics.rs` into the new structure is a non-trivial migration. Treat it as Tidy First work landed in its own commits before any new feature work.
- Convention-based enforcement depends on reviewer vigilance. Acceptable at our scale; revisit if rule violations slip through.
- Some short-term verbosity (longer use paths) in exchange for long-term clarity. Acceptable; IDE auto-import handles the typing.

### Migration

The current code maps to the new structure roughly as:

| Current | New |
|---|---|
| `src/main.rs` (dispatch + arg parsing) | `src/main.rs` (Composition Root only) + `presentation::cli` |
| `src/cli.rs` (clap defs, enum converters, parse_date) | `presentation::cli` (clap defs) + `presentation::prompt` (date parsing helpers) |
| `src/commands.rs` (use case orchestration + fs I/O) | `application::use_cases::*` (orchestration only) + `infrastructure::FileCalendarRepository` (fs I/O) |
| `src/ics.rs` (types + parser/formatter) | `domain::event`, `domain::calendar`, `domain::parser` |

The migration is non-trivial and should land as its own Tidy First PR, **before** the ADR-001 type-shape changes begin. Order:

1. Land this ADR (Accepted).
2. Restructure into the new layout, moving existing code with minimal logic change. All tests pass.
3. Begin ADR-001 Migration Step 1 (`RawProperty` + `VEvent.unknown`) on the new layout.

This ordering avoids doing two large changes (layering + typing) in one PR.
