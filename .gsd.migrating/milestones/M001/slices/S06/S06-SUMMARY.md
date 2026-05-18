---
id: S06
parent: M001
milestone: M001
provides:
  - Typed F4SE domain rows/status/snapshots independent of Slint.
  - Fakeable F4SE scan service and Pelite-based production DLL inspector.
  - Owned F4SE worker payloads and lazy tab activation pattern for future diagnostics tabs.
  - A reference-shaped read-only table/progress/error pattern that S07 Scanner can reuse.
requires:
  - slice: S03
    provides: Fakeable platform filesystem/discovery seams and owned worker handoff pattern.
  - slice: S04
    provides: Overview-derived current Fallout4.exe/game classification inputs.
  - slice: S05
    provides: MainWindow callback/state projection conventions for non-placeholder tabs.
affects:
  - S07 Scanner Read Only Results can reuse the row model, status/error projection, and worker handoff patterns.
  - S08 Scanner Auto Fix Actions should preserve the distinction between unknown/warning and confirmed unsupported states.
  - Future F4SE enhancements should avoid filename/mod-name heuristics unless a new scoped decision changes the compatibility source of truth.
key_files:
  - src/domain/f4se.rs
  - src/services/f4se.rs
  - src/app/f4se_controller.rs
  - src/workers/events.rs
  - src/workers/mod.rs
  - ui/f4se_tab.slint
  - ui/main.slint
  - src/main.rs
  - Cargo.toml
  - Cargo.lock
key_decisions:
  - D026: Combined or non-concrete discovery install states map to `F4seGameTarget::Unknown` so the F4SE `Your Game` column preserves uncertainty with a warning.
  - Production F4SE inspection uses Pelite over raw PE bytes and never OS-loads DLLs.
  - NG/AE compatibility is true-only from known `compatibleVersions` values; unproven support remains unknown/warning.
  - F4SE scan diagnostics travel alongside the UI snapshot for structured tracing without exposing raw DLL content.
  - F4SE worker tasks use `WorkerTaskKind::Scan` and the `s06-f4se-scan:` task-id prefix for stale-result matching.
  - Lazy activation uses `TabWidget.current-index` plus a guarded Slint `Timer` rather than a manual refresh button.
patterns_established:
  - Fail-closed local binary inspection: parse untrusted DLL bytes without loading/executing them and keep malformed rows visible.
  - Proof-only compatibility mapping that distinguishes unknown/unproven from confirmed incompatible.
  - Slint-free controller with monotonic scan ids and owned worker payloads for stale-safe lazy tab work.
  - Reference-shaped Slint source-contract tests for tab labels, columns, loading text, and legend copy.
observability_surfaces:
  - Visible F4SE status/loading/error/unknown-game states in the tab.
  - Structured tracing for scan scheduling, worker start/completion, missing folders, directory/read failures, per-DLL inspection failures, version-data issues, row counts, stale events, ignored events, and spawn failures.
  - Focused automated tests for domain, service, inspector, controller, worker payload, Slint source contract, and runtime wiring.
drill_down_paths:
  - .gsd/milestones/M001/slices/S06/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S06/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S06/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S06/tasks/T04-SUMMARY.md
  - .gsd/milestones/M001/slices/S06/tasks/T05-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-18T04:50:51.704Z
blocker_discovered: false
---

# S06: F4SE Diagnostics

**Delivered a read-only, lazy, non-blocking F4SE DLL compatibility diagnostics tab with reference-shaped columns, legend, safe failures, and owned worker handoff.**

## What Happened

S06 replaced the inert F4SE placeholder with a faithful diagnostics workflow for `Data/F4SE/Plugins`. The implementation added a Slint-free F4SE domain contract with reference-locked labels, loading text, table columns (`DLL`, `OG`, `NG`, `AE`, `Your Game`), icon/legend semantics, row severities, scan snapshots, and current-game warning behavior. The scan service enumerates only direct child `.dll` files under the discovered plugins folder, skips `msdia*` DLLs, preserves empty-plugin-folder behavior as an empty table plus legend, and maps missing Data/plugins folders to the reference messages with the mod-manager hint only when appropriate.

DLL inspection is fail-closed and proof-only. Production parsing uses Pelite over local PE bytes and never loads or executes DLL code. OG support is proven from `F4SEPlugin_Query`; NG/AE support is proven only from `F4SEPlugin_Version.compatibleVersions`; malformed, unreadable, unsupported-host, or unclassifiable DLLs remain visible as warning/unknown rows instead of aborting the scan. Unknown current-game classification keeps DLL facts visible and renders `Your Game` as a warning rather than manufacturing a false compatibility result.

