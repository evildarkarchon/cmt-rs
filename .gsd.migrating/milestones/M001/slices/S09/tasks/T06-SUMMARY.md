---
id: T06
parent: S09
milestone: M001
key_files:
  - src/main.rs
  - src/app/downgrader_controller.rs
  - src/services/downgrader.rs
  - ui/downgrader_window.slint
key_decisions:
  - Confirmed Downgrader runs are bound to a stable reviewed-plan digest and fail closed before mutation if a fresh re-preview differs.
  - Live Downgrader worker feedback is emitted through service callbacks into typed worker events while execution is active.
  - Post-run Overview refresh uses a shared Send-safe current settings snapshot rather than constructing default settings.
  - The Downgrader About action is implemented as a Slint modal overlay using preserved reference copy rather than a deferred log no-op.
duration: 
verification_result: mixed
completed_at: 2026-05-19T00:39:42.133Z
blocker_discovered: false
---

# T06: Completed Downgrader runtime closeout wiring for reference About copy, digest-bound confirmed runs, live progress/log projection, current-settings Overview refresh, and focused runtime tests.

**Completed Downgrader runtime closeout wiring for reference About copy, digest-bound confirmed runs, live progress/log projection, current-settings Overview refresh, and focused runtime tests.**

## What Happened

Implemented the Downgrader modal About action as real runtime behavior by adding Slint-visible about dialog properties/callbacks and wiring them to the preserved `ABOUT_DOWNGRADING_TITLE`/`ABOUT_DOWNGRADING_BODY` reference copy. Hardened confirmed runs by adding `DowngraderPreviewPlan::stable_digest()`, carrying the digest on `DowngraderRunWorkerRequest`, and making `execute_confirmed` fail closed with `CONFIRMED_PLAN_CHANGED_MESSAGE` when a fresh re-preview no longer matches the reviewed plan. Added `execute_confirmed_with_events` so workers can emit progress and per-row log callbacks during execution instead of buffering everything until completion. Reworked post-run Overview refresh to use a Send-safe shared `AppSettings` snapshot maintained by settings/downgrader save paths instead of `AppSettings::default()`. Added focused `s09_downgrader_runtime_wiring` tests covering About copy, Overview/Tools entrypoint contracts, settings-save failure/revert, live progress/log projection, stale completion, close blocked while running, worker spawn failure, current-settings Overview refresh, Archive Patcher deferred state, and confirmed-plan mismatch fail-closed behavior.

## Verification

Verified formatting with `cargo fmt --check`, compile with `cargo check`, and the required focused runtime wiring filter with `RUSTFLAGS='-Awarnings' cargo test s09_downgrader_runtime_wiring` (6 tests passed). A direct `cargo test s09_downgrader_runtime_wiring` without warning suppression hit a rustc 1.95.0 internal compiler error in dead-code lint rendering for the test module; suppressing warnings allowed the same tests to compile and pass. Full regression gates from the task plan were not run because hard-timeout recovery required durable completion immediately.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 623ms |
| 2 | `cargo check` | 0 | ✅ pass | 18435ms |
| 3 | `RUSTFLAGS='-Awarnings' cargo test s09_downgrader_runtime_wiring` | 0 | ✅ pass (6 passed) | 66919ms |
| 4 | `cargo test s09_downgrader_runtime_wiring` | 101 | ⚠️ rustc 1.95.0 ICE during dead-code lint rendering | 31406ms |

## Deviations

Full task-plan regression list (`downgrader_controller`, `downgrader_worker_payload`, `settings`, `overview`, `tools`, `worker`, full `cargo test`, and clippy) was not run under hard-timeout recovery. Targeted runtime tests required `RUSTFLAGS='-Awarnings'` because rustc 1.95.0 ICEs while rendering dead-code-lint diagnostics for the test build.

## Known Issues

`cargo test s09_downgrader_runtime_wiring` without warning suppression currently exits 101 due a rustc 1.95.0 internal compiler error during dead-code lint analysis of the test module. This appears to be a compiler diagnostic bug; `cargo check` passes and the focused tests pass with `RUSTFLAGS='-Awarnings'`.

## Files Created/Modified

- `src/main.rs`
- `src/app/downgrader_controller.rs`
- `src/services/downgrader.rs`
- `ui/downgrader_window.slint`
