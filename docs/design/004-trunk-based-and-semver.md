# 004. Trunk-Based Development and SemVer

- Status: **Accepted** (retroactive)
- Date: 2026-05-27
- Amended by: [ADR-024](024-solo-phase-branching-carve-out.md) — suspends the feature-branch + PR ceremony during the solo phase.

## Context

`makeholiday` is a pre-1.0 project starting with a single maintainer and expected to grow contributors over time. Without an explicit branching/release model, defaults drift — long-lived feature branches accumulate, "release branches" appear without anyone deciding to add them, and breaking changes either become impossible or get smuggled in silently.

[PRD §6 NFR](../PRD.md#6-non-functional-requirements) commits to SemVer post-1.0 and to "breaking changes pre-1.0 documented in the changelog." That implies an upstream branching model and an explicit pre-1.0 contract. [CONTRIBUTING.md](../CONTRIBUTING.md) already states "Trunk-based development" but without elaboration.

## Decision

### Branching

- **Trunk-based development.** `main` is the single long-lived branch and is always shippable.
- **Short-lived feature branches** off `main`, named `<type>/<short-slug>` (e.g., `feat/edit-subcommand`, `fix/parse-date`).
- **Branches merge via small PRs** with at most a few logical changes each. One concern per PR; do not mix refactor with behavior change.
- **No `develop` branch, no per-release branch** pre-1.0.
- Post-1.0, **release branches may be introduced for maintenance backports** of critical fixes; the rule will be revisited in a superseding ADR if needed.

### Versioning

- **SemVer post-1.0.** `MAJOR.MINOR.PATCH` with the usual semantics.
- **Pre-1.0** (`0.x.y`), minor bumps (`0.x → 0.x+1`) MAY introduce breaking changes. Each breaking change is documented in the CHANGELOG with a migration note.
- **Public surface for SemVer purposes** consists of: the CLI invocation contract (subcommands, flags, exit codes), the file format `makeholiday` writes, and (post-crate-split) the library's public API.

### Tagging and releases

- Each release is tagged `vX.Y.Z` on `main`.
- Release notes are derived from the CHANGELOG; no separate release-branch ceremony.
- Hotfixes always land via a PR to `main` and ship in the next patch release. No sideways merging.

## Consequences

### Positive

- New contributors do not need to learn a branching model; "PR off main" is the only path.
- Pre-1.0 we can iterate quickly without negotiating breaking changes through preserved long branches.
- CHANGELOG-based release notes are deterministic given Conventional Commits ([ADR-005](005-conventional-commits.md)).
- Post-1.0 SemVer commitment is unambiguous to integrators.

### Negative

- Requires every PR to be reviewable and mergeable on short notice — large refactors must be split, which is friction we accept (Tidy First in CONTRIBUTING.md aligns).
- No "stable train" for downstream consumers pre-1.0; integrators must pin minor versions if they want stability. Acceptable for the experimental phase.
- Hotfix windows are constrained to "next patch release" with no parallel maintenance branch pre-1.0. Acceptable until usage warrants the maintenance overhead.
- CI must run on every PR to keep `main` shippable. The CI platform itself is the subject of a future ADR.
