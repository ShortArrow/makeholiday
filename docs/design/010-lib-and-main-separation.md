# 010. `lib.rs` and `main.rs` Separation

- Status: **Accepted**
- Date: 2026-05-28

## Context

[ADR-009](009-module-layering.md) established the layer structure (`presentation`, `application`, `domain`, `infrastructure`, `error`) and named `main.rs` as the Composition Root. ADR-009 did **not** decide whether a separate `lib.rs` should exist.

Today the project has only `src/main.rs` with `mod cli; mod commands; mod ics;` declared inside it. Consequences:

- The crate is binary-only; nothing is exposed as a library.
- Integration tests (`tests/cli.rs`) can only exercise the CLI surface via `assert_cmd`, never the use cases directly.
- [PRD §2 Goal 4](../PRD.md#2-goals) (library reusability) has no foothold in the codebase.
- [ADR-006](006-testing-strategy.md) `cargo test` covers integration, but use-case-level testing requires either subprocess overhead or in-process exposure.

A future crate-split ADR will eventually move the library into its own workspace member. That move is mechanical only if a `lib.rs` boundary already exists.

## Decision

Introduce **`src/lib.rs`** alongside `src/main.rs` immediately. The repository becomes a dual-target Cargo package (library + binary). Public surface is intentionally minimal at this stage; widening happens in a future ADR.

### `src/lib.rs` shape

```rust
// src/lib.rs
pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;
```

- These four are the library-facing layers (per [ADR-009](009-module-layering.md)).
- `presentation` is **not** exposed from `lib.rs` — it is binary-only (see below).
- Each of the four `pub mod`s contains predominantly `pub(crate)` items. The names that are genuinely public — e.g., `domain::event::VEvent`, `application::use_cases::add_event` — get explicit `pub` markers.
- The library has **no top-level re-exports** at this stage. Consumers write `use makeholiday::application::use_cases::add_event;`. A re-export decision (e.g., `pub use crate::error::MhError;` at the root) is deferred to whichever ADR widens the public surface.
- Doc-comments on `pub` items are encouraged but not yet enforced by tooling. Reviewers verify they exist on the items they would themselves want documented.

### `src/main.rs` shape

```rust
// src/main.rs
mod presentation;   // binary-only module tree

use makeholiday::{
    application::use_cases,
    error::MhError,
    infrastructure::FileCalendarRepository,
};

fn main() {
    let cli = presentation::cli::Cli::parse();
    let repo = FileCalendarRepository::new(&cli.file);
    let result: Result<(), MhError> = match cli.command {
        /* dispatch to use_cases::*, passing &repo */
    };
    if let Err(e) = result {
        presentation::output::emit_error(&e);
        std::process::exit(1);
    }
}
```

- `main.rs` declares its own `mod presentation;` — the directory `src/presentation/` is **owned by the binary crate**, not by the library crate.
- `main.rs` `use`s only what it needs from the library via the `makeholiday::` path.
- `main.rs` is the Composition Root: it instantiates the concrete `FileCalendarRepository` and injects it into use cases.

### Why `presentation` is binary-only

A library consumer who embeds `makeholiday` for ICS handling does not want clap, prompts, or human-readable output formatters. They want the typed domain, the use cases, and the file repository. Keeping `presentation` in the binary crate:

- Cleanly separates "ICS as a library" from "ICS as a CLI tool."
- Lets a future TUI (or any alternative front-end) become its own binary consumer of the same library, without depending on the CLI's `presentation` choices.
- Avoids forcing library consumers to take on a clap dependency.

### `Cargo.toml`

No changes required. Cargo auto-detects:
- `src/lib.rs` → library target named `makeholiday`
- `src/main.rs` → binary target named `makeholiday`

Both targets share the same `[dependencies]`. clap and the other CLI deps technically end up in the library's dep graph, but since the binary owns `presentation` they are only *used* by the binary. A future ADR may move clap into a `[[bin]]`-only dependency once we add `[[bin]]` and `[features]` configuration to gate it.

### Testing implications

[ADR-006](006-testing-strategy.md) gains a second integration test surface:

- `tests/cli.rs` continues to exercise the binary via `assert_cmd` (end-to-end CLI behavior).
- New integration tests in `tests/*.rs` may `use makeholiday::application::use_cases::*;` and call use cases directly with in-memory or temp-file repositories. Faster, no subprocess.

Unit tests in `#[cfg(test)] mod tests` continue to work inside any module.

## Consequences

### Positive

- `lib.rs` exists as a load-bearing boundary now, so the future crate split (PRD Goal 4) is mechanical: move the four library modules into a new crate, leave `main.rs` and `presentation/` behind.
- Use-case-level integration tests become possible without subprocess overhead.
- The dependency graph for library consumers is "domain + application + infrastructure + error" only — no clap, no prompts.
- `presentation` being binary-only signals intent: a future TUI front-end is the same shape as today's CLI front-end (different binary, same library).

### Negative

- One extra file in the source tree (`src/lib.rs`). Trivial.
- A small refactor of `main.rs`: imports change from `crate::` to `makeholiday::`. One-time pain.
- Library consumers technically pull clap into their build via the binary's `[dependencies]` table until we gate it with features. Acceptable until felt; the binary fix is straightforward (add `[features]` and move CLI deps).
- Doc-comment enforcement is convention-only. Acceptable until felt.

### Migration

This ADR's implementation happens together with the [ADR-009](009-module-layering.md) restructure. The combined Tidy First steps:

1. Create the layer directories (`src/{application,domain,infrastructure,presentation}/`) and `src/error.rs`.
2. Move existing code into the new homes with minimal logic change.
3. Add `src/lib.rs` with the four `pub mod` declarations.
4. Rewrite `src/main.rs` to `use makeholiday::*` paths.
5. Verify `cargo test` passes; add at least one new library-API integration test under `tests/` to prove the boundary works.

These all land **before** the [ADR-001](001-vendor-extension-typing.md) type-shape migration begins.
