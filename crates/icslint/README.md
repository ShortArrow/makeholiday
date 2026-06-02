# icslint

**Status:** Placeholder version `0.0.0`. First real release planned at v0.2.0.

`icslint` is the lint tool of the [makeholiday] ecosystem — it flags
RFC 5545 cardinality violations, vendor-only extensions, TEXT-escape
mistakes, and structural quirks in `.ics` files.

The tool is currently developed in-tree at
<https://github.com/ShortArrow/makeholiday> under `crates/icslint`.
See ADR-026 for the rule families and exit-code contract.

Currently implemented rules (preview):

- `RFC5545/required-uid`
- `RFC5545/required-dtstamp`
- `RFC5545/duplicate-summary`

Sister tools:

- [`ics-core`](https://crates.io/crates/ics-core) — typed RFC 5545 model + parser (ADR-017).
- [`icscli`](https://crates.io/crates/icscli) — general-purpose ICS CLI (renamed from `makeholiday` at v0.2.0).
- [`lazyics`](https://crates.io/crates/lazyics) — `lazygit`-inspired TUI (ADR-025).

## License

Dual-licensed under `MIT OR Apache-2.0`.

[makeholiday]: https://github.com/ShortArrow/makeholiday
