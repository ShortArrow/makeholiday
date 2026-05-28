# 008. Minimum Supported Rust Version (MSRV)

- Status: **Accepted**
- Date: 2026-05-28

## Context

[ADR-002](002-language-and-edition.md) committed to Rust edition 2024 but deferred the exact Minimum Supported Rust Version (MSRV) to a later ADR. Without a declared MSRV, the project has no machine-checkable contract with downstream consumers about which `rustc` they need, and contributors can unintentionally pull in language or library features that require a newer `rustc` than expected.

[PRD §6 NFR](../PRD.md#6-non-functional-requirements) commits to first-class Win/macOS/Linux support and to "stability" once 1.0 ships. A declared MSRV makes "stability" measurable. [ADR-006](006-testing-strategy.md) makes CI the verification surface for behavioral and infrastructure invariants; MSRV is one such invariant if we want it enforced rather than aspirational.

## Decision

### Declared MSRV

- **`rust-version = "1.85"`** in `Cargo.toml`.
- 1.85 is the `rustc` that stabilized edition 2024; declaring this is the floor implied by [ADR-002](002-language-and-edition.md) and adds no further constraint than already exists.
- MSRV applies to the **main package surface only** (the binary, and post-crate-split the library). `dev-dependencies` may require a newer `rustc` if they need to; that constraint only affects contributors running `cargo test`, not end-users running `cargo install makeholiday`.

### CI verification

- CI runs **two `rustc` tracks** per supported OS: **MSRV (`1.85`)** and **stable** (latest at CI run time).
- Both tracks must pass for a PR to merge. A change that compiles on stable but breaks MSRV is rejected.
- Beta and nightly are **not** in the CI matrix; their cost outweighs the early-warning value at this scale.
- The exact CI workflow definition (GitHub Actions vs alternatives, job matrix shape) is the subject of a future CI/CD ADR; this ADR only commits to *what* must be verified, not *how*.

### Bump policy

- **Motivation-driven.** MSRV is raised only when:
  - a chosen upstream dependency raises its own MSRV beyond ours, OR
  - a `rustc` feature we want to adopt has a clear, recorded benefit (capability we cannot get cheaply at the current MSRV), OR
  - a security advisory or correctness fix demands it.
- **Not raised by schedule, not raised by "stable - N", not raised on a whim.**
- Every MSRV bump is recorded by **superseding this ADR** with a new ADR that names the trigger and the new floor. The superseded ADR's Status changes to `Superseded by ADR-NNN` per [ADR-policy](000-ADR-policy.md).
- MSRV bumps are noted in the CHANGELOG entry for the release that contains them.

### Local toolchain

- The repository **does not ship a `rust-toolchain.toml`** file. Contributors use whatever `rustc` `rustup` provides them, as long as it satisfies MSRV. This keeps the repo lean and lets contributors update independently.

## Consequences

### Positive

- `cargo install makeholiday` fails fast and clearly when the user's `rustc` is too old, instead of producing cryptic compile errors deep in our code.
- The MSRV/stable two-track CI catches "accidentally requires newer `rustc`" mistakes at PR time, not at downstream install time.
- The bump policy gives a clear answer to "should we bump MSRV?" — only when motivated, never just because.
- The motivation requirement aligns with the user CLAUDE.md principles of non-ad-hoc decisions: every bump is an ADR.

### Negative

- CI cost grows (two tracks per OS). At project scale this is negligible; if matrix runtime ever matters, beta/nightly removal already keeps us conservative.
- Contributors who default to nightly may be surprised that the project does not exercise nightly features in CI. We accept this — nightly tracking is a deliberate non-goal here.
- "Motivation-driven" puts judgment on whoever proposes the bump. Mitigated by the ADR-supersession requirement: the case must be made in writing.
- If a transitively-depended-upon crate silently raises its MSRV, our MSRV CI track will catch it on the next `cargo update`, but that may force an unplanned bump conversation. Acceptable — the alternative is silent breakage.
