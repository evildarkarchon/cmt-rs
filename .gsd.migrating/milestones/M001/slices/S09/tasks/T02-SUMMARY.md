---
id: T02
parent: S09
milestone: M001
key_files:
  - src/services/downgrader.rs
  - src/services/mod.rs
  - src/domain/downgrader.rs
  - src/domain/mod.rs
key_decisions:
  - Kept Downgrader status/preview planning read-only over the existing `Filesystem` trait so mutation, download, and xdelta apply behavior must be introduced later through separate confirmed worker seams.
  - Represented preview actions as structured `DowngraderPlanStepKind`/`DowngraderPlanStep` domain payloads rather than UI strings alone, preserving testable plan semantics for later modal confirmation work.
duration: 
verification_result: passed
completed_at: 2026-05-18T10:38:44.106Z
blocker_discovered: false
---

# T02: Added a read-only DowngraderService that builds CRC status snapshots and inline preview plans without mutation.

**Added a read-only DowngraderService that builds CRC status snapshots and inline preview plans without mutation.**

## What Happened

Implemented `src/services/downgrader.rs` and exported it from `src/services/mod.rs`. The service validates an optional discovered Fallout 4 root, rejects unsafe roots and malformed/escaping managed paths, resolves only the six reference downgrader file definitions, computes CRC32 classifications, applies the `steam_api64.dll` Next-Gen/Anniversary display rule, derives the reference default target from `Fallout4.exe`, and builds detailed per-file preview steps for skips, invalid backup cleanup, current backup creation/reuse, desired backup restore, delta download/apply, and optional backup/delta cleanup. Added pure domain plan-step payload types in `src/domain/downgrader.rs` so later UI/worker code can consume structured plan details without duplicating step vocabulary. Tests use an in-memory fake filesystem and generated CRC fixtures to prove status classification, reference row ordering, path safety, target defaulting, backup CRC planning, read-error failure handling, and that preview generation leaves fake filesystem nodes unchanged.

## Verification

Ran the task-targeted test filter plus format, compile, full test, and clippy gates. `cargo test downgrader_service_plan` passed 8 service tests. `cargo test` passed the full suite (303 tests). `cargo clippy --all-targets --all-features` exited 0 with an existing unrelated scanner `too_many_arguments` warning at `src/services/scanner.rs:1118`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 583ms |
| 2 | `cargo check` | 0 | ✅ pass | 16553ms |
| 3 | `cargo test downgrader_service_plan` | 0 | ✅ pass (8 passed) | 39047ms |
| 4 | `cargo test` | 0 | ✅ pass (303 passed) | 47087ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (unrelated existing scanner warning) | 26316ms |

## Deviations

None.

## Known Issues

`cargo clippy --all-targets --all-features` still reports an unrelated existing warning in `src/services/scanner.rs:1118` (`clippy::too_many_arguments`); the command exits 0 and this task did not modify scanner code.

## Files Created/Modified

- `src/services/downgrader.rs`
- `src/services/mod.rs`
- `src/domain/downgrader.rs`
- `src/domain/mod.rs`
