# S07: Scanner Read Only Results

**Goal:** Deliver the Scanner tab's reference-shaped read-only scan settings, explicit scan execution flow, progress, grouped results, details pane, and safe copy/open actions while keeping Auto-Fix writes deferred.
**Demo:** User can run Scanner, see progress, grouped read-only results, details, and copy/open actions while the UI remains responsive.

## Must-Haves

- Scanner tab shows embedded Slint `Scan Settings` with the seven reference checkbox labels in order: Overview Issues, Errors, Wrong File Formats, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs.
- `Scan Game` is disabled when every scanner checkbox is off; checkbox changes are only persisted when a scan starts, preserving reference save timing.
- Starting a scan clears old rows/details, disables the button as `Scanning...`, emits `Refreshing Overview...`, `Building mod file index...`, and `Scanning... n/N: folder` progress, and finishes with `N Results ~ Select an item for details` including the exact zero-result text.
- Read-only scanner categories are implemented behind their toggles: Overview problem feed mapping, scanner error rows, wrong file formats, loose previs, junk files, problem overrides, and race subgraph count.
- MO2 staged attribution is used when available; Vortex remains Data-only without fabricated mod names; missing or unreadable paths produce visible safe rows or feedback where partial scan results are still useful.
- Results group and sort deterministically by the reference problem type order and stable row keys.
- Selected result details show `Mod:`, `Problem:`, `Summary:`, `Solution:`, `Copy Details`, optional `File List`, and safe read-only open/copy actions; Auto-Fix, Fixed, and Fix Failed controls are absent.
- Verification covers fake-backed domain/service/controller tests, Slint source-contract tests, runtime wiring tests, and full cargo gates.
- Threat Surface Q3: S07 reads untrusted local mod files and exposes open path, open URL, and clipboard actions. It must not write, delete, rename, patch, archive, or execute user files; action failures use safe inline messages and logs rather than raw diagnostics.
- Requirement Impact Q4: No root requirements were active. Reverify Settings persistence, Overview problem feed assumptions, F4SE lazy worker patterns, Tools/About action feedback patterns, and the shell tab order while keeping completed slices immutable.
- Failure and Negative Coverage Q5/Q7: tests must include all toggles off, save failure rollback, missing Data, missing MO2 modlist, unreadable child folder, unreadable module bytes, malformed or unexpected file extensions, stale worker events, zero results, and desktop/clipboard adapter failures.
- Load Profile Q6: traversal is O(number of Data and MO2 staged entries) and should visit/read one directory or module file at a time; it must not collect full file contents except the race-subgraph per-module read required by the reference behavior.

## Proof Level

- This slice proves: Integration proof. Executors should prove the scanner contract with fake-backed Rust tests, source-contract tests for Slint labels/callbacks, runtime wiring tests that exercise MainWindow projection/worker payload handling without launching a real GUI, and then `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`. Real human UAT is not required for S07 closeout, but the compiled app entrypoint must remain valid.

## Integration Closure

S07 consumes SettingsController and `ScannerSettings`, Overview problem feed and diagnostics builders, Discovery/ModManager context, fakeable filesystem/desktop/clipboard adapters, and WorkerRuntime handoff. It produces `src/domain/scanner.rs`, `src/services/scanner.rs`, `src/app/scanner_controller.rs`, scanner worker payloads, Scanner Slint bindings, and runtime wiring through `src/main.rs`. Remaining milestone work after this slice is S08 Auto-Fix write actions, S09 Downgrade Manager, and S10 Archive Patcher.

## Verification

- Scanner adds visible status/progress/result-count/detail/action-feedback surfaces plus structured tracing for scan request, settings persist success/failure, overview refresh phase, MO2 index build, traversal progress, partial read failures, race-subgraph counts, completion counts, stale worker events, spawn failures, and read-only action failures. Worker events must carry scan ids and safe messages so future agents can localize failures without exposing raw paths beyond user-selected local file locations.

## Tasks

- [x] **T01: Define Scanner domain contract** `est:2h`
  Expected executor skills: tdd, decompose-into-slices, verify-before-complete.
  - Files: `src/domain/scanner.rs`, `src/domain/mod.rs`
  - Verify: cargo test scanner_domain

- [x] **T02: Implement read only scanner engine** `est:4h`
  Expected executor skills: tdd, rust-async-patterns, verify-before-complete.
  - Files: `src/services/scanner.rs`, `src/services/mod.rs`
  - Verify: cargo test scanner_scan_service

- [x] **T03: Add Scanner controller and worker payloads** `est:3h`
  Expected executor skills: tdd, rust-async-patterns, verify-before-complete.
  - Files: `src/app/scanner_controller.rs`, `src/app/mod.rs`, `src/app/settings_controller.rs`, `src/workers/events.rs`, `src/workers/mod.rs`, `src/domain/scanner.rs`
  - Verify: cargo test scanner_controller
cargo test scanner_worker_payload
cargo test settings_controller_saves_scanner

- [x] **T04: Build Scanner Slint surface** `est:2h`
  Expected executor skills: tdd, write-docs, verify-before-complete.
  - Files: `ui/scanner_tab.slint`, `ui/main.slint`, `src/main.rs`
  - Verify: cargo test s07_scanner_slint_contract
cargo check

- [x] **T05: Wire Scanner runtime and final gates** `est:3h`
  Expected executor skills: rust-async-patterns, tdd, review, verify-before-complete.
  - Files: `src/main.rs`, `src/app/scanner_controller.rs`, `src/services/scanner.rs`, `src/workers/events.rs`, `ui/main.slint`
  - Verify: cargo test s07_scanner_runtime_wiring
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Files Likely Touched

- src/domain/scanner.rs
- src/domain/mod.rs
- src/services/scanner.rs
- src/services/mod.rs
- src/app/scanner_controller.rs
- src/app/mod.rs
- src/app/settings_controller.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/scanner_tab.slint
- ui/main.slint
- src/main.rs
