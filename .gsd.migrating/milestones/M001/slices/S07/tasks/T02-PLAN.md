---
estimated_steps: 7
estimated_files: 2
skills_used: []
---

# T02: Implement read only scanner engine

Expected executor skills: tdd, rust-async-patterns, verify-before-complete.

Why: The scanner's highest-risk behavior is reference-compatible classification, staged MO2 attribution, partial-failure continuation, and progress production over untrusted local mod trees while remaining read-only.

Do: Create `src/services/scanner.rs` and export it from `src/services/mod.rs`. Implement an adapter-backed `ScannerScanService` over `Filesystem` that accepts a typed scan request containing the scanner settings snapshot, optional discovered installation/Data path, overview problems, collected enabled module/archive facts, and optional MO2/Vortex manager context. Use a scanner-specific recursive traversal built from `Filesystem::read_dir` rather than `walk_dir` so top-down pruning, root-folder progress, and unreadable child continuation are possible. Implement the reference read-only rules from `CMT/src/tabs/_scanner.py` and `CMT/src/scan_settings.py`: Data whitelist, ignored folders, skip suffixes, junk files/fomod folder, loose `vis` and `meshes/precombined`, loose `meshes/animtextdata`, F4SE script override CRC names, wrong-format/proper-format detection, invalid archive name suffix rules, overview problem mapping, and race subgraph SADD count over enabled modules. Build MO2 file indices from enabled modlist order plus overwrite when the context is complete; surface missing staged prerequisites or modlist as safe scanner error rows instead of panicking. Treat Vortex as Data-only and leave mod attribution empty. Gate generated rows by their scanner toggles, with the `Errors` toggle covering scanner-generated warning/error rows while fatal worker failures still surface through controller status.

Done when: `cargo test scanner_scan_service` passes with fake filesystem fixtures for MO2 attribution, Vortex Data-only behavior, all rule categories, unreadable child folders, missing Data, missing MO2 modlist, race-subgraph threshold, zero results, stable group ordering, and no writes.

Failure Modes Q5: missing Data returns a safe visible row/status and no traversal; unreadable directories produce error rows and continue siblings; unreadable module bytes skip or report safely without aborting race counting; malformed paths/extensions are classified or ignored according to reference rules.
Load Profile Q6: scanning is linear in traversed directory entries plus enabled module bytes for race counting; do not retain full traversal snapshots beyond result rows and MO2 attribution indexes.
Negative Tests Q7: permission-denied directory, denied file read, malformed modlist, all data-scan toggles off, unexpected extension with/without proper replacement, invalid BA2 suffix, and archive already enabled.

## Inputs

- `CMT/src/tabs/_scanner.py`
- `CMT/src/scan_settings.py`
- `CMT/src/enums.py`
- `CMT/src/globals.py`
- `src/domain/scanner.rs`
- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/services/overview_collector.rs`
- `src/platform/filesystem.rs`
- `src/services/mod.rs`

## Expected Output

- `src/services/scanner.rs`
- `src/services/mod.rs`

## Verification

cargo test scanner_scan_service

## Observability Impact

Adds structured scanner diagnostics and progress metadata at the service boundary: counts for indexed mods/files, traversed folders/files, rows by problem type, partial read failures, and skipped prerequisites.
