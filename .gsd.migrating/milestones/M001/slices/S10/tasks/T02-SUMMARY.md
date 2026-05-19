---
id: T02
parent: S10
milestone: M001
key_files:
  - src/platform/filesystem.rs
  - src/domain/archive_patcher.rs
  - src/services/archive_patcher.rs
  - src/services/downgrader.rs
key_decisions:
  - Archive Patcher confirmed patching now writes the latest restore manifest before archive mutation and then performs only bounded version-field byte-range writes after per-file revalidation.
  - Restore-last-run resolves app-owned manifest relative paths under the current Data root and skips stale files instead of writing when current header facts do not match the expected patched state.
duration: 
verification_result: passed
completed_at: 2026-05-19T02:04:10.575Z
blocker_discovered: false
---

# T02: Added a fail-closed Archive Patcher executor and restore path using manifest-first, bounded BA2 header byte-range writes.

**Added a fail-closed Archive Patcher executor and restore path using manifest-first, bounded BA2 header byte-range writes.**

## What Happened

Extended the filesystem mutation seam with `WritableFilesystem::write_byte_range` and implemented it in `RealFilesystem` using random-access open, seek, write, and sync so Archive Patcher can update only the BA2 version field instead of replacing multi-GB archives. Expanded the Archive Patcher domain with BA2 version-field constants, exact original/patched header-prefix manifest data, execution outcomes/results, restore summary/log messages, and safe stale-change messages. Implemented confirmed patch execution in `ArchivePatcherService`: it rebuilds the preview plan, rejects digest mismatches before writing anything, writes the app-owned latest restore manifest before any archive mutation, then processes files sequentially with immediate containment, symlink/reparse, canonical-root, metadata, BTDX magic, known format, source version, target transition, byte-range write, and post-write validation. Implemented restore-last-run by reading and schema-validating the latest manifest, resolving only app-owned relative Data paths, validating current files still match the expected patched state, restoring the saved original version bytes, and skipping stale/ambiguous files safely. Added fake-backed executor tests for v7/v8 to v1, v1 to v8, already-patched skip, unknown version, bad magic, missing file, permission/write failure, manifest write failure abort, partial success, restore success, and stale restore skip. Updated the existing Downgrader test fake to satisfy the extended writable filesystem trait; production Downgrader behavior is unchanged.

## Verification

Verified the focused task gates (`cargo test archive_patcher_executor --quiet`, `cargo test platform::filesystem --quiet`), adjacent archive-patcher domain/planner regressions, formatting, compile, full test suite, and clippy. `cargo clippy --all-targets --all-features --quiet` exits 0 while still emitting the pre-existing `field_reassign_with_default` warnings in `src/main.rs`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test archive_patcher_executor --quiet` | 0 | ✅ pass | 55769ms |
| 2 | `cargo test platform::filesystem --quiet` | 0 | ✅ pass | 8677ms |
| 3 | `cargo test archive_patcher_domain --quiet && cargo test archive_patcher_service_plan --quiet` | 0 | ✅ pass | 17232ms |
| 4 | `cargo fmt --check` | 0 | ✅ pass | 686ms |
| 5 | `cargo check --quiet` | 0 | ✅ pass | 25046ms |
| 6 | `cargo test --quiet` | 0 | ✅ pass | 8646ms |
| 7 | `cargo clippy --all-targets --all-features --quiet` | 0 | ✅ pass (pre-existing warnings emitted) | 32298ms |

## Deviations

Touched `src/services/downgrader.rs` test support to implement the new `WritableFilesystem::write_byte_range` method required by the trait extension. No production Downgrader behavior changed.

## Known Issues

`cargo clippy --all-targets --all-features --quiet` exits 0 but emits existing `src/main.rs` `field_reassign_with_default` warnings unrelated to this task.

## Files Created/Modified

- `src/platform/filesystem.rs`
- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/services/downgrader.rs`
