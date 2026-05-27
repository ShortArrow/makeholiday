# 001. Vendor Extension Typing Model

- Status: **Accepted**
- Date: 2026-05-27

## Context

`makeholiday` operates on iCalendar (RFC 5545) files. Real-world `.ics` files almost never contain only RFC properties: Outlook adds the `X-MICROSOFT-CDO-*` family, Google adds `X-GOOGLE-*`, Apple/iCloud adds `X-APPLE-*` and `X-CALENDARSERVER-*`, individual integrations sprinkle ad-hoc `X-*` properties, and the file as a whole may contain components (`VTIMEZONE`, `VTODO`, `VALARM`, …) we do not yet type-handle.

[PRD §2](../PRD.md#2-goals) commits to constraints that drive this decision:

1. **CLI UX (highest priority).** Whatever the type model is, the CLI surface must remain ergonomic.
2. **Round-trip losslessness.** Every property and every component the parser sees must be representable and re-emittable, including unknown ones.
3. **Typed handling of vendor extensions.** Vendor extensions are first-class typed values, not opaque `X-*` strings, with a documented boundary between RFC and each vendor profile.
4. **Library reusability.** The ICS core must be consumable as an independent crate later.

Today (`src/ics.rs`), the `VEvent` struct mixes RFC and vendor concerns and silently drops everything it does not recognize. Three shapes were considered:

- **Option A — Flat fields on `VEvent`.** Trivial access; no vendor boundary in the type; unknown `X-*` properties have nowhere to live; violates Goal 2.
- **Option B — Vendor profile bundles + raw fallback.** Explicit vendor boundary; named RFC fields for ergonomics; unknown preserved. Slightly more types to maintain.
- **Option C — `properties: Vec<Property>` enum.** Perfect order preservation by construction; weakens type-level guarantees on required fields; breaks every existing accessor.

## Decision

Adopt **Option B — Vendor profile bundles with prefix-based pre-reservation and recursive raw-component fallback**.

### Type shape

```rust
pub struct VCalendar {
    // RFC standard fields
    pub version: String,
    pub prodid:  String,
    pub calscale: Option<String>,
    pub method:   Option<String>,

    // Typed events
    pub events: Vec<VEvent>,

    // Per-vendor calendar-level bundles
    pub microsoft:   Option<microsoft::CalendarExtensions>,
    pub google:      Option<google::CalendarExtensions>,
    pub icloud:      Option<icloud::CalendarExtensions>,
    pub makeholiday: Option<makeholiday::CalendarExtensions>,

    // Properties at calendar level matching no known vendor prefix
    pub unknown: Vec<RawProperty>,

    // Components other than VEVENT (VTIMEZONE, VTODO, VALARM-at-cal-level, …)
    pub unrecognized_components: Vec<RawComponent>,
}

pub struct VEvent {
    // RFC standard fields
    pub uid: String,                    // required
    pub dtstamp: NaiveDateTime,         // required
    pub dtstart: NaiveDate,             // required, date-only (timed events handled separately, see "Out of scope")
    pub dtend:   Option<NaiveDate>,     // optional per RFC
    pub summary: String,
    pub class:      Option<EventClass>,
    pub categories: Vec<String>,
    pub transp:     Option<Transp>,

    // Per-vendor event-level bundles
    pub microsoft:   Option<microsoft::EventExtensions>,
    pub google:      Option<google::EventExtensions>,
    pub icloud:      Option<icloud::EventExtensions>,
    pub makeholiday: Option<makeholiday::EventExtensions>,

    // Properties matching no known vendor prefix
    pub unknown: Vec<RawProperty>,

    // Components inside VEVENT (VALARM, …) we do not yet type
    pub unrecognized_components: Vec<RawComponent>,
}

pub struct RawProperty {
    pub name:   String,                  // UPPERCASE-normalized
    pub params: Vec<(String, String)>,   // names UPPERCASE-normalized, order preserved, multi-value params split into multiple entries
    pub value:  String,                  // raw value, no escape interpretation
    pub source_index: u32,               // semantics defined in ADR-002
}

pub struct RawComponent {
    pub name: String,                    // UPPERCASE: VTIMEZONE / VTODO / VALARM / …
    pub properties: Vec<RawProperty>,
    pub sub_components: Vec<RawComponent>,
}
```

Each vendor profile (e.g., `crate::ics::profile::microsoft`) exposes its own `EventExtensions` and `CalendarExtensions` types. Inside each vendor bundle, the same fallback exists for properties that match the vendor prefix but are not yet typed:

```rust
pub struct EventExtensions {       // per-vendor
    // typed fields ...
    pub unrecognized: Vec<RawProperty>,
}
```

### Rules

1. **RFC 5545 properties are named fields on `VEvent` / `VCalendar`.** They are the lingua franca; all profiles agree on them.

2. **Prefix-based pre-reservation.** Each vendor module exports a `pub const PREFIXES: &[&str]`. Initial assignment:
   - `microsoft`: `["X-MICROSOFT-CDO-", "X-MICROSOFT-"]`
   - `google`:    `["X-GOOGLE-"]`
   - `icloud`:    `["X-APPLE-", "X-CALENDARSERVER-"]`
   - `makeholiday`: `["X-MAKEHOLIDAY-"]`
   - `X-WR-*` (shared by Apple Calendar and Mozilla Lightning, calendar-level only) is **out of scope for this ADR** and handled in ADR-002 alongside `VCALENDAR`-level prefix mapping.

3. **Longest prefix wins.** `X-MICROSOFT-CDO-BUSYSTATUS` matches `X-MICROSOFT-CDO-` before `X-MICROSOFT-`. Both happen to route to `microsoft`; the rule is stated for correctness when ambiguous.

4. **Routing:**
   - Property name matches a registered prefix → routed to that vendor's bundle. Typed if recognized, else into `vendor.unrecognized`.
   - Property name matches no registered prefix → `VEvent.unknown` / `VCalendar.unknown`.
   - This guarantees that promoting a property from `vendor.unrecognized` to a typed field is intra-bundle and never cross-bundle. `unknown` is a stable bucket.

5. **A vendor bundle is `Option<_>`.** `None` means the source contained nothing from that vendor; emitting `None` writes nothing.

6. **Distributed module ownership.** Each vendor is a plain Rust module exporting its `PREFIXES`, its `EventExtensions` / `CalendarExtensions` types, and its `parse_property` / `format_extensions` functions. No `trait Profile` is introduced; trait abstraction can be added later when external crate extension becomes a real concern.

7. **Cross-vendor synonyms are not auto-translated.** If both `X-MICROSOFT-CDO-BUSYSTATUS` and a Google equivalent are present, both are preserved in their respective bundles. The CLI surface decides which to display.

8. **Cardinality follows RFC 5545.** Single-occurrence properties → `Option<T>` or `T`; multi-occurrence → `Vec<T>`. If a single-occurrence property appears more than once in the input, the first wins; subsequent occurrences are stashed in `vendor.unrecognized` (vendor properties) or `unknown` (otherwise) as `RawProperty`, preserving round-trip. A warning is written to stderr; exit code remains 0. A future `--strict` flag (ADR-007) may turn this into a hard error.

9. **Required-field handling is RFC-loose.** A typed `VEvent` requires `UID`, `DTSTAMP`, and `DTSTART`. `DTEND` is optional (single-point events are allowed). If any of the three required fields is missing or the property cannot be parsed in date-only form, the entire `VEVENT` is downgraded to `RawComponent` under `VCalendar.unrecognized_components` and a warning is written to stderr.

10. **Migration of existing fields:**
    - `busystatus: BusyStatus` splits into:
      - RFC-level `transp: Option<Transp>` on `VEvent`
      - Microsoft-specific `microsoft.busystatus: Option<MsBusyStatus>`
    - `icon: Option<String>` moves into `makeholiday.icon: Option<String>`.
    - The CLI flag `--busystatus` is a **Microsoft-vocabulary shortcut**: it populates `microsoft.busystatus` and derives `transp` (as the current code does — `Free → TRANSPARENT`, else `OPAQUE`). No `--transp` flag is added in this ADR.

### Out of scope for this ADR

- **Property order on output and source-order preservation semantics** — `RawProperty.source_index` is reserved here; its semantics are defined in ADR-002.
- **`VCALENDAR`-level prefix mapping for `X-WR-*`** — ADR-002.
- **Parser implementation strategy** (line-based / lexer+parser / streaming) — ADR-003.
- **Crate split / library layering** — ADR-004. This ADR assumes the typed model lives in the same crate today and can be lifted later without changing its shape.
- **Timed events** (`DTSTART;VALUE=DATE-TIME`, TZID, UTC, floating). Such `VEVENT`s fall to `VCalendar.unrecognized_components` via rule 9. Typing them is a deliberate later step.
- **CLI ergonomics for nested-type mutation** (`event.microsoft.get_or_insert_with(...).busystatus = ...`) — implementation detail, addressed in code with helper methods.

## Consequences

### Positive

- The vendor boundary is visible in the type system, satisfying PRD Goal 3.
- Unknown properties and unrecognized components are preserved, satisfying PRD Goal 2 (the part scoped to typing; ordering belongs to ADR-002).
- Each vendor profile can evolve in its own module without rippling through `VEvent` / `VCalendar`.
- `unknown` is a stable bucket; integrators reading it never see properties migrate out from under them when a vendor profile gains a typed field.
- The library surface for integrators (PRD §4 secondary persona) maps directly onto the type they care about.
- The CLI continues to operate on typed `VEvent`s only; raw components are preserved transparently across read-modify-write, but are invisible to `list` / `remove` indices. A future opt-in (`list --include-raw`) can expose them without breaking today's UX.

### Negative

- Breaking change to today's `VEvent` shape and to `list --json` output. Acceptable pre-1.0; recorded in the CHANGELOG when each migration step lands.
- More boilerplate up front (vendor modules, raw fallback parsing, `RawComponent` handling) than today's flat parser.
- Cross-vendor synonym resolution is pushed to the CLI layer, where it becomes a UX question instead of a type one. Acceptable per UX-first prioritization.

### Migration (incremental, Tidy First)

Each step ships its own commit/PR with tests passing and the CLI working:

1. Introduce `RawProperty` and add `VEvent.unknown: Vec<RawProperty>`. Parser stores prefix-unmatched `X-*` properties instead of dropping them. No behavior change to existing typed fields.
2. Introduce `RawComponent`. Add `VCalendar.unrecognized_components` and `VEvent.unrecognized_components`. Parser captures `VTIMEZONE`, `VALARM`, etc. round-trip preservation kicks in here.
3. Split `busystatus` field: add `VEvent.transp: Option<Transp>`, leave `busystatus: BusyStatus` semantically Microsoft-only for one transitional step.
4. **(breaking step A)** Introduce `microsoft::EventExtensions` and move `busystatus` into it. Update `commands.rs`, JSON output, tests. CHANGELOG note.
5. **(breaking step B, separate PR)** Introduce `makeholiday::EventExtensions` and move `icon` into it. CHANGELOG note.
6. Add per-vendor `unrecognized: Vec<RawProperty>` fallback. Parser routes prefix-matched-but-untyped properties there. Stable `unknown` bucket invariant becomes real.
7. Skeleton `google` and `icloud` modules: prefix registration and empty `EventExtensions` / `CalendarExtensions` with `unrecognized`. No typed fields yet.

Step 4 and Step 5 are deliberately separate PRs to keep each breaking change focused.