The app layer now has a Slint-free `F4seController` that models not-started, scanning, ready, and loading-error states; lazily schedules the initial scan once; uses monotonic scan ids and the `s06-f4se-scan:` worker task-id prefix; ignores stale or unrelated worker events; and maps spawn/worker failures to safe visible messages. Worker payloads carry owned `F4seScanSnapshot` values through `WorkerPayload::F4se`, preserving the existing event-loop handoff pattern. `ui/f4se_tab.slint`, `ui/main.slint`, and `src/main.rs` now expose the reference-shaped table, status/loading/error/unknown-game states, exact legend text, and one-shot lazy activation via the F4SE tab selection without adding a manual refresh button.

Operational readiness: health is visible through the F4SE status/loading/ready states and row counts, while automated health is covered by focused F4SE tests plus full cargo gates. Failure signals include safe inline UI messages and structured tracing around scan start/completion, missing folders, directory/read failures, per-DLL inspection/version-data failures, stale events, spawn failures, and completion counts. Recovery is user-safe because scans are read-only; users can correct missing paths/plugins and restart/reopen the app for this slice because manual refresh is intentionally deferred. Monitoring gaps are limited to the current desktop context: no persistent in-app diagnostics viewer or manual rescan control is included in S06.

## Verification

Closeout verification passed through `gsd_exec`: `cargo fmt --check` exit 0 (`58c8d4a8-62c7-4be3-8b96-57aa4285aa94`); `cargo check` exit 0 (`556db084-8a52-40ed-9990-e31731ac66a0`); `cargo test` exit 0 with 214 passed, 0 failed (`d9068a96-de9a-4495-9c3d-4905ea20f681`); `cargo clippy --all-targets --all-features` exit 0 (`8aedcb08-88bc-42e0-91f0-c35bc3e98c05`); focused S06 filters `cargo test f4se_domain`, `f4se_scan_service`, `f4se_dll_inspector`, `f4se_controller`, `f4se_worker_payload`, `s06_f4se_slint_contract`, and `s06_f4se_runtime_wiring` all completed in one closeout run with exit 0 (`1fd83145-3085-4c08-8360-41f63ca9e883`). T05 also recorded the required reference-submodule check: `git status --short CMT` exit 0 with no output, confirming the read-only `CMT/` reference tree remained unmodified; the closer did not rerun git because the auto-mode closeout instruction prohibited git commands.

## Requirements Advanced

None.

## Requirements Validated

None.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

None.

## Operational Readiness

None.

## Deviations

No implementation-scope deviations from S06. The closeout did not rerun `git status --short CMT` because this verification-lane instruction explicitly prohibited git commands; T05 had already recorded that required command with exit 0 and no output.

## Known Limitations

The tab is intentionally read-only and one-shot lazy for S06: no manual refresh, selected-row details, copy/open actions, or context menus are included. Compatibility remains unknown/warning when the reference-inspected PE exports/version data do not prove support. Real GUI human UAT was not executed in this automated closeout.

## Follow-ups

S07 should reuse the table/progress/error and worker handoff patterns for Scanner read-only results. Future slices may add Scanner actions, Downgrade Manager, Archive Patcher, and optional F4SE row details or manual refresh only if explicitly scoped.

## Files Created/Modified

- `src/domain/f4se.rs` — Added Slint-free F4SE domain contract, constants, compatibility rows/cells, snapshots, statuses, and render helpers.
- `src/domain/mod.rs` — Exported the F4SE domain module.
- `Cargo.toml` — Added Pelite dependency for local PE/DLL export inspection.
- `Cargo.lock` — Refreshed dependency lockfile for Pelite.
- `src/services/f4se.rs` — Added fakeable scan service, diagnostics, inspector trait, and Pelite production inspector.
- `src/services/mod.rs` — Exported the F4SE service module.
- `src/app/f4se_controller.rs` — Added lazy/stale-safe Slint-free controller for F4SE scan state and worker event application.
- `src/app/mod.rs` — Exported the F4SE controller module.
- `src/workers/events.rs` — Added F4SE worker payload variant carrying owned scan snapshots.
- `src/workers/mod.rs` — Re-exported F4SE worker payload support.
- `ui/f4se_tab.slint` — Replaced placeholder with reference-shaped read-only diagnostics table, status/error/empty states, and legend.
- `ui/main.slint` — Forwarded F4SE properties/models and lazy activation callback through MainWindow.
- `src/main.rs` — Wired F4SE controller, discovery/current-game classification, background scan scheduling, event-loop projection, and runtime tests.
