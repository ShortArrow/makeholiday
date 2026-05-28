# 019. Parser Implementation Strategy

- Status: **Accepted**
- Date: 2026-05-28

## Context

The current parser in `src/ics.rs::parse_events` is a flat `for line in normalized.lines() { if let Some(val) = line.strip_prefix("UID:") { ... } else if ... }` chain over `\r\n`-normalized input. It:

- Does not implement line folding ([ADR-018](018-round-trip-strategy.md) §4 violation — long properties get truncated).
- Does not implement value escaping decode ([ADR-018](018-round-trip-strategy.md) §6 violation — `\,` stays as `\,` in `summary`).
- Does not handle quoted parameter values ([ADR-018](018-round-trip-strategy.md) §6 violation).
- Drops unknown properties ([ADR-001](001-vendor-extension-typing.md) violation — round-trip lost).
- Has no separation between lexical layer (line → tokens) and dispatch layer (token → typed field).

[ADR-001](001-vendor-extension-typing.md) deferred the parser implementation strategy explicitly to this ADR. [ADR-018](018-round-trip-strategy.md) made the correctness requirements concrete. [ADR-006](006-testing-strategy.md) requires the parser to be testable in isolation.

## Decision

### Strategy — lightweight lexer + line dispatch

Implement the parser as a small two-layer module inside `crates/ics-core` ([ADR-017](017-workspace-and-ics-core-crate.md)):

1. **Lexical layer:** unfold physical lines into logical lines; parse each logical line into a `LogicalLine { name, params, value }` token.
2. **Dispatch layer:** walk `BEGIN:`/`END:` markers to build the component tree; for each property, route to the typed field, the appropriate vendor bundle, or `unknown`.

No external parser-combinator dependency (`nom`, `pest`, `chumsky`). The ICS grammar is simple enough (two-level: component tree + line properties) that a domain-written lexer is clearer than expressing the grammar through a general combinator library. Avoids the dependency, the compile-time tax, and the contributor learning curve.

### Module layout

```
crates/ics-core/src/
  parser/
    mod.rs              # pub entry points (parse_calendar)
    unfold.rs           # physical → logical lines (RFC 5545 §3.1)
    line.rs             # logical line → LogicalLine (name, params, value)
    escape.rs           # TEXT value encode/decode (RFC 5545 §3.3.11)
    dispatch.rs         # LogicalLine → VEvent / VCalendar field / vendor bundle / unknown
  formatter/
    mod.rs              # pub entry points (format_calendar)
    fold.rs             # logical line → folded physical lines
    line.rs             # LogicalLine → text (with param quoting per §3.2)
```

Parser and formatter share the `LogicalLine` type (and `escape.rs` helpers) so the round-trip path is structurally symmetric.

### `LogicalLine` token shape

```rust
pub(crate) struct LogicalLine<'a> {
    pub name: String,                     // UPPERCASE-normalized
    pub params: Vec<(String, String)>,    // param names UPPERCASE, order preserved, quotes stripped
    pub value: &'a str,                   // raw bytes, escapes intact, no length limit
}
```

Borrowing `&str` for `value` lets the parser zero-copy into the input. The name and params are owned because they go through case normalization.

### Lexical entry points (private to `parser/`)

```rust
fn unfold_lines(input: &str) -> Vec<&str>;
fn parse_logical_line(line: &str) -> Result<LogicalLine<'_>, Error>;
```

`unfold_lines` strips any leading UTF-8 BOM ([ADR-018](018-round-trip-strategy.md)), splits on `CRLF` or `LF`, and joins continuation lines (whose first char is space or tab) onto the preceding logical line.

`parse_logical_line`:

1. Find the first colon outside of any quoted parameter region — that is the name/params-vs-value boundary.
2. Split the prefix on `;` (outside quotes) → first segment is property name, remaining are params.
3. Parse each param as `KEY=VALUE`, stripping surrounding `"..."` from VALUE if present.
4. Uppercase property name and all param keys.

### Dispatch entry points (pub)

```rust
pub fn parse_calendar(input: &str) -> Result<VCalendar, Error>;
```

`parse_calendar` walks the unfolded logical lines, maintains a `Vec<ComponentFrame>` stack on `BEGIN:`/`END:`, and dispatches each property:

1. If inside `VEVENT`: try recognized RFC properties → typed `VEvent` fields. Then try vendor prefixes (longest match per [ADR-001](001-vendor-extension-typing.md)) → typed bundle field or bundle's `unrecognized`. Otherwise → `VEvent.unknown` with monotonic `source_index`.
2. If inside `VCALENDAR` at the top level: try `VERSION` / `PRODID` / `CALSCALE` / `METHOD` / `X-WR-CALNAME` / `X-WR-CALDESC` / `X-WR-TIMEZONE` / `X-WR-RELCALID` → typed `VCalendar` fields. Then try vendor prefixes. Otherwise → `VCalendar.unknown`.
3. If inside any unrecognized component (e.g., `VTIMEZONE`, `VALARM`): collect all properties and nested `BEGIN:`/`END:` blocks into a `RawComponent`. No typed dispatch.

