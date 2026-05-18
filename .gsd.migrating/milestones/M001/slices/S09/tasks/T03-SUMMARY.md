---
id: T03
parent: S09
milestone: M001
key_files:
  - src/platform/mod.rs
  - src/platform/filesystem.rs
  - src/services/downgrader.rs
  - Cargo.toml
  - Cargo.lock
key_decisions:
  - Kept filesystem mutation in a new `WritableFilesystem` trait instead of expanding the read-only `Filesystem` contract.
  - Used fakeable `DeltaDownloader` and `DeltaApplier` seams so executor behavior is testable without real network or game paths.
  - Used pure-Rust `vcdiff-decoder` for the production applier because `xdelta3` conflicted with the existing Slint bindgen/clang dependency graph.
duration: 
verification_result: passed
completed_at: 2026-05-18T11:09:46.533Z
blocker_discovered: false
---

# T03: Added the Downgrader confirmed executor with write, download, and VCDIFF apply seams plus failure-path tests.

**Added the Downgrader confirmed executor with write, download, and VCDIFF apply seams plus failure-path tests.**

## What Happened

Implemented a separate `WritableFilesystem` trait in `src/platform/filesystem.rs` and extended `PlatformOperation` with safe file write/copy/rename/remove labels. Added real `std::fs` mutation implementations with typed `PlatformError` mapping. Extended `src/services/downgrader.rs` with `DeltaDownloader`, `ReqwestDeltaDownloader`, `DeltaApplier`, and `VcdiffDeltaApplier` seams, plus a confirmed executor that reuses the read-only preview plan, revalidates CRCs before mutation, processes the six managed files independently, restores valid desired backups, creates/reuses/removes current backups according to the reference backup semantics and `Keep Backups`, downloads or reuses patch files, applies VCDIFF deltas, honors `Delete Patches`, and preserves source backups on download/apply failures. Added recording filesystem/downloader/applier test fakes and executor tests for restore, backup cleanup, existing patch reuse, skipped unsupported rows, failed download/apply preservation, permission failures, and a deterministic VCDIFF fixture proof. I initially evaluated the `xdelta3` crate, but it conflicted with Slint’s bindgen/clang dependency graph, so I used the pure-Rust `vcdiff-decoder` dependency and proved the applier with an inline source/patch fixture.

## Verification

Ran the required targeted gate `cargo test downgrader_executor` after the final refactor; all 6 executor tests passed. Also ran `cargo fmt --check` after the final refactor. Earlier in the same implementation pass, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features` completed successfully; clippy reported warnings only, including an existing scanner warning and a downgrader helper arity warning that was refactored before the final targeted test rerun.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test downgrader_executor` | 0 | ✅ pass | 39475ms |
| 2 | `cargo fmt --check` | 0 | ✅ pass | 626ms |
| 3 | `cargo check` | 0 | ✅ pass | 23779ms |
| 4 | `cargo test` | 0 | ✅ pass | 8553ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass (warnings reported; downgrader warning refactored afterward) | 38229ms |

## Deviations

Used pure-Rust `vcdiff-decoder` instead of `xdelta3`; the `xdelta3` crate could not be used because its older bindgen dependency introduced a `clang-sys` links conflict with Slint/skia. The production applier was still implemented and fixture-proven via VCDIFF decoding.

## Known Issues

`cargo clippy --all-targets --all-features` was run before the final helper refactor and exited 0 with warnings; the new downgrader warning was refactored away, but hard timeout recovery prevented rerunning clippy after that final refactor. An unrelated existing scanner `too_many_arguments` clippy warning remains.

## Files Created/Modified

- `src/platform/mod.rs`
- `src/platform/filesystem.rs`
- `src/services/downgrader.rs`
- `Cargo.toml`
- `Cargo.lock`
