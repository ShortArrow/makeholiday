# 007. Documentation Language Policy — English Primary, Japanese Mirror

- Status: **Accepted** (retroactive)
- Date: 2026-05-27

## Context

The project is bilingual by nature: the maintainer's working language is Japanese, while the broader Rust ecosystem (crates.io, GitHub discoverability, library consumers) operates in English. Existing documentation (`README`, `PRD`, `CONTRIBUTING`, `USAGE`, `SETUP`) already ships in both languages, with a tab-style language switcher at the top of each file.

Without an explicit policy, language drift is easy: JP-only contributions, EN-only contributions, inconsistent switcher styling, divergent translations. This ADR pins the convention so the project does not drift toward "the docs say different things in EN and JP" — a state worse than monolingual.

## Decision

### Source of truth

- **English is the source of truth** for every document.
- **Japanese translations are mirrors**, kept in sync within the same PR as the English change.
- Translations cannot diverge across commits. A PR that updates EN without updating JP (or vice versa) is incomplete and should be revised in review.

### File layout

- The primary (English) file uses its natural name: `README.md`, `docs/PRD.md`, `docs/CONTRIBUTING.md`, etc.
- The Japanese mirror sits alongside with a `.jp.md` suffix: `docs/README.jp.md`, `docs/PRD.jp.md`, `docs/CONTRIBUTING.jp.md`.
- For top-level `README.md`, the JP mirror lives at `docs/README.jp.md`.

### Language switcher

Every bilingual file opens with a pseudo-tab switcher:

```markdown
[ **English** | [日本語](path/to/jp.md) ]
```

or, on the JP side:

```markdown
[ [English](path/to/en.md) | **日本語** ]
```

The currently-displayed language is **bold**; the other language is a link.

### What is *not* translated

- **Code comments.** English only.
- **Commit messages.** English only (consistent with [ADR-005](005-conventional-commits.md)).
- **CLI help text.** English only (per [PRD §6 NFR](../PRD.md#6-non-functional-requirements)).
- **ADR bodies.** English only (per [ADR-policy](000-ADR-policy.md)). A JP translation MAY be added as a sibling `NNN-title.jp.md` for an individual ADR but is not required and is not the source of truth.
- **Changelog entries.** English only.

### Free-text user data

Event summaries, categories, and other user-supplied strings are **language-agnostic UTF-8**: the tool round-trips whatever the user provides without escaping or interpretation.

## Consequences

### Positive

- Both audiences — Rust ecosystem (EN) and the maintainer's working language (JP) — have first-class access to project documentation.
- The translation discipline catches ambiguity in EN drafts: writing the JP forces the author to actually understand what the EN sentence means.
- The convention is simple enough to follow without tooling: one suffix, one switcher pattern.

### Negative

- Translation work is a fixed per-PR cost for any doc change. We accept this as the price of bilingual accessibility.
- No automation today verifies that EN and JP stay in sync; relies on PR review discipline. A future tooling ADR could introduce a `cargo xtask check-translations` style check.
- ADRs being English-only means JP-speakers lose first-class access to decision history. The "JP sibling MAY be added" escape hatch is intentional but unused so far; if individual ADRs become contentious in JP discussions, we revisit.
- A change that touches many documents (e.g., a project-wide rename) doubles the diff size. Tolerable.
