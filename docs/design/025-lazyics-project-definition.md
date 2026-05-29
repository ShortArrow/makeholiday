# 025. lazyics — TUI Editor Project Definition

- Status: **Accepted**
- Date: 2026-05-29
- Supersedes: [ADR-022](022-tui-front-end-policy.md) (TUI front-end policy)

## Context

[ADR-022](022-tui-front-end-policy.md) pinned a TUI front-end as a future workspace member under the placeholder name `makeholiday-tui`, with launch trigger left to "maintainer judgment." [PRD §9](../PRD.md#9-roadmap) now stages a TUI as a v0.2.0 deliverable named **lazyics** alongside `ics-core` repo extraction and `icslint`. This ADR records the lazyics project definition — naming, distribution, dependencies, library choice, and the initial release scope — and supersedes ADR-022's "no launch date / maintainer judgment" stance.

The naming is deliberate: `lazyics` mirrors the [`lazygit`](https://github.com/jesseduffield/lazygit) / `lazydocker` / `lazyssh` convention for interactive TUIs that sit on top of a CLI or data format. Brand independence makes the TUI discoverable to users who never heard of `makeholiday` and signals that the TUI is a first-class consumer of `ics-core`, not a side feature of the CLI.

## Decision

### Brand and distribution: separate binary `lazyics`

`lazyics` ships as a **separate binary** named `lazyics`, not as a subcommand of `makeholiday` and not as a feature-gated alternate compilation of the CLI.

- Installation: `cargo install lazyics` (post-publish) or `cargo install --path crates/lazyics` (during in-tree development).
- Repository while in-tree: `crates/lazyics/` in this workspace until [ADR-017](017-workspace-and-ics-core-crate.md) repo split is executed (which is itself part of the v0.2.0 milestone — see [§Workspace lifecycle](#workspace-lifecycle)).
- Repository after v0.2.0 split: separate `ShortArrow/lazyics` repo consuming `ics-core` via crates.io.

Rejected alternatives:

- **`makeholiday tui` subcommand bundling.** Tempting for distribution simplicity but forces every `makeholiday` user to compile `ratatui` and its dependencies, and entangles the CLI's strict `--quiet` / TTY-detection diagnostic policy ([ADR-015](015-diagnostic-output.md)) with TUI runtime concerns. Convenience for the small overlap "CLI user who also wants TUI" does not justify the dependency weight for the larger "scripted CLI user" persona.
- **Cargo feature flag on `makeholiday`.** Same dependency weight problem, plus feature-gated binaries are surprising to install (`cargo install makeholiday --features tui`). Discoverability worse than a separately named tool.

### Initial scope (v0.2.0 release)

The first releasable lazyics covers the [ADR-022](022-tui-front-end-policy.md) §"Initial scope" set:

1. **Interactive list view** — render every `VEvent` (and optionally `VTodo` once [ADR-021](021-vtodo-scope.md) typing lands) from a single calendar file, with:
   - keyboard navigation (`j`/`k` / arrow keys),
   - search-as-you-type filter on `summary`,
   - date-range jump (`g` / `G` to top/bottom; date prompt),
   - status bar showing the file path and event count.
2. **Add via form** — invoked by `a`. A form captures `--summary` / `--start` / `--end` / `--busystatus` / `--class` / `--category` / `--icon` (same flags as `makeholiday add`), then calls the `makeholiday` library's `application::use_cases::add` so behavior matches the CLI exactly.
3. **Edit existing event** — invoked by `e`. Same form as add, pre-populated; submits through `application::use_cases::edit` (added in v0.1.0).
4. **Select-and-remove** — invoked by `d` (delete) or `x`. Multi-select via space, confirm via `D` / Enter. Submits through `application::use_cases::remove`.

Out of scope for v0.2.0:

- VTODO editing (read-only display only per [ADR-021](021-vtodo-scope.md); same scope discipline as the CLI).
- Calendar-level metadata editing (`X-WR-CALNAME`, etc.).
- Multi-file / multi-calendar view.
- Plugin / theming systems. lazyics is one cohesive UI, not a framework.
- Keybinding customization. Defaults only at launch.

### TUI library: `ratatui`

`ratatui` is selected per [ADR-013](013-dependency-policy.md):

- **License** — MIT.
- **MSRV** — current ratatui (0.30 at time of writing) builds on Rust 1.85.
- **Maintenance** — active commits, broad widget library, the de facto Rust TUI choice (succeeded `tui-rs`).
- **Alternatives considered** — `cursive` (smaller widget surface, simpler programming model, but less active). The widget breadth and active maintenance of ratatui win for a project intended as a long-running tool.
- **Surface** — small, focused crate. Pulls in `crossterm` for terminal handling.

### Layered architecture (Clean Architecture continues)

The same 4-layer split [ADR-009](009-module-layering.md) imposes on the CLI applies to lazyics:

```
crates/lazyics/src/
├── main.rs               # Composition Root — wires Repository + opens screen
├── lib.rs                # Optional library surface for unit tests
├── presentation/
│   ├── screens/          # List view, Add form, Edit form, Confirm dialog
│   ├── widgets/          # Reusable view primitives
│   └── keymap.rs         # Key → intent mapping
├── application/
│   └── use_cases.rs      # Re-uses `makeholiday::application::use_cases` via path dep
└── infrastructure/
    └── terminal.rs       # crossterm setup / teardown
```

Notes:

- lazyics depends on the **`makeholiday` library crate** for use cases (`add`, `edit`, `remove`), not directly on `ics-core`'s mutation helpers. This means lazyics and the CLI cannot diverge on what an "edit" means.
- The presentation layer is the only layer that pulls in `ratatui`. The application + infrastructure layers stay reusable for any future front-end.
- `RunContext` from [ADR-015](015-diagnostic-output.md) is supplied with `quiet = true` (TUI owns its own status display) and `allow_prompts = false` (TUI does its own prompting via screens, not via stdin). The CLI's interactive resolver is bypassed.

### Dependency on `makeholiday`

For v0.2.0, lazyics depends on `makeholiday`'s library crate via path dependency:

```toml
# crates/lazyics/Cargo.toml
[dependencies]
ics-core = { path = "../ics-core", version = "0.0.0" }
makeholiday = { path = "../makeholiday", version = "0.1.0" }
ratatui = "0.30"
crossterm = "0.29"
```

When `ics-core` splits to its own repo and crates.io publication (post-v0.2.0 development, pre-v0.2.0 tag), lazyics swaps its path dep for a `version =` dep. When `lazyics` itself splits to its own repo, it depends on `makeholiday` via crates.io as well.

Reusing `makeholiday::application::use_cases::add` from a separate TUI binary forces the makeholiday crate's library surface to stay clean — itself a quality signal for v0.2.0.

### Output and exit codes

- **TTY required.** Without a TTY, lazyics exits 1 with a clear message. There is no `--no-interactive` path.
- **Exit codes**: 0 on normal quit (user pressed `q` / saved-and-quit), 1 on I/O or terminal error, 2 on parse error from the loaded file.
- **Logging**: `tracing` setup behind a `--log <PATH>` flag so users can investigate misbehavior without polluting the screen. No on-screen log panel in v0.2.0.

### Workspace lifecycle

While lazyics develops in-tree at `crates/lazyics/`, the workspace test gate covers all three crates. When [ADR-017](017-workspace-and-ics-core-crate.md)'s repo split executes (planned within the v0.2.0 cycle, after the three tools settle their dependencies):

1. `crates/ics-core/` → its own repository via `git filter-repo --subdirectory-filter`.
2. `crates/lazyics/` → its own repository (same technique, separate split).
3. `crates/icslint/` → its own repository (see [ADR-026](026-icslint-project-definition.md)).
4. `crates/makeholiday/` stays in this repository as the original `makeholiday` repo.
5. Each new repository gets its own CI (cloned from this repo's `.github/workflows/`), `LICENSE-*`, `CHANGELOG`, and crates.io publish step.

History preservation: `git filter-repo` keeps every commit that touched the subdirectory, rewritten so the subdirectory is the new repo root. Pre-existing rename history before files entered `crates/lazyics/` is not followable by default; this is acceptable because lazyics is largely greenfield in v0.2.0.

[ADR-024](024-solo-phase-branching-carve-out.md) solo-phase carve-out **reactivates** at the moment `ics-core` lands in its own repository — the carve-out's first explicit trigger. From that point forward all three repos move to feature-branch + PR ceremony.

## Consequences

### Positive

- The "ICS ecosystem" v0.2.0 narrative gains a concrete branded TUI consumer, not a sub-feature of the CLI.
- `ratatui` selection is documented and discoverable; future contributors don't re-litigate the choice.
- Reusing `makeholiday::application::use_cases::*` mechanically prevents CLI / TUI divergence — both go through the same add / edit / remove logic.
- Path-dependency development followed by crates.io publication mirrors the workflow most consumers will use, dogfooding the published API surface.

### Negative

- ratatui + crossterm pulls in a non-trivial dependency graph for a project that mostly does typed text munging. Acceptable: lazyics's value is precisely the interactive UI; the dependencies pay for it.
- Maintaining a separate binary doubles the documentation surface (installation, getting-started, screenshots). Acceptable: the dual brand is the v0.2.0 goal.
- A user who installs only `makeholiday` and then wants the TUI must run a second `cargo install lazyics`. Mitigation: README / SETUP docs cross-link the two tools.
- Repo split happens twice within v0.2.0 (ics-core then lazyics) — administrative cost.

### Migration

No code changes are required to *accept* this ADR. The implementation lands as the v0.2.0 development cycle progresses:

1. **Add `crates/lazyics/` workspace member** with `Cargo.toml`, `src/main.rs` placeholder, and a minimal `Hello, lazyics` smoke test. `cargo test --workspace` keeps passing.
2. **List view first** — load a `VCalendar` via `FileCalendarRepository`, render events with date column + summary column. Keyboard nav + quit.
3. **Search filter** — filter the events vec by case-insensitive substring on summary.
4. **Add form** — single-pane form, fields match `add` subcommand flags, submission via `application::use_cases::add`.
5. **Edit form** — same shape, pre-populated, via `application::use_cases::edit`.
6. **Remove with multi-select** — selection state on the list view, confirm dialog, via `application::use_cases::remove`.
7. **Final polish** — error display, status bar, color theming (single default), `--log` flag.

Each step lands as its own commit (per [ADR-024](024-solo-phase-branching-carve-out.md) solo-phase carve-out — direct to main during v0.2.0 development, before the ics-core repo split flips the PR ceremony back on).

PRD §9 and ADR-017 references to `makeholiday-tui` are updated to `lazyics` in a separate doc-only commit.
