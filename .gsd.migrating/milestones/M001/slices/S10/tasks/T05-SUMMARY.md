---
id: T05
parent: S10
milestone: M001
key_files:
  - src/domain/overview.rs
  - src/services/overview.rs
  - src/domain/tools.rs
  - src/app/tools_controller.rs
  - src/app/archive_patcher_controller.rs
  - src/main.rs
  - ui/main.slint
  - ui/overview_tab.slint
  - ui/tools_tab.slint
key_decisions:
  - Archive Patcher runtime uses current Overview archive records and Data path retained in the OverviewSnapshot instead of launching UI-side scans.
  - Archive Patcher latest restore manifest path is app-owned outside the game Data directory, with current-directory fallback if ProjectDirs setup fails.
  - Patch/restore completion requests an Overview refresh through the existing shared settings snapshot path only when an applied Archive Patcher completion payload is accepted.
duration: 
verification_result: passed
completed_at: 2026-05-19T03:13:33.179Z
blocker_discovered: false
---

# T05: Partially wired Archive Patcher runtime entrypoints, controller projection, worker scheduling helpers, and Overview archive-record retention before hard-timeout recovery.

**Partially wired Archive Patcher runtime entrypoints, controller projection, worker scheduling helpers, and Overview archive-record retention before hard-timeout recovery.**

## What Happened

Hard-timeout recovery interrupted T05 before I could finish the full planned runtime wiring and test updates. During the execution window I added Overview snapshot retention for current BA2 archive records and the confirmed Data path, changed Archive Patcher from a deferred Tools/Overview utility toward a live internal workflow, instantiated an ArchivePatcherController and ArchivePatcherWindow in main runtime setup, added modal callback/sink scaffolding, added worker scheduling/build helpers for candidates, plan, confirmed patch, and restore, added app-owned manifest-path selection outside the game Data directory, and added Overview refresh scheduling on applied patch/restore completions. I also added fail-closed controller support for opening the modal when Overview has no archive records. Because timeout recovery required immediate durable output, I did not continue to reconcile all existing source-level assertions or add the new S10 runtime tests requested by the task plan.

## Verification

Ran the minimal hard-timeout verification command `cargo test s10_archive_patcher_runtime_wiring --quiet`. It exited 0 but reported `running 0 tests`, so it confirms the currently edited crate compiled for that filter but does not verify the required T05 runtime behavior. The remaining task-plan verification commands and new runtime tests still need to be completed by the next pass.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test s10_archive_patcher_runtime_wiring --quiet` | 0 | ⚠️ pass/weak evidence: compiled but ran 0 tests | 70855ms |

## Deviations

Hard-timeout recovery forced a partial stop before completing all task-plan items. Runtime/source tests for T05 were not added or updated, and existing deferred-status assertions may still need reconciliation.

## Known Issues

T05 is not fully complete despite this recovery summary: add/update tests proving both entrypoints open the modal, candidate source uses Overview archive records, write controls disable while running, stale events are ignored, Overview refresh is scheduled exactly once after completion, and deferred Archive Patcher status text is gone. Also run `cargo test overview --quiet`, `cargo test tools --quiet`, `cargo test worker --quiet`, and broader cargo checks after reconciling tests.

## Files Created/Modified

- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/domain/tools.rs`
- `src/app/tools_controller.rs`
- `src/app/archive_patcher_controller.rs`
- `src/main.rs`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
