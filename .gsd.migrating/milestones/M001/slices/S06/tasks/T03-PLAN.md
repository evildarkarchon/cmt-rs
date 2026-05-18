---
estimated_steps: 14
estimated_files: 4
skills_used: []
---

# T03: Add F4SE controller and worker payloads

Expected executor skills for task-plan frontmatter: tdd, rust-async-patterns, verify-before-complete.

Why: The F4SE scan must follow the established owned worker-event pattern and keep lazy UI state testable without moving Slint handles or models into worker threads.

Do:
1. Add src/app/f4se_controller.rs and export it from src/app/mod.rs.
2. Add a F4seWorkerPayload variant to src/workers/events.rs and re-export it from src/workers/mod.rs. Use an owned F4seScanSnapshot payload with a scan_id so stale completions can be ignored.
3. Keep WorkerTaskKind as Scan unless a more specific existing convention requires expansion; use an S06 F4SE task id prefix such as s06-f4se-scan: for task matching.
4. Implement F4seController with states for not-started, scanning, ready, and failed or loading-error. It should expose a request_initial_scan method that returns a task only the first time the F4SE tab is activated, a scan_started or spawn_failed transition, and handle_worker_event that applies only matching F4SE payloads.
5. Map WorkerFailure and WorkerSpawnError into safe user-facing status or loading-error messages without leaking raw diagnostics.
6. Preserve prior successful rows if an unrelated or stale worker event arrives; do not clear ready rows on ignored events.
7. Add tests named with f4se_controller and f4se_worker_payload proving lazy activation happens once, scan ids increment monotonically, stale completions are ignored, worker failures and spawn failures surface safely, unrelated payloads are ignored, and owned snapshots round-trip through WorkerPayload.

Failure Modes Q5: worker spawn failure, worker panic failure event, stale result after a newer scan, and unrelated worker event must all leave a safe inspectable controller state.

Load Profile Q6: controller stores only current rows and one active scan id; repeated tab switches must not create unbounded queued scans.

Negative Tests Q7: duplicate activation, stale scan completion, non-F4SE payload, failed worker event, and scan completion with unknown-game warning rows.

Done when: controller and worker tests prove the non-blocking handoff contract independently of Slint.

## Inputs

- `src/domain/f4se.rs`
- `src/services/f4se.rs`
- `src/app/mod.rs`
- `src/app/overview_controller.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`

## Expected Output

- `src/app/f4se_controller.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Verification

cargo test f4se_controller
cargo test f4se_worker_payload

## Observability Impact

Adds inspectable F4SE controller phases, scan ids, and safe failure transitions, giving future agents clear signals for duplicate scheduling, stale results, and worker failures.
