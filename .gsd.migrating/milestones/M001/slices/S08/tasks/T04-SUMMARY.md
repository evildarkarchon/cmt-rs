---
id: T04
parent: S08
milestone: M001
key_files:
  - ui/scanner_tab.slint
  - ui/main.slint
  - src/main.rs
  - src/app/scanner_controller.rs
  - src/services/autofix.rs
key_decisions:
  - Production Scanner Auto-Fix worker execution uses `AutoFixService::new` and therefore an empty registry, preserving the reference app behavior where normal users see no Auto-Fix buttons.
  - Runtime scheduling carries an owned snapshot plus controller-issued request into `WorkerTaskKind::Patch`, and service rejections are converted into safe failed completion payloads so UI text never depends on raw diagnostics.
duration: 
verification_result: mixed
completed_at: 2026-05-18T09:41:34.251Z
blocker_discovered: false
---

# T04: Exposed gated Scanner Auto-Fix UI wiring and production-empty runtime scheduling with fake-backed lifecycle tests.

**Exposed gated Scanner Auto-Fix UI wiring and production-empty runtime scheduling with fake-backed lifecycle tests.**

## What Happened

Added gated Scanner Auto-Fix properties, callback forwarding, inline Auto-Fix Results fields, and row fixed/check markers through `ui/scanner_tab.slint`, `ui/main.slint`, and `src/main.rs`. The main runtime now maps controller Auto-Fix render state into Slint, prepares owned Auto-Fix worker requests from the controller, schedules them as `WorkerTaskKind::Patch`, executes production workers through `AutoFixService::new` with the empty registry, maps service rejections into safe failed completions, and applies spawn/worker failures through the controller's fail-closed feedback path. Source-contract tests were updated from the S07 prohibition to S08 gated UI assertions, and runtime tests cover production hidden state, tampered callback rejection, fake success/failure lifecycle feedback, worker spawn/failure feedback, and stale completion ignoring. During final lint cleanup I added one narrow `clippy::result_large_err` allow to an existing Auto-Fix validation helper; a second cosmetic clippy-warning cleanup was interrupted by the hard timeout, but the required clippy command had already exited successfully under the current lint configuration.

## Verification

Verified with targeted S08 Slint contract tests, targeted S08 runtime wiring tests, formatting after rustfmt, cargo check, full cargo test, and clippy all-targets/all-features. `cargo clippy --all-targets --all-features` exited successfully but reported two warn-by-default lints in pre-existing service helpers; no deny-level lint failures remained.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test s08_scanner_autofix_slint_contract` | 0 | ✅ pass | 51213ms |
| 2 | `cargo test s08_scanner_autofix_runtime_wiring` | 0 | ✅ pass | 36637ms |
| 3 | `cargo fmt --check` | 1 | ❌ initial rustfmt diffs found | 626ms |
| 4 | `cargo fmt` | 0 | ✅ pass | 572ms |
| 5 | `cargo fmt --check` | 0 | ✅ pass | 539ms |
| 6 | `cargo check` | 0 | ✅ pass with one unused-import warning before cleanup | 23683ms |
| 7 | `cargo fmt --check` | 0 | ✅ pass after import cleanup | 574ms |
| 8 | `cargo check` | 0 | ✅ pass | 19537ms |
| 9 | `cargo test` | 0 | ✅ pass — 287 passed | 44225ms |
| 10 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass with warn-by-default lint warnings | 29464ms |

## Deviations

Applied `cargo fmt` after the initial `cargo fmt --check` reported rustfmt diffs. One narrow clippy allow was added after the final successful clippy run; timeout prevented rerunning clippy after that cosmetic warning cleanup.

## Known Issues

`cargo clippy --all-targets --all-features` exited 0 but reported warn-by-default lints in `src/services/autofix.rs` and `src/services/scanner.rs` during the recorded run. The Auto-Fix validation warning was partially addressed with a local allow; the scanner helper warning remains cosmetic unless lint policy changes warnings to errors.

## Files Created/Modified

- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`
- `src/app/scanner_controller.rs`
- `src/services/autofix.rs`
