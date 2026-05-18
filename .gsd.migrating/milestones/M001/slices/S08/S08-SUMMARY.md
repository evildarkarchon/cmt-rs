---
id: S08
parent: M001
milestone: M001
provides:
  - Typed Auto-Fix extension seam for future supported Scanner repairs.
  - Fail-closed production behavior that preserves current reference parity and avoids destructive operations.
  - Inline Scanner result feedback for fake/future supported operations.
  - Worker/runtime scheduling pattern that S09/S10 can reuse only with explicit operation-scoped safety gates.
requires:
  - slice: S07
    provides: Scanner typed results, details reducer, worker handoff, read-only actions, and Slint callback surfaces extended by S08.
affects:
  - S09
  - S10
key_files:
  - src/domain/autofix.rs
  - src/domain/scanner.rs
  - src/services/autofix.rs
  - src/services/scanner.rs
  - src/app/scanner_controller.rs
  - src/workers/events.rs
  - ui/scanner_tab.slint
  - ui/main.slint
  - src/main.rs
key_decisions:
  - D028: Model Scanner Auto-Fix as a typed, registry-gated operation contract with an empty production registry for current parity.
  - Eligibility is keyed by retained typed ScannerSolutionKind/AutoFixOperationKey metadata, never by Slint/display-string matching.
  - Future real operations must use plan preview, explicit confirmation, and immediate pre-mutation revalidation before any file mutation.
patterns_established:
  - Empty production registry plus fake injected registries for lifecycle coverage and future extension.
  - Controller-issued owned Auto-Fix requests and scan snapshots cross the worker boundary; Slint handles and models do not.
  - Selection identity combines scan id, result index, and deterministic result fingerprint so stale/tampered requests and completions fail closed.
  - Service rejections and worker failures are converted to safe UI feedback while diagnostics remain available for tests/logs.
observability_surfaces:
  - Auto-Fix service/controller tracing for requested, planned, rejected, scheduled, completed, failed, stale, and worker-spawn-failed flows.
  - Typed rejection/completion diagnostics separate safe user-facing text from raw failure details.
  - Targeted tests verify failure signals, stale-event recovery, worker payload round trips, and source-contract UI gates.
drill_down_paths:
  - .gsd/milestones/M001/slices/S08/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S08/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S08/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S08/tasks/T04-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-18T09:47:38.170Z
blocker_discovered: false
---

# S08: Scanner Auto Fix Actions

**Added fail-closed Scanner Auto-Fix domain, service, controller, worker, Slint, and runtime plumbing while keeping the production registry empty for reference parity.**

## What Happened

S08 extends the S07 Scanner foundation with a typed Auto-Fix architecture without enabling any real production file mutations. The reference app exposes Auto-Fix lifecycle strings, but `CMT/src/autofixes.py::AUTO_FIXES` is empty, so the Rust production path now mirrors that behavior: normal Scanner results do not render an Auto-Fix button unless a typed solution maps to a registered operation. The implementation added `src/domain/autofix.rs` with lifecycle labels, operation keys, operation plans, confirmation/precondition/revalidation contracts, completion/rejection payloads, and deterministic selected-result identities. Scanner results now retain optional typed `ScannerSolutionKind` values so eligibility is driven by typed metadata, never display-string matching.

The Auto-Fix service seam now has an empty production registry and injectable fake registries for tests/future operations. It validates scan id, result index, selected-result identity, typed operation key, target requirements, explicit confirmation, revalidation policy, and preconditions before a runner can execute. Unsupported, stale, tampered, missing-target, unconfirmed, and failed-precondition requests fail closed with safe UI text and no mutation runner call. The controller was extended with Auto-Fix render state keyed by scan id/result index/identity, state transitions for `Auto-Fix`, `Fixing...`, `Fixed!`, and `Fix Failed`, inline `Auto-Fix Results` detail storage, stale completion rejection, row fixed/check markers on success, and safe worker-failure feedback.

The UI and runtime now forward gated Auto-Fix properties and callbacks through `ui/scanner_tab.slint`, `ui/main.slint`, and `src/main.rs`. Runtime scheduling carries an owned controller-issued request and scan snapshot into `WorkerTaskKind::Patch`; production workers call `AutoFixService::new` with the empty registry and convert service rejections into safe failed completion payloads. Fake-backed tests prove the full lifecycle and worker handoff while the real app remains non-mutating and hides unsupported Auto-Fix controls.

## Verification

Fresh closeout verification was run through `gsd_exec` in run `eaa555ac-0265-4c43-a786-ed3d9c347bfa`. All required slice checks passed: `cargo test scanner_autofix_domain` (8 passed), `cargo test scanner_autofix_service` (5 passed), `cargo test scanner_controller_autofix` (6 passed), `cargo test scanner_worker_payload_autofix` (1 passed), `cargo test s08_scanner_autofix_slint_contract` (2 passed), `cargo test s08_scanner_autofix_runtime_wiring` (6 passed), `cargo fmt --check`, `cargo check`, full `cargo test` (287 passed), and `cargo clippy --all-targets --all-features`. Clippy exited 0 and still reports one warn-by-default existing scanner helper warning (`too_many_arguments`), which is non-blocking under the current lint policy.

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

Production Auto-Fix operations remain intentionally empty because the checked-in Python reference registers no operations. Inline details replace the reference modal while retaining the `Auto-Fix Results` heading/copy, as planned for the Rust embedded Scanner details area.

## Known Limitations

`cargo clippy --all-targets --all-features` exits successfully but reports one warn-by-default existing scanner helper warning (`too_many_arguments`). No real production Auto-Fix operation exists yet; future S09/S10 work must not treat S08 as authorization to mutate files without a new scoped plan.

## Follow-ups

When a real Auto-Fix operation is requested, define exact supported solution types, operation plan preview copy, confirmation requirements, backup/undo expectations, sandbox fixtures, and pre-mutation revalidation before adding it to the production registry. Consider separately cleaning up the existing scanner helper `too_many_arguments` clippy warning if lint policy tightens.

## Files Created/Modified

- `src/domain/autofix.rs` — New typed Auto-Fix labels, operation keys, request/completion/rejection payloads, plan preview, confirmation, and precondition contracts.
- `src/domain/scanner.rs` — Scanner results retain typed solution metadata and compute deterministic Auto-Fix selection identities.
- `src/services/autofix.rs` — Empty production registry plus injected fake registry execution service with fail-closed validation and safe completion/rejection outputs.
- `src/services/scanner.rs` — Scanner result construction preserves typed solution kinds where known while display-only strings remain ineligible.
- `src/app/scanner_controller.rs` — Scanner details reducer gained Auto-Fix button/status/details state, request acceptance/rejection, stale completion handling, and worker-failure feedback.
- `src/workers/events.rs` — Added owned Scanner Auto-Fix worker request/completion payloads and task metadata/accessors.
- `ui/scanner_tab.slint` — Added gated Auto-Fix controls, lifecycle labels, inline Auto-Fix Results details, and row fixed/check visual state.
- `ui/main.slint` — Forwarded Scanner Auto-Fix callback/properties through the MainWindow boundary.
- `src/main.rs` — Mapped controller Auto-Fix state to Slint, scheduled owned requests as Patch worker tasks, and applied safe completions/failures.
