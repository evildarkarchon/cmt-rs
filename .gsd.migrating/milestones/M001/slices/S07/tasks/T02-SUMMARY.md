---
id: T02
parent: S07
milestone: M001
key_files:
  - src/services/scanner.rs
  - src/services/mod.rs
key_decisions:
  - ScannerScanService is adapter-backed over Filesystem and intentionally infallible; recoverable local filesystem/manager failures become safe diagnostics and optional Errors-category rows.
  - MO2 staged attribution is built from enabled modlist order plus overwrite when prerequisites are complete; Vortex remains Data-only.
  - Scanner traversal uses scanner-specific recursive Filesystem::read_dir calls instead of walk_dir so pruning, top-level progress, and unreadable-child continuation are explicit.
duration: 
verification_result: passed
completed_at: 2026-05-18T07:21:38.637Z
blocker_discovered: false
---

# T02: Added a read-only, adapter-backed Scanner scan service with MO2 attribution, Vortex Data-only handling, reference rule classification, safe diagnostics, progress metadata, and fake-filesystem coverage.

**Added a read-only, adapter-backed Scanner scan service with MO2 attribution, Vortex Data-only handling, reference rule classification, safe diagnostics, progress metadata, and fake-filesystem coverage.**

## What Happened

Implemented `src/services/scanner.rs` as the Slint-free Scanner scan engine over the fakeable `Filesystem` adapter and exported it from `src/services/mod.rs`. The service accepts typed `ScannerScanRequest` snapshots carrying persisted scanner settings, optional installation/Data paths, Overview problems, enabled module/archive facts, and optional Mod Organizer/Vortex context. It emits owned `ScannerScanOutput` values with grouped/flat results, safe final status, progress events, and structured diagnostics.

The scan path follows the reference read-only behavior from the Python Scanner and scan settings: Overview problem mapping, Data root whitelist/pruning, ignored folders, skip suffixes, junk file/fomod detection, loose `vis` and `meshes/precombined`, loose `meshes/animtextdata`, F4SE script override detection, wrong-format/proper-format detection, invalid BA2 suffix checks, archive-enabled exemptions, and race-subgraph SADD counting. Traversal uses recursive `Filesystem::read_dir` calls so top-level folder progress, top-down pruning, stable ordering, and unreadable-child continuation are testable without writing to disk.

MO2 attribution builds an index from enabled `modlist.txt` order plus overwrite when the context is complete; missing prerequisites or missing/unreadable modlists become safe scanner diagnostics and Errors-category rows instead of panics. Vortex context intentionally remains Data-only without fabricated mod names. Missing Data, unreadable directories, unreadable module bytes, malformed inputs, disabled toggles, and zero-result scans all return safe statuses/rows/diagnostics while preserving read-only behavior.

## Verification

Ran the authoritative T02 verification `cargo test scanner_scan_service` using `gsd_exec`; it passed with 15 scanner scan service tests and no failures. The suite covers fake filesystem fixtures for MO2 attribution, Vortex Data-only behavior, reference rule categories, unreadable child continuation, missing Data, missing MO2 modlist, unreadable module bytes, race-subgraph thresholding, zero results, stable group ordering, toggle gating, malformed/unexpected extensions, invalid BA2 suffix, archive-enabled exemptions, and no-write behavior.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test scanner_scan_service` | 0 | ✅ pass (15 passed, 0 failed) | 8800ms |

## Deviations

None. This auto-fix invocation restored the missing canonical T02 completion artifact from the already-present implementation; no source edits were required.

## Known Issues

None.

## Files Created/Modified

- `src/services/scanner.rs`
- `src/services/mod.rs`
