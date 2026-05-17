---
id: S04
milestone: M001
status: ready
---

# S04: Overview Diagnostics & Updates — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver a faithful, responsive Overview tab that populates game, mod-manager, PC specs, binary, archive, module, update, and Overview-problem status from typed Rust discovery and diagnostics.

## Why this Slice

S04 is the first visible consumer of the S03 discovery, platform, desktop, and worker foundations. It validates that Fallout 4 discovery can become user-facing Slint state without blocking the UI, establishes the shared Overview problem feed that the Scanner later consumes through the reference `Overview Issues` setting, and introduces safe refresh/open-link/open-folder behavior before Tools/About and mutation-heavy utility workflows.

## Scope

### In Scope

- Populate Overview automatically using background work, with an initial loading/status state and a Refresh action that reruns the same diagnostics without blocking the UI.
- Preserve the reference top status block: `Mod Manager`, `Game Path`, `Version`, `PC Specs`, plus the refresh affordance.
- Preserve the three reference diagnostic panels and labels as closely as Slint allows:
  - `Binaries (EXE/DLL/BIN)` with base binary/install-type status, Address Library status, and version/detail data.
  - `Archives (BA2)` with General, Texture, Total, Unreadable, `v1 (OG)`, and `v7/8 (NG)` counts/status.
  - `Modules (ESM/ESL/ESP)` with Full, Light, Total, Unreadable, `HEDR v1.00`, `HEDR v0.95`, and `HEDR v????` counts/status.
- Respect the persisted update-source setting and match the reference update UX: no update check when disabled; run only the selected Nexus/GitHub source(s); show the green update banner only when a newer version is found; keep no-update and failed network checks silent except for logs/diagnostics.
- Wire safe open-only helper actions in S04: clicking the game path opens the folder or reports failure, update links open Nexus/GitHub or report failure, and Refresh reruns diagnostics.
- Keep Downgrade Manager and Archive Patcher entry points visually aligned with the reference placement but present them only as deferred/disabled/explanatory controls until their later workflow slices.
- Handle partial or broken discovery inline: missing Data, missing `Fallout4.ccc`, missing `plugins.txt`, unreadable archives/modules, invalid archives/modules, exceeded limits, and failed open actions should be represented as visible panel/problem/status states rather than modal interruptions.
- Build the full typed Overview problem feed now, including problem type, path/relative path where applicable, summary, solution, link/detail metadata, and source marker data needed by the later Scanner `Overview Issues` category.
- Target conservative visual fidelity over redesign: the reference top block plus side-by-side Binaries, Archives, and Modules group panels is the preferred layout target.

### Out of Scope

- Live Downgrade Manager behavior, download/patch execution, backups, delta cleanup, or other destructive downgrade actions; these remain deferred to S09.
- Live Archive Patcher behavior, BA2 mutation, backups, or patch execution; these remain deferred to S10.
- Scanner UI, scan execution, scanner result tree, scanner details pane, or scanner auto-fix behavior; S04 only produces scanner-ready Overview problem data.
- User-visible no-update, checking, or update-failure banners/statuses that diverge from the reference silent update behavior.
- Modal warning dialogs for missing `Fallout4.ccc` or `plugins.txt` in S04; use inline warnings/problem states instead.
- Full Tools/About link surfaces beyond the update links and game-path open action needed by Overview.
- Visual redesign or dashboard modernization before reference parity is achieved.

## Constraints

- `CMT/` is read-only reference material; do not edit, format, move, delete, or generate files under it.
- Preserve labels, panel names, button text, ordering, defaults, warning copy, and problem messages from the reference unless an intentional difference is documented.
- Long-running discovery, version/hash reads, archive/module parsing, filesystem traversal, and update checks must run off the Slint UI thread and marshal owned results back safely.
- S04 should consume existing typed settings, discovery, platform, desktop, and worker seams from S02/S03 rather than duplicating OS access in UI code.
- Keep the application buildable after the slice and run the relevant Rust gates: `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`, plus `git status --short CMT`.
- Prefer typed Rust domain/view-model structures over string-only status maps; display strings should be produced at the UI boundary.

## Integration Points

### Consumes

- `src/domain/settings.rs` / `src/app/settings_controller.rs` — update-source setting controls which update checks run and whether update work is skipped.
- `src/domain/discovery.rs` — installation, install type, optional Data/F4SE paths, archive records, module records, INI state, and discovery errors drive Overview diagnostics.
- `src/domain/mod_manager.rs` — Mod Organizer and Vortex identity/configuration data drives the Mod Manager row, profile text, Vortex partial-support warning, and MO2/Windows 11 24H2 warning.
- `src/services/discovery.rs` — orchestrates game, manager, and PC specs discovery; S04 should schedule it through worker-safe paths rather than querying OS state from Slint callbacks.
- `src/platform/desktop.rs` — safe URL/path open results for game-path clicks and update-link actions.
- `src/workers/*` — background execution, progress/completion/failure events, cancellation state where already available, and Slint event-loop handoff.
- `CMT/src/tabs/_overview.py` — primary reference for Overview layout, panel labels, counts, problem creation, helper buttons, and partial-failure behavior.
- `CMT/src/cm_checker.py` — primary reference for startup update banner behavior and selected update-source handling.
- `CMT/src/globals.py` — reference constants for limits, links, base binary version/hash maps, archive/module limits, and labels/tooltips.
- `CMT/src/enums.py` — reference install-type, archive-version, problem-type, and solution-type labels.
- `CMT/src/helpers.py` — reference `ProblemInfo` and `SimpleProblemInfo` shape.
- `CMT/src/utils.py` — reference update-check behavior and silent failure/no-update handling.

### Produces

- `ui/overview_tab.slint` — reference-shaped Overview UI replacing the placeholder with top status rows, diagnostic panels, update banner surface, refresh control, open-only helper affordances, and deferred utility controls.
- `src/app/*` Overview controller/view-model wiring — dispatches background refresh/update/open actions and projects typed Overview state to Slint properties/models on the UI thread.
- `src/domain/*` or `src/services/*` Overview diagnostics — computes binary/archive/module summaries and scanner-ready Overview problems from discovery/platform facts.
- `src/domain/*` problem model, if not already present — typed Overview problem data compatible with later Scanner `Overview Issues` consumption.
- Tests — source-level Slint contract tests plus domain/view-model/controller tests for label/order fidelity, panel counts/status colors/classes, update-source behavior, silent update failures, inline partial-failure states, open-action failures, and problem-feed contents.

## Open Questions

- Exact deferred presentation for `Downgrade Manager...` and `Archive Patcher...` — current thinking: keep reference placement but disable the controls or show a clear deferred status so users do not think mutation workflows are ready.
- Exact Slint representation for the reference hover-to-version behavior in the Binaries panel — current thinking: preserve the data and use tooltip/detail text if Slint hover text replacement is awkward.
- Whether the first S04 implementation should include every base-file hash/version map from `CMT/src/globals.py` immediately or stage binary classification behind focused tests — current thinking: preserve the full reference map when feasible because partial binary classification would undermine Overview problem accuracy.
