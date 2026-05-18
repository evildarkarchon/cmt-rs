---
id: T04
parent: S06
milestone: M001
key_files:
  - ui/f4se_tab.slint
  - ui/main.slint
  - src/main.rs
key_decisions:
  - Used `TabWidget.current-index` plus a guarded Slint `Timer` as the lazy activation bridge because the tab selection needs to invoke a Rust callback only once without adding a manual refresh button.
  - Kept F4SE worker closures returning owned `F4seScanSnapshot` payloads and rebuilt the Slint row model once per scan result on the event loop.
duration: 
verification_result: passed
completed_at: 2026-05-18T04:43:51.062Z
blocker_discovered: false
---

# T04: Wired the F4SE Slint tab to lazy background scanning with UI-visible status, rows, legend, safe errors, and focused source/runtime tests.

**Wired the F4SE Slint tab to lazy background scanning with UI-visible status, rows, legend, safe errors, and focused source/runtime tests.**

## What Happened

Replaced the inert `ui/f4se_tab.slint` placeholder with a conservative read-only diagnostics layout: a DLL compatibility table with DLL/OG/NG/AE/Your Game headers, severity-colored rows, optional safe detail text, empty/loading states, and the exact F4SE DLL legend text. Updated `ui/main.slint` to export and forward F4SE status/busy/loading/error/unknown-game/row-model properties and to fire a one-shot lazy activation callback when `TabWidget.current-index` reaches the F4SE tab index. Updated `src/main.rs` to create and project a `F4seController`, bind F4SE worker events through `SlintEventLoopSink`, schedule `F4seScanService` via `WorkerRuntime::spawn_blocking_task`, reuse real discovery plus `OverviewCollector` Fallout4.exe facts to classify the current game target, project domain rows into generated `F4seUiRow` values on the Slint event loop, and add tracing around scan scheduling, worker start/completion, discovery failures, current-game classification, counts, missing folders, inspection issues, stale events, ignored events, and spawn failures. Added S06 source-contract tests for the Slint tab and MainWindow wiring plus runtime wiring tests for projection, one-shot activation request behavior, spawn-failure safe mapping, worker completion, unrelated event ignores, empty/error states, and unknown-game warning visibility.

## Verification

Ran the required focused tests (`cargo test s06_f4se_slint_contract`, `cargo test s06_f4se_runtime_wiring`) and `cargo check`; also ran the broader project gates `cargo fmt --check`, full `cargo test`, and `cargo clippy --all-targets --all-features`. All final verification commands passed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test s06_f4se_slint_contract` | 0 | ✅ pass | 34499ms |
| 2 | `cargo test s06_f4se_runtime_wiring` | 0 | ✅ pass | 8527ms |
| 3 | `cargo check` | 0 | ✅ pass | 16630ms |
| 4 | `cargo fmt --check` | 0 | ✅ pass | 498ms |
| 5 | `cargo test` | 0 | ✅ pass | 8539ms |
| 6 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 22254ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `ui/f4se_tab.slint`
- `ui/main.slint`
- `src/main.rs`
