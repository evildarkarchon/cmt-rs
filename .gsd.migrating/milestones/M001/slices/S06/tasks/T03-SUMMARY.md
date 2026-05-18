---
id: T03
parent: S06
milestone: M001
key_files:
  - src/app/f4se_controller.rs
  - src/app/mod.rs
  - src/workers/events.rs
  - src/workers/mod.rs
key_decisions:
  - Kept F4SE scan tasks under `WorkerTaskKind::Scan` and matched them with the `s06-f4se-scan:` task-id prefix.
  - F4SE worker completions carry owned `F4seScanSnapshot` values boxed inside `F4seWorkerPayload` so stale filtering happens in the Slint-free controller.
  - Visible F4SE failure state uses safe messages while raw diagnostics are only emitted through structured tracing.
duration: 
verification_result: passed
completed_at: 2026-05-18T04:17:03.601Z
blocker_discovered: false
---

# T03: Added a Slint-free F4SE controller and owned worker payloads for lazy, stale-safe DLL scan handoff.

**Added a Slint-free F4SE controller and owned worker payloads for lazy, stale-safe DLL scan handoff.**

## What Happened

Implemented `src/app/f4se_controller.rs` and exported it from `src/app/mod.rs`. The controller models not-started, scanning, ready, and loading-error phases around `F4seScanSnapshot`, exposes lazy `request_initial_scan` behavior that only schedules once, supports explicit `request_scan` for future refreshes, tracks monotonic scan ids, and uses `s06-f4se-scan:` task ids with `WorkerTaskKind::Scan` for matching. It applies only matching F4SE completion/failure/running events, ignores stale or unrelated events without clearing ready rows, and maps spawn/worker failures into safe visible status messages while keeping raw diagnostics in tracing. Added `WorkerPayload::F4se(F4seWorkerPayload::ScanCompleted { scan_id, snapshot })` and re-exported `F4seWorkerPayload` so worker closures can round-trip owned snapshots across the handoff boundary without Slint handles.

## Verification

Verified the new controller and worker payload contracts with focused tests, then ran formatting, compile, full test, and clippy gates. Focused coverage includes lazy duplicate activation, monotonic scan ids, latest-result application, stale completion preservation, worker failure safety, spawn failure safety, unrelated/non-F4SE payload ignoring, unmatched failed scan ignoring, and unknown-game warning row visibility.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test f4se_controller` | 0 | ✅ pass | 38450ms |
| 2 | `cargo test f4se_worker_payload` | 0 | ✅ pass | 8464ms |
| 3 | `cargo fmt --check` | 0 | ✅ pass | 522ms |
| 4 | `cargo check` | 0 | ✅ pass | 17089ms |
| 5 | `cargo test` | 0 | ✅ pass | 8858ms |
| 6 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 19675ms |

## Deviations

Added a small public `request_scan` helper in addition to the planned `request_initial_scan` so future explicit rescans and stale-result handling can use the same monotonic id path; the lazy initial activation still returns work only once.

## Known Issues

None.

## Files Created/Modified

- `src/app/f4se_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
