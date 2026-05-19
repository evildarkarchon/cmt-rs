---
id: T04
parent: S10
milestone: M001
key_files:
  - ui/archive_patcher_window.slint
  - ui/main.slint
  - src/main.rs
key_decisions:
  - Archive Patcher modal uses exported typed candidate/plan/log UI row structs and model-driven Slint properties rather than embedding filesystem or patching logic.
  - Archive Patcher About body remains a projected property so runtime code can supply the domain reference text without duplicating body copy in Slint.
duration: 
verification_result: passed
completed_at: 2026-05-19T02:40:31.381Z
blocker_discovered: false
---

# T04: Added the Archive Patcher Slint modal contract with typed row exports, fail-closed UI state, About overlay surfaces, and source-level S10 contract tests.

**Added the Archive Patcher Slint modal contract with typed row exports, fail-closed UI state, About overlay surfaces, and source-level S10 contract tests.**

## What Happened

Inspected the existing Downgrader Slint modal, the Archive Patcher domain/controller contracts, and the Python reference files `CMT/src/patcher/_base.py`, `CMT/src/patcher/_archives.py`, and `CMT/src/globals.py`. Added `ui/archive_patcher_window.slint` exporting `ArchivePatcherWindow` plus candidate, plan, and log row structs. The modal preserves the reference title, desired-version group, `v1 (OG)` / `v8 (NG)` radio order with old-gen as the default target, dynamic filter explanation text, `Name Filter:` input, `Patch All`, `Restore Last Run`, `About`, candidates, confirmation/plan, log, progress/status, close-blocked, and About overlay surfaces. Write controls fail closed by default and all candidate/log/plan data comes from projected row arrays. Updated `ui/main.slint` to import/export the Archive Patcher component and row structs for `slint::include_modules!()`. Added source-level S10 contract tests in `src/main.rs` that compare labels and dimensions to domain constants, prove model/callback/default disabled surfaces exist, verify empty/confirmation/About negative states are property-driven, and lock the existing Overview and Tools entrypoint surfaces.

## Verification

Ran formatting, targeted S10 contract tests, full Rust tests, cargo check, and clippy. The required `cargo test s10_archive_patcher_slint_contract --quiet` passed with 4 tests, and `cargo check --quiet` passed. Full `cargo test --quiet` passed 364 tests. Clippy exited 0 while surfacing an unrelated existing S09 warning noted under known issues.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 715ms |
| 2 | `cargo test s10_archive_patcher_slint_contract --quiet` | 0 | ✅ pass (4 tests) | 68971ms |
| 3 | `cargo check --quiet` | 0 | ✅ pass | 20634ms |
| 4 | `cargo test --quiet` | 0 | ✅ pass (364 tests) | 42857ms |
| 5 | `cargo clippy --all-targets --all-features --quiet` | 0 | ✅ pass (existing warning emitted) | 44596ms |

## Deviations

Did not modify `ui/tools_tab.slint`; T04 expected outputs were limited to the modal, `ui/main.slint`, and source tests, so the Tools entrypoint is locked through its existing `tools.archive_patcher` action id and generic `tool-action-requested(string)` surface until runtime wiring.

## Known Issues

`cargo clippy --all-targets --all-features --quiet` exits 0 but reports an existing `field_reassign_with_default` warning in an S09 downgrader test around `src/main.rs:6432`; this task did not change that unrelated test.

## Files Created/Modified

- `ui/archive_patcher_window.slint`
- `ui/main.slint`
- `src/main.rs`
