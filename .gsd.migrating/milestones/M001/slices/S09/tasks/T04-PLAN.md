---
estimated_steps: 13
estimated_files: 4
skills_used: []
---

# T04: Add controller and worker payloads

---
estimated_steps: 8
estimated_files: 4
skills_used:
  - rust-async-patterns
  - tdd
  - write-docs
---
Why: The Downgrader modal needs a Slint-free state machine and owned worker payloads so status, plan, and execution events can cross background boundaries without moving Slint handles or models into worker threads.
Do: Add `src/app/downgrader_controller.rs` and export it from `src/app/mod.rs`. Model phases such as closed, loading status, ready, planning, plan ready, running, completed, and safe error. Implement transitions for open from settings snapshot, status-loaded default target selection, target/option changes, first `Patch All` plan request, second explicit confirmation run request, log/progress updates, completion status refresh, worker failure, stale request rejection, and close/Escape blocking while running. Extend `src/workers/events.rs` with `DowngraderWorkerPayload` variants for status loaded, plan ready, log row, progress, run completed, and safe failure data; re-export from `src/workers/mod.rs`. Keep request ids monotonic so stale plan/run/status events fail closed.
Failure Modes Q5: Worker spawn failure maps to safe visible error and unblocks close unless a run is still active. Stale events are ignored and traced. Malformed UI target/option values revert to the last controller state.
Negative Tests Q7: Cover open/loading, default target from status, settings option changes, plan confirmation gating, no execution request on first click, blocked close while running, stale status/plan/run events, worker failure recovery, and completion re-enabling patch action.
Done when: Controller and worker tests prove the lifecycle and payload round trips through `RecordingEventSink` without Slint types.

## Inputs

- `src/domain/downgrader.rs`
- `src/services/downgrader.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/workers/handoff.rs`

## Expected Output

- `src/app/downgrader_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Verification

cargo test downgrader_controller
cargo test downgrader_worker_payload

## Observability Impact

Adds controller transition tracing for open, plan-ready, confirmed-run, stale-event rejection, blocked close, worker failure, and completion so failures can be localized without GUI automation.
