---
id: T02
parent: S08
milestone: M001
key_files:
  - src/services/autofix.rs
  - src/services/mod.rs
  - src/domain/autofix.rs
key_decisions:
  - Production Auto-Fix registry remains empty to match the Python reference while the service seam supports injected fake/future registries.
  - Auto-Fix eligibility is exposed via a closure-free support catalog keyed by retained typed solution/operation keys, never display strings.
  - Execution fails closed before operation runners on scan mismatch, missing result, stale identity, unsupported operation, target mismatch, missing/declined confirmation, disabled revalidation, or failed preconditions.
duration: 
verification_result: passed
completed_at: 2026-05-18T08:45:39.173Z
blocker_discovered: false
---

# T02: Added a fail-closed Scanner Auto-Fix service with an empty production registry and fake-backed registry lifecycle tests.

**Added a fail-closed Scanner Auto-Fix service with an empty production registry and fake-backed registry lifecycle tests.**

## What Happened

Implemented `src/services/autofix.rs` as the service seam for Scanner Auto-Fix planning and execution. The production constructor uses an empty registry to preserve the reference `AUTO_FIXES = {}` behavior, while injected registries expose a closure-free support catalog and private operation runners for tests/future operations. The service validates scan id, result index, retained selected-result identity, typed operation key, target-path requirements, confirmation, revalidation policy, and operation preconditions before invoking a runner. Successes, operation failures, and pre-mutation rejections return owned domain payloads with safe UI messages, diagnostics, scan/result context, and tracing events for requested/planned/rejected/scheduled/completed/failed/stale flows. Updated the domain Auto-Fix completion/rejection payloads with result-index context and additional rejection kinds needed by the service.

## Verification

Final timeout-recovery verification ran `cargo fmt --check` and `cargo test scanner_autofix_service` against the current tree; both passed. Earlier in the task, broader `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` also exited successfully; clippy reported warnings, including an existing scanner helper warning and Auto-Fix helper warnings that were being narrowed with private helper annotations before the final minimal verification.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 563ms |
| 2 | `cargo test scanner_autofix_service` | 0 | ✅ pass | 37406ms |
| 3 | `cargo check` | 0 | ✅ pass (earlier in task) | 16206ms |
| 4 | `cargo test` | 0 | ✅ pass (earlier in task) | 8402ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass with warnings (earlier in task) | 20861ms |

## Deviations

Due to hard-timeout recovery, final verification focused on the required targeted test and formatting gate instead of rerunning the full suite after the last comment/brace cleanup. Broader cargo check/test/clippy had passed earlier in the same task.

## Known Issues

`cargo clippy --all-targets --all-features` exited successfully earlier but reported warnings; the final timeout-recovery pass did not rerun clippy after the last cleanup.

## Files Created/Modified

- `src/services/autofix.rs`
- `src/services/mod.rs`
- `src/domain/autofix.rs`
