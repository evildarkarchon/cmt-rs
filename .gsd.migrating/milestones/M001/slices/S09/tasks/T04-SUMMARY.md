---
id: T04
parent: S09
milestone: M001
key_files:
  - src/app/downgrader_controller.rs
  - src/app/mod.rs
  - src/workers/events.rs
  - src/workers/mod.rs
key_decisions:
  - Kept the Downgrader modal lifecycle in a Slint-free reducer with monotonic request IDs and owned worker request payloads so UI code can schedule status/plan/run work without moving Slint handles across threads.
  - Represented Downgrader background handoff as typed `DowngraderWorkerPayload` variants carried by the existing `WorkerEvent` envelope, preserving the project-wide worker sink pattern and enabling `RecordingEventSink` tests.
  - Queued completion status refresh as an owned status worker request instead of doing any IO or service calls inside the controller.
duration: 
verification_result: passed
completed_at: 2026-05-18T11:41:30.375Z
blocker_discovered: false
---

# T04: Added a Slint-free Downgrader modal controller with owned worker payloads and stale-safe lifecycle tests.

**Added a Slint-free Downgrader modal controller with owned worker payloads and stale-safe lifecycle tests.**

## What Happened

Implemented `src/app/downgrader_controller.rs` as a pure reducer for the Downgrader modal lifecycle, covering closed/loading/ready/planning/plan-ready/running/completed/safe-error phases. The controller opens from a persisted `DowngraderSettings` snapshot and optional owned `Fallout4Installation`, assigns monotonic request IDs, applies default target selection from loaded status, handles target and option changes with malformed UI value rejection, gates Patch All so the first click only requests a read-only plan and the second explicit click requests a confirmed run, blocks close/Escape while running, applies log/progress/run completion events, maps spawn and worker failures to safe visible errors, and queues a post-run status refresh without performing IO itself. Extended worker events with `DowngraderWorkerPayload` and `DowngraderWorkerStage` variants for status loaded, plan ready, log row, progress, run completed, and safe failure data, and re-exported them from `src/workers/mod.rs`. Added round-trip tests through `RecordingEventSink` and controller lifecycle/negative tests for stale status/plan/run events, worker failure recovery, malformed UI values, close blocking, and completion re-enabling patch action.

## Verification

Ran the required targeted verification commands `cargo test downgrader_controller` and `cargo test downgrader_worker_payload`; both passed. Also ran `cargo fmt --check`, `cargo check`, and `cargo clippy --all-targets --all-features`; all exited successfully. Clippy still reports a pre-existing warning in `src/services/scanner.rs:1118` for `too_many_arguments`, but no warnings remain from the Downgrader files touched by this task.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 684ms |
| 2 | `cargo test downgrader_controller` | 0 | ✅ pass | 79500ms |
| 3 | `cargo test downgrader_worker_payload` | 0 | ✅ pass | 87647ms |
| 4 | `cargo check` | 0 | ✅ pass | 39998ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (with unrelated existing scanner warning) | 23207ms |

## Deviations

Added `cargo fmt --check`, `cargo check`, and `cargo clippy --all-targets --all-features` beyond the task's two required targeted test commands to catch formatting, compile, and lint issues before completion.

## Known Issues

`cargo clippy --all-targets --all-features` exits 0 but emits an existing warning for `src/services/scanner.rs:1118` (`too_many_arguments`), outside this task's touched Downgrader/controller files.

## Files Created/Modified

- `src/app/downgrader_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
