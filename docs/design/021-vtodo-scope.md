# 021. VTODO Scope

- Status: **Accepted**
- Date: 2026-05-28

## Context

[PRD §8 Open Question](../PRD.md#8-open-questions) asked: "should `VTODO` be supported with read/write parity to `VEVENT`, or read-only round-trip preservation only?"

The relevant constraints have changed since the PRD was written:

- [ADR-001](001-vendor-extension-typing.md) committed to `RawComponent` round-trip for any non-`VEVENT` component, so VTODO is *already* preserved on round-trip — no data is lost today regardless of typing decisions.
- [ADR-017](017-workspace-and-ics-core-crate.md) split the codebase into `ics-core` (the shared library, future icslint consumer) and `makeholiday` (the CLI). The question splits into two: what does `ics-core` expose? what does the `makeholiday` CLI surface?
- icslint, when it launches, will want typed access to VTODO (linting tasks is a natural icslint scope).
- makeholiday itself is named for and oriented around holidays/events; its primary persona does not edit todo lists in `makeholiday`.

## Decision

Two decisions, one per layer.

### `ics-core` — typed `VTodo`, full parity

`ics-core` ships a typed `VTodo` component that mirrors `VEvent` in structure:

```rust
pub struct VTodo {
    // RFC 5545 typed core
    pub uid: String,                          // required
    pub dtstamp: NaiveDateTime,               // required
    pub dtstart: Option<NaiveDate>,           // optional, date-only per ADR-018 scope
    pub due: Option<NaiveDate>,               // optional, date-only
    pub summary: String,
    pub status: Option<TodoStatus>,           // NEEDS-ACTION / COMPLETED / IN-PROCESS / CANCELLED
    pub priority: Option<u8>,                 // 0..=9 per RFC
    pub percent_complete: Option<u8>,         // 0..=100
    pub class: Option<EventClass>,            // reused from VEvent
    pub categories: Vec<String>,

    // Per-vendor extension bundles, same shape as VEvent (ADR-001)
    pub microsoft: Option<microsoft::TodoExtensions>,
    pub google:    Option<google::TodoExtensions>,
    pub icloud:    Option<icloud::TodoExtensions>,

    // Unknown property and component preservation
    pub unknown: Vec<RawProperty>,
    pub unrecognized_components: Vec<RawComponent>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TodoStatus {
    NeedsAction,
    Completed,
    InProcess,
    Cancelled,
}
```

`VCalendar` gains a parallel `Vec<VTodo>` alongside `Vec<VEvent>`:

```rust
pub struct VCalendar {
    // ... existing fields ...
    pub events: Vec<VEvent>,
    pub todos:  Vec<VTodo>,            // ← new
    // ... vendor bundles, unknown, etc. ...
}
```

Parser dispatch ([ADR-019](019-parser-implementation.md)) handles `BEGIN:VTODO ... END:VTODO` blocks the same way it handles `VEVENT`. Required-field handling follows [ADR-001](001-vendor-extension-typing.md) Rule 9 (missing `UID` / `DTSTAMP` falls to `RawComponent` under `VCalendar.unrecognized_components`).

Time-of-day VTODOs (`DTSTART;VALUE=DATE-TIME`, `DUE;VALUE=DATE-TIME`) follow [ADR-018](018-round-trip-strategy.md) — typed model is date-only; timed VTODOs fall to `RawComponent` preservation.

### `makeholiday` CLI — read-only display, opt-in

The CLI does **not** ship VTODO-editing subcommands. Specifically:

- **No `add-todo` / `edit-todo` / `remove-todo` / `complete-todo`.** makeholiday is positioned as a holiday/event CLI per [PRD §4](../PRD.md#4-target-users); todo CRUD belongs to a hypothetical separate todo CLI.
- **`list` gains an opt-in flag** `--include-todos`. When set, the output interleaves todos with events:

  ```
  1: [E] 2026-01-01           : New Year's Day
  2: [T] DUE 2026-03-15 (50%) : File taxes
  3: [E] 2026-05-10 to 2026-05-12 : Business trip
  ```

  The prefix `[E]` marks events, `[T]` marks todos. Index numbering is shared so a later `remove` would refer to either, but **`remove` deliberately does not accept todo indices** — todos are display-only in the CLI surface.
- **`list --json` with `--include-todos`** emits a flat array with a `"kind"` discriminator (`"event"` vs `"todo"`) per entry, or with both fields populated as appropriate. Concrete JSON shape lands with the implementation; this ADR commits only to the principle.
- **`remove --include-todos`**: not provided. A future ADR may add it if a real use case appears.
- **Round-trip writes**: any save (e.g., after `add` / `remove` on events) preserves all VTODOs unchanged. This is automatic because the underlying `VCalendar` carries the `todos: Vec<VTodo>` and the formatter emits them.

## Consequences

### Positive

- icslint (and any future consumer) gets typed access to `VTodo` from `ics-core` without makeholiday having to ship a todo CLI.
- The most common user expectation ("my calendar has tasks in it; please don't lose them") is mechanically satisfied: round-trip preservation is automatic, and `--include-todos` lets users see what's there without learning a new command set.
- makeholiday stays scope-disciplined as an event/holiday CLI. The "I want a todo list manager" desire goes to a different tool.
- The `[E]` / `[T]` prefix in `list --include-todos` extends naturally to future component types (e.g., `[J]` for VJOURNAL if ever added) without redesigning the output.

### Negative

- Users who actually want to edit todos from the CLI cannot. Acceptable: that user can use a dedicated todo tool, and makeholiday's round-trip preserves whatever that other tool wrote.
- `ics-core::VTodo` is more code than `ics-core::VEvent` alone, and most makeholiday users will not touch it. Cost is bounded — VTODO type mirrors VEvent so the implementation is largely parallel.
- The decision not to support `remove` of todos means a malformed VTODO that the user wants gone requires editing the `.ics` file manually. Friction; revisit if asked.

### PRD update

This decision closes [PRD §8 Open Question](../PRD.md#8-open-questions) on `VTODO` scope. The PRD §8 line "**`VTODO` scope** — full read/write parity with `VEVENT`, or read-only preservation for round-trip?" should be removed in a follow-up PRD update, with a pointer to this ADR added to PRD §5.2 alongside the planned-features list.

The PRD update is a small documentation commit, independent of this ADR's acceptance.

### Migration

Implementation lands after the [ADR-017](017-workspace-and-ics-core-crate.md) restructure and the [ADR-001](001-vendor-extension-typing.md) typed-vendor migration have settled. Order:

1. **In `ics-core`:** add `VTodo`, `TodoStatus`, parser dispatch for `BEGIN:VTODO`, formatter, round-trip tests. Comes after [ADR-001](001-vendor-extension-typing.md) Migration Steps 1–7 so VTODO inherits the vendor-extension model from the start.
2. **In `crates/makeholiday/`:** extend the `list` use case to optionally include todos. Add `--include-todos` flag per [ADR-020](020-cli-subcommand-policy.md)'s "common-meaning, common-name" rule (so any future read-only subcommand that wants the same flag uses the same name).
3. **In tests:** an integration test asserts that `makeholiday add` on a calendar containing VTODOs preserves the todos byte-equivalently (modulo [ADR-018](018-round-trip-strategy.md) canonical ordering) on round-trip.
4. **In docs:** [USAGE.md](../USAGE.md) / [USAGE.jp.md](../USAGE.jp.md) gain a section under `list` describing `--include-todos`.
