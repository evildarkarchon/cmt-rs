---
id: S06
milestone: M001
status: ready
---

# S06: F4SE Diagnostics — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver a non-blocking, reference-shaped F4SE diagnostics tab that scans `Data/F4SE/Plugins` DLLs and displays DLL compatibility in the `DLL`, `OG`, `NG`, `AE`, and `Your Game` table with the reference legend.

## Why this Slice

S06 is the next read-only diagnostics slice after Overview, Tools, and About because it reuses the established discovery, filesystem, worker, and Slint projection seams while validating a focused DLL compatibility workflow before the larger Scanner slices consume similar table/progress/error patterns.

## Scope

### In Scope

- Replace the inert F4SE placeholder with the reference-shaped F4SE tab: left compatibility table, `F4SE DLLs` heading, and the `ABOUT_F4SE_DLLS` legend text/icon meanings from the reference.
- Preserve reference table columns and labels: `DLL`, `OG`, `NG`, `AE`, and `Your Game`.
- Trigger a lazy, automatic scan when the F4SE tab is first opened, showing the reference loading text `Scanning DLLs...`; do not add a manual refresh button in this slice.
- Use the checked-in reference source behavior for compatibility classification: inspect F4SE DLL exports/version data as in `CMT/src/utils.py::parse_dll`, and only show support when that source proves it.
- Do not guess support from DLL file names, file versions, mod names, or external heuristics; inconclusive compatibility stays unknown/warning.
- Ignore `msdia*` DLLs and only scan `.dll` files directly under the discovered `Data/F4SE/Plugins` directory, matching the reference scope.
- Keep unreadable, malformed, unsupported-host, or unclassifiable DLLs visible as rows and continue scanning other DLLs; surface safe inline unknown/warning feedback instead of crashing or hiding the file.
- Preserve reference loading failures: `Data folder not found`; `Data/F4SE/Plugins folder not found`; append `Try launching via your mod manager.` when no manager is detected.
- If `Data/F4SE/Plugins` exists but contains no DLLs, render the normal empty table and legend rather than an error.
- If the current game version is not classifiable as OG, NG, or AE, keep DLL facts visible and render `Your Game` as unknown/warning with a clear explanation instead of a misleading hard failure.
- Run filesystem enumeration and DLL inspection off the Slint UI thread and marshal owned scan results back through the existing worker/event-loop handoff pattern.
- Add fake-backed tests for loading states, row classification, parse failures, empty folders, ignored `msdia*` files, unknown game versions, and worker/UI state transitions.

### Out of Scope

- Adding selected-row details, scanner-style details panes, copy actions, open-location actions, or per-DLL context menus.
- Adding a manual F4SE Refresh button or automatically rescanning when Overview refreshes.
- Adding or maintaining a curated compatibility allow-list/database; the user confirmed S06 should use the current checked-in reference source behavior.
- Adding new compatibility heuristics for mods that do not expose the reference-inspected F4SE symbols/version data.
- Changing Scanner F4SE override detection, Address Library checks, or Overview binary/archive/module diagnostics beyond consuming their existing discovery/install-type state.
- Auto-fixing, deleting, moving, patching, or otherwise mutating any DLL or game/mod files.

## Constraints

- `CMT/` remains read-only; inspect it for parity but implement all Rust/Slint behavior outside the submodule.
- Preserve reference labels, loading messages, column order, ignored file rules, and legend text unless an intentional difference is documented.
- The F4SE tab should stay read-only and diagnostic-only in this slice.
- Slint must not perform filesystem enumeration, DLL parsing, or compatibility classification directly; UI callbacks/properties only trigger and display typed Rust state.
- Background scan results must be owned Rust data applied on the Slint event loop; do not move Slint handles/models into worker threads.
- Bad DLLs or parser failures must not panic or abort the entire tab; users should still see the DLL name and any other successful rows.
- Unknown or unsupported compatibility is not the same as confirmed incompatibility; preserve the distinction visibly.
- Keep visual changes conservative: the goal is the reference-shaped F4SE table and legend, not a redesigned diagnostics experience.

## Integration Points

### Consumes

- `CMT/src/tabs/_f4se.py` — Source of truth for tab title, loading text, missing-folder messages, table columns, `msdia*` ignore rule, row icon/tag semantics, and layout shape.
- `CMT/src/utils.py` — Source of truth for `parse_dll` compatibility inputs: F4SE load/preload/query symbols and `F4SEPlugin_Version.compatibleVersions` handling.
- `CMT/src/globals.py` — Source of truth for `ABOUT_F4SE_DLLS` legend copy and icon meanings.
- `src/domain/discovery.rs` — Supplies `Fallout4Installation`, optional `data_path`/`f4se_plugins_path`, install-type labels, and existing reference-compatible loading error constants.
- `src/services/discovery.rs` — Supplies discovered Fallout 4 and mod-manager state for deciding paths and whether to append the mod-manager launch hint.
- `src/platform/filesystem.rs` — Provides fakeable directory enumeration and file reads for DLL scan fixtures and production scans.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — Provide the owned worker event and Slint-safe handoff pattern that F4SE scanning should extend.
- `ui/main.slint` and `ui/f4se_tab.slint` — Existing shell and placeholder surface to replace with F4SE properties/models/callback forwarding.
- `src/main.rs` and existing app controllers — Provide established projection, source-contract test, and worker-result application patterns from Overview/Tools/About.

### Produces

- `src/domain/f4se.rs` — Typed F4SE scan row/status/compatibility models and display/icon semantics independent of Slint.
- `src/services/f4se.rs` — Pure or adapter-backed scanning/classification service for enumerating plugin DLLs and producing typed rows/failures.
- `src/app/f4se_controller.rs` — Slint-free reducer/controller state for lazy scan status, rows, loading errors, stale-result handling, and visible unknown/warning feedback.
- `src/workers/events.rs` — F4SE-specific worker payloads or status variants carrying owned scan results and safe failures.
- `ui/f4se_tab.slint` — Reference-shaped F4SE tab UI with table rows, loading/error/empty states, and legend text.
- `ui/main.slint` — F4SE property/model/callback forwarding through `MainWindow`.
- Rust tests and Slint source-contract tests — Coverage for compatibility mapping, failure/empty states, non-blocking worker handoff, labels/order, and read-only behavior.

## Open Questions

- None at discussion closeout. If a curated compatibility allow-list is requested later, treat it as a future scoped change rather than part of S06.
