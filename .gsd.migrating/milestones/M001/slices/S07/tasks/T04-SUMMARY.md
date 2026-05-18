---
id: T04
parent: S07
milestone: M001
key_files:
  - ui/scanner_tab.slint
  - ui/main.slint
  - src/main.rs
key_decisions:
  - Kept Scanner UI write-free by omitting Auto-Fix/Fixed/Fix Failed controls entirely and exposing only read-only copy/open/file-list callbacks.
  - Represented grouped Scanner results in Slint as a flat row model with `row_kind` plus `result_index`, allowing a simple embedded grouped list while preserving controller-owned selection semantics.
duration: 
verification_result: passed
completed_at: 2026-05-18T06:40:47.216Z
blocker_discovered: false
---

# T04: Replaced the Scanner placeholder with a reference-shaped read-only Slint surface and MainWindow API/projection tests.

**Replaced the Scanner placeholder with a reference-shaped read-only Slint surface and MainWindow API/projection tests.**

## What Happened

Replaced `ui/scanner_tab.slint` with an embedded Scanner layout matching the S07 contract: read-only `Scan Settings` with the seven reference categories in order, `Scan Game`/`Scanning...` state, visible progress/status/result count, grouped result rows with an optional Mod column, embedded Details pane, File List affordance/panel, action feedback, and read-only `Copy Details`, `Open Path`, `Open URL`, and `Copy URL` actions. Updated `ui/main.slint` to import Scanner structs, expose Scanner properties/models/callbacks through `MainWindow`, and forward callbacks from `ScannerTab` to the root. Added Rust-side Scanner UI projection from the existing Slint-free `ScannerController`, including initial settings/state application at startup, plus `s07_scanner_slint_contract*` tests covering labels/order, hidden Auto-Fix/Fixed/Fix Failed text, detail/action labels, MainWindow forwarding, and runtime projection behavior. During verification, an initial Slint property-name conflict (`row`) and two test expectation mismatches were found and corrected; final required checks pass.

## Verification

Ran `cargo fmt --check`, `cargo test s07_scanner_slint_contract`, and `cargo check`. The targeted S07 contract test suite passed 3/3 tests, formatting passed, and `cargo check` completed successfully with the new Slint API.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 517ms |
| 2 | `cargo test s07_scanner_slint_contract` | 0 | ✅ pass (3 passed, 0 failed) | 37435ms |
| 3 | `cargo check` | 0 | ✅ pass | 22823ms |

## Deviations

Added a small Rust runtime projection and initial Scanner state application in `src/main.rs` in addition to source-contract tests so the new Slint properties are populated from the existing controller rather than remaining disconnected defaults.

## Known Issues

No background Scanner scan/action worker wiring was added in this task; the task plan explicitly scoped this as the Slint surface with no background wiring yet.

## Files Created/Modified

- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`
