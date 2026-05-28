# 013. Dependency Policy

- Status: **Accepted**
- Date: 2026-05-28

## Context

The project has accumulated runtime dependencies (clap, chrono, uuid, serde, serde_json) without an explicit policy. [ADR-011](011-io-boundary-and-repository.md) adds `tempfile` and [ADR-012](012-error-handling.md) adds `thiserror`. Without a stated rule, future contributors will either over-vet routine additions (slowing work) or under-vet risky ones (license/MSRV/security incidents).

The policy must cover:

- License compatibility with [ADR-003](003-dual-license.md)'s MIT-OR-Apache-2.0.
- MSRV compatibility with [ADR-008](008-msrv.md)'s 1.85.
- Long-term maintenance signal — avoid adopting abandoned crates.
- Minimal surface area — don't pull in transitive dependencies we don't need.
- Discipline against ad-hoc dependency adoption (user CLAUDE.md `第3原則`).

## Decision

### Required checks for every dependency addition

| ID | Check | What it means |
|---|---|---|
| α | **License compatibility** | The crate's license is MIT, Apache-2.0, BSD, ISC, Unicode-DFS, or equivalent permissive license. GPL / AGPL / LGPL / proprietary are **rejected** (incompatible with [ADR-003](003-dual-license.md)). Dual-licensed crates count if one option is compatible. |
| β | **Maintenance signal** | Last release within the past two years; open-issue count not catastrophic; the maintainer or org appears active. A crate that has not shipped a release in three years requires explicit justification. |
| δ | **MSRV compatibility** | The crate's declared `rust-version` is ≤ 1.85 ([ADR-008](008-msrv.md)) **or** the crate's documented MSRV pledge does not raise ours. If adding the crate would force an MSRV bump, that is itself a separate decision (supersedes [ADR-008](008-msrv.md)). |
| ε | **Minimal feature surface** | Only the features actually needed are enabled. `default-features = false` is the starting point if defaults pull in machinery we don't use. |
| η | **Alternatives considered** | The PR explains why `std`, an already-present dep, or a small hand-rolled solution is insufficient. Avoids dependency creep by reflex. |

A PR that adds a new dependency must address α/β/δ/ε/η in its description. This is part of the [CONTRIBUTING.md](../CONTRIBUTING.md) PR template.

### Recommended (not required) checks

| ID | Check |
|---|---|
| γ | crates.io download trend (helpful sanity check, not a gatekeeper — niche but mature crates are fine) |
| ζ | `cargo audit` and `cargo deny` results (CI will enforce this once [ADR-014](#) lands; until then, run locally when in doubt) |
| θ | Compile-time and binary-size impact (relevant only if change is large; CLI tools tolerate moderate overhead) |

### Free-pass list

For the following crates, **α (license) and β (maintenance) are auto-satisfied** — they are de facto Rust ecosystem infrastructure with permissive licenses and active maintenance. The PR still must address **δ (MSRV)**, **ε (features)**, and **η (alternatives)** because those depend on how we use them, not on the crate's intrinsic standing.

- `serde`, `serde_json`, `serde_derive` (data serialization)
- `clap` (CLI parsing — already adopted)
- `chrono` (date/time — already adopted)
- `uuid` (UUID generation — already adopted)
- `tempfile` (atomic-write helper — adopted by [ADR-011](011-io-boundary-and-repository.md))
- `thiserror` (error derive — adopted by [ADR-012](012-error-handling.md))
- `anyhow` (NOT free-pass for adoption — [ADR-012](012-error-handling.md) explicitly rejects it; listed here to clarify that the rejection is policy-level, not licensing/maintenance)
- `regex`, `once_cell`, `log`, `tracing` (mature ecosystem crates likely to be considered in the future)

A crate not on this list is **not forbidden** — it just must justify all five required checks.

### Removal policy

- Dropping a dependency is welcomed and does not require ADR ceremony — it's the reverse of adoption.
- A dropped dep that was central to a previous ADR's decision (e.g., removing `thiserror` would invalidate [ADR-012](012-error-handling.md)) requires that ADR's supersession.

### Version constraints

- Use **major-version constraints** (`"4"` not `"4.5.2"`) for crates that follow SemVer. Cargo's resolver picks the latest compatible version.
- **Lock file (`Cargo.lock`) is committed** for binary crates (library crates per Rust convention do not commit lockfiles; we are dual-target — the binary target wins). [ADR-014](#) will state how CI uses or refreshes the lockfile.
- Yanking events on upstream crates are handled by `cargo update` plus, eventually, automated tooling.

### Dev-dependency policy

- The same checks (α/β/δ/ε/η) apply to `[dev-dependencies]`, but δ (MSRV) **does not** propagate to library consumers since dev-deps are not in the dependency tree of downstream users.
- Test infrastructure crates (`assert_cmd`, `predicates`, `tempfile` in dev-deps context, future `proptest` or `criterion`) are pre-approved subject to the same license check.

## Consequences

### Positive

- Adding a "boring" crate (anything on the free-pass list) requires minimal ceremony — three short bullet points in the PR description.
- Adding a novel crate forces the contributor to think about *why* before *which*, catching dependency creep at the earliest gate.
- License and MSRV compatibility are mechanical checks, not vibes — easy for reviewers to verify.
- A future `cargo audit` / `cargo deny` automation in CI ([ADR-014](#)) fits naturally on top of this policy.

### Negative

- New contributors must read the checklist on their first dep-adding PR. One-time cost.
- The free-pass list is partly subjective and may grow over time. We accept that growth is fine as long as each addition is itself justified at the time the crate is first adopted.
- "Maintenance signal" is judgment-laden. We accept that some abandoned-looking crates may still be fine (Rust standardized many "complete" small crates), and review handles the gray zone.

### Migration

Two cleanups follow this ADR:

1. **Backfill missing PR justifications.** For the deps already in `Cargo.toml` (clap, chrono, uuid, serde, serde_json), no retroactive justification is required since this ADR retroactively grandfathers them via the free-pass list.
2. **Update [CONTRIBUTING.md](../CONTRIBUTING.md)** with a "Dependency Policy" section pointing to this ADR. Future PR templates may surface the α/β/δ/ε/η checklist directly.

The CONTRIBUTING update is a small follow-up commit; this ADR's acceptance is independent of it.
