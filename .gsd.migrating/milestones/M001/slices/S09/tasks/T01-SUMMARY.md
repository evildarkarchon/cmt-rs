---
id: T01
parent: S09
milestone: M001
key_files:
  - src/domain/downgrader.rs
  - src/domain/mod.rs
key_decisions:
  - Kept the Downgrade Manager source of truth in a dedicated `domain::downgrader` module so later S09 service/controller/UI work can consume exact reference strings, CRC classifications, and helper semantics without duplicating them or touching Slint/IO.
duration: 
verification_result: passed
completed_at: 2026-05-18T10:20:46.236Z
blocker_discovered: false
---

# T01: Added a pure downgrader domain contract with reference labels, CRC maps, backup/patch helpers, and typed row payloads.

**Added a pure downgrader domain contract with reference labels, CRC maps, backup/patch helpers, and typed row payloads.**

## What Happened

Created `src/domain/downgrader.rs` from the read-only Python downgrader, globals, and enum references. The module now exposes exact modal labels, group labels, desired-version labels, checkbox and button labels, initial log copy, about dialog copy, tooltip copy, patch URL/direction constants, six reference-managed file definitions in display/patch order, CRC-to-status mappings, CRC bucket helpers, backup filename helpers, patch name/URL helpers, status/target/group/log/progress types, options snapshots, status rows, plan rows, and execution log rows. Exported the module from `src/domain/mod.rs` and extended the existing domain visibility test with public-import assertions for the new downgrader types and constants. Added source-contract tests that assert the reference strings, CRC classifications, file grouping/order, NG/AE CRC bucket behavior, labels, backup names, patch names/URLs, log messages, plan actions, and progress clamping without reading ignored `.gsd`, `.planning`, or `.audits` paths.

## Verification

Ran formatting, targeted downgrader domain tests, broader compile/test gates, and clippy. `cargo test downgrader_domain` passed 8 focused source-contract tests. `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features` all exited 0. Clippy emitted one existing warning in `src/services/scanner.rs:1118` (`too_many_arguments`) outside this task; it did not fail the gate.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt` | 0 | ✅ pass | 572ms |
| 2 | `cargo test downgrader_domain` | 0 | ✅ pass (8 passed; 0 failed) | 45960ms |
| 3 | `cargo fmt --check` | 0 | ✅ pass | 614ms |
| 4 | `cargo check` | 0 | ✅ pass | 20718ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (unrelated existing scanner warning) | 26370ms |
| 6 | `cargo test` | 0 | ✅ pass (295 passed; 0 failed) | 8563ms |

## Deviations

None.

## Known Issues

`cargo clippy --all-targets --all-features` still reports an existing `too_many_arguments` warning in `src/services/scanner.rs:1118`; this task did not modify scanner code and clippy exited 0.

## Files Created/Modified

- `src/domain/downgrader.rs`
- `src/domain/mod.rs`
