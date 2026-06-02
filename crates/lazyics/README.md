# lazyics

**Status:** Placeholder. First real release planned at v0.2.0.

`lazyics` will be the `lazygit`-inspired terminal UI of the `ics*`
ecosystem — a keyboard-driven TUI for browsing, filtering, and
editing iCalendar (RFC 5545) files. Built on
[`ratatui`](https://crates.io/crates/ratatui).

The tool is currently developed in-tree at
<https://github.com/ShortArrow/makeholiday> (repository name preserved
per ADR-027). ADR-025 supersedes the older `makeholiday-tui` plan
from ADR-022.

Sister tools:

- [`ics-core`](https://crates.io/crates/ics-core) — typed RFC 5545 model + parser (ADR-017).
- [`icscli`](https://crates.io/crates/icscli) — general-purpose ICS CLI (renamed from `makeholiday`; ADR-027).
- [`icslint`](https://crates.io/crates/icslint) — lint tool for ICS files (ADR-026).

## License

Dual-licensed under `MIT OR Apache-2.0`.
