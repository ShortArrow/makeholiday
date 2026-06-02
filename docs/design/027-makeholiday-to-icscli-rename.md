# 027. `makeholiday` → `icscli` Rename

- Status: **Accepted**
- Date: 2026-06-02
- Related: [ADR-017](017-workspace-and-ics-core-crate.md) (workspace + `ics-core` split), [ADR-025](025-lazyics-project-definition.md) (lazyics), [ADR-026](026-icslint-project-definition.md) (icslint)
- Affects: [ADR-004](004-trunk-based-and-semver.md) (SemVer CLI-surface freeze applies to the renamed binary), [ADR-009](009-module-layering.md) / [ADR-010](010-lib-and-main-separation.md) / [ADR-011](011-io-boundary-and-repository.md) / [ADR-012](012-error-handling.md) (layer / type names mentioning `makeholiday` re-anchored on `icscli`)

## Context

[ADR-017](017-workspace-and-ics-core-crate.md) named the CLI crate `makeholiday`. The name originated from the project's first deliverable — generating a year of Japanese national holidays as a `.ics` file — and was kept when the scope was generalised in 2026-Q1 to "lossless typed ICS calendar editor."

In the meantime [ADR-025](025-lazyics-project-definition.md) (lazyics, TUI) and [ADR-026](026-icslint-project-definition.md) (icslint, linter) committed the project to shipping a multi-tool ICS ecosystem at v0.2.0:

| Binary | Role | Crate |
|---|---|---|
| `ics-core` | Typed ICS lingua franca library | shared lib |
| `icscli` *(was `makeholiday`)* | Local file CLI editor | this ADR |
| `lazyics` | Interactive TUI | ADR-025 |
| `icslint` | RFC 5545 + vendor hygiene linter | ADR-026 |

Three out of four tools follow the `ics*` prefix convention. `makeholiday` is the only outlier, and it advertises its original holiday-generator scope rather than its current "general ICS CLI" role. The four crate names were reserved on crates.io on 2026-06-02 (`ics-core`, `icscli`, `icslint`, `lazyics`) precisely to make this rename mechanical.

The rename is decided **before** lazyics implementation begins so that the in-tree TUI never sees the old name in its `Cargo.toml`, imports, or docs. Reaching v0.2.0 with `lazyics → makeholiday` as a library dependency would be a permanent embarrassment in the dependency graph; doing the rename first costs one mechanical pass and produces a coherent ecosystem name set.

## Decision

### Rename the CLI crate and binary

- Workspace member: `crates/makeholiday/` → `crates/icscli/`.
- Cargo package name: `makeholiday` → `icscli`.
- Binary name: `makeholiday` → `icscli`.
- Library crate import path: `use makeholiday::…` → `use icscli::…` (the crate still exposes `lib.rs` per [ADR-010](010-lib-and-main-separation.md), so the new library name is `icscli`).
- Error type: `MhError` → `IcsError`, `mherror` references in docs / comments updated accordingly.

The renamed binary keeps every CLI flag, subcommand, exit code, and output policy unchanged. This is a brand rename, **not** a CLI surface change. [ADR-015](015-diagnostic-output.md) / [ADR-020](020-cli-subcommand-policy.md) / [ADR-021](021-vtodo-scope.md) / [ADR-023](023-no-convert-subcommand.md) carry over verbatim.

### `X-MAKEHOLIDAY-*` property prefix is renamed too

The vendor X-property prefix introduced by [ADR-001](001-vendor-extension-typing.md) Rule 6 (currently `X-MAKEHOLIDAY-ICON`) is renamed to **`X-ICSCLI-ICON`**.

Rationale: there are no production v0.1.x users to break round-trip with (pre-1.0, no installs of record). Pre-1.0 coherence beats backward compatibility (`anti_adhoc_over_backcompat` principle). Leaving one fossil instance of `MAKEHOLIDAY` inside calendar files exported by v0.2.0+ would be an avoidable archaeology artifact for every future reader.

The new prefix is `X-ICSCLI-` (not `X-ICS-CORE-`) because the icon-painting feature lives in the CLI crate per [ADR-001](001-vendor-extension-typing.md) Rule 6 and [ADR-017](017-workspace-and-ics-core-crate.md). `ics-core` ships no built-in CLI-specific X-* handling.

