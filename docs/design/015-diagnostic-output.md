# 015. Diagnostic Output Policy

- Status: **Accepted**
- Date: 2026-05-28

## Context

Pieces of the diagnostic output behavior have already been decided across earlier ADRs and documentation, but no single ADR consolidates them. Without consolidation, contributors must hunt across [USAGE.md](../USAGE.md), [ADR-012](012-error-handling.md), and the source code to learn the rules.

Additionally, three open questions on output behavior have accumulated and need an explicit answer:

1. Should the CLI support `--quiet` / `--verbose`?
2. How should the CLI behave when required arguments are missing and stdin is not a terminal (CI / piped input)?
3. Should exit codes be subdivided by error category?

[PRD §6 NFR](../PRD.md#6-non-functional-requirements) says "Errors identify the input line and the offending property name when parsing ICS; commands fail closed (non-zero exit) rather than silently dropping data." — settled by [ADR-012](012-error-handling.md).

The primary persona (CLI-comfortable individuals) expects modern CLI norms: quiet operation when scripted, helpful prompts when interactive.

## Decision

### Output channel rules (consolidated from USAGE.md)

| Channel | Contents |
|---|---|
| **stdout** | Command's primary user-readable output: `list` output, `icons` output, `list --json` output. Pipe-friendly. |
| **stderr** | Status messages (`Added: ...`, `Removed: ...`), interactive prompts (`Summary: `), warnings, errors (per [ADR-012](012-error-handling.md)). |

Commands that write to stdout do so on their *primary success path* only. Status messages confirming the side effect (file mutation) go to stderr. This makes piping safe: `makeholiday list --json | jq ...` does not see "Added: ..." noise.

### `--quiet` flag

- A single global flag `--quiet` / `-q` suppresses status messages and warnings on stderr. **Errors still print** (PRD §6 NFR fail-closed).
- No `--verbose`. Today's default verbosity is sufficient; the source-chain expansion in [ADR-012](012-error-handling.md) already exposes the underlying cause. If a future ADR introduces `tracing` for deeper diagnostics, `--verbose` may be added then.
- No `-v`/`-vv` stacked levels. Out of scope at this scale.

### Interactive vs non-interactive mode

The current behavior — prompt on missing `--summary` / `--start` — works in a terminal but hangs forever when stdin is piped or closed. New behavior:

- **TTY auto-detection.** If a required argument is missing **and** `std::io::stdin().is_terminal()` returns true, enter interactive mode. Otherwise, fail immediately with a `Conflict`-equivalent error: `error: --summary is required when stdin is not a terminal`.
- **Override flags:**
  - `--interactive` forces interactive mode even when stdin is not a terminal (e.g., for tests that pipe canned answers).
  - `--no-interactive` forces non-interactive mode even when stdin *is* a terminal (e.g., for testing the error path or for shell aliases that want strict argument enforcement).
- `--interactive` and `--no-interactive` are mutually exclusive; supplying both yields `MhError::Conflict`.
- TTY check uses `std::io::IsTerminal` (stabilized in Rust 1.70, well under our [ADR-008](008-msrv.md) MSRV of 1.85) — no new dependency.

### Exit codes

- **`0`** = success.
- **`1`** = any user-facing error (all `MhError` variants from [ADR-012](012-error-handling.md)).
- **No further subdivision** at this stage. Scripts that need to distinguish error categories can `grep` the stderr message, which is structured per [ADR-012](012-error-handling.md).
- Adopting `sysexits.h`-style codes (e.g., `EX_USAGE=64`, `EX_IOERR=74`) is deliberately not done — the conventional gain (POSIX semantic codes) is outweighed by the bikeshedding around the mapping and the risk that scripts come to depend on specific numbers. If a real need surfaces, a superseding ADR adds the categorization.

### Prompt format

- Interactive prompts on stderr use a trailing `: ` (e.g., `Summary: `, `Start date: `, `End date (empty for single day): `).
- Prompts do **not** echo the typed line back; the user's terminal echoes naturally.
- The interactive `remove` command lists candidates with 1-based indices on stderr, then prompts `Remove # (or 'q' to cancel): `. `q` or empty input cancels with exit 0.

### Color / styling

- **No color in stdout.** Pipe-friendliness wins.
- **Color in stderr** is permitted if added in the future, but only when `stderr().is_terminal()` is true and the user has not set `NO_COLOR` (per [no-color.org](https://no-color.org) convention). Today the implementation outputs no color anywhere; this rule is recorded for the future.

### What is not covered

- Structured logging (`tracing`, `log`). Not introduced today; subject to a future ADR if integration consumers ask for it.
- Machine-readable warning streams (e.g., JSON warnings to a separate channel). Not introduced today; `--json` output covers structured *success* output only.
- Localized output. CLI help and status messages are English-only per [ADR-007](007-documentation-language-policy.md).

## Consequences

### Positive

- Pipe safety is preserved: `makeholiday list --json | jq` never breaks because status went to stdout by mistake.
- CI and scripts get a deterministic non-interactive path: missing args fail fast rather than hanging on a prompt.
- `--quiet` covers the common "shut up and just do the thing" use case without adding a multi-level scheme that would need a verbose counterpart.
- Exit code `1` for everything keeps scripts simple; the typed error categories in [ADR-012](012-error-handling.md) are available via library use (PRD Goal 4) for callers that need them programmatically.
- No new runtime dep; `IsTerminal` is in `std`.

### Negative

- A user who previously relied on the prompt being available in a piped context (unlikely but possible) loses that — they must pass `--interactive` explicitly.
- `--quiet` does not silence stderr errors; users who want truly silent operation (uncommon for a calendar tool) must redirect `2>/dev/null` themselves.
- A single exit code may frustrate sophisticated scripts; we accept that until someone has a concrete case.

### Migration

1. Add `--quiet` / `-q` as a global flag in `presentation::cli::Cli`.
2. Add `--interactive` and `--no-interactive` as global flags.
3. Implement TTY detection in `presentation::prompt`; gate interactive entry on it.
4. Update each prompt site (`add`, `remove`) to consult the resolved mode.
5. Update [USAGE.md](../USAGE.md) and [USAGE.jp.md](../USAGE.jp.md) with the new flags and the non-interactive failure behavior.
6. Add integration tests for: `add` without `--summary` in a piped context fails; `add --interactive` with piped stdin still prompts.

These lands as a follow-up after the [ADR-009](009-module-layering.md)/[010](010-lib-and-main-separation.md)/[011](011-io-boundary-and-repository.md)/[012](012-error-handling.md) restructure, or alongside it if convenient.
