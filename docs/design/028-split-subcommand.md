# 028. `split` Subcommand and `ics-core` Split API

- Status: **Accepted**
- Date: 2026-06-04
- Amended: 2026-06-05 — UID list predicate (`--uid`) lands as the second slice. The amendment adds `split_by_uids` to ics-core, extends the use-case signature with a `uids` parameter, and fixes the AND-composition rule originally pre-declared. See §"Amendment — UID list slice".
- Related: [ADR-017](017-workspace-and-ics-core-crate.md) (maturity gate #4 — ICS file split), [ADR-020](020-cli-subcommand-policy.md) (verb naming), [ADR-021](021-vtodo-scope.md) (VTODO read-only — split copies VTODO components untouched), [ADR-018](018-round-trip-strategy.md) (calendar-level field preservation), [ADR-025](025-lazyics-project-definition.md) (lazyics consumes icscli use cases — `split --uid` reaches lazyics future "export selected" mode for free)

## Context

[ADR-017](017-workspace-and-ics-core-crate.md) §"Publishing strategy" lists "ICS file split" as the fourth and final maturity gate before `ics-core` can leave the workspace. The ADR words the feature as:

> typed extraction of a subset of events into a new `VCalendar` (predicate-based, date-range, or UID list). Surfaced through `icscli` as a `split` / `extract` subcommand.

The wording bundles three independent predicate kinds and two verb candidates. This ADR fixes the verb, the first predicate kind to ship, and the wire semantics — leaving the remaining predicate kinds as additive future work.

The first-slice scope is deliberately the smallest one that closes the maturity gate end-to-end (ics-core function + use case + CLI verb + tests + help text). Anti-ad-hoc rationale: lock the verb, output shape, and predicate composition rules once; then later predicates inherit those decisions instead of relitigating them.

## Decision

### Verb name: `split`

Per [ADR-020](020-cli-subcommand-policy.md) common-meaning-common-name:

- **`split`** — universally understood as "divide a whole into parts." Composes naturally with future qualifiers (`split --from / --to`, `split --uid`, hypothetical `split --predicate`). Accepted.
- **`extract`** — connotes "take one piece, discard the rest." That framing biases the design toward a destructive single-output operation, which conflicts with the non-destructive default below. Rejected.

### First-slice predicate: date range only

v0.2.x post-release work ships exactly one predicate kind: date range, expressed as `--from <DATE>` / `--to <DATE>` (both inclusive, either optional, at least one required).

Deferred to follow-up commits — not separate ADRs unless their semantics diverge from this one:

- `--uid <UID>` (repeatable) — explicit UID list extraction.
- `--summary <PATTERN>` — substring or regex match on `SUMMARY`.

These additions reuse the wire shape and output policy below; the only design question they introduce is "what does `--from` combined with `--uid` mean?" Answer in advance: **multiple predicate flags AND together** (intersection). Stated here so future PRs don't relitigate.

### Date-range matching: overlap semantics

An event matches `[from, to]` when its date span overlaps the closed range. Formally, with `from` and `to` as `Option<NaiveDate>`:

```
matches(event)
  := (from.is_none() || event.last_inclusive_day() >= from.unwrap())
  && (to.is_none()   || event.dtstart            <= to.unwrap())

event.last_inclusive_day()
  := event.dtend - 1 day   (RFC 5545 DTEND is exclusive for DATE-typed values)
```

Rationale:

- **Overlap** is the intuition users carry from "show me everything happening this week." Containment ("event fully inside the range") rejects multi-day events that straddle the boundary, which is rarely what the user wants.
- **Inclusive bounds** match the rest of the CLI surface — `add --start` / `add --end` are both inclusive ([crates/icscli/src/presentation/cli.rs](../../crates/icscli/src/presentation/cli.rs) `Add`).
- **Single-bound forms** (only `--from` or only `--to`) work as half-open ranges naturally. Useful for "everything before this date" archival workflows.

Validation (CLI / use-case layer, not ics-core):

- At least one of `--from` / `--to` must be present. Both omitted → `IcsError::InvalidInput("split: at least one of --from or --to is required")`.
- If both present, `from > to` → `IcsError::InvalidInput("split: --from must not be after --to")`.

ics-core's `split_by_date_range` is itself total over all bound combinations (see §"ics-core surface"); the rejection above is purely a CLI ergonomics decision.

### ics-core surface

Adds one free function to `crates/ics-core/src/query.rs`, exported from `lib.rs`:

```rust
/// Return a new `VCalendar` containing only the events that overlap
/// `[from, to]`. Either bound may be `None` to leave that side of the
/// range open.
///
/// Total: no validation errors are raised — both bounds `None` returns
/// every event (the `(-∞, +∞)` range), `from > to` returns no events
/// (empty range). Caller-side UX validation (e.g., requiring at least
/// one CLI flag) lives in `icscli` per the layer separation below.
///
/// Calendar-level fields (prodid, version, X-WR-*) and unrecognized
/// components (incl. VTODOs per ADR-021) are preserved verbatim. Events
/// are returned in the input order.
pub fn split_by_date_range(
    cal: &VCalendar,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> VCalendar;
```

Implementation is a pure filter — no I/O, no mutation, no cloning of fields that aren't borrowed (uses `Clone` because `VEvent` is `Clone`). Matches the shape of the existing `remove_event_by_summary` / `remove_events_by_indices` siblings in `query.rs`.

**Layer separation rationale.** `ics-core::Error` only carries the `Parse` variant ([crates/ics-core/src/error.rs](../../crates/ics-core/src/error.rs)). Returning `Error::Parse` for a CLI-policy decision (validate range inputs) would mislabel the failure — the user sees "parse error: ..." for what is clearly an argument validation error. Keeping `split_by_date_range` total means:

- ics-core stays a typed data core with no CLI policy baked in.
- `icscli` raises `IcsError::InvalidInput` with the right semantics.
- Future consumers (icslint, lazyics, library users) pick their own validation policy.

### CLI surface: `Commands::Split`

```
icscli split --out <PATH> [--from <DATE>] [--to <DATE>]
```

| Flag | Required | Meaning |
|---|---|---|
| `--out <PATH>` | yes | Destination ICS file. Fails with `AlreadyExists` if the path is taken (no overwrite). |
| `--from <DATE>` | one-of | Inclusive lower bound (YYYY-MM-DD or YYYY/M/D). |
| `--to <DATE>` | one-of | Inclusive upper bound (YYYY-MM-DD or YYYY/M/D). |

`-f` / `--file` (global) supplies the input. The input file is **not** mutated by `split`. The matched subset is written atomically to `--out` via the existing `FileCalendarRepository` atomic-write pipeline (ADR-011).

### Non-destructive by default

`split` does not modify the input calendar. This matches "extraction" semantics — the user receives a new file and decides separately whether to prune the original (`icscli remove` is the existing tool for that).

A future `--move` flag (or destructive variant) is left out of scope for this ADR. When it lands, it must:

- be opt-in (default stays non-destructive), and
- compose with whatever predicate flags exist at that point — not just `--from`/`--to`.

`--remainder-out <PATH>` (write the non-matching events to a third file) is similarly deferred. The single-output shape covers the common "yearly → quarterly" workflow today.

### Help text contract

Per [feedback_help_text_is_a_contract](#) the `long_about` for `Split` must enumerate every reachable behavior: overlap semantics, non-destructive default, AlreadyExists on `--out`, at-least-one-bound validation. If a later PR adds `--uid` / `--move` / `--remainder-out`, the same PR rewrites the `long_about` rather than leaving stale documentation.

## Consequences

### Positive

- Closes [ADR-017](017-workspace-and-ics-core-crate.md) maturity gate #4 with a minimal, anti-ad-hoc surface: one function in `ics-core`, one use case, one CLI verb.
- The verb / output / predicate-composition policy is fixed once, so adding UID / summary / move predicates later is mechanical.
- Non-destructive default protects the user's source data; destructive workflow remains expressible (`split` then `remove`).
- Overlap semantics matches user intuition and the rest of the CLI's inclusive-date convention.

### Negative

- Without `--remainder-out`, "split a year into four quarters" requires four invocations + a final `remove`. Acceptable v0.2.x cost; revisit when concrete user feedback arrives.
- The ics-core function takes `Option<NaiveDate>` bounds, not a fully generic predicate. When predicate-based / UID-list variants land, the public surface widens (two more functions, or one function with an enum). Either is fine; this ADR does not preemptively design the union type.

### Migration

Lands as a single PR in trunk-based fashion ([ADR-024](024-solo-phase-branching-carve-out.md) carve-out applies):

1. `ics-core` Red — failing unit tests for `split_by_date_range` (overlap, single-bound, no-match, calendar-level preservation, validation errors).
2. `ics-core` Green — minimal implementation; re-export from `lib.rs`.
3. `icscli` Red — failing use-case tests against `FileCalendarRepository` covering write-to-`--out`, input-untouched, AlreadyExists on `--out`.
4. `icscli` Green — `application::use_cases::split` orchestrating load + filter + write.
5. `icscli` CLI — `Commands::Split { from, to, out }` with `value_parser = parse_date`, dispatch in `main.rs`, `long_about` per help-text contract.
6. CHANGELOG / PRD / README updates referencing this ADR and the new verb.

No changes required to `lazyics` or `icslint` for this slice.

## Amendment — UID list slice (2026-06-05)

The second slice of [ADR-017](017-workspace-and-ics-core-crate.md) maturity gate #4 lands as `--uid <UID>` (repeatable). This amendment fixes the concrete shape that the original ADR pre-declared.

### ics-core: sibling pure function

```rust
/// Return a new `VCalendar` containing only the events whose `UID`
/// appears in `uids`. Set-membership semantics: an empty `uids` list
/// returns no events (the empty intersection). UIDs in `uids` that no
/// event matches are silently skipped — they do not error.
///
/// Calendar-level fields and unrecognized components are preserved.
/// Events appear in the input order.
pub fn split_by_uids(cal: &VCalendar, uids: &[String]) -> VCalendar;
```

Rationale for **set-membership semantics** (empty = empty, not identity):

- Matches the mathematical reading "events in this set" — predictable.
- The "treat missing flag as identity" policy belongs to the caller, not the function. ics-core stays a typed data core; the use case decides what an absent CLI flag means.
- Symmetric with `split_by_date_range`'s total semantics: each function has a single, crisp filter contract; the use case composes them.

### Composition rule (use case layer)

The use case applies filters in a **pipeline** rather than constructing a unified predicate:

```rust
let mut filtered = input.load()?;
if from.is_some() || to.is_some() {
    filtered = ics::split_by_date_range(&filtered, from, to);
}
if !uids.is_empty() {
    filtered = ics::split_by_uids(&filtered, uids);
}
output.create_with(&filtered)?;
```

Why pipeline, not a single multi-arg function:

- Each pure function in ics-core stays single-purpose. Adding a third predicate (`--summary`) later means a third function + a third pipeline step, not a fourth argument to one function.
- Use case's branching `if` clauses encode the "missing flag = no-op" CLI policy in one place. ics-core does not need to know about CLI flags.
- AND-composition (intersection) emerges naturally — each stage narrows the candidate set further. The ADR-028 pre-declaration "multiple predicate flags AND together" is satisfied by the pipeline being a sequence of narrowing filters; no enum or dispatch.

### CLI surface change

```
icscli split --out <PATH> [--from <DATE>] [--to <DATE>] [--uid <UID>]...
```

| Flag | Required | Meaning |
|---|---|---|
| `--out <PATH>` | yes | (unchanged) |
| `--from <DATE>` | one-of | (unchanged) |
| `--to <DATE>` | one-of | (unchanged) |
| `--uid <UID>` | one-of | Match events with this UID. Repeatable (`--uid A --uid B`); the union of values forms the candidate set. |

Validation now reads "at least one of `--from`, `--to`, or `--uid` must be present." Both bounds omitted with no `--uid` → `IcsError::InvalidInput("split: at least one of --from, --to, or --uid is required")`.

`--uid` with a value that no event in the input matches is **not** an error — the use case prints how many events were written (which may be zero), keeping the operation idempotent for scripts that union UID lists from elsewhere.

### Consumer reach (lazyics)

[ADR-025](025-lazyics-project-definition.md) lazyics calls `icscli::application::use_cases::*` directly. A future lazyics "export selected events to file" mode (multi-select on List view → save to a new ICS) is now one call: pass the selected UIDs through `use_cases::split(input, output, ctx, None, None, &selected_uids)`. No new lazyics-specific use case needed — this amendment alone unlocks it.

### Migration (this amendment)

1. ics-core Red — `split_by_uids` unit tests: subset by UID, order preserved, empty uids = empty, missing UID silently skipped, calendar-level preservation.
2. ics-core Green — `split_by_uids` implementation (`HashSet<&str>` membership), re-export from `lib.rs`.
3. icscli Red — use case tests: `--uid` only, `--from` + `--uid` intersection, validation accepts non-empty `--uid` alone, empty `--uid` is no-op.
4. icscli Green — extend `use_cases::split` signature with `uids: &[String]`; apply pipeline.
5. icscli CLI — `--uid` repeatable on `Commands::Split`; update `long_about` per "help text is a contract".
6. CHANGELOG / USAGE updates.
