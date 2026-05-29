# 024. Solo-Phase Branching Carve-out

- Status: **Accepted**
- Date: 2026-05-29
- Amends: [ADR-004](004-trunk-based-and-semver.md)

## Context

[ADR-004](004-trunk-based-and-semver.md) mandates "short-lived feature branches off `main`, named `<type>/<short-slug>`" with "branches merge via small PRs." The policy assumes a contributor pool large enough that PR review adds value: a second pair of eyes, an audit trail for proposed changes, a place to discuss before merge.

The reality of [project_dev_phase_2026_05_29] is:

- Single maintainer (ShortArrow).
- No external contributors.
- [ADR-014](014-ci-cd-platform.md) CI gate (test matrix, clippy `-D warnings`, fmt, `cargo deny`) runs on every push to `main` and on PRs alike.
- [PRD §6 NFR](../PRD.md#6-non-functional-requirements) commits to a working main with passing CI; that's enforced by the CI, not by the PR ceremony.

During [ADR-017](017-workspace-and-ics-core-crate.md) Migration Steps 1–3, work landed via direct-to-`main` commits rather than feature branches + PRs. The choice was deliberate but undocumented. This ADR records it explicitly and pins a deactivation trigger.

The branching ceremony in ADR-004 is not deleted — it is the policy this project converges to. What this ADR does is define a bounded *solo phase* in which the ceremony is suspended.

## Decision

### Direct-to-`main` commits are permitted during the solo phase.

The full ADR-004 ceremony (feature branch + PR) is **suspended**, with the following invariants still enforced:

- **CI must be green** for the commit before it is considered landed. A push that turns CI red is rolled back with another push, not left alone.
- **Conventional Commits** ([ADR-005](005-conventional-commits.md)) — every commit still follows `feat:` / `fix:` / `chore:` / `refactor:` / `docs:` / `test:`.
- **One concern per commit** — the "one concern per PR" spirit carries over. Refactor commits do not mix in behavior changes; feature commits do not silently restructure unrelated code. Multi-commit work series land as multi-commit pushes, not as one omnibus commit.
- **CHANGELOG `[Unreleased]`** stays current. Each meaningful change gets an entry. The CHANGELOG is the audit-trail substitute for PR descriptions.

### Deactivation trigger

This carve-out **automatically expires**, and the full ADR-004 policy reactivates, the first time **any** of these happens:

1. **`ics-core` is split to its own repository** — see [ADR-017](017-workspace-and-ics-core-crate.md) §Repository split strategy. The split is the explicit "this project graduates from a solo experiment" moment.
2. **An external contributor (anyone other than ShortArrow) opens a PR.** From that PR forward, all subsequent work uses feature branches + PRs. The original ADR-004 ceremony exists in part *to give that contributor a fair review path*; once they exist, the path is on.
3. **A `1.0.0` release is tagged.** Post-1.0 SemVer ([ADR-004](004-trunk-based-and-semver.md)) makes accidental breaking changes much more expensive; the PR ceremony provides a brake.

Whichever trigger fires first ends the solo phase. No follow-up ADR is required — this ADR is self-deprecating.

### Scope of the suspension

The carve-out covers branching/PR mechanics only. Specifically **unchanged** by this ADR:

- Trunk-based development (`main` is the single long-lived branch).
- SemVer post-1.0.
- Pre-1.0 breaking-change permissiveness (CHANGELOG documents each one).
- Tagging convention (`vX.Y.Z` on `main`).
- Hotfix discipline (always a forward roll, no sideways merges).
- All CI / fmt / clippy / deny rules.
- Commit message style.

What this ADR explicitly does *not* permit:

- **`git push --force` to `main`.** Force push is hostile even in solo mode (loses commits irrecoverably, breaks downstream clones). Use `git revert` for mistakes.
- **`--no-verify` skipping hooks.** Hooks exist to catch mistakes; bypassing them is anti-solo (no reviewer to catch what the hook missed).
- **Skipping the CHANGELOG.** The CHANGELOG carries the audit-trail load that PR descriptions normally carry.

## Consequences

### Positive

- Lower iteration friction during the heavy in-progress phase (ADR-017 Migration, ADR-001 Migration, future implementation work). Stops counting "is this small enough for a PR?" against multi-step refactors that naturally span several commits.
- The carve-out is honest documentation: the actual practice as of 2026-05-29 now has an ADR backing it, instead of silently violating ADR-004.
- Self-deprecating: no maintainer attention required to lift the suspension; the triggers do it automatically.
- The deactivation trigger is concrete, not "when it feels right." Less drift.

### Negative

- No self-review forcing function. Mitigation: CI is the gate; pre-commit `cargo fmt && cargo clippy && cargo test` is the personal discipline.
- A contributor reading the repo history sees direct-to-main commits and might assume that's the long-term policy. Mitigation: this ADR + the deactivation triggers are written in the open.
- If a contributor opens a PR mid-large-refactor, the maintainer must immediately switch to feature-branch discipline for *their own* concurrent work. Acceptable: the trigger is the moment when the ceremony genuinely earns its keep.

### Migration

No code changes.

Add a pointer at the top of [ADR-004](004-trunk-based-and-semver.md): "Amended by [ADR-024](024-solo-phase-branching-carve-out.md) for the solo phase."

Update the [Workflow section in CONTRIBUTING.md](../CONTRIBUTING.md#workflow) to summarize: "Direct-to-`main` is permitted during the solo phase per [ADR-024](design/024-solo-phase-branching-carve-out.md). When that phase ends, the full feature-branch + PR flow described above applies."

When the deactivation trigger fires:

1. Add a "Superseded on YYYY-MM-DD: trigger fired (icslint repo split / external PR / 1.0.0 tag)" line to this ADR's header.
2. Remove the corresponding pointer from CONTRIBUTING.md.
3. No further ADR is needed.
