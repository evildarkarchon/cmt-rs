---
id: T03
parent: S08
milestone: M001
key_files:
  - src/app/scanner_controller.rs
  - src/workers/events.rs
key_decisions:
  - Production Scanner Auto-Fix catalog remains empty by default in `ScannerController::new`, with support injection limited to tests/future wiring.
  - Auto-Fix controller state is keyed by scan id, flat result index, and computed selection identity so selection changes and newer scans make old completions stale.
  - Worker failure UI text uses safe failure messages only; raw diagnostics are retained for logs/tests but not promoted into primary Auto-Fix result text.
duration: 
verification_result: passed
completed_at: 2026-05-18T09:10:17.666Z
blocker_discovered: false
---

# T03: Wired Scanner Auto-Fix lifecycle state through the pure controller and owned worker payloads with fail-closed selection, request, completion, and worker-failure handling.

**Wired Scanner Auto-Fix lifecycle state through the pure controller and owned worker payloads with fail-closed selection, request, completion, and worker-failure handling.**

## What Happened

Extended `ScannerController` with an empty-by-default `AutoFixSupportCatalog`, an injected fake-catalog constructor for tests, and per-result Auto-Fix render state keyed by scan id, flat result index, and computed selection identity. Selecting a row now exposes Auto-Fix button/status/details only when the retained typed scanner solution maps to an injected supported operation; the default production constructor still uses an empty catalog, so normal users see no Auto-Fix controls.

Added `ScannerAutoFixWorkerRequest`, S08 `s08-scanner-autofix:` Patch task metadata, task parsing, and request transitions. `request_selected_auto_fix` rejects no selection, unsupported/missing support, stale scan/result identity, and operation mismatches with safe UI status text and structured tracing; accepted requests move the selected state to `Fixing...`, disable the button, clear stale detail, and return only owned worker payload data.

Added Auto-Fix completion and worker-failure reducers. Matching completions set `Fixed!` or `Fix Failed`, store inline `Auto-Fix Results` details, set row fixed/checked state only on success, re-enable the button after terminal states, and ignore stale completions after selection changes or newer scans. Worker failures for S08 Patch tasks now map to safe `Fix Failed` feedback while preserving raw diagnostics only in diagnostic fields.

Extended `ScannerWorkerPayload` with an owned `AutoFixCompleted` variant, constructor, scan-id accessor support, and completion accessor tests. No Slint handles, filesystem adapters, or operation closures enter the controller.

## Verification

Ran formatting and targeted task verification plus broader compile/regression gates. `cargo test scanner_controller_autofix` passed 6 lifecycle tests covering empty production catalog, supported fake requests, fixing state, success/failure details, stale completion rejection, row fixed/check state, and safe worker-failure messages. `cargo test scanner_worker_payload_autofix` passed the owned Auto-Fix worker payload round-trip test. `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features` all exited successfully; clippy still reports warning-level pre-existing findings outside the new controller payload path.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 555ms |
| 2 | `cargo test scanner_controller_autofix` | 0 | ✅ pass | 38027ms |
| 3 | `cargo test scanner_worker_payload_autofix` | 0 | ✅ pass | 8565ms |
| 4 | `cargo check` | 0 | ✅ pass | 21705ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (warning-level findings only) | 26455ms |
| 6 | `cargo test` | 0 | ✅ pass | 8617ms |

## Deviations

`src/workers/mod.rs` did not need changes because `ScannerWorkerPayload` was already publicly re-exported and the new Auto-Fix payload helper is an associated constructor on that exported type.

## Known Issues

`cargo clippy --all-targets --all-features` exits 0 but still emits warning-level findings for an existing large `Err` return in `src/services/autofix.rs` and an existing too-many-arguments helper in `src/services/scanner.rs`; these were not part of this task's controller/worker payload wiring.

## Files Created/Modified

- `src/app/scanner_controller.rs`
- `src/workers/events.rs`
