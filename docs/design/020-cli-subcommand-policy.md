# 020. CLI Subcommand Policy

- Status: **Accepted**
- Date: 2026-05-28

## Context

[ADR-015](015-diagnostic-output.md) settled stdout/stderr split and the `--quiet` / `--interactive` flags. [ADR-012](012-error-handling.md) settled error reporting format. [ADR-018](018-round-trip-strategy.md) settled wire-format semantics. What remains is a coherent policy across the CLI surface itself — subcommand naming, flag reuse across subcommands, and several "shall we add this?" decisions that have surfaced in earlier ADRs as "deferred to CLI policy ADR."

Without a written policy, every new subcommand will re-invent its flag names, and `--strict` / `--dry-run` style ideas will get added under deadline pressure with no coherent rule. [PRD §2 Goal 1](../PRD.md#2-goals) prioritizes CLI UX above all; UX coherence at this surface is the load-bearing case.

## Decision

### Subcommand naming

- **Current naming is kept.** No rename of `icons` to `list-icons` or `add` to `add-event`.
- **Future subcommands use verbs alone** when possible (`edit`, `search`, `import`, `export`, `convert`). `git` and `cargo` follow this convention; we match the broader Rust/Unix ecosystem.
- **The single noun exception (`icons`)** stays because it is a helper command listing built-in data, not an action on the calendar. New helper-style commands may be nouns (`profiles`, `prefixes`, etc.) if the listing-built-in-data shape repeats — but the bias is toward verbs.
- **No deeper hierarchy** (no `makeholiday event add` / `makeholiday icon list`). The flat verb-set is enough for the foreseeable scope.

### Global vs subcommand-local flags

A **global flag** applies to every subcommand and is declared once on the top-level CLI struct. A **subcommand-local flag** applies only inside one subcommand.

| Flag | Scope | Notes |
|---|---|---|
| `--file <PATH>` / `-f` | global | The calendar file every subcommand operates on. Default `calendar.ics`. |
| `--quiet` / `-q` | global | Suppresses status / warning output. See [ADR-015](015-diagnostic-output.md). |
| `--interactive` | global | Forces interactive mode. See [ADR-015](015-diagnostic-output.md). |
| `--no-interactive` | global | Forces non-interactive mode. See [ADR-015](015-diagnostic-output.md). |
| `--help` / `-h` | global (clap default) | Per-subcommand help. |
| `--version` / `-V` | top-level only | Version of `makeholiday`. |
| `--json` | local to `list` | JSON output mode. Reserved name for any future read-only subcommand that wants the same. |
| `--summary <TEXT>` | local to `add`, `remove`, and future `edit` / `search` | Event title field. **Same name everywhere it appears.** |
| `--start <DATE>`, `--end <DATE>` | local to `add` and future `edit` / `search` | Event date range. Same names everywhere. Inclusive end on CLI; converted to RFC-exclusive `DTEND` internally per [USAGE.md](../USAGE.md). |
| `--busystatus`, `--class`, `--category`, `--icon` | local to `add` and future `edit` | Same names everywhere. |
| `<INDEX_SPEC>` | positional to `remove` and future `edit` | 1-based index expression: `4`, `2,4`, `6-10`, `1,3-5,8`. |

### Common-meaning, common-name rule

When a flag has the same conceptual meaning across multiple subcommands, **its name and short form (if any) MUST be identical**. The table above is the canonical reference; new subcommands look up their flag names there before inventing new ones.

Conversely, subcommands MUST NOT use the same flag name for different concepts — e.g., adding a `--summary` to a future `convert` subcommand that means anything other than "match this event title" is forbidden.

Reviewers verify this when subcommands are added or extended.

### What is *not* introduced

- **`--strict` flag** — deferred. [ADR-001](001-vendor-extension-typing.md) Rule 8 commits to "first wins + warning" for duplicate single-occurrence properties. A `--strict` mode that promotes warnings to errors would be useful only for a linter persona; that persona belongs to a sibling project (icslint, see [ADR-017](017-workspace-and-ics-core-crate.md)). If after icslint exists makeholiday users still ask for `--strict`, a future ADR adds it. Until then, do not add the flag.
- **`--dry-run` flag** — deferred. The primary persona (CLI individual) edits files under version control or backup; the cost of a wrong invocation is low. Adding `--dry-run` proactively to mutating subcommands creates a "what does this actually do?" question that we accept users solving via VCS. If a CI/scripting use case demands it, a future ADR adds it across mutating subcommands uniformly.
- **`--verbose` / `-v` flag** — deferred per [ADR-015](015-diagnostic-output.md). Today's output is information-dense enough; `--verbose` is reserved for a future ADR that introduces structured logging (`tracing`).
- **`-o <PATH>` / output redirection** — deferred. All commands write to stdout or to the `--file` calendar; users redirect with shell `>` if they want.
- **Subcommand aliases** — deferred. `ls` for `list`, `rm` for `remove` — clap supports `#[command(alias = "ls")]`. Adding aliases is friendly, but defining the policy is a separate decision; for now we ship without aliases and reconsider when users ask.

### Subcommand documentation policy

Every subcommand's clap help text MUST include:

1. **One-line summary** as the `#[command]` attribute or `///` doc comment.
2. **At least one usage example** in the long help (`#[command(long_about = "...")]` or extended doc comment). Example: `Add: makeholiday add --summary "Holiday" --start 2026-01-01`.
3. **Flag descriptions** that name the data type and any defaults (clap derive surfaces these from `#[arg]` doc comments).

A subcommand whose `--help` lacks an example is incomplete and should be revised in review.

This is recorded here (rather than only in [CONTRIBUTING.md](../CONTRIBUTING.md)) because it is a UX requirement, not a development-process preference. CONTRIBUTING.md will reference back to this ADR.

### Interactive-mode boundary per subcommand

Per [ADR-015](015-diagnostic-output.md), interactive-mode entry depends on TTY detection and the `--interactive` / `--no-interactive` overrides. Each subcommand's contribution:

| Subcommand | Required args | Interactive fallback |
|---|---|---|
| `init` | (none) | no prompt; `init` has no missing-arg case |
| `add` | `--summary`, `--start` | both prompt if missing (in interactive mode) |
| `list` | (none) | no prompt |
| `icons` | (none) | no prompt |
| `remove` | one of `<INDEX_SPEC>` or `--summary` | prompts with event listing if both absent (in interactive mode) |
| future `edit` | `<INDEX_SPEC>` | prompts for new field values |
| future `search` / `filter` | (depends on subcommand semantics) | TBD |

### Exit codes

Per [ADR-015](015-diagnostic-output.md): `0` on success, `1` on any user-facing error. No subcommand has its own exit code scheme.

## Consequences

### Positive

- New subcommands have a written rule for naming and flag-naming; reviewers verify mechanically.
- Users learn a flag once and find it consistently — `--summary` always means the event title, `--file` always means the calendar file, etc.
- The deferred items are documented explicitly so the next contributor does not re-litigate them in a PR comment thread.
- Help examples are required from day one, addressing the most common documentation-debt path for CLIs.

### Negative

- No subcommand aliases means users type `remove` instead of `rm`. Friction; revisit later.
- No `--dry-run` means destructive operations need user confidence (or VCS). For the personal persona, fine; CI/scripted use may want it eventually.
- The verb-only naming convention is a soft rule, not enforced by tooling. Reviewers verify.

### Migration

This ADR records policy. The implementation work it implies:

1. **Audit current subcommands** against the flag table and resolve any deviations. The current code already matches (verified at this ADR's writing): `--summary` is the title across `add` and `remove`, `--file` is global.
2. **Add help examples** to subcommands that lack them. `init`, `icons`, and `remove` lack a documented usage example in their clap definitions; add `long_about` strings with one example each.
3. **Update [CONTRIBUTING.md](../CONTRIBUTING.md)** with a brief "CLI flag naming" section referencing this ADR.
4. **Update [USAGE.md](../USAGE.md) / [USAGE.jp.md](../USAGE.jp.md)** when `--quiet` / `--interactive` / `--no-interactive` land (separate from this ADR's acceptance — those flags arrive with the [ADR-015](015-diagnostic-output.md) implementation).

Each item is a small follow-up; this ADR's acceptance does not require any of them to land simultaneously.
