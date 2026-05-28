# 011. I/O Boundary and Repository Pattern

- Status: **Accepted**
- Date: 2026-05-28

## Context

[ADR-009](009-module-layering.md) declared `application::ports::CalendarRepository` as the trait that bridges the application layer and concrete I/O. [ADR-010](010-lib-and-main-separation.md) placed `infrastructure::FileCalendarRepository` as the concrete implementation and `main.rs` as the Composition Root that injects it.

This ADR pins the **shape of the trait**, the **atomicity guarantees** for writes, and the **error mapping rules** at the I/O boundary. Without these, infrastructure could leak `std::io::Error` upward, atomic write expectations could drift, and use case authors would have no contract to program against.

Current state (`src/commands.rs`): every command reads the entire ICS file into memory, mutates the parsed result, and writes the whole file back via `fs::write`. Writes are not atomic — a process interruption during write can leave a half-written file. There is no abstraction; tests use real temp files.

## Decision

### Trait shape

```rust
// src/application/ports/calendar_repository.rs
pub trait CalendarRepository {
    /// Create a new empty calendar. Fails with MhError::AlreadyExists
    /// if the target already exists.
    fn create(&self) -> Result<()>;

    /// Load the calendar from the underlying store.
    fn load(&self) -> Result<VCalendar>;

    /// Atomically replace the stored calendar with `calendar`.
    fn save(&self, calendar: &VCalendar) -> Result<()>;

    /// True if the underlying store has a calendar to load.
    fn exists(&self) -> bool;
}
```

- **File-level granularity.** `load` returns the whole `VCalendar`; `save` replaces it whole. Mirrors the current pattern; matches ADR-001's typed model where `VCalendar` is the unit of preservation.
- Methods are non-generic (object-safe is irrelevant since [ADR-009](009-module-layering.md) Q4 settled on generic DI; we never need `dyn CalendarRepository`).
- Repository instances own their identity (e.g., file path) — passed at construction, not per call.

### `FileCalendarRepository`

```rust
// src/infrastructure/file_calendar_repository.rs
pub struct FileCalendarRepository {
    path: PathBuf,
}

impl FileCalendarRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self { ... }
}

impl CalendarRepository for FileCalendarRepository {
    fn create(&self) -> Result<()> { ... }  // tempfile + rename, fails if exists
    fn load(&self) -> Result<VCalendar> { ... }
    fn save(&self, cal: &VCalendar) -> Result<()> { ... }  // tempfile + rename
    fn exists(&self) -> bool { self.path.exists() }
}
```

### Atomic writes

- All writes go through `tempfile::NamedTempFile` in the same directory as the target, then `persist()` to the final path. This makes the write **atomic at the filesystem level** on all supported platforms (Win/macOS/Linux).
- Half-written `.ics` files are no longer possible from a clean process abort. They remain possible from filesystem-level corruption, which is outside our scope.
- `tempfile` becomes a runtime dependency (currently dev-only). This is the first added runtime dep since the project's docs scaffold; the addition is justified by atomicity being a non-negotiable correctness property for a CLI that edits user files. The decision to add a dep is itself recorded here; future dep additions follow a Dependency Policy ADR (deferred — Task #23).

### Error mapping at the I/O boundary

- Inside `FileCalendarRepository`, all `std::io::Error`, `tempfile::PersistError`, and parser-layer errors are mapped to `MhError` variants:
  - File I/O failures → `MhError::Io { path: self.path.clone(), source: io::Error }`
  - Parser errors → `MhError::Parse { line, message, property }` (propagated from `domain::parser`)
  - `create()` on existing file → `MhError::AlreadyExists { path: self.path.clone() }`
- The application layer never sees raw `std::io::Error`. Use cases program against `MhError` only.
- The exact `MhError` shape is finalized in the deferred Error Handling Strategy ADR (Task #22); this ADR commits only to the boundary rule "infrastructure maps to `MhError`."

### Path stored in repository, not passed per call

- `FileCalendarRepository::new(path)` captures the path. Subsequent `load` / `save` / `create` / `exists` calls use the captured path.
- Use cases that need a different file create a different repository instance. This matches the Composition Root pattern: `main.rs` instantiates one repository per `--file` argument.

### In-memory repository for testing

- An `InMemoryCalendarRepository` is provided **for tests** (not as production code). Lives at `src/infrastructure/in_memory_calendar_repository.rs` behind `#[cfg(any(test, feature = "test-support"))]` if we later want to expose it to downstream integration tests; until then plain `#[cfg(test)]`.
- Allows use case unit tests to assert calendar mutations without touching disk. Faster than real-file tests, and decouples use case correctness from filesystem behavior.

### Concurrent access

- **Out of scope for v0.x.** If two processes edit the same `.ics` file concurrently, behavior is undefined (last write wins; no file locking). This matches the primary persona (one user editing their own calendar) and the existing implementation.
- File locking, advisory locks, or CRDT-style merging may be introduced in a future ADR if the persona evolves.

## Consequences

### Positive

- Use cases program against `CalendarRepository` only; swapping `FileCalendarRepository` for `InMemoryCalendarRepository` in tests is a one-line change in the Composition Root.
- Atomic writes via `tempfile` close a quiet data-loss bug class that the current `fs::write` path has.
- All I/O errors funnel through a single mapping point (`FileCalendarRepository`), making error-message quality a single-file concern (path information is always available).
- The trait surface is small (4 methods) and stable; future use cases compose on top instead of growing the trait.

### Negative

- One additional runtime dependency (`tempfile`). Acceptable; mature, widely used, MIT/Apache-2.0 licensed, satisfies [ADR-003](003-dual-license.md).
- `load → mutate → save` triple in every mutating use case looks verbose compared to "just append a line." Tolerable; the explicitness reveals what each use case actually does.
- File-level granularity means a 10k-event calendar gets fully rewritten on every `add`. Acceptable per [PRD §6 NFR](../PRD.md#6-non-functional-requirements) ("up to ~10,000 events well under a second"). Streaming or partial rewrite is a stretch goal.
- No concurrent-write protection. Accepted; documented as out of scope.

### Migration

This ADR's implementation is part of the same Tidy First restructure that lands [ADR-009](009-module-layering.md) and [ADR-010](010-lib-and-main-separation.md):

1. Create `src/application/ports/calendar_repository.rs` with the trait definition.
2. Create `src/infrastructure/file_calendar_repository.rs` implementing it; consolidate today's `fs::read_to_string` / `fs::write` calls here.
3. Switch all writes to `tempfile + persist`.
4. Add `tempfile` to `[dependencies]` (currently in `[dev-dependencies]` — move it; dev-only deps stay for tests that need them).
5. Refactor use cases (formerly `commands.rs::*`) to accept `&R: CalendarRepository` instead of `&Path`.
6. Add `InMemoryCalendarRepository` under `#[cfg(test)]` and use it in at least one new use-case-level test under `tests/`.

Lands before [ADR-001](001-vendor-extension-typing.md) Migration Step 1.
