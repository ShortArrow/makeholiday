# 003. Dual Licensing under MIT OR Apache-2.0

- Status: **Accepted** (retroactive)
- Date: 2026-05-27

## Context

The project intends to be published on crates.io and embedded as a library in third-party tooling (per [PRD §2 Goal 4](../PRD.md#2-goals)). License choice affects who can redistribute the binary, who can vendor the library, and under what terms contributions are accepted.

License decisions are effectively irreversible once external contributions accumulate: re-licensing typically requires unanimous consent from every past contributor. Recording the decision early prevents drift and signals expectations to potential contributors.

## Decision

- **License: dual-licensed under MIT OR Apache-2.0**, at the user's option.
- License manifest: `Cargo.toml` declares `license = "MIT OR Apache-2.0"`.
- License texts: `LICENSE-MIT` and `LICENSE-APACHE` live at the repository root.
- Copyright holder: "ShortArrow" (the original author). Year: 2026 onward.
- Contributions are dual-licensed under the same terms (documented in [CONTRIBUTING.md](../CONTRIBUTING.md)).

The choice is recorded retroactively; both license files and the `Cargo.toml` field have been in place since the initial documentation scaffold.

### Why MIT OR Apache-2.0

- **De-facto convention in the Rust ecosystem.** Most major crates (the Rust standard library, `serde`, `tokio`, `clap`, `chrono`, `regex`, …) adopt this combination, so downstream license-compatibility analysis is well-understood and routine.
- **Maximizes compatibility:** MIT is the simplest permissive license; Apache-2.0 adds an explicit patent grant. Users pick whichever fits their downstream license matrix.
- **Friction-free for crates.io publishing.** The `MIT OR Apache-2.0` SPDX expression is the conventional Rust crate license string and is accepted everywhere.
- **Matches PRD §4 secondary persona expectations.** Integrators embedding ICS handling into their own product can do so under whichever license their product uses.

## Consequences

### Positive

- Downstream license compatibility is "the same as everything else in Rust" — minimal cognitive load for integrators.
- Patent grant via Apache-2.0 protects users from contributor patent assertions without requiring all users to accept Apache-2.0's longer text.
- Contribution intake is unambiguous: contributors agree their work is dual-licensed by submitting (documented in CONTRIBUTING.md).

### Negative

- Two license files at the repo root instead of one. Acceptable, as it is the dominant Rust pattern.
- **Switching to a copyleft (GPL family) or proprietary license later is effectively impossible** without rewriting from scratch or obtaining consent from every contributor whose work remains in the codebase. We accept this as a deliberate one-way door.
- Cannot accept contributions that the contributor explicitly licenses under incompatible terms; in practice, contributors implicitly accept the dual license by opening a PR.
