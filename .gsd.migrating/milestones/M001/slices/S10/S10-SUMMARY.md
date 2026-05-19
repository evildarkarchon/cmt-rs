---
id: S10
parent: M001
milestone: M001
provides:
  - A complete live Archive Patcher modal reachable from Overview and Tools.
  - Reusable fail-closed byte-range mutation and latest-header-restore manifest pattern for future file-mutating workflows.
  - Fakeable filesystem write/read-prefix seams and automated tests covering BA2 header validation, patching, restore, stale events, and runtime wiring.
requires:
  - slice: S03
    provides: Fakeable filesystem/platform seams and worker event handoff conventions.
  - slice: S04
    provides: Overview archive diagnostics and archive record state consumed by Archive Patcher candidates.
  - slice: S05
    provides: Tools entrypoint/action-id patterns that now route to the live Archive Patcher workflow.
  - slice: S09
    provides: Downgrader modal/controller/worker safety pattern reused for Archive Patcher mutation flow.
affects:
  - Overview archive diagnostics and Archive Patcher entrypoint
  - Tools Toolkit Utilities metadata and Archive Patcher entrypoint
  - Filesystem platform adapter write seam
  - Worker event/payload dispatch
  - Slint app shell modal exports
key_files:
  - src/domain/archive_patcher.rs
  - src/services/archive_patcher.rs
  - src/platform/filesystem.rs
  - src/app/archive_patcher_controller.rs
  - src/workers/events.rs
  - src/workers/mod.rs
  - src/domain/overview.rs
  - src/services/overview.rs
  - src/domain/tools.rs
  - src/services/tools.rs
  - src/app/tools_controller.rs
  - src/app/overview_controller.rs
  - src/main.rs
  - ui/archive_patcher_window.slint
  - ui/main.slint
  - ui/overview_tab.slint
  - ui/tools_tab.slint
key_decisions:
  - Archive Patcher planning remains Slint-free and read-only, using Overview archive records as the candidate source and stable preview digests for confirmation.
  - Confirmed patching writes an app-owned latest restore manifest before any archive mutation and then performs bounded BA2 version-field byte-range writes with post-write validation.
  - Restore-last-run resolves manifest entries relative to the current Data root and skips stale, moved, malformed, or changed files safely.
  - Runtime entrypoints from Overview and Tools are live internal workflow entrypoints, not deferred utilities, and refresh Overview after accepted patch/restore completions.
patterns_established:
  - Fail-closed mutation workflow pattern: read-only preview plan, digest-confirmation gate, manifest-before-write, per-file revalidation, bounded write, post-write validation, and per-file failure continuation.
  - Slint-free modal controller pattern with request IDs, stage-aware worker events, safe error phases, close blocking during mutation, and UI mutation only after event-loop handoff.
  - Overview-authoritative candidate source pattern: mutation utilities consume typed Overview snapshots instead of scanning paths from UI code.
observability_surfaces:
  - Archive Patcher controller phases and request IDs distinguish load, plan, patch, restore, completion, safe-error, and stale-event states.
  - User-visible candidate, preview-plan, log, progress text, and progress percent surfaces show per-file failures and final success/failure summaries.
  - Worker payloads carry stage-tagged Archive Patcher events so future diagnostics can tell whether a failure occurred during candidate loading, planning, patching, restore, manifest persistence, or Overview refresh.
drill_down_paths:
  - .gsd/milestones/M001/slices/S10/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S10/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S10/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S10/tasks/T04-SUMMARY.md
  - .gsd/milestones/M001/slices/S10/tasks/T05-SUMMARY.md
  - .gsd/milestones/M001/slices/S10/tasks/T06-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-19T04:10:12.536Z
blocker_discovered: false
---

# S10: Archive Patcher Workflow

**Delivered a live Archive Patcher workflow from Overview and Tools with fail-closed BA2 header patching, latest-run restore manifests, and off-thread Slint-safe workers.**

## What Happened

S10 added the Archive Patcher as a real Rust/Slint workflow instead of a deferred utility. The slice began by defining Slint-free Archive Patcher domain models and a read-only preview planner that preserves the reference target inversion, labels, filter behavior, bounded 12-byte BA2 prefix probes, stable preview digests, candidate rows, plan rows, log rows, progress summaries, and restore-manifest payloads. It then implemented safe execution behind the filesystem seam: confirmed patching rebuilds the preview plan, rejects digest mismatches, writes the app-owned latest restore manifest before archive mutation, revalidates Data-root containment and BA2 header facts per file, writes only the BA2 version field with `write_byte_range`, post-validates the result, and continues across per-file failures with reference-style log messages.

The controller and worker lifecycle were added in the same pattern as prior background workflows: `ArchivePatcherController` owns desired version, name filter, candidates, preview confirmation state, manifest availability, progress/logs, safe errors, close blocking, and request-id/stage tracking while worker payloads carry owned archive/data-root/manifest snapshots across the handoff boundary. The Slint modal preserves the reference-shaped `Archive Patcher` surface with `Desired Version`, `v1 (OG)`, `v8 (NG)`, `Name Filter:`, `Patch All`, `About`, candidate list, confirmation/plan area, progress/status log, and `Restore Last Run`. Runtime wiring now opens the modal from both Overview `Archive Patcher...` and Tools `Archive Patcher`, consumes the current Overview archive records and Data path as the only candidate authority, schedules patch/restore worker work off the UI thread, and refreshes Overview after accepted patch/restore completions.

