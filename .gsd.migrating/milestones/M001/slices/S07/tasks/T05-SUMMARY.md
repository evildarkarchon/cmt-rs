---
id: T05
parent: S07
milestone: M001
key_files:
  - src/main.rs
  - src/services/scanner.rs
  - Cargo.toml
key_decisions:
  - Scanner Scan Game requests persist toggles at scan start and only schedule workers after the settings save succeeds.
  - Scanner scan workers emit progress and completion through owned worker events; Slint handles/models remain outside worker closures.
  - Scanner read-only actions execute through fakeable clipboard/desktop adapters and reduce safe feedback through ScannerController.
  - Cargo manifest rust-version is 1.85.0 to match edition 2024's Cargo validation floor while remaining compatible with the user's Rust 1.95 toolchain.
duration: 
verification_result: passed
completed_at: 2026-05-18T07:17:28.309Z
blocker_discovered: false
---

# T05: Wired the Scanner tab to the real blocking scan runtime, progress events, safe read-only actions, and final verification gates.

**Wired the Scanner tab to the real blocking scan runtime, progress events, safe read-only actions, and final verification gates.**

## What Happened

Connected MainWindow Scanner callbacks to the Slint-free ScannerController and SettingsController. Scanner checkbox state now initializes from persisted settings, scan start persists the current toggle snapshot before scheduling work, failed saves revert the UI state and skip scheduling, and valid Scan Game requests clear stale result/detail/action state before starting a single WorkerRuntime blocking scan task. The scan worker rebuilds discovery and Overview facts with the real filesystem/registry/process adapters, feeds Overview problems/modules/archives into ScannerScanService, emits safe progress through WorkerTaskContext, logs structured discovery/overview/scanner diagnostics, and returns an owned ScannerWorkerPayload guarded by scan id. Scanner result selection, file-list toggling, Copy Details, Open Path, Open URL, and Copy URL are now wired through controller-owned selected action descriptors; read-only copy/open work uses RealClipboardActions and RealDesktopActions in workers and displays only safe feedback. Added targeted s07_scanner_runtime_wiring tests for startup projection, scheduling/save behavior, negative no-schedule paths, progress/completion/stale-event handling, zero results, selected detail actions, action failure feedback, and worker-failure feedback mapping. ScannerScanService now also supports scan_with_progress while preserving the returned progress history for diagnostics/tests. Cargo.toml rust-version was corrected to 1.85.0 because edition 2024 requires at least Rust 1.85 and cargo clippy refused to parse the manifest with the previous lower value.

## Verification

Ran the required final gates. `cargo test s07_scanner_runtime_wiring` passed 6 targeted runtime wiring tests. `cargo fmt --check`, `cargo check`, and `cargo test` all passed. `cargo clippy --all-targets --all-features` exited 0 after the manifest MSRV correction; it currently reports one non-fatal warning about the private Scanner traversal helper having too many arguments.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test s07_scanner_runtime_wiring` | 0 | ✅ pass | 38320ms |
| 2 | `cargo fmt --check` | 0 | ✅ pass | 605ms |
| 3 | `cargo check` | 0 | ✅ pass | 8532ms |
| 4 | `cargo test` | 0 | ✅ pass | 8256ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (non-fatal warning) | 35326ms |

## Deviations

Updated `Cargo.toml` rust-version to `1.85.0` to satisfy Cargo's edition-2024 manifest validation during clippy; this was not listed in the task's expected output but was required for the final gate on Rust 1.95.

## Known Issues

`cargo clippy --all-targets --all-features` exits 0 but reports a non-fatal `clippy::too_many_arguments` warning for the private `ScannerScanService::scan_data_tree` helper.

## Files Created/Modified

- `src/main.rs`
- `src/services/scanner.rs`
- `Cargo.toml`
