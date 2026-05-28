# 018. Round-trip Strategy

- Status: **Accepted**
- Date: 2026-05-28

## Context

[ADR-001](001-vendor-extension-typing.md) settled the typed model for vendor extensions but deferred several questions about how that model maps to the wire format:

- The semantics of `RawProperty.source_index`.
- The output order of properties inside `VEvent` / `VCalendar`.
- The vendor profile mapping for `VCALENDAR`-level prefixes (notably `X-WR-*`).
- Line folding (RFC 5545 §3.1).
- Line endings.
- Value escaping (RFC 5545 §3.3.11) and parameter quoting (§3.2).

[PRD §2 Goal 2](../PRD.md#2-goals) commits to "round-trip losslessness" — re-emission preserves order, whitespace where semantically meaningful, and all properties. This ADR pins what that pledge concretely means.

## Decision

### Round-trip granularity — semantic completeness

`makeholiday` (and `ics-core` per [ADR-017](017-workspace-and-ics-core-crate.md)) target **semantic completeness**, not byte-perfect equality:

- All properties present on input are present on output.
- All property values, parameter names, and parameter values are preserved.
- Unknown property *relative order* is preserved.
- Whitespace, line-folding column, and arbitrary parameter ordering are not preserved.

Byte-perfect round-trip is explicitly rejected: the implementation cost is high and almost no real ICS consumer depends on bit-level equality. If a future use case (e.g., diff/patch over `.ics` files) requires it, a superseding ADR adds the byte-level path behind a flag.

### Property output order

Inside any `VEvent` or `VCalendar`:

1. **Typed RFC-recognized fields** are emitted in a **canonical fixed order**. For `VEvent`: `UID`, `DTSTAMP`, `DTSTART`, `DTEND`, `SUMMARY`, `TRANSP`, `CLASS`, `CATEGORIES`. For `VCalendar`: `VERSION`, `PRODID`, `CALSCALE`, `METHOD`, then the X-WR-* typed fields (below), then nested `VEVENT`s, then unrecognized components.
2. **Typed vendor extension bundles** follow the typed RFC block, in fixed order across bundles: `microsoft`, `google`, `icloud` (alphabetical by profile name). Within each bundle, fields emit in their struct declaration order.
3. **Unknown properties** (`unknown: Vec<RawProperty>`) emit after all typed content, in the order given by their `source_index` (relative order from input).
4. **Unrecognized components** (`unrecognized_components: Vec<RawComponent>`) emit after all properties.

This trades *input position parity for typed properties* for implementation simplicity. The ADR-001 Rule that "unknown is a stable bucket" is preserved; unknown properties keep their relative order to each other through `source_index`.

### `RawProperty.source_index` semantics

- **Definition:** zero-based index recording the order in which the unknown property appeared inside its containing `VEvent` / `VCalendar`, **relative only to other unknown properties in the same container**.
- It is *not* a global line number, not a byte offset, and not an index that includes typed properties.
- On parse: `source_index` is assigned monotonically as unknown properties are encountered.
- On format: unknown properties emit in ascending `source_index` order.
- On programmatic construction (no parse): newly added unknowns are appended with monotonically increasing `source_index`.
- `source_index: u32` is sufficient (≪ 4 billion properties per VEvent).

### `X-WR-*` calendar-level fields — promoted to typed

`X-WR-*` is a de facto standard introduced by Apple Calendar and adopted by Mozilla Lightning, Google Calendar import, Outlook import, and most consumer-facing calendar tooling. Strictly it is an `X-` extension; in practice it is treated as RFC-equivalent by every major consumer.

Promote four `X-WR-*` properties to typed `VCalendar` fields, **not** routed through the vendor profile mechanism:

| Property | Field |
|---|---|
| `X-WR-CALNAME` | `VCalendar.name: Option<String>` |
| `X-WR-CALDESC` | `VCalendar.description: Option<String>` |
| `X-WR-TIMEZONE` | `VCalendar.timezone: Option<String>` |
| `X-WR-RELCALID` | `VCalendar.rel_cal_id: Option<String>` |

Routing rule: a property name starting with `X-WR-` is matched **before** any vendor-prefix matching (longest-match rule still applies among the four). If the property is one of the four above, it goes to the typed field. Other `X-WR-*` (none currently known to exist) fall through to `VCalendar.unknown`.

This partially supersedes [ADR-001](001-vendor-extension-typing.md)'s "every vendor extension is under a vendor profile" rule. The exception is justified by industry-wide treatment.

### Line folding (RFC 5545 §3.1)

**Parser:** before any other parsing, the input stream is **unfolded** — every occurrence of `CRLF` or `LF` followed immediately by a space (`U+0020`) or horizontal tab (`U+0009`) is removed (continuation marker), joining the next line to the current one. The result is a stream of logical lines.

**Formatter:** every output logical line is **folded** if it exceeds **75 octets** (UTF-8 byte count). The fold is `CRLF` + single space (`U+0020`). The fold point must respect UTF-8 character boundaries — never split inside a multi-byte sequence. The folded continuation line resumes at column 1 of the next physical line, with the leading space being part of the continuation marker.

### Line endings

- **Output:** `CRLF` always, on every platform. Per RFC 5545.
- **Input:** accepts `CRLF` and `LF` (both treated as line terminators by the unfolder).
- This is independent of the host OS — Windows builds output the same bytes as Linux builds.

### Value escaping (RFC 5545 §3.3.11)

The escape rules apply only to TEXT-typed property values.

| Wire | Internal |
|---|---|
| `\\` | `\` (backslash) |
| `\;` | `;` (semicolon) |
| `\,` | `,` (comma) |
| `\N` or `\n` | `\n` (LF in the internal string) |

Application to our fields:

- **TEXT-typed fields** (`summary`, `categories[i]`, future `description`, future `comment`): stored **unescaped** in the internal `String`. Parser decodes on read; formatter encodes on write. Users of the library set the natural text and the wire-format escaping is invisible.
- **Non-TEXT typed fields** (`uid` (TEXT-like in name only; UUIDs never contain escapable chars), `dtstamp`, `dtstart`, `dtend`, `transp` enum, `class` enum): no escape processing; stored as their natural typed value.
- **`RawProperty.value`** (per [ADR-001](001-vendor-extension-typing.md) Q6): stored raw with escapes intact. Parser does not touch; formatter emits as-is. This preserves bytes for unknowns that the round-trip depends on.

### Parameter quoting (RFC 5545 §3.2)

- **Input:** parameter value matching `"..."` has its surrounding double quotes stripped before storage. The stored value is the inner content.
- **Output:** a parameter value emits surrounded by `"..."` **iff** it contains any of `:`, `;`, or `,`. Otherwise it emits raw.
- `was_quoted_originally` flags are **not** preserved — quoting is a function of the value alone.

### Multi-value vs multi-occurrence

RFC 5545 allows both representations for multi-cardinality properties:

```
CATEGORIES:work,travel
```
vs.
```
CATEGORIES:work
CATEGORIES:travel
```

**Output:** always single-occurrence, multi-value form (`CATEGORIES:work,travel`). This matches today's implementation and produces the shortest output.

**Input:** both forms accepted. Multiple `CATEGORIES:` lines are merged into one `Vec<String>` in the typed field; the comma-separated form is split. The first wins / rest to unrecognized rule ([ADR-001](001-vendor-extension-typing.md) Rule 8) applies only when **single-occurrence** properties (e.g., `SUMMARY`) appear more than once.

### Byte order mark (BOM)

- **Input:** a leading UTF-8 BOM (`U+FEFF` / `EF BB BF`) is silently consumed by the parser. Microsoft tools sometimes emit it; treating it as part of the first property name is a parser bug.
- **Output:** no BOM is emitted. RFC 5545 does not specify one; emitting one breaks tools that don't expect it.

### Character encoding

- **UTF-8 throughout.** Input is read as UTF-8; invalid sequences fail the parser with `MhError::Parse { line, message, property }`.
- **No locale-dependent transcoding.** Free-text user data (summaries, categories) is preserved byte-for-byte after UTF-8 validation, in line with [PRD §6 NFR](../PRD.md#6-non-functional-requirements).

## Consequences

### Positive

- The "round-trip losslessness" pledge in [PRD §2 Goal 2](../PRD.md#2-goals) becomes mechanically verifiable: parse any conforming ICS file, re-emit, parse again — both ASTs are equal (same `VCalendar` struct value).
- `source_index` semantics are simple enough to test exhaustively: it's just "order among unknowns."
- The `X-WR-*` promotion turns the most-used calendar-level extensions into ergonomic typed fields, which is what consumers (icslint included) will want first.
- Line folding compliance closes the only correctness gap in the current parser (long summaries would currently silently corrupt).
- CRLF output + UTF-8 + RFC escaping make our output indistinguishable (semantically) from Outlook / Google / Apple output.

### Negative

- Typed property input order is not preserved. Users comparing input and output byte-by-byte will see ordering differences. We accept this; the alternative (full source_index on every property) is heavy.
- The escape rules differ between typed TEXT fields (decoded) and `RawProperty.value` (raw). New contributors must remember the asymmetry. Documented in the ADR and in the formatter.
- `X-WR-*` promotion is a stretch of [ADR-001](001-vendor-extension-typing.md)'s "vendor properties live in vendor bundles" rule. We accept the inconsistency in exchange for the UX win.
- The fold algorithm with UTF-8 boundary awareness is more code than naive byte folding. Acceptable; the alternatives (no folding, naive byte folding) silently corrupt non-ASCII summaries.

### Migration

Round-trip behavior is implemented as part of [ADR-001](001-vendor-extension-typing.md) Migration Steps 1–7, all in `crates/ics-core/` ([ADR-017](017-workspace-and-ics-core-crate.md)). The ordering refinements below merge into those steps:

- **In ADR-001 Step 1** (introduce `RawProperty` + `unknown`): also implement `source_index` assignment during parse.
- **In ADR-001 Step 2** (`RawComponent` for unrecognized components): also implement the canonical output-order rule and the unfolder.
- **Add a new sub-step before Step 1**: implement the line folder/unfolder against the current flat type model. This is small and lands the long-summary bug fix immediately.
- **Add a new sub-step alongside Step 1**: promote `X-WR-*` to typed `VCalendar` fields.
- **Each step ships round-trip tests:** parse an example file, re-emit, parse again, assert struct equality. A small canonical-form `.ics` test corpus lands in `crates/ics-core/tests/data/`.

Compliance with RFC 5545 escape rules and CRLF output is part of Step 2's formatter rewrite; before then, the existing formatter is unchanged and known to be partially non-compliant on long lines and embedded special characters. The CHANGELOG entry for the release containing Step 2 calls out the corrected behavior.
