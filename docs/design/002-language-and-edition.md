# 002. Language and Edition

- Status: **Accepted** (retroactive)
- Date: 2026-05-27

## Context

The project was started in Rust without an explicit ADR recording the choice. As `makeholiday` grows beyond a personal tool — toward a library that other crates depend on and a CLI installable from crates.io — the language and edition choice becomes a load-bearing assumption that contributors should be able to find without spelunking `Cargo.toml`.

The relevant constraints from [PRD §2 Goals](../PRD.md#2-goals):

- **Typed handling of vendor extensions (Goal 3)** wants a language with a strong, expressive type system.
- **Library reusability (Goal 4)** wants a language whose ecosystem makes "publish as a dependency" routine.
- **Round-trip losslessness (Goal 2)** wants predictable performance and no surprise behavior across platforms.

[PRD §6 NFR](../PRD.md#6-non-functional-requirements) commits to first-class Windows / macOS / Linux support from a single toolchain.

## Decision

- **Language: Rust.**
- **Edition: 2024.**

The choice is recorded retroactively; the codebase has been Rust since inception and `Cargo.toml` declares `edition = "2024"`.

### Why Rust

- Strong static type system supports the typed vendor-extension model (ADR-001) without runtime tax.
- Memory safety + no GC matches a CLI/library distribution model where startup latency and binary size matter.
- Mature ecosystem for our needs: `clap` (CLI), `chrono` (dates), `serde` (JSON), `assert_cmd` / `tempfile` (testing).
- Single toolchain (`rustup` + `cargo`) builds on all three target platforms.
- `cargo install` is a well-trodden install path that satisfies the primary persona's "scriptable, plain-text storage, minimal ceremony" preferences.

### Why edition 2024

- Latest stable Rust edition at project inception.
- Adopting the newest edition by default reduces friction when adding contemporary idioms (let-else, async-fn-in-traits, etc.) without per-feature decision overhead.
- Editions are opt-in language flavors; adopting 2024 commits us to recent rustc but not to any unstable feature.

## Consequences

### Positive

- Contributors only need `rustup` to be productive (see [SETUP.md](../SETUP.md)).
- Cross-platform build is essentially free; CI complexity reduces to "run `cargo test` on three OSes".
- ADR-001's typed vendor model is expressible without acrobatics.
- crates.io publishing path is the default Rust experience.

### Negative

- Excludes contributors not comfortable with Rust. Acceptable for a tool whose primary persona is "CLI-comfortable individual."
- edition 2024 raises the rustc floor — exact MSRV (Minimum Supported Rust Version) is unspecified by this ADR and is the subject of a future ADR.
- Migrating to a different language later requires a new superseding ADR and effectively a rewrite. We accept this as a deliberate one-way door.
