---
id: T04
parent: S05
milestone: M001
key_files:
  - src/main.rs
  - src/services/tools.rs
  - src/app/tools_controller.rs
key_decisions:
  - Use pure service-level preflight helpers before scheduling Tools/About workers so untrusted callback ids fail closed on the UI side without touching platform adapters.
  - Use surface-specific worker task-id prefixes to map generic worker failure payloads back into safe Tools/About controller feedback while keeping reducers Slint-free.
duration: 
verification_result: passed
completed_at: 2026-05-18T02:11:18.442Z
blocker_discovered: false
---

# T04: Wired the real MainWindow Tools/About callbacks through fail-closed preflight, background workers, production desktop/clipboard adapters, and Slint state projection.

**Wired the real MainWindow Tools/About callbacks through fail-closed preflight, background workers, production desktop/clipboard adapters, and Slint state projection.**

## What Happened

Added shared Tools and About controllers next to the existing Settings and Overview controllers in `src/main.rs`, applied their initial render state before `run()`, and bound the Tools/About Slint callbacks to safe runtime request handlers. Tools callback ids are parsed through the Rust reference contract before scheduling; unknown/deferred/internal ids update visible safe feedback immediately, while enabled external links run through `WorkerRuntime::spawn_blocking_task` and `RealDesktopActions`. About open/copy callbacks are similarly preflighted, with callback-kind mismatch rejected before scheduling so an open callback cannot execute a copy id; valid opens use the desktop adapter and valid copies use the production clipboard adapter off the Slint event thread. Added surface-specific worker sinks, state projection helpers, spawn-failure feedback mapping, worker-failure mapping by task-id prefix, copy-label reset wiring, and structured tracing around scheduling, rejection, spawn failure, applied/ignored worker events, dropped events, copy reset, and poisoned controller locks. Extended the Tools service with pure preflight helpers and a `WorkerUnavailable` rejection kind, and added a default Tools disabled-utility status constant for initial projection.

## Verification

Ran the focused T04 verification (`cargo test s05_runtime_wiring`) after formatting, plus the project quality gates required by the repository: `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features`. All final checks passed. An initial `cargo fmt --check` reported formatting drift after edits; `cargo fmt` was applied and the check was rerun successfully.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 464ms |
| 2 | `cargo check` | 0 | ✅ pass | 17264ms |
| 3 | `cargo test s05_runtime_wiring` | 0 | ✅ pass | 33732ms |
| 4 | `cargo test` | 0 | ✅ pass | 8661ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 19070ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/main.rs`
- `src/services/tools.rs`
- `src/app/tools_controller.rs`
