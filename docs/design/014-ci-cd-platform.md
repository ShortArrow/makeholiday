# 014. CI/CD Platform and Matrix

- Status: **Accepted**
- Date: 2026-05-28

## Context

Several earlier ADRs imply CI obligations without specifying *how* CI runs:

- [ADR-006](006-testing-strategy.md): `cargo test` is the test command; integration tests live in `tests/cli.rs`.
- [ADR-008](008-msrv.md): MSRV (1.85) and stable both must pass; declared without a specific CI platform.
- [ADR-013](013-dependency-policy.md): `cargo deny` / `cargo audit` should run regularly.
- [PRD §6 NFR](../PRD.md#6-non-functional-requirements): first-class Win / macOS / Linux support.

The repository is hosted on GitHub. This ADR pins the CI/CD platform, the workflow file layout, the matrix shape, and the release pipeline.

The release pipeline follows the same shape as [`github.com/ShortArrow/runex`](https://github.com/ShortArrow/runex)'s `release.yml`: tag push triggers an automated test → build → publish flow, scaled down to this project's needs.

## Decision

### Platform

**GitHub Actions**, hosted runners. No self-hosted runners; no third-party CI.

### Workflow files

```
.github/
  workflows/
    ci.yml       # PR + push-to-main: test matrix + clippy + fmt + cargo deny
    release.yml  # Tag push (v*): test → build matrix → GitHub Release with provenance
    audit.yml    # Weekly scheduled: cargo deny advisories re-check
```

### ci.yml — test matrix

Triggers: `pull_request` against `main`; `push` to `main`.

| Job | OS × toolchain | Purpose |
|---|---|---|
| Test | `{ubuntu, windows, macos}-latest` × `{1.85, stable}` | 6 cells; `cargo test --locked` on each |
| Clippy | `ubuntu-latest` × `stable` | `cargo clippy --all-targets -- -D warnings` |
| Format | `ubuntu-latest` × `stable` | `cargo fmt --check` |
| Deny | `ubuntu-latest` × `stable` | `cargo deny check` (license + advisories + bans) |

`cargo doc` is **not** part of CI at this scale. `cargo audit` is folded into `cargo deny check advisories` to avoid two tools doing the same work.

`fail-fast: false` on the 6-cell test matrix so all combinations report independently.

### release.yml — semi-automatic release pipeline

Triggers: `push` to tags matching `v*`. **No `workflow_dispatch`** — the manual step is the tag push.

Jobs, in order (each gates the next via `needs:`):

1. **Test** — re-runs `cargo test --workspace --locked` on `{ubuntu, windows, macos}-latest` × `stable` only. Gates the release behind passing tests on every native platform we ship a binary for. Does not re-run MSRV (CI already covered that on the bump commit).
2. **Build** — matrix of 5 targets:

   | Target | Runner | Archive |
   |---|---|---|
   | `x86_64-pc-windows-msvc` | `windows-latest` | `zip` |
   | `x86_64-unknown-linux-gnu` | `ubuntu-latest` | `tar.gz` |
   | `aarch64-unknown-linux-gnu` | `ubuntu-latest` | `tar.gz` (cross via `gcc-aarch64-linux-gnu`) |
   | `x86_64-apple-darwin` | `macos-latest` | `tar.gz` |
   | `aarch64-apple-darwin` | `macos-latest` | `tar.gz` |

   Android targets are **out of scope** (calendar CLI usage on mobile is not a current persona).

3. **Release** — downloads all artifacts, runs `actions/attest-build-provenance` to publish SLSA provenance for every binary, then `gh release create "$TAG" --generate-notes` to mint the GitHub Release with binaries attached.

**No `publish-crates` job yet.** [ADR-010](010-lib-and-main-separation.md) intentionally keeps the library public surface minimal pre-1.0. crates.io publishing waits for a future ADR that decides the public surface is ready. When that ADR lands, the publish-crates job is added following the runex pattern (OIDC Trusted Publishing via `rust-lang/crates-io-auth-action`, no stored tokens).

### audit.yml — weekly scheduled re-check

Triggers: `schedule` (Monday 08:00 UTC) and `workflow_dispatch` (for manual reruns).

Runs `cargo deny check advisories` against the current `Cargo.lock`. Picks up newly disclosed RUSTSEC advisories on transitive deps without waiting for a PR. On failure, the workflow surfaces it via the standard GitHub Actions notification path.

### Action version pinning

- All third-party actions are pinned to **commit SHA**, not floating tag (`v6`, `latest`). The tag is preserved as a comment for human readability:

  ```yaml
  - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
  ```

- SHA pinning prevents a compromised tag from substituting malicious action code on the next CI run. Updates happen via explicit PR; Dependabot may be enabled later to automate the SHA bumps.
- The Rust toolchain action (`dtolnay/rust-toolchain`) is also SHA-pinned; the toolchain version it installs is referenced by the `with:` block.

### `--locked` everywhere

- `cargo test --locked`, `cargo build --release --locked`. Reproducible from `Cargo.lock`.
- `Cargo.lock` is committed (per [ADR-013](013-dependency-policy.md)) — the binary half of the dual-target package owns the lockfile.

### `cargo deny` configuration

A `deny.toml` at the repo root configures:

- **Licenses**: allow MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2024, Zlib. Deny GPL/AGPL/LGPL family (matches [ADR-003](003-dual-license.md) and [ADR-013](013-dependency-policy.md)).
- **Advisories**: deny `unmaintained`, `vulnerability`, `unsound`.
- **Bans**: empty initially; populated when a specific crate needs to be banned.

### Cache strategy

Use `actions/cache@<sha>` keyed on `Cargo.lock` for `~/.cargo/registry/cache` and `target/`. Skipped on release builds (we want clean builds for shipped artifacts).

## Consequences

### Positive

- Every PR gets the full MSRV-and-stable matrix on three OSes — [ADR-006](006-testing-strategy.md) and [ADR-008](008-msrv.md) become mechanically enforced rather than aspirational.
- The release pipeline is "push a tag, get binaries with SLSA provenance attached." Manual cost is one `git push origin v0.X.Y` per release; no per-release artifact wrangling.
- SHA-pinned actions remove a quiet supply-chain hole (compromised tag → poisoned CI).
- `cargo deny` in `ci.yml` + the weekly `audit.yml` together catch new advisories within a week of disclosure without manual intervention.
- The structure mirrors the runex pipeline, so contributors moving between the two repos see a consistent shape.

### Negative

- Five build targets × shared dependency graph means ~10 minutes of release-pipeline wall time per release. Acceptable for a non-frequent-release tool.
- SHA-pinning bumps are manual unless Dependabot is enabled. We accept the friction in exchange for the security floor.
- `cargo deny` setup requires a `deny.toml` to live in the repo and stay current; it is one more thing to maintain. We accept it because the license/advisory enforcement is non-negotiable.
- No crates.io auto-publish yet. Manual `cargo publish` is required if we publish pre-1.0 (we don't intend to per [ADR-010](010-lib-and-main-separation.md)).

### Migration

1. Add `.github/workflows/ci.yml` with the test matrix + clippy + fmt + deny.
2. Add `deny.toml` with license/advisory/bans configuration.
3. Add `.github/workflows/audit.yml` with the weekly schedule.
4. Add `.github/workflows/release.yml` with the tag-push pipeline.
5. Verify on a throwaway tag (e.g., `v0.0.0-test`) that the release pipeline produces artifacts; delete the test release afterward.
6. Lift the existing local-only test discipline into the CI requirement — `cargo test --locked` must pass before merge.

Each workflow file is its own commit; the `deny.toml` lands with `ci.yml`.