### TEXT escape policy

`escape.rs` exports `decode_text_value(raw: &str) -> String` and `encode_text_value(text: &str) -> String`. Applied per [ADR-018](018-round-trip-strategy.md) §6 to typed TEXT fields. **Not** applied to `RawProperty.value` — which goes through `parser/dispatch.rs` straight from `LogicalLine.value` without translation.

### Error handling

- **Fail-fast.** The first `Error` returned by any layer aborts parsing. The caller (use case → CLI) maps this to `MhError::Parse(#[from] ics_core::Error)` per [ADR-017](017-workspace-and-ics-core-crate.md).
- Errors carry the logical-line number (1-based, post-unfold), the offending property name when identifiable, and a human-readable message — matching the `ics_core::Error::Parse { line, message, property }` shape from [ADR-012](012-error-handling.md)/[ADR-017](017-workspace-and-ics-core-crate.md).
- **No best-effort recovery** in this version. A future ADR may add a parallel `parse_calendar_recovering(input) -> (Option<VCalendar>, Vec<Error>)` if a linter consumer (icslint) demands it. Today the simpler API wins.

### Streaming

- **Whole-file parsing.** Input is read entirely into memory by the caller, passed as `&str` to `parse_calendar`. Output is the complete `VCalendar`.
- **No pull-based or push-based streaming API** in this version. [PRD §6 NFR](../PRD.md#6-non-functional-requirements) targets up to ~10,000 events under one second on commodity hardware; the whole-file approach comfortably meets that bound at expected calendar sizes (a 10k-event ICS file is on the order of a few megabytes — trivial for in-memory processing).
- If a future workload pushes past this (a future ADR), pull/push variants can be added without changing the `parse_calendar` signature.

### Whole-file API symmetry with formatter

```rust
pub fn parse_calendar(input: &str) -> Result<VCalendar, Error>;
pub fn format_calendar(cal: &VCalendar) -> String;
```

The formatter is the mirror of the parser: walk the typed `VCalendar` in canonical output order ([ADR-018](018-round-trip-strategy.md) §3), produce `LogicalLine` tokens, fold per RFC 5545 §3.1, write `CRLF` terminators.

## Consequences

### Positive

- The lexer/dispatch separation makes each layer independently testable per [ADR-006](006-testing-strategy.md). Unit tests cover folding, line tokenization, escape decode/encode in isolation. Integration tests cover the full parse → format → parse round-trip.
- No new external dependency. Compile time stays low. Contributors don't need to learn a parser-combinator DSL.
- The shape mirrors most production ICS libraries (icalendar, libical, ical.js): a small lexer + a dispatcher. Familiar to anyone who has read those.
- Future round-trip property tests (`parse(format(parse(x))) == parse(x)`) are mechanical to write because the dispatcher and the formatter share `LogicalLine`.
- Fail-fast error semantics match the CLI primary persona's expectation (one error at a time, fix and retry).

### Negative

- Lexer rules are hand-written — UTF-8-safe folding, quoted-param detection, BOM stripping all need care. We accept the implementation burden in exchange for skipping a dependency.
- Whole-file parsing constrains us to calendars that fit comfortably in memory. Acceptable until a real consumer hits the wall.
- Fail-fast cannot satisfy a linter consumer (icslint) that wants every error reported. We document the limitation and accept that linter-style recovery is a future ADR with a parallel API entry point.
- No partial-parse-after-error means a single malformed line aborts the calendar's parse, so the CLI can never list "five errors at once." Acceptable; the CLI never claimed to be a linter.

### Migration

Lands as the implementation of [ADR-001](001-vendor-extension-typing.md) Migration Steps 1–2 inside `crates/ics-core/`:

1. **Step 0 (new, lands first):** introduce `parser/unfold.rs` and the unfolder. Behavior remains identical to today's flat parser for short inputs; long inputs now parse correctly. Tests for fold edge cases (UTF-8 boundary, tab vs space continuation, multiple consecutive folds).
2. **Step 1:** rewrite the dispatch path to produce `RawProperty` for unknowns and populate `VEvent.unknown`. Add `parse_logical_line` token type. Existing typed-field parsing keeps working unchanged in behavior, but is now backed by `LogicalLine`.
3. **Step 2:** introduce `RawComponent` handling for `VTIMEZONE` / `VALARM` / `VTODO`. Round-trip preservation activates.
4. Subsequent steps follow [ADR-001](001-vendor-extension-typing.md) Migration.

Each step ships with a round-trip test: load an example `.ics` file, parse, format, parse again, assert `VCalendar` equality. The example corpus grows under `crates/ics-core/tests/data/` as steps land.
