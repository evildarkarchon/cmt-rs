---
estimated_steps: 7
estimated_files: 5
skills_used: []
---

# T05: Wire Scanner runtime and final gates

Expected executor skills: rust-async-patterns, tdd, review, verify-before-complete.

Why: The slice is only user-meaningful once MainWindow callbacks schedule the real scanner worker off the Slint UI thread, project controller state back into Slint models, and route read-only actions through fakeable platform adapters with safe feedback.

Do: Wire `ScannerController` into `src/main.rs` alongside Overview, F4SE, Tools, and About controllers. Initialize scanner checkbox state from persisted settings. Bind checkbox callbacks, `Scan Game`, row selection, file-list toggling, copy-details, open-path, open-url, and copy-url callbacks. On scan start, persist scanner toggles through `SettingsController`, apply the controller loading state, then schedule one `WorkerRuntime::spawn_blocking_task` with `WorkerTaskKind::Scan`. Inside the worker, emit `Refreshing Overview...`, rebuild discovery and Overview facts/problem feed using existing real adapters, emit `Building mod file index...` when a Data scan is needed, pass progress callbacks into `ScannerScanService`, and complete with an owned `ScannerWorkerPayload`. Keep Slint handles/models out of the worker closure. Project grouped rows/details/actions into `ScannerUiRow` and related Slint properties, using `ModelRc`/`VecModel` patterns already present. Route read-only actions through `RealDesktopActions` and `RealClipboardActions` in workers or safe synchronous action services, then feed results back through the controller. Add `s07_scanner_runtime_wiring` tests for startup projection from settings, scan scheduling state, progress/completion handling, stale event rejection, zero-result projection, selected detail actions, action failure feedback, and safe worker payload application. Run final cargo gates.

Done when: Scanner works end to end through the real app wiring at test/runtime level, remains read-only, old result/detail state is cleared on new scan, and all final verification commands pass.

Failure Modes Q5: no active Tokio runtime reports a safe start failure; discovery failure still yields useful Overview/scanner error rows; worker panic is converted by WorkerRuntime to safe failure; desktop/clipboard failures show inline scanner feedback; stale scan events are ignored by scan id.
Load Profile Q6: long scans run on Tokio blocking pool with progress events and owned payloads; UI model updates happen only on the Slint event loop.
Negative Tests Q7: no runtime scan when settings save reverts or all toggles are off, missing Data path, Vortex Data-only manager, failed open/copy adapters, stale worker completion, and zero-result scan.

## Inputs

- `src/domain/scanner.rs`
- `src/services/scanner.rs`
- `src/app/scanner_controller.rs`
- `src/app/settings_controller.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
- `src/services/discovery.rs`
- `src/services/overview.rs`
- `src/services/overview_collector.rs`
- `src/platform/filesystem.rs`
- `src/platform/desktop.rs`
- `src/platform/clipboard.rs`
- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Expected Output

- `src/main.rs`
- `src/app/scanner_controller.rs`
- `src/services/scanner.rs`
- `src/workers/events.rs`
- `ui/main.slint`

## Verification

cargo test s07_scanner_runtime_wiring
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Observability Impact

Completes runtime observability with scan/action task ids, structured scheduling/start/progress/completion/failure logs, stale-event diagnostics, visible safe action errors, and full cargo-gate evidence.
