# 016. Configuration File Policy

- Status: **Accepted**
- Date: 2026-05-28

## Context

CLI tools commonly accumulate "settings I want to set once" — default file paths, default values for repeated flags, preferred output formats. Without a stated policy, the project will drift toward an ad-hoc config mechanism the first time someone wants a persistent setting, and the format/location/precedence rules will get bikeshedded under deadline pressure.

The user CLAUDE.md `第3原則` says non-ad-hoc and Tidy First. This ADR makes the configuration-file question explicit before it becomes pressing.

Candidate settings someone might want to persist:

- Default `--file` path (so they don't type `--file ~/holidays.ics` every time).
- Default `--busystatus` for new events.
- Preferred output format (`--json` always vs human always).
- Default category list.
- Editor command for a future `edit` subcommand.

All of these can be expressed today via shell aliases or shell functions wrapping `makeholiday`. None of them are forced into a config-file shape by current functionality.

## Decision

### No configuration file mechanism today

The project does **not** ship a config-file reader. Behavior is fully determined by CLI arguments and built-in defaults (e.g., `--file calendar.ics`).

### No environment variable reading today

`makeholiday` does not read `MAKEHOLIDAY_*` or any other environment variable for behavior. CLI flags are the only input channel.

Users who want persistent preferences use shell-level mechanisms:

```sh
alias mh='makeholiday --file ~/holidays.ics'
mh() { command makeholiday --file "$HOME/holidays.ics" "$@"; }
```

These give the user full control without our needing to define a config schema.

### Future evolution is left open

This ADR records a *not yet*, not a *never*. If a real need surfaces — e.g., multiple commands need shared state, or a setting is too complex for shell-alias-level expression — a future ADR will:

1. Decide whether a config file, environment variables, or both is the appropriate channel.
2. If a file: pick a format (most likely TOML), a location (most likely `$XDG_CONFIG_HOME/makeholiday/config.toml` with `~/.config/makeholiday/config.toml` fallback per the XDG Base Directory Spec), and a precedence order (most likely CLI > env > file > default).
3. If env vars: pick a `MAKEHOLIDAY_*` naming convention.

That future ADR supersedes this one.

### Why not now

- **YAGNI.** No current feature needs persistent state beyond what `--file` already provides.
- **Format / location / precedence bikeshedding is expensive.** TOML vs YAML, XDG vs home-dot, CLI-overrides vs file-overrides — each is a small decision with strong opinions attached. We don't pay that cost until we know what the file would actually carry.
- **Anti-ad-hoc.** Adding a config file "because eventually we'll want it" creates an empty interface that grows by accretion. Pre-committing the interface tends to lock in a shape that does not match the eventual real need.
- **Shell-alias coverage.** For a CLI manipulating a single file, shell-level wrapping covers the persistent-flag use case completely.

## Consequences

### Positive

- Zero added complexity in argument parsing.
- No new dependencies (no `toml`, no `serde_yaml`, no `dirs`).
- No precedence-order ambiguity: CLI flags are the only source of truth.
- Users with multiple calendars (work vs personal) naturally separate them via separate aliases or separate shell functions, which is the conventional shell-level pattern.

### Negative

- Users who expect a config file (because many tools have one) must learn this tool deliberately does not. Documented in [USAGE.md](../USAGE.md) under "Conventions" as a small note.
- Adding a config file later is a behavior addition that must be a versioned change (CHANGELOG entry, possibly a minor bump pre-1.0 / minor or feature flag post-1.0). Acceptable.
- Power-user features (e.g., "use a different default busy status per category") cannot be expressed without a richer mechanism. Those features do not exist today and will arrive with their own ADRs.

### Migration

No code changes required — this ADR records the absence of a mechanism.

A small doc note in [USAGE.md](../USAGE.md) / [USAGE.jp.md](../USAGE.jp.md) explaining "persistent preferences are configured via shell aliases" is the only follow-up. That update is independent of this ADR's acceptance.
