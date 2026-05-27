# 000. ADR Policy

- Status: **Accepted**
- Date: 2026-05-27

## Context

`makeholiday` is expanding from a small ICS CLI toward a broader toolset (CRUD strengthening, vendor extension handling for Outlook / Google / iCloud, a reusable ICS handling library). As scope grows, design decisions accumulate, and we need a durable record that future contributors can read instead of reconstructing reasoning from `git log` and code archaeology.

Architectural Decision Records (ADRs) are short Markdown documents that capture a single non-trivial decision, the context that motivated it, and the consequences accepted in exchange. We adopt them as the canonical place for such records.

## Decision

ADRs in this repository follow the **Michael Nygard** format:

```
# NNN. <Title>

- Status: <Proposed | Accepted | Deprecated | Superseded by ADR-NNN>
- Date: YYYY-MM-DD

## Context
<Why this decision is needed. The forces at play, constraints, and prior state.>

## Decision
<What we decided. Stated declaratively in the present tense.>

## Consequences
<What follows from the decision, both positive and negative. What gets easier; what gets harder; what we accept.>
```

### Conventions

- **Location:** `docs/design/`
- **Filename:** `NNN-kebab-case-title.md` where `NNN` is a zero-padded three-digit sequence. `000` is reserved for this policy document; new ADRs start at `001`.
- **Numbering:** monotonically increasing, never reused. Skipping numbers is acceptable if a draft is abandoned.
- **One decision per ADR.** If multiple decisions are entangled, write multiple ADRs that reference each other.
- **Status lifecycle:**
  - `Proposed` — under discussion; not yet adopted.
  - `Accepted` — adopted; the codebase is expected to conform.
  - `Deprecated` — no longer applies, but not replaced by a specific successor.
  - `Superseded by ADR-NNN` — replaced by a later ADR. The superseding ADR links back to this one in its Context.
- **Immutability of decided content.** Once an ADR is `Accepted`, do not silently rewrite its Decision or Consequences. Update its Status and write a new ADR that supersedes it.
- **Cross-linking.** ADRs reference each other and relevant code paths by relative link.

### Language

- ADR body is written in **English** to keep technical decisions accessible across contributors.
- A Japanese translation MAY be added as a sibling file `NNN-title.jp.md`. When present, the English version remains the source of truth.

### When to write an ADR

Write an ADR when a decision is:

- non-trivial to reverse (data formats, public APIs, dependency choices),
- likely to surprise a future reader who only sees the resulting code, or
- the result of a deliberate trade-off worth recording.

Skip ADRs for routine refactors, dependency bumps, and decisions that are obvious from the code itself.

## Consequences

- Future architectural decisions have a single, predictable location and shape.
- Onboarding cost drops: a contributor can read `docs/design/` to learn the project's accumulated reasoning.
- Lightweight ceremony: writing an ADR takes minutes, not hours, because the format is short and fixed.
- Reviewers gain a checkpoint — "does this change need an ADR?" becomes a routine question in PR review.
- Risk: ADRs can drift from the code if not actively maintained. Mitigation: link ADRs from relevant modules' doc comments and update Status promptly when decisions change.
