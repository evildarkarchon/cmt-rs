---
id: T03
parent: S10
milestone: M001
key_files:
  - src/app/archive_patcher_controller.rs
  - src/app/mod.rs
  - src/workers/events.rs
  - src/workers/mod.rs
key_decisions:
  - Archive Patcher uses the Downgrader-style Slint-free lifecycle with request IDs, stage-aware worker payloads, safe failure states, and mutation close blocking.
  - Archive Patcher controller keeps filesystem mutation out of app state; worker requests carry owned archives/data-root/manifest-path snapshots for off-thread services.
duration: 
verification_result: passed
completed_at: 2026-05-19T02:26:59.247Z
blocker_discovered: false
---

# T03: Added the Slint-free Archive Patcher controller lifecycle and stage-aware worker payload contract.

**Added the Slint-free Archive Patcher controller lifecycle and stage-aware worker payload contract.**

## What Happened

Created `src/app/archive_patcher_controller.rs` and exported it from `src/app/mod.rs`. The controller now tracks closed/loading/ready/planning/plan-ready/patch-running/restore-running/completed/safe-error phases, desired target, name filter, owned Overview archive records, candidate rows, preview plan rows, manifest availability, user-visible log rows, progress text/percent, About dialog state, write-control enablement, close blocking, and active request id/stage. Patch All now follows a plan-first/confirm-second lifecycle, restore is gated on manifest availability, worker spawn failures surface safe retry states, and stale or malformed events are ignored with structured tracing. Extended `src/workers/events.rs` and `src/workers/mod.rs` with `ArchivePatcherWorkerStage`, `ArchivePatcherWorkerPayload`, helper constructors, request-id/stage accessors, and a `WorkerPayload::ArchivePatcher` variant. Added lifecycle tests for stale event rejection, disabled write controls while running, close blocking during mutation, confirmation gating, target/filter recandidate invalidation, spawn failure visibility, restore availability, restore-without-manifest rejection, non-Archive worker-event ignoring, and payload round-trips. An initial borrow-check failure in `confirm_plan` was fixed by copying immutable plan facts before mutating controller state; rustfmt was then applied.

## Verification

Ran the task-required test filters after formatting: `cargo test archive_patcher_controller --quiet` (7 passed), `cargo test archive_patcher_worker_payload --quiet` (1 passed), and `cargo test worker --quiet` (37 passed). Also ran `cargo fmt --check`, which passed after applying `cargo fmt`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test archive_patcher_controller --quiet` | 0 | ✅ pass | 42555ms |
| 2 | `cargo test archive_patcher_worker_payload --quiet` | 0 | ✅ pass | 8579ms |
| 3 | `cargo test worker --quiet` | 0 | ✅ pass | 8535ms |
| 4 | `cargo fmt --check` | 0 | ✅ pass | 711ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/app/archive_patcher_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
