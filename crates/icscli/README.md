# icscli

General-purpose iCalendar (RFC 5545) CLI — read, write, query, and
edit `.ics` files. Part of the `ics*` ecosystem:

- [`ics-core`](https://crates.io/crates/ics-core) — typed RFC 5545 model + parser (ADR-017).
- [`icscli`](https://crates.io/crates/icscli) — this crate.
- [`icslint`](https://crates.io/crates/icslint) — lint tool for ICS files (ADR-026).
- [`lazyics`](https://crates.io/crates/lazyics) — `lazygit`-inspired TUI (ADR-025).

> `icscli` was previously named `makeholiday` (v0.1.x). Renamed at v0.2.0 to align with the rest of the ecosystem — see [ADR-027](../../docs/design/027-makeholiday-to-icscli-rename.md). The repository name remains `ShortArrow/makeholiday`.

## Install

```bash
# In-tree development (current).
cargo install --path crates/icscli

# After the v0.2.0 release.
cargo install icscli
```

## Quick start

```bash
icscli init
icscli add --summary 元日 --start 2026-01-01
icscli list
icscli edit 1 --summary 元日（新名称）
icscli remove 1
```

See the [usage reference](../../docs/USAGE.md) for the full command
catalogue, and the [setup guide](../../docs/SETUP.md) for platform
prerequisites.

## License

Dual-licensed under `MIT OR Apache-2.0`.
