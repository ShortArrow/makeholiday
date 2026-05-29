# 026. icslint — ICS Lint Tool Project Definition

- Status: **Accepted**
- Date: 2026-05-29
- Related: [ADR-017](017-workspace-and-ics-core-crate.md) (ic-core repo split trigger), [ADR-025](025-lazyics-project-definition.md) (sibling project, lazyics)

## Context

[PRD §9](../PRD.md#9-roadmap) names **icslint** as a v0.2.0 deliverable alongside `ics-core` repo extraction and `lazyics`. [ADR-017](017-workspace-and-ics-core-crate.md) treats "icslint launches and needs an external dependency" as one of two explicit triggers for moving `ics-core` to its own repository. Until this ADR, icslint existed only as a name; this ADR pins its project definition — scope, rule set, distribution, dependencies, and the relationship to the rest of the ecosystem.

icslint exists because the ICS ecosystem in the wild is sloppy. Real-world calendar exports routinely contain:

- properties marked `REQUIRED` by RFC 5545 but missing (`UID`, `DTSTAMP`).
- properties whose cardinality the RFC restricts (e.g., `SUMMARY` allowed at most once per `VEVENT`) but which appear multiple times due to merge artifacts.
- vendor-specific X-properties (`X-MICROSOFT-CDO-BUSYSTATUS`, `X-WR-CALNAME`, …) that work in one client but silently break in others.
- conflicting time anchors — both `DTEND` and `DURATION` present on the same `VEVENT`.
- character escape sequences in `TEXT` fields that some producers double-escape or fail to escape at all.

makeholiday and lazyics together can read most of these files (thanks to [ADR-018](018-parser-philosophy.md) tolerant + lossless parsing), but **reading the data is not the same as flagging the data as risky**. icslint is the lint tool that turns "ics-core parsed it" into actionable diagnostics for calendar authors, integrators, and CI pipelines that consume ICS files from external sources.

## Decision

### Brand and distribution: separate binary `icslint`

`icslint` ships as a **separate binary** named `icslint`, not as a subcommand of `makeholiday` or `lazyics`.

- Installation: `cargo install icslint` (post-publish) or `cargo install --path crates/icslint` during in-tree development.
- Repository while in-tree: `crates/icslint/` in this workspace until [ADR-017](017-workspace-and-ics-core-crate.md) repo split is executed (during the v0.2.0 development cycle — see [§Workspace lifecycle](#workspace-lifecycle)).
- Repository after v0.2.0 split: separate `ShortArrow/icslint` repository consuming `ics-core` via crates.io.

Rejected alternatives:

- **`makeholiday lint` subcommand.** A pure-text linter has no reason to share a binary with a calendar editor. CI users who only need lint enforcement should not pay for makeholiday's editing surface. Cargo install ergonomics also work better with a single-purpose binary that prints to stdout/stderr.
- **`lazyics --lint` mode.** lazyics is interactive and TTY-bound ([ADR-025](025-lazyics-project-definition.md)). Lint is non-interactive and primarily run in CI / pre-commit.

### Initial rule set (v0.2.0 release)

icslint v0.2.0 ships with four rule families. Each rule has a stable string identifier (e.g., `RFC5545/required-uid`) used in output and in suppression directives.

#### RFC 5545 cardinality and required fields (`RFC5545/...`)

Drawn directly from the RFC 5545 §3.6.1 / §3.6.2 / §3.6.5 cardinality tables and [ADR-001](001-vcalendar-vevent-typed-model.md) Rule 8:

| ID | Diagnostic | Severity |
|---|---|---|
| `RFC5545/required-uid` | `VEVENT` / `VTODO` missing `UID` | error |
| `RFC5545/required-dtstamp` | `VEVENT` / `VTODO` missing `DTSTAMP` | error |
| `RFC5545/required-dtstart` | `VEVENT` missing `DTSTART` (RFC: required when no method) | warning |
| `RFC5545/duplicate-summary` | `SUMMARY` appears more than once on a single `VEVENT` | error |
| `RFC5545/duplicate-dtstart` | `DTSTART` appears more than once | error |
| `RFC5545/conflicting-end-and-duration` | both `DTEND` and `DURATION` present | error |
| `RFC5545/end-before-start` | `DTEND` is strictly before `DTSTART` | error |
| `RFC5545/empty-summary` | `SUMMARY` value is the empty string | warning |

#### Vendor extension hygiene (`vendor/...`)

Built on the prefix-based vendor profile pre-reservation introduced in [ADR-001](001-vcalendar-vevent-typed-model.md):

| ID | Diagnostic | Severity |
|---|---|---|
| `vendor/microsoft-only` | property uses `X-MICROSOFT-...` prefix; consumer support outside Outlook is uneven | info |
| `vendor/google-only` | property uses `X-GOOGLE-...` / `X-GOOGLE-CALENDAR-...` prefix | info |
| `vendor/icloud-only` | property uses `X-APPLE-...` / `X-CALENDARSERVER-...` prefix | info |
| `vendor/unrecognized-x` | `X-...` prefix not in any known vendor profile | warning |
| `vendor/typed-clash` | the same logical concept is set via two different vendor extensions (e.g., busy status via Microsoft and iCloud bundles simultaneously) | warning |

These rules are deliberately "info" by default for known vendors — the goal is to surface portability tradeoffs, not to discourage vendor extensions outright.

#### Text encoding (`text/...`)

Built on [ADR-019](019-parser-implementation-strategy.md) Step 2 escape handling:

| ID | Diagnostic | Severity |
|---|---|---|
| `text/unescaped-comma-in-summary` | `,` literal in `SUMMARY` value without backslash | warning |
| `text/unescaped-semicolon-in-summary` | `;` literal in `SUMMARY` value without backslash | warning |
| `text/double-escape` | `\\,` or `\\;` patterns suggesting a producer double-escaped | warning |
| `text/non-utf8-bytes` | source bytes are not valid UTF-8 | error |
| `text/bom` | source begins with a UTF-8 BOM | info |

#### Structure (`structure/...`)

| ID | Diagnostic | Severity |
|---|---|---|
| `structure/unfolded-long-line` | logical line exceeds 75 octets and is not folded ([RFC 5545 §3.1](https://datatracker.ietf.org/doc/html/rfc5545#section-3.1)) | warning |
| `structure/crlf-violation` | line endings use bare `\n` instead of `\r\n` | warning |
| `structure/orphan-end` | `END:` without matching `BEGIN:` | error |
| `structure/empty-calendar` | `VCALENDAR` contains zero components | info |

#### Severity levels

- **error** — RFC 5545 violation that breaks at least one consumer in the wild. Exit code 2.
- **warning** — strongly discouraged but tolerable. Exit code 1 if any warning fired and `-Werror` is set.
- **info** — portability or stylistic note. Never affects exit code.

Out of scope for v0.2.0 rule set:

- RRULE / EXRULE semantic validation. The recurrence model is large; deferred.
- Timezone reference validation (`TZID=Asia/Tokyo` matches a known IANA TZ). Deferred until VTIMEZONE typing lands in v0.3.0.
- Cross-event uniqueness of `UID`. Possible to add, but requires multi-pass evaluation; deferred.
- VTODO-specific cardinality beyond `UID` / `DTSTAMP`. Aligns with [ADR-021](021-vtodo-scope.md) read-only stance.
- Custom user-defined rules via configuration file. v0.2.0 ships a fixed rule set.

### CLI surface

```
icslint [OPTIONS] <PATH>...

Options:
  -W, --warnings-as-errors     Treat warnings as errors (exit 1 on warning)
  -f, --format <FORMAT>        Output format: human (default), json, github
  -r, --rules <IDS>            Comma-separated rule IDs to enable (allow-list)
      --no-rules <IDS>         Comma-separated rule IDs to disable (deny-list)
      --color <WHEN>           Color output: auto (default), always, never
  -q, --quiet                  Suppress info-level diagnostics
      --version
      --help
```

Arguments are ICS file paths. `-` reads from stdin. Globbing is the shell's job; icslint does not expand globs internally.

Exit codes:

- `0` — no diagnostics, or only info-level diagnostics emitted.
- `1` — at least one warning emitted (and not promoted to error).
- `2` — at least one error emitted (or any warning with `-W`).
- `3` — internal error: file unreadable, parse failure that the tolerant parser could not recover from.

Output formats:

- **`human`** — default. Color-coded by severity; one diagnostic per line; mirrors `cargo check` style.
- **`json`** — array of `{file, line, rule, severity, message}` objects. Stable schema across patch releases.
- **`github`** — GitHub Actions workflow annotations format (`::error file=...::message`), so CI runs can surface findings on PRs.

### Dependency surface

```toml
# crates/icslint/Cargo.toml
[dependencies]
ics-core = { path = "../ics-core", version = "0.0.0" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Notably absent: `makeholiday` library dependency. icslint operates one level lower than the use-case layer — it reads ICS text, lints it, prints diagnostics, and exits. There is no `Repository`, no `RunContext`, no calendar mutation. Keeping the dependency surface minimal is a deliberate design choice that makes icslint cheap to install in CI.

### Architecture: rule registry + tolerant traversal

```
crates/icslint/src/
├── main.rs            # Composition root — clap parse, dispatch, exit code
├── lib.rs             # `lint(source: &str) -> Vec<Diagnostic>` for tests
├── rules/
│   ├── mod.rs         # Rule trait + registry
│   ├── rfc5545.rs     # RFC5545/* rules
│   ├── vendor.rs      # vendor/* rules
│   ├── text.rs        # text/* rules
│   └── structure.rs   # structure/* rules
├── diagnostic.rs      # Diagnostic struct + severity
└── reporter/
    ├── human.rs
    ├── json.rs
    └── github.rs
```

Each rule implements:

```rust
pub trait Rule {
    fn id(&self) -> &'static str;
    fn default_severity(&self) -> Severity;
    fn visit(&self, ctx: &LintContext, sink: &mut DiagnosticSink);
}
```

The `LintContext` exposes:

- the parsed `VCalendar` (typed view via `ics_core::parse_calendar`),
- the source text (for byte-level rules like BOM detection),
- the per-line offset table (for accurate line numbers in diagnostics) — supplied by ADR-019's lexer output.

Rules are **stateless** singletons stored in a static registry. Adding a rule is one struct + one registry entry; no plugin loading machinery.

### Relationship to `ics-core`

icslint is the **second consumer** of `ics-core` after `makeholiday` (and the third counting `lazyics`). This satisfies [ADR-017](017-workspace-and-ics-core-crate.md)'s primary repo-split trigger.

Consequence: pressure on `ics-core` to expose parse-error line numbers, lossless RawProperty access, and per-component source spans is now real. [ADR-019](019-parser-implementation-strategy.md) Steps 0-2 (v0.1.0) gave us line numbers on parse errors; icslint adds the requirement of **byte / line spans on successfully-parsed properties**, so a `RFC5545/duplicate-summary` diagnostic can point at the duplicate occurrence rather than the component start.

This is anticipated to be a small additive change to `parser/line.rs` (carry the source `LineRange` on each `RawProperty`), tracked as a v0.2.0 internal task and not a separate ADR.

### Workspace lifecycle

Mirroring [ADR-025](025-lazyics-project-definition.md)'s lifecycle:

1. `crates/icslint/` added as workspace member; the workspace test gate covers it from day one.
2. Rule families land incrementally — RFC5545 first (existing knowledge), then vendor (leverages existing vendor bundles), then text (leverages ADR-019 Step 2), then structure (needs lexer source span work).
3. Once the three v0.2.0 deliverables (ics-core split, lazyics, icslint) are mutually stable, `git filter-repo --subdirectory-filter crates/icslint/` extracts the icslint history into its own repository.
4. icslint's new repository gets the CI workflow cloned from this repo (`ci.yml`, `audit.yml`, `release.yml` adapted to a `icslint`-binary matrix).
5. crates.io publication happens together with the lazyics + ics-core publication batch.

[ADR-024](024-solo-phase-branching-carve-out.md) solo-phase carve-out **reactivates** when `ics-core` lands in its own repo — the explicit trigger ADR-024 names. icslint development from that point on follows the standard feature-branch + PR workflow.

## Consequences

### Positive

- ICS ecosystem story gains a concrete "quality gate" tool, completing the v0.2.0 trio (typed model = ics-core, editor = makeholiday + lazyics, validator = icslint).
- Pressure on `ics-core` from a non-editing consumer surfaces design weaknesses (e.g., source spans on parsed properties) that file-write-focused consumers don't expose.
- CI pipelines that consume ICS from external sources gain a Rust-native, single-binary linter with no runtime dependencies — competitive with `eslint` / `shellcheck` in feel.
- Vendor extension hygiene rules give the prefix-based vendor profile bundles introduced in [ADR-001](001-vcalendar-vevent-typed-model.md) a user-visible payoff beyond round-trip preservation.

### Negative

- Three Rust projects to maintain (makeholiday + lazyics + icslint) instead of one. Mitigation: small, focused crates with shared `ics-core` library.
- Rule definitions risk drifting from the actual RFC 5545 wording. Mitigation: each rule's docstring links to the specific RFC section it enforces.
- The "no user-defined rules" stance will frustrate power users. Mitigation: revisit in a follow-up ADR once concrete user requests arrive; do not preemptively design a plugin system in v0.2.0.
- Adding a third repository post-split increases release coordination cost (three crates.io publishes per ecosystem release).

### Migration

No code changes are required to *accept* this ADR. Implementation lands as v0.2.0 work:

1. **Add `crates/icslint/` workspace member** — Cargo.toml, lib.rs with the `lint()` entry point, main.rs with the clap skeleton, a "Hello, icslint" smoke test. `cargo test --workspace` keeps passing.
2. **RFC5545 family first** — implement and test the eight RFC5545 rules above against a fixture corpus.
3. **Diagnostic reporter — human format** — produce the canonical human output.
4. **Vendor family** — leverage existing `microsoft` / `google` / `icloud` prefix bundles for classification.
5. **Text family** — implement byte-level rules; this requires source text retention in the lint context.
6. **JSON + GitHub Actions reporters** — straightforward serialization once the diagnostic model is stable.
7. **Structure family** — requires `ics-core` source-span enhancement; lands last.
8. **Documentation** — `README.md`, rule reference doc per rule family.

Each step lands as its own commit ([ADR-024](024-solo-phase-branching-carve-out.md) solo-phase carve-out applies during in-tree development).

PRD §9 reference to "icslint" gains a forward link to this ADR in a separate doc-only commit.