Round-trip behavior for inbound `X-MAKEHOLIDAY-ICON` from old files: nothing special. The property falls into `VEvent.unknown` via the standard prefix-unmatched path (ADR-001 Rule 5). v0.2.0 will not re-emit it as `X-MAKEHOLIDAY-ICON` (the typed icon writer always emits `X-ICSCLI-ICON`); any preserved `X-MAKEHOLIDAY-*` lines in `unknown` are formatted back verbatim by the round-trip path, which is the same behavior `ics-core` already gives every other unknown X-property. No migration tool needed.

### Timing: v0.2.0 development cycle, ahead of lazyics

- The rename lands during the v0.2.0 development cycle, not as a v0.1.x point release. v0.1.x users got `makeholiday`; v0.2.0 users get `icscli`.
- Inside the v0.2.0 cycle, the rename happens **before lazyics scaffolding starts** (ADR-025 implementation) so lazyics's first `Cargo.toml` already depends on `icscli`.
- icslint v0.2.0 release (ADR-026) is unaffected — icslint depends on `ics-core`, not `makeholiday` / `icscli`.

### crates.io publication path

- `icscli 0.0.0` is already published as a placeholder. The first real release will be `icscli 0.2.0`.
- The `makeholiday` crate name has **never been published** to crates.io. Therefore the rename has zero crates.io migration cost: no yank, no final-deprecation publish, no `[package] name = "makeholiday"` redirect crate, no Cargo manifest backward-compat shim. The four reserved placeholders on crates.io are already `ics-core` / `icscli` / `icslint` / `lazyics`; `makeholiday` was deliberately not reserved at name-reservation time (2026-06-02 morning) specifically because this rename was already on the roadmap.
- Pre-1.0, no installed-user base ([[feedback-anti-adhoc-over-backcompat]] principle). v0.1.0 release artifacts on GitHub Releases stay accessible by tag; that is the entire migration story for any hypothetical existing user.

### GitHub repository name: keep `ShortArrow/makeholiday`

The repo itself is **not** renamed in this ADR. The CLI binary is `icscli`; the repo URL stays `https://github.com/ShortArrow/makeholiday`. Rationale:

- **crates.io Trusted Publishing binds `(repo_owner, repo_name, workflow_file)`**. GitHub's automatic redirect for renamed repositories does **not** translate the OIDC `repository` claim. Renaming the repo would invalidate the Trusted Publisher entries for all four placeholder crates (`ics-core`, `icscli`, `icslint`, `lazyics`) and require manual re-registration on crates.io — for zero functional gain.
- Every `Cargo.toml`'s `repository = "https://github.com/ShortArrow/makeholiday"` field would need updating across the workspace. GitHub redirect works, but `cargo publish` records the exact string into the registry index; the stale value would persist on crates.io until the next publish.
- SLSA build provenance subjects baked into `release.yml` reference the repo path; renaming would change the provenance subject URI.
- The mismatch between repo name and primary binary is cosmetic and survivable. `crates/icscli/` lives inside the `makeholiday` workspace; the README clearly labels the binary.
- If/when [ADR-017](017-workspace-and-ics-core-crate.md) `ics-core` repo split happens, the leftover host repo can be renamed at that point as part of the split ADR, in a single coordinated Trusted Publisher re-registration sweep. Not before.

## Migration

This rename is a single sweep over the workspace, executed in one PR / commit series. Tests must remain green at each commit.

1. **Land this ADR** (`docs/design/027-makeholiday-to-icscli-rename.md`).
2. **`git mv crates/makeholiday crates/icscli`** — preserves history.
3. **Cargo manifests**
   - `crates/icscli/Cargo.toml`: `[package].name = "icscli"`, add `[[bin]] name = "icscli"` if not already explicit.
   - Workspace root `Cargo.toml`: `[workspace.package].repository` stays `github.com/ShortArrow/makeholiday` (repo not renamed — see §"GitHub repository name").
3. **Rust sources**
   - `use makeholiday::*` / `use makeholiday::application::*` → `use icscli::*` etc.
   - `MhError` → `IcsError`, file `crates/icscli/src/error.rs` doc comments updated.
   - Doc comments / module docs / rustdoc examples sweep.
4. **Tests**
   - Integration tests in `crates/icscli/tests/` use `assert_cmd::Command::cargo_bin("makeholiday")` — update to `"icscli"`.
   - Snapshot outputs that include the binary name (`makeholiday --help` style) — re-record after running.
