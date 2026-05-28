# 023. No `convert` Subcommand

- Status: **Accepted**
- Date: 2026-05-28

## Context

[PRD §5.2 Planned](../PRD.md#52-planned) listed a candidate `convert` subcommand:

> **`convert` subcommand (candidate)** — translate between vendor profiles (e.g., Outlook-flavored ICS → Google-flavored ICS) with explicit loss reporting. Scope to be confirmed.

[PRD §8 Open Question](../PRD.md#8-open-questions) asked: "is vendor profile conversion a goal, or is 'preserve as input vendor profile' sufficient?"

[ADR-001](001-vendor-extension-typing.md) and [ADR-018](018-round-trip-strategy.md) committed to **round-trip preservation** of every vendor extension — Outlook-emitted ICS that flows through `makeholiday` retains its Outlook flavor on the way out unchanged. No translation happens automatically. The question is whether a *user-invoked* translation step should also be provided.

Evaluation: vendor-profile semantics differ enough that "convert Outlook to Google" is inherently lossy. The features that exist in one vendor's extension namespace often have no equivalent in another. A `convert` subcommand would necessarily emit a lossy result and a loss report — which is closer to a linter's output than to a CLI tool's primary contract. Consumers (Outlook, Google Calendar, iCloud) that need to interpret cross-vendor ICS do their own translation on import, with their own (lossy) heuristics.

## Decision

**`makeholiday` does not provide a `convert` subcommand.** Vendor profile conversion is **out of scope**, both now and going forward, modulo a superseding ADR.

What `makeholiday` *does* provide remains:

- **Lossless round-trip preservation** of every vendor extension on the input → output path (see [ADR-001](001-vendor-extension-typing.md), [ADR-018](018-round-trip-strategy.md)).
- **Typed access to vendor profile bundles** via the `ics-core` library, so consumers who care to translate can do so themselves.

What is *not* provided:

- `makeholiday convert --from outlook --to google`. No such subcommand.
- Flag-level removal of vendor extensions (`--strip-vendor microsoft`). Adding the flag would be a partial step toward conversion, and would be lossy without obvious value over "use a different tool" — rejected.
- Vendor-profile normalization passes inside `import` / `export` (future subcommands per PRD §5.2). `import` / `export`, if/when implemented, preserve the source profile as-is.

### Why this is out of scope

- **Semantic mismatch is fundamental.** Outlook's `X-MICROSOFT-CDO-BUSYSTATUS` has five values (`FREE`, `TENTATIVE`, `BUSY`, `OOF`, `WORKINGELSEWHERE`); the RFC `TRANSP` has two. Google's freebusy semantics differ again. Apple's color handling has no Microsoft equivalent. A faithful conversion needs case-by-case judgment that a CLI cannot make.
- **Conversion targets exist elsewhere.** Outlook's "subscribe to ICS URL" and Google Calendar's "import" both perform their own (lossy) translation at the receiving end. Inserting makeholiday into the middle adds a second lossy pass with no compensating benefit.
- **The linter persona is the right home.** icslint (per [ADR-017](017-workspace-and-ics-core-crate.md)) can produce useful output of the form "this property is Microsoft-specific and will be ignored by Google clients" — this is a linter rule, not a CLI mutation. Conversion is the linter user's job once the warnings are read.
- **PRD scope discipline.** [PRD §2](../PRD.md#2-goals) names CLI UX as the top priority. A `convert` subcommand whose contract is "produces wrong output and prints warnings" damages UX more than it helps.

### What to do if a user actually needs to convert

Documented alternatives, for users who land on this ADR via search:

- Re-import the ICS into the target calendar service (Outlook / Google / iCloud) and re-export — let the target's import path do its own (lossy) translation.
- Use a dedicated migration tool from one calendar service to another.
- Use `ics-core` (the library, per [ADR-017](017-workspace-and-ics-core-crate.md)) directly to write a one-off translation script — the typed vendor bundles are exposed precisely so this is possible.

## Consequences

### Positive

- The CLI surface stays small. One fewer subcommand to design, document, test, and explain.
- The "round-trip lossless" pledge is unambiguous: the CLI never mutates the vendor profile, period. No "except when `convert` is invoked" footnote.
- icslint has clearer scope: the cross-vendor interoperability story belongs to the linter.
- Users who want conversion get a real (unambiguous) answer instead of an underbaked subcommand.

### Negative

- Users hoping to migrate a calendar from Outlook to Google via `makeholiday` are turned away. Acceptable: that hope was unrealistic given the semantic mismatch.
- A "lossy conversion is honest about being lossy" tool has educational value we forgo. Acceptable: icslint can deliver the same education without the CLI mutation.

### PRD updates

This decision closes [PRD §8 Open Question](../PRD.md#8-open-questions) on `convert`. Two changes follow:

1. **Remove** the `convert subcommand (candidate)` bullet from [PRD §5.2 Planned](../PRD.md#52-planned).
2. **Add** to [PRD §3 Non-Goals](../PRD.md#3-non-goals): "**Vendor profile conversion** — translating ICS from one vendor's flavor (Outlook / Google / iCloud) to another's is out of scope. See [ADR-023](design/023-no-convert-subcommand.md)."
3. **Remove** the convert-related entry from [PRD §8 Open Questions](../PRD.md#8-open-questions).

These PRD updates land as a follow-up doc commit, alongside the [ADR-021](021-vtodo-scope.md) and [ADR-022](022-tui-front-end-policy.md) PRD updates already pending.

### Migration

No code changes follow from this ADR — it removes a candidate feature, it does not add anything.

The PRD updates above are a single follow-up commit (combined with the ADR-021/022 PRD updates if convenient).
