---
id: S04
milestone: M001
status: draft
---

# S04: Overview Diagnostics & Updates — Context Draft

## Goal

Deliver a faithful, responsive Overview tab that populates game, mod-manager, PC specs, binary, archive, module, update, and Overview-problem status from typed Rust discovery/diagnostics.

## Why this Slice

S04 validates the S03 discovery/platform/worker foundations through a visible tab and produces the shared Overview problem feed that later Scanner work consumes via the reference `Overview Issues` setting. It also establishes safe open-link/open-folder behavior before Tools/About and mutation-heavy utility workflows.

## Scope

### In Scope

- Populate Overview automatically using background work, with a visible loading/status state and a Refresh action that reruns the same diagnostics without blocking the UI.
- Respect the persisted update-source setting and match reference update UX: no check when disabled; show the green update banner only when a newer Nexus/GitHub version is found; keep no-update or failed network checks silent except logs/diagnostics.
- Wire safe open-only helper actions: game path open, update links, and Refresh. Downgrade Manager and Archive Patcher entry points should be present only as deferred/disabled/explanatory controls until their later workflow slices.
- Handle partial/broken discovery inline: missing Data, missing `Fallout4.ccc`, missing `plugins.txt`, unreadable files, invalid archives/modules, and failed open actions should appear as visible panel/problem/status states rather than modal interruptions.
- Build the full typed Overview problem feed now, including summary, solution, and link/detail metadata needed by Scanner's later `Overview Issues` category.
- Target reference layout fidelity: top status block (`Mod Manager`, `Game Path`, `Version`, `PC Specs`, Refresh) plus `Binaries (EXE/DLL/BIN)`, `Archives (BA2)`, and `Modules (ESM/ESL/ESP)` panels with labels/action placement as close to the Tkinter reference as Slint allows.

### Out of Scope

- Live Downgrade Manager and Archive Patcher workflows; destructive or mutation-heavy behavior remains deferred to S09/S10.
- Scanner UI or scan execution, except for producing scanner-ready Overview problem data.
- User-visible no-update or network-failure update statuses that diverge from the reference silent behavior.
- Modal warning dialogs for missing `Fallout4.ccc`/`plugins.txt` in S04; use inline warnings instead.
- Visual redesign/dashboard modernization beyond conservative Slint fidelity.

## Constraints

- `CMT/` remains read-only and must be used only as reference material.
- Preserve user-facing labels/messages from `CMT/src/tabs/_overview.py`, `CMT/src/cm_checker.py`, `CMT/src/enums.py`, and related constants unless an intentional difference is documented.
- Long-running discovery, version/hash reads, archive/module parsing, and update checks must run off the Slint UI thread and marshal owned results back safely.
- S04 should consume existing typed settings, discovery, platform, desktop, and worker seams from S02/S03 rather than duplicating OS access in UI code.

## Integration Points

### Consumes

- `src/domain/settings.rs` / `src/app/settings_controller.rs` — update-source setting controls which update checks run.
- `src/domain/discovery.rs` — installation, install type, archive/module records, INI state, and discovery errors drive Overview panels/problems.
- `src/services/discovery.rs` — game, manager, and PC specs discovery orchestration.
- `src/platform/desktop.rs` — safe URL/path open results for update links and game-path clicks.
- `src/workers/*` — background execution and UI-thread-safe event handoff.
- `CMT/src/tabs/_overview.py`, `CMT/src/cm_checker.py`, `CMT/src/globals.py`, `CMT/src/enums.py`, `CMT/src/helpers.py`, `CMT/src/utils.py` — reference labels, layout, update banner, problem messages, and status rules.

### Produces

- `ui/overview_tab.slint` — reference-shaped Overview UI with status rows, diagnostic panels, update banner, refresh, and deferred utility controls.
- `src/app/*` Overview controller/view-model wiring — projects typed domain diagnostics to Slint properties/models and dispatches Refresh/open actions.
- `src/domain/*` or `src/services/*` Overview diagnostics — computes binary/archive/module summaries and scanner-ready Overview problems.
- Tests — source-level UI contract tests plus domain/view-model tests for counts, statuses, update-source behavior, inline failure states, and problem-feed contents.

## Open Questions

- Exact disabled/deferred presentation for Downgrade Manager and Archive Patcher buttons — current thinking: keep reference button placement but disable or show a clear "reserved for later" status so users do not think mutation workflows are ready.
- Exact Slint representation for reference hover-to-version behavior in the Binaries panel — current thinking: preserve the data and use tooltip/detail text if Slint hover replacement is awkward.
