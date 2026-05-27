# 005. Conventional Commits

- Status: **Accepted** (retroactive)
- Date: 2026-05-27

## Context

Commit history is the project's primary durable record of *what changed and why*. As contributors grow, an unstructured `git log` becomes noisy and resists tooling (changelog generation, release-note assembly, bisect-by-category).

The existing commit history (`git log --oneline`) already shows an informal Conventional Commits flavor: `feat:`, `fix:`, `chore:`. [CONTRIBUTING.md](../CONTRIBUTING.md) names the convention but does not pin it. This ADR records the choice and the exact prefix vocabulary so contributors and future automation can rely on it.

## Decision

Use **Conventional Commits-flavored prefixes** on every commit subject line.

### Allowed prefixes

| Prefix | When to use |
|---|---|
| `feat:` | New user-facing capability (new subcommand, new flag, new exposed behavior) |
| `fix:` | Bug fix in shipped behavior |
| `chore:` | Tooling, build, gitignore, dependency bumps, repo plumbing |
| `refactor:` | Internal restructuring with no behavior change |
| `docs:` | Documentation-only change |
| `test:` | Tests-only change |

No other prefixes are accepted. If a change does not fit any category, split it.

### Subject and body

- Subject line **under ~72 characters**, written in imperative mood (`add`, not `added`).
- Body (optional) explains the **why**, not the **what** — the diff already shows what.
- Breaking changes get a `BREAKING CHANGE:` footer paragraph naming the affected surface.

### Scope

- An optional `(scope)` is allowed: `feat(remove): support comma-separated indices`. Not required; use when it aids skim-reading.

## Consequences

### Positive

- `git log --oneline` reads as a structured changelog without external tooling.
- Future automation — release-note generation, CHANGELOG assembly, commit categorization — is trivially implementable because the contract is already met.
- Reviewer cognitive load on commit subjects drops to "does the prefix match the diff?"
- Bisecting by category (`git log --grep '^feat:'`) becomes useful.

### Negative

- Contributors must learn the prefix vocabulary; the table above is short enough that the cost is one-time.
- No enforcement hook today — relies on contributor discipline and PR review. Adding a commit-msg hook is straightforward but out of scope for this ADR (would be a follow-up tooling decision).
- A change that legitimately spans multiple categories must be split into multiple commits. This is friction we accept because it improves history.
