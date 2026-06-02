# icscli

**Status:** Placeholder. First real release planned at v0.2.0.

`icscli` will be the general-purpose iCalendar (RFC 5545) CLI of the
[makeholiday] ecosystem — read, write, query, and edit ICS files.

The tool is currently developed in-tree under the name `makeholiday`
(domain-specific entry point for holiday-calendar generation) and will
be renamed at v0.2.0 once the broader ecosystem (`ics-core`,
`icslint`, `lazyics`) launches.

Sister tools:

- [`ics-core`](https://crates.io/crates/ics-core) — typed RFC 5545 model + parser (ADR-017).
- [`icslint`](https://crates.io/crates/icslint) — lint tool for ICS files (ADR-026).
- [`lazyics`](https://crates.io/crates/lazyics) — `lazygit`-inspired TUI (ADR-025).

See ADR-017 (ecosystem split timing) in the repository for the public
roadmap.

## License

Dual-licensed under `MIT OR Apache-2.0`.

[makeholiday]: https://github.com/ShortArrow/makeholiday