A hard-timeout recovery during T05 left an intermediate partial summary, but T06 reconciled the stale tests and contracts: Archive Patcher is now asserted as an enabled internal utility, the `s10_archive_patcher_runtime_wiring` filter contains a real non-zero test, and adjacent Overview/Tools/Worker expectations reflect the completed S10 workflow. Fresh closeout verification and a security-focused subagent review found no completion blockers.

## Verification

Fresh closeout verification was run through `gsd_exec` run `1cf70896-bf3c-45be-9cff-299e8d80c916` and all required S10 plan gates passed: `cargo test archive_patcher_domain --quiet` (4 passed), `cargo test archive_patcher_service_plan --quiet` (8 passed), `cargo test archive_patcher_executor --quiet` (9 passed), `cargo test archive_patcher_controller --quiet` (7 passed), `cargo test archive_patcher_worker_payload --quiet` (1 passed), `cargo test s10_archive_patcher_slint_contract --quiet` (4 passed), `cargo test s10_archive_patcher_runtime_wiring --quiet` (1 passed), `cargo test overview --quiet` (60 passed), `cargo test tools --quiet` (21 passed), `cargo test worker --quiet` (38 passed), `cargo fmt --check`, `cargo check --quiet`, `cargo test --quiet` (361 passed), and `cargo clippy --all-targets --all-features --quiet`. Clippy exited 0; it emitted non-fatal existing suggestions around test setup. A security subagent review also returned PASS with no blockers for fail-closed BA2 header patching, restore manifest safety, path containment, parsing/mutation behavior, or UI-thread handoff.

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

T05 was interrupted by a hard-timeout recovery and initially recorded weak/partial runtime evidence; T06 completed the reconciliation, added a real `s10_archive_patcher_runtime_wiring` test, and reran the full slice gates successfully. The modal includes the planned safety-oriented `Restore Last Run` action, which is an intentional Rust-port addition beyond the original patch-only reference flow.

## Known Limitations

No manual UAT against a real Fallout 4 install was performed in this closeout; destructive behavior is proven by fake-backed and fixture-style tests. Archive Patcher currently stores only the most recent header restore manifest, has no cancellation, does not browse historical manifests, and only operates on Overview-enabled archive records. `archive_patcher_manifest_path()` falls back to `archive-patcher-latest.json` in the current working directory if the app config directory cannot be created; restore remains fail-closed, but future hardening could surface that directory failure instead.

## Follow-ups

Optional hardening: replace the current-directory restore-manifest fallback with a visible config-directory error. Optional manual validation: run the UAT on a disposable copy of a real Fallout 4 `Data` directory with representative BA2 files.

## Files Created/Modified

- `src/domain/archive_patcher.rs` — Added Archive Patcher domain constants, desired targets, candidate/plan/log/progress models, execution summaries, restore manifest payloads, and digest support.
- `src/services/archive_patcher.rs` — Added read-only planning, fail-closed patch execution, restore-last-run execution, BA2 header validation, manifest handling, and per-file result aggregation.
- `src/platform/filesystem.rs` — Extended the filesystem seam with bounded byte-range write support and real implementation.
- `src/app/archive_patcher_controller.rs` — Added Slint-free modal lifecycle, candidate/filter/plan/progress/log state, request IDs, confirmation gating, close blocking, safe errors, and event application.
- `src/workers/events.rs` — Added Archive Patcher worker stage and payload event types.
- `src/workers/mod.rs` — Added Archive Patcher worker payload variant and helper accessors.
- `src/domain/overview.rs` — Retained Overview archive records and Data-root state needed by Archive Patcher.
- `src/services/overview.rs` — Projected archive record/Data-root facts into Overview snapshots for downstream Archive Patcher use.
- `src/domain/tools.rs` — Updated Tools contract so Archive Patcher is an enabled internal utility.
- `src/services/tools.rs` — Updated Tools service expectations for the live Archive Patcher action.
- `src/app/tools_controller.rs` — Routed Tools Archive Patcher action into the live internal workflow.
- `src/app/overview_controller.rs` — Supported Overview Archive Patcher entrypoint/candidate snapshot use.
- `src/main.rs` — Wired Archive Patcher modal construction, projection, callbacks, worker scheduling, manifest path selection, Overview refresh after completion, and source/runtime tests.
- `ui/archive_patcher_window.slint` — Added the reference-shaped Archive Patcher modal UI contract.
- `ui/main.slint` — Imported/exported Archive Patcher window and row types for runtime use.
- `ui/overview_tab.slint` — Connected the Overview Archive Patcher entrypoint to the live workflow surface.
- `ui/tools_tab.slint` — Connected the Tools Archive Patcher utility entrypoint to the live workflow surface.
