---
id: T04
parent: S03
milestone: M001
key_files:
  - src/workers/events.rs
  - src/workers/handoff.rs
  - src/workers/mod.rs
  - .gsd/DECISIONS.md
key_decisions:
  - D017: WorkerRuntime remains a constructible unit facade that uses the active Tokio runtime handle, emits owned WorkerEvent envelopes through sinks, and returns a typed no-runtime spawn error instead of panicking.
duration: 
verification_result: passed
completed_at: 2026-05-17T10:18:58.304Z
blocker_discovered: false
---

# T04: Added typed worker events, handoff sinks, and a Tokio blocking facade for cancellable off-UI work.

**Added typed worker events, handoff sinks, and a Tokio blocking facade for cancellable off-UI work.**

## What Happened

Replaced the inert worker boundary with owned, reusable background-worker contracts. `src/workers/events.rs` now defines the shared `WorkerEvent` envelope with task identity, task kind, lifecycle status, and typed payload variants for progress, discovery, scan, patch, download, external actions, cancellation, errors, and generic messages. The task kind set distinguishes discovery, scan, patch, download, external process, desktop action, generic, and unknown work; progress carries optional safe text plus optional current/total counts without requiring percentages, rates, or ETA; cancellation has separate requested, acknowledged, and final cancelled events. External process/desktop action outcomes are represented by `ExternalActionPayload`, including operation, target, success/failure state, typed platform failure kind, and safe message, with conversion from existing `DesktopActionResult`.

Added `src/workers/handoff.rs` with `WorkerEventSink`, `RecordingEventSink` for tests/diagnostics, typed handoff errors, and `SlintEventLoopSink` that uses `slint::invoke_from_event_loop` so background workers emit owned events without receiving Slint component handles or models. Updated `src/workers/mod.rs` with `WorkerRuntime::spawn_blocking_task`, `WorkerTaskContext`, `WorkerTaskHandle`, cancellation tokens, typed spawn errors, worker outcomes, panic-to-failure conversion, and decision-point tracing for spawn, completion, cancellation, failure, panic, and handoff failure paths. Unit tests cover the event contracts, optional progress shape, cancellation distinction, desktop-action payload conversion, recording sink ownership, Slint sink construction without a window, off-calling-thread blocking execution, cancellation flow, safe failure events, and no-runtime spawn errors.

## Verification

Ran the required Rust gates after formatting. `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` all passed. `cargo test` reported 87 passing tests, including the new worker event, handoff, cancellation, failure, Slint-sink construction, and blocking facade tests.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 323ms |
| 2 | `cargo check` | 0 | ✅ pass | 11755ms |
| 3 | `cargo test` | 0 | ✅ pass (87 passed) | 28869ms |
| 4 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 11786ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
- `.gsd/DECISIONS.md`
