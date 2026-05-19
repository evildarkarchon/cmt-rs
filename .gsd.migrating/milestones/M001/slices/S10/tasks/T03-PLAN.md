---
estimated_steps: 22
estimated_files: 4
skills_used: []
---

# T03: Add controller and worker payload lifecycle

---
estimated_steps: 8
estimated_files: 4
skills_used:
  - rust-async-patterns
  - tdd
  - verify-before-complete
---
Why: The modal needs the same Slint-free lifecycle discipline as S09 Downgrader: request ids, stale-event rejection, disabled write controls while running, close blocking during mutation, and owned worker payloads crossing the handoff boundary.

Do:
1. Create `src/app/archive_patcher_controller.rs` and export it from `src/app/mod.rs`.
2. Model controller phases for unopened/loading or ready, planning, plan-ready awaiting confirmation, patch-running, restore-running, completed, and safe-error states.
3. Track desired target, name filter, candidate rows, preview plan rows, manifest availability, log rows, progress text/percent, write-control enabled state, close allowed state, and the active request id.
4. Add controller intents for open/load, desired-version change, filter change, patch-all click, confirm-plan, restore-last-run, about open/close, worker spawn failure, close attempt, and worker-event reduce.
5. Extend `src/workers/events.rs` and `src/workers/mod.rs` with `ArchivePatcherWorkerStage` and `ArchivePatcherWorkerPayload` variants for candidates loaded, plan ready, log row, progress, patch completed, restore completed, and safe failure.
6. Add helper constructors and stage/request-id accessors mirroring the Downgrader payload style.
7. Add tests for stale payload rejection, controls disabled while running, close blocked while running, plan confirmation gating, filter/target recandidate behavior, worker spawn failure visibility, and restore availability.
8. Keep controller code free of Slint component handles and filesystem mutation.

Done when: controller and worker-payload tests can exercise the entire Archive Patcher modal lifecycle with owned fake payloads and no UI launch.

Failure Modes Q5: Stale worker events are ignored and traced; spawn failures leave the modal visible with safe retry state; close attempts during patch/restore are blocked by controller state.
Load Profile Q6: Controller memory scales with candidate/log row count only and should replace candidate models on target/filter changes instead of accumulating stale rows.
Negative Tests Q7: Stale load/plan/run payloads, confirm without a plan, restore without a manifest, filter changed after plan, close while running, and non-Archive worker event on the sink.

## Inputs

- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/app/downgrader_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Expected Output

- `src/app/archive_patcher_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Verification

cargo test archive_patcher_controller --quiet
cargo test archive_patcher_worker_payload --quiet
cargo test worker --quiet

## Observability Impact

Adds request-id and stage-aware worker signals so runtime logs can identify whether a failure happened during load, planning, patching, restore, or stale-event handling.
