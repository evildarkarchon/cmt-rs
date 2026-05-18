---
estimated_steps: 6
estimated_files: 6
skills_used: []
---

# T03: Add Scanner controller and worker payloads

Expected executor skills: tdd, rust-async-patterns, verify-before-complete.

Why: Scanner UI state must be reducer-driven like Overview and F4SE so Slint callbacks never perform filesystem work and stale background results cannot overwrite newer scans.

Do: Add `src/app/scanner_controller.rs` and export it from `src/app/mod.rs`. Extend `SettingsController` with a scanner save-on-scan-start method that persists a candidate `ScannerSettings` snapshot and returns the snapshot Slint must display, reverting to the last persisted scanner settings on save failure. Extend `src/workers/events.rs` and `src/workers/mod.rs` with a typed `ScannerWorkerPayload` carrying scan id, owned scan snapshot, and read-only action feedback where needed. The controller should own transient checkbox state, scan lifecycle, active scan id, progress text/counts, button text/enabled state, grouped/flat result rows, selected detail state, file-list visibility/text, action availability, last safe action feedback, and stale-event handling. It should expose intent methods for checkbox toggles, scan requests, progress, completion, failure/spawn failure, row selection, file-list toggling, and action completion. Use a stable prefix such as `s07-scanner-scan:` for scan tasks.

Done when: `cargo test scanner_controller` and `cargo test scanner_worker_payload` pass, and settings-controller tests prove scanner toggles are not saved on individual toggles but are saved or reverted when `Scan Game` starts.

Failure Modes Q5: settings save failure reverts visible scanner toggles and does not schedule a scan with unpersisted state; worker spawn failure restores `Scan Game` and safe error text; stale progress/completion/action events are ignored; selecting a missing result clears details safely.
Negative Tests Q7: all toggles off disables scan, zero-result completion text, duplicate scan ids, stale completion after a newer scan, worker failure with raw diagnostic, invalid action id, copy/open action failure, and file-list toggle without a file list.

## Inputs

- `src/domain/scanner.rs`
- `src/app/settings_controller.rs`
- `src/app/f4se_controller.rs`
- `src/app/overview_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/app/mod.rs`

## Expected Output

- `src/app/scanner_controller.rs`
- `src/app/mod.rs`
- `src/app/settings_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/domain/scanner.rs`

## Verification

cargo test scanner_controller
cargo test scanner_worker_payload
cargo test settings_controller_saves_scanner

## Observability Impact

Adds visible lifecycle states and traceable scan ids for scan request, start, progress, completion, failure, stale ignored events, settings persist failures, and action feedback.
