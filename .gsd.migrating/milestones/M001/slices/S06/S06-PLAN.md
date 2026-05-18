# S06: F4SE Diagnostics

**Goal:** Deliver a read-only, non-blocking, reference-shaped F4SE diagnostics tab that lazily scans Data/F4SE/Plugins DLLs, classifies F4SE export compatibility for OG, NG, AE, and Your Game, and keeps malformed or unreadable DLLs visible without crashing.
**Demo:** User can inspect F4SE plugin DLL compatibility in a reference-shaped table without blocking the UI.

## Must-Haves

- Threat Surface Q3: S06 parses untrusted local mod DLL bytes and displays local filenames/paths. It must never load or execute DLL code, must parse off the Slint UI thread, must continue after malformed or unreadable files, and must avoid logging secrets or file contents. Requirement Impact Q4: no active REQUIREMENTS.md entries were preloaded; re-verify established S03/S04/S05 promises for fakeable platform seams, owned worker events, Overview-derived game classification, Tools/About tab order, and CMT read-only status. Decisions revisited or locked: D024 and D025.
- Done means: the F4SE tab replaces the placeholder with the reference title, columns DLL, OG, NG, AE, Your Game, heading F4SE DLLs, and exact ABOUT_F4SE_DLLS legend; opening the F4SE tab for the first time triggers one background scan and displays Scanning DLLs... while busy; missing Data and Data/F4SE/Plugins paths show the reference messages and append Try launching via your mod manager. only when no manager is detected; empty plugin folders render an empty table plus legend, not an error; direct child DLLs are scanned, msdia-prefixed DLLs are ignored, and no recursive traversal is introduced; F4SEPlugin_Load or F4SEPlugin_Preload, F4SEPlugin_Query, F4SEPlugin_Version, and compatibleVersions mapping match CMT/src/utils.py::parse_dll without filename or mod-name heuristics; unreadable, malformed, unsupported-host, or unclassifiable DLLs remain visible as unknown or warning rows; unknown current game versions keep DLL facts visible and render Your Game as warning with a clear explanation.
- Slice verification commands: cargo fmt --check; cargo check; cargo test; cargo clippy --all-targets --all-features; git status --short CMT should remain empty. Focused test filters expected during implementation: cargo test f4se_domain; cargo test f4se_scan_service; cargo test f4se_controller; cargo test s06_f4se_slint_contract; cargo test s06_f4se_runtime_wiring.

## Proof Level

- This slice proves: Integration proof. Automated Rust unit, source-contract, worker, and Slint compile checks are required. Real GUI human UAT is not required for S06 planning, but cargo check must compile the real Slint entrypoint and the main wiring must exercise the actual lazy-scan scheduling path with fakeable seams in tests.

## Integration Closure

Consumes S03 platform adapters and worker handoff, S04 OverviewCollector current-game classification, and S05 MainWindow callback/state projection patterns. Introduces F4SE domain, scan service, controller, worker payload, MainWindow properties/callback, and Slint table UI. Leaves selected-row details, manual refresh, copy/open row actions, Scanner results, auto fixes, Downgrade Manager, and Archive Patcher workflows to later slices.

## Verification

- Adds visible F4SE status, loading, error, row detail, and Your Game warning states plus structured tracing around scan start, missing folders, directory enumeration, per-DLL inspection failures, counts, stale worker events, spawn failures, and scan completion. Future agents can inspect cargo test failures, controller state tests, Slint source-contract tests, and UI-visible safe messages without reading raw DLL contents.

## Tasks

- [x] **T01: Define F4SE domain contract** `est:1h`
  Expected executor skills for task-plan frontmatter: tdd, verify-before-complete.
  - Files: `src/domain/f4se.rs`, `src/domain/mod.rs`
  - Verify: cargo test f4se_domain

- [x] **T02: Implement DLL inspection scan service** `est:3h`
  Expected executor skills for task-plan frontmatter: tdd, best-practices, verify-before-complete.
  - Files: `Cargo.toml`, `Cargo.lock`, `src/services/f4se.rs`, `src/services/mod.rs`
  - Verify: cargo test f4se_scan_service
cargo test f4se_dll_inspector

- [x] **T03: Add F4SE controller and worker payloads** `est:2h`
  Expected executor skills for task-plan frontmatter: tdd, rust-async-patterns, verify-before-complete.
  - Files: `src/app/f4se_controller.rs`, `src/app/mod.rs`, `src/workers/events.rs`, `src/workers/mod.rs`
  - Verify: cargo test f4se_controller
cargo test f4se_worker_payload

- [x] **T04: Wire F4SE Slint tab and lazy scan** `est:3h`
  Expected executor skills for task-plan frontmatter: rust-async-patterns, tdd, verify-before-complete.
  - Files: `ui/f4se_tab.slint`, `ui/main.slint`, `src/main.rs`
  - Verify: cargo test s06_f4se_slint_contract
cargo test s06_f4se_runtime_wiring
cargo check

- [x] **T05: Run S06 quality gates** `est:1h`
  Expected executor skills for task-plan frontmatter: verify-before-complete.
  - Verify: cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Files Likely Touched

- src/domain/f4se.rs
- src/domain/mod.rs
- Cargo.toml
- Cargo.lock
- src/services/f4se.rs
- src/services/mod.rs
- src/app/f4se_controller.rs
- src/app/mod.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/f4se_tab.slint
- ui/main.slint
- src/main.rs
