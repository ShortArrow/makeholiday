# ics-core

**Status:** Placeholder version `0.0.0`. First real release planned at v0.2.0.

`ics-core` is the typed iCalendar (RFC 5545) model + parser /
formatter shared by the `ics*` ecosystem:

- [`icscli`](https://crates.io/crates/icscli) — general-purpose ICS CLI (renamed from `makeholiday` at v0.2.0; see ADR-027).
- [`icslint`](https://crates.io/crates/icslint) — lint tool for ICS files (ADR-026).
- [`lazyics`](https://crates.io/crates/lazyics) — `lazygit`-inspired TUI (ADR-025).

The crate is currently developed in-tree at
<https://github.com/ShortArrow/makeholiday> under `crates/ics-core`
(repository name preserved per ADR-027). This `0.0.0` placeholder
reserves the name on crates.io while development continues toward
the first real release (v0.2.0).

See ADR-017 (split timing), ADR-018 (round-trip strategy), ADR-019
(parser implementation), and ADR-001 (vendor extension typing) in
the repository for the public roadmap and design rationale.

## License

Dual-licensed under `MIT OR Apache-2.0`.
