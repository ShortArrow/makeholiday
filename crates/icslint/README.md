# icslint

`icslint` is the lint tool of the [makeholiday] ecosystem — it flags
RFC 5545 cardinality violations, vendor-only extensions, TEXT-escape
mistakes, and structural quirks in `.ics` files. It is designed for
CI pipelines that consume calendar data from external sources and
need an actionable, machine-readable quality gate.

**Status:** in-tree at `crates/icslint/` of `ShortArrow/makeholiday`
through the v0.2.0 development cycle (ADR-017). Splits to its own
repository once the ecosystem is mutually stable.

## Install

```bash
# In-tree development (current).
cargo install --path crates/icslint

# After the v0.2.0 split + crates.io publish.
cargo install icslint
```

## Quick start

```bash
icslint calendar.ics
icslint *.ics
cat calendar.ics | icslint -
icslint -W -f github calendar.ics    # CI-friendly: GitHub Actions annotations
```

## CLI

```
icslint [OPTIONS] <PATH>...

Options:
  -W, --warnings-as-errors     Treat warnings as errors (exit 2 on any warning)
  -f, --format <FORMAT>        Output format [default: human] [possible values: human, json, github]
  -q, --quiet                  Suppress info-level diagnostics
  -h, --help                   Print help
  -V, --version                Print version
```

Arguments are ICS file paths; `-` reads from stdin. Globbing is the
shell's responsibility; `icslint` does not expand globs internally.

## Output formats

| Format | Stream | Use case |
|---|---|---|
| `human` (default) | stderr | Local development; compiler-style messages |
| `json` | stdout | Pipelines into `jq`, dashboards, custom tooling |
| `github` | stdout | GitHub Actions — emits `::error file=...::` workflow command annotations |

### JSON schema

```json
[
  {
    "file": "calendar.ics",
    "line": 42,
    "rule": "RFC5545/required-uid",
    "severity": "error",
    "message": "VEVENT #1 has no UID..."
  }
]
```

`line` is omitted when the rule cannot localize the finding to a
specific line. Severity values are `info` / `warning` / `error`.

### GitHub Actions

Output is a stream of workflow commands:

```
::error file=calendar.ics,line=42,title=RFC5545/required-uid::VEVENT #1 has no UID...
::warning file=calendar.ics,line=12,title=text/bom::input begins with UTF-8 BOM...
::notice file=calendar.ics,title=structure/empty-calendar::VCALENDAR has no components...
```

`info` severity maps to GitHub's `notice` because workflow commands
only have three severities. Message bodies are percent-encoded
(`%` → `%25`, `\r` → `%0D`, `\n` → `%0A`) so embedded newlines do not
truncate the annotation.

Drop this into a workflow:

```yaml
- name: Lint calendars
  run: icslint -f github calendar/*.ics
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | No diagnostics, or only `info`-level diagnostics emitted |
| `1` | At least one `warning` emitted (and not promoted to error) |
| `2` | At least one `error` emitted, or any warning when `-W` is set |
| `3` | Internal error: file unreadable, or parse failure the tolerant parser could not recover from |

## Rules

`icslint` v0.2.0 ships with **20 rules** across four families.
Severity tiers (`error` / `warning` / `info`) follow ADR-026 §Severity
levels.

| Family | Rules | Reference |
|---|---|---|
| `RFC5545/*` | 8 | [docs/icslint/rules/rfc5545.md](../../docs/icslint/rules/rfc5545.md) |
| `vendor/*` | 4 (1 deferred) | [docs/icslint/rules/vendor.md](../../docs/icslint/rules/vendor.md) |
| `text/*` | 4 (1 deferred) | [docs/icslint/rules/text.md](../../docs/icslint/rules/text.md) |
| `structure/*` | 4 | [docs/icslint/rules/structure.md](../../docs/icslint/rules/structure.md) |

The full rule set is **fixed** in v0.2.0 — there is no configuration
file, no plugin loader, and no user-defined rules. Selective
disable / enable is on the roadmap (a `--no-rules` / `--rules`
allow / deny list) but did not make the v0.2.0 cut.

## Architecture

- `crates/icslint/src/lib.rs` — `lint(source: &str) -> Vec<Diagnostic>` entry point.
- `crates/icslint/src/diagnostic.rs` — `Severity`, `Diagnostic`, `exit_code_for`.
- `crates/icslint/src/walker.rs` — raw `VEVENT` block walker; gives rules access to per-property line numbers and the un-decoded TEXT value.
- `crates/icslint/src/rules/` — one module per rule family.
- `crates/icslint/src/reporter/` — one module per output format.
- `crates/icslint/src/main.rs` — composition root: clap parse → `lint()` → reporter → exit code.

Each rule is a stateless singleton implementing the `Rule` trait
([rules/mod.rs](src/rules/mod.rs)). Adding a rule is one struct + one
registry entry; no plugin machinery is involved.

## Related

- [`ics-core`](https://crates.io/crates/ics-core) — typed RFC 5545 model + parser. `icslint`'s only non-test dependency apart from `clap` and `serde_json`.
- [`icscli`](https://crates.io/crates/icscli) — general-purpose ICS CLI of the same ecosystem.
- [`lazyics`](https://crates.io/crates/lazyics) — `lazygit`-inspired TUI for ICS files.
- [ADR-026](../../docs/design/026-icslint-project-definition.md) — project definition, rule families, severity tiers.

## License

Dual-licensed under `MIT OR Apache-2.0`.

[makeholiday]: https://github.com/ShortArrow/makeholiday
