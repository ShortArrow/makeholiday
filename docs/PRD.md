[ **English** | [日本語](PRD.jp.md) ]

# Product Requirements Document — makeholiday

> Status: **Skeleton**. Headings only; bodies are placeholders to be filled in follow-up work.

## 1. Background

<!-- TODO: Why this tool exists. The gap in existing ICS tooling. -->

## 2. Goals

<!-- TODO: What success looks like. Bullet list of measurable outcomes. -->

## 3. Non-Goals

<!-- TODO: Explicitly out of scope. -->

## 4. Target Users

<!-- TODO: Personas. CLI-comfortable developers managing personal calendars; integrators embedding ICS handling. -->

## 5. Functional Requirements

### 5.1 Currently shipped (v0.1.0)

<!-- TODO: Formalize the shipped surface as requirements. -->

- `init` — create a `VCALENDAR` file
- `add` — append a `VEVENT` (all-day, single or multi-day, with busy status / class / categories / icon)
- `list` — enumerate events, with multi-key sort and JSON output
- `icons` — list bundled icon names
- `remove` — delete events by 1-based index, range expression, or summary match

### 5.2 Planned

<!-- TODO: Expand each into proper requirements with acceptance criteria. -->

- **ICS CRUD strengthening** — richer queries, in-place edit, bulk import/export
- **Vendor extension support** — Outlook, Google Calendar, iCloud extensions handled losslessly
- **RFC compliance vs vendor extension boundary** — clear, documented separation; round-trip guarantees per vendor profile
- **Reusable ICS handling library** — extract the parsing / formatting core as a separately consumable crate

## 6. Non-Functional Requirements

<!-- TODO: Performance, portability (Windows / macOS / Linux), stability, error reporting, i18n. -->

## 7. Out of Scope

<!-- TODO: Things explicitly not in this product (e.g. server-side sync, GUI). -->

## 8. Open Questions

<!-- TODO: Unresolved decisions. -->