5. **Docs**
   - `docs/PRD.md`, `docs/PRD.jp.md`: §1 Overview, §5 CLI Surface, §9 Roadmap — update the CLI binary name. Keep "makeholiday" only when referring to v0.1.x historically.
   - `docs/README.md`, `docs/README.jp.md`, `README.md`, `README.jp.md`: install commands, usage examples.
   - `docs/USAGE.md`, `docs/USAGE.jp.md`, `docs/SETUP.md`, `docs/SETUP.jp.md`: any `makeholiday <subcommand>` invocation.
   - `docs/CHANGELOG.jp.md` (and CHANGELOG.md if present): `[Unreleased — v0.2.0 track]` gets a top-line "Renamed CLI binary `makeholiday` → `icscli` (ADR-027)" entry.
   - `docs/CONTRIBUTING.md`, `docs/CONTRIBUTING.jp.md`: any path or binary-name reference.
6. **Related ADRs**
   - ADR-017 §"Workspace structure" diagram + §"`makeholiday` crate scope" header + §"Naming" subsection updated; add a `Superseded by ADR-027 for the CLI crate name` note. The `ics-core` decisions in ADR-017 stand unchanged.
   - ADR-025 §"Brand and distribution" lazyics dependency reference: `makeholiday` library → `icscli` library.
   - ADR-026 §"Dependencies" / examples: icslint does **not** depend on `makeholiday`, so most of ADR-026 is unaffected; only stray mentions in the prose update.
   - ADR-009 / ADR-010 / ADR-011 / ADR-012: any prose that names `makeholiday::` paths or `MhError` is updated. The architectural decisions themselves are unchanged.
7. **Memory sweep** — `memory/*.md` `makeholiday` references updated to `icscli` except where the entry is historical (e.g., "v0.1.0 shipped as `makeholiday`" stays).
8. **`cargo test --workspace --locked` / `cargo clippy --workspace --all-targets` / `cargo fmt --check`** all green.
9. **Conventional Commits** ([ADR-005](005-conventional-commits.md)): `refactor(workspace): rename makeholiday CLI to icscli (ADR-027)` — single commit covering crate + sources + tests + docs + ADRs. Memory sweep can ride along or be a separate commit since memory is outside the workspace.

## Consequences

### Positive

- The v0.2.0 ecosystem (`ics-core` / `icscli` / `icslint` / `lazyics`) has a coherent name prefix. New users encountering one tool can predict the others.
- lazyics is implemented against `icscli` from day one. No follow-up "lazyics rename" PR is ever needed.
- The brand inversion ("ICS lingua franca" library underneath every tool) is now visible from the CLI binary's name down, not just at the library layer.
- Discoverability on crates.io improves: a search for "ics" finds the whole family.

### Negative

- One-time mechanical churn: ~70 files touched in the rename PR.
- v0.1.x users typing `makeholiday foo` will get "command not found" after upgrading. Acceptable — v0.1.x release notes plus the v0.2.0 CHANGELOG entry are the migration document. No installation count is large enough to justify a compatibility shim.
- v0.1.x calendar files containing `X-MAKEHOLIDAY-ICON` will not have that property re-emitted under the typed icon writer in v0.2.0 — the line survives via the `VEvent.unknown` raw round-trip path but loses typed icon semantics. Acceptable per pre-1.0 / no-installed-users assumption above.
- The repo itself remains `ShortArrow/makeholiday`. See §"GitHub repository name" above for the rationale and the conditions under which a future repo rename would be coordinated.

### Alternatives considered

- **Keep `makeholiday` for v0.2.0, rename later.** Rejected. Every day `lazyics → makeholiday` exists in code is a day the rename gets harder. The ADR-024 carve-out (solo phase, direct-to-`main`) makes the rename almost free now.
- **Rename to `ics`.** Rejected. The bare `ics` name on crates.io is taken (creator-only, last updated 2022-09 — see [ADR-017](017-workspace-and-ics-core-crate.md) §Naming). Pre-emption is not in scope.
- **Rename to `icsedit` or `ics-edit`.** Rejected. The `lazy*` / `*lint` / `*cli` conventional suffix family is more discoverable than a verb-based name. `icscli` reads as "the CLI of the ics family."
- **Subcommand-bundle into `ics` umbrella binary** (`ics lint`, `ics tui`, `ics edit`). Rejected previously in [ADR-025](025-lazyics-project-definition.md) and [ADR-026](026-icslint-project-definition.md) — each tool has different dependency weight and different runtime model; bundling forces every install to pay for ratatui + linter rules. Same arguments apply here.
