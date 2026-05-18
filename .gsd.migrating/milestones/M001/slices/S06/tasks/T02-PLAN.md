---
estimated_steps: 18
estimated_files: 4
skills_used: []
---

# T02: Implement DLL inspection scan service

Expected executor skills for task-plan frontmatter: tdd, best-practices, verify-before-complete.

Why: F4SE compatibility must be proven from DLL exports and F4SEPlugin_Version.compatibleVersions, not guessed from filenames or external metadata, and scanning must be fakeable and safe around bad DLLs.

Do:
1. Add pelite = "0.10.0" to Cargo.toml and update Cargo.lock.
2. Create src/services/f4se.rs and export it from src/services/mod.rs.
3. Add an F4seDllInspector trait and production PeliteF4seDllInspector that inspects PE bytes without loading DLLs. It should detect F4SEPlugin_Load, F4SEPlugin_Preload, F4SEPlugin_Query, and F4SEPlugin_Version exports and read the compatibleVersions array using the structure offsets from CMT/src/utils.py::F4SEPluginVersionData.
4. Implement the reference-compatible version mapping: SupportsNG is proven only by 0x010A3D40 or 0x010A3D80; SupportsAE is proven only by any compatible version greater than 0x010B0890; otherwise compatible NGAE support remains unknown or warning rather than false.
5. Implement F4seScanService over injected Filesystem and F4seDllInspector. The service accepts a discovered Fallout4Installation or absence, the current F4seGameTarget, and whether a mod manager was detected.
6. Preserve folder behavior: no installation or no data_path yields Data folder not found; no f4se_plugins_path yields Data/F4SE/Plugins folder not found and appends Try launching via your mod manager. only when manager_detected is false.
7. Enumerate only direct children of Data/F4SE/Plugins with case-insensitive .dll extension and the reference msdia prefix skip rule; do not recurse.
8. Read and inspect one DLL at a time, keep deterministic row ordering, keep unreadable or malformed DLLs as visible warning or unknown rows with safe diagnostic text, and continue scanning remaining DLLs.
9. Add fake-backed tests named with f4se_scan_service for missing Data, missing Plugins with and without manager hint, empty plugin folder, direct-child-only enumeration, msdia ignore, unreadable file, malformed parser failure, classification rows, and unknown game target.
10. Add at least one production-inspector negative test named with f4se_dll_inspector proving malformed bytes return a typed parse failure rather than panicking. Add synthetic PE export fixtures if practical, but keep the service behavior testable through the fake inspector regardless.

Threat Surface Q3: untrusted DLL bytes may be maliciously malformed or oversized. Do not call operating-system DLL loading APIs; keep parsing local, typed, and fallible.

Failure Modes Q5: directory read errors produce safe scan errors; file read or parse errors produce per-row warnings; unsupported PE format or platform does not abort the whole scan.

Load Profile Q6: per scan is O(number of direct plugin DLLs plus bytes read per DLL), with no recursion and no UI-thread work. At 10x plugin count, memory should be bounded by reading or parsing one DLL at a time rather than retaining every file's bytes.

Negative Tests Q7: empty directory, non-DLL files, msdia DLLs, nested DLLs, unreadable DLL, malformed DLL bytes, inspector error, unknown game target, and manager hint branch.

Done when: service tests prove the scan contract using fakes and the real inspector fails closed on malformed input.

## Inputs

- `Cargo.toml`
- `Cargo.lock`
- `CMT/src/utils.py`
- `src/domain/f4se.rs`
- `src/domain/discovery.rs`
- `src/platform/filesystem.rs`
- `src/services/mod.rs`
- `src/services/overview.rs`
- `src/services/overview_collector.rs`

## Expected Output

- `Cargo.toml`
- `Cargo.lock`
- `src/services/f4se.rs`
- `src/services/mod.rs`

## Verification

cargo test f4se_scan_service
cargo test f4se_dll_inspector

## Observability Impact

Service results carry row-level safe diagnostics and scan counts so the controller and worker logs can distinguish missing folders, unreadable DLLs, malformed DLLs, and unknown compatibility.
