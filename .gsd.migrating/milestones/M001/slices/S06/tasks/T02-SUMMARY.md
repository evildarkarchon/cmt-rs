---
id: T02
parent: S06
milestone: M001
key_files:
  - Cargo.toml
  - Cargo.lock
  - src/services/f4se.rs
  - src/services/mod.rs
key_decisions:
  - Production F4SE inspection uses Pelite over raw PE bytes and never OS-loads DLLs.
  - NG/AE compatibility is true-only from known compatibleVersions values; unproven compatibility remains unknown/warning rather than false.
  - F4SE scan diagnostics are returned alongside the UI snapshot so later controller/worker code can log missing folders, skipped files, read failures, parse failures, and counts without raw DLL content.
duration: 
verification_result: passed
completed_at: 2026-05-18T04:07:03.917Z
blocker_discovered: false
---

# T02: Added a fakeable F4SE DLL scan service and Pelite-based fail-closed PE export inspector.

**Added a fakeable F4SE DLL scan service and Pelite-based fail-closed PE export inspector.**

## What Happened

Added `pelite = "0.10.0"` and refreshed `Cargo.lock`, then created `src/services/f4se.rs` and exported it from `src/services/mod.rs`. The new service accepts an optional discovered `Fallout4Installation`, a current `F4seGameTarget`, and the mod-manager-detected flag. It preserves the reference missing-folder behavior, including the mod-manager hint only for unmanaged missing `Data/F4SE/Plugins`, enumerates only direct plugin-folder children, filters case-insensitive `.dll` files, applies the reference lowercase `msdia` skip rule, reads and inspects one DLL at a time, and keeps unreadable or malformed DLLs visible as warning rows with safe diagnostics while continuing remaining files. Added structured scan diagnostics and tracing events for scan start, missing prerequisites, directory enumeration/read failures, per-DLL read/inspection/version-data failures, counts, and scan completion.

Added the `F4seDllInspector` trait, typed `F4seDllInspection` facts, typed `F4seDllInspectionError`, and production `PeliteF4seDllInspector`. The production inspector parses PE bytes locally with Pelite and never calls OS DLL loading APIs. It detects `F4SEPlugin_Load`, `F4SEPlugin_Preload`, `F4SEPlugin_Query`, and `F4SEPlugin_Version`; reads `F4SEPlugin_Version.compatibleVersions` at the reference structure offset 528; proves NG only from `0x010A3D40` or `0x010A3D80`; proves AE only from versions greater than `0x010B0890`; and otherwise preserves unknown/warning state instead of manufacturing false support. Added fake-backed service tests for missing Data, missing Plugins with/without manager hint, empty folder, direct-child-only enumeration, msdia ignore, unreadable file, malformed inspector failure, classification rows, unknown game target, and plugin directory read errors. Added production-inspector negative/mapping tests for malformed bytes and the true-only NG/AE compatibility mapping.

## Verification

Verified the required scoped tests and broader Rust gates after the final edits. `cargo test f4se_scan_service` passed 10 service tests. `cargo test f4se_dll_inspector` passed 2 production-inspector/mapping tests. `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features` all passed without warnings.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 476ms |
| 2 | `cargo check` | 0 | ✅ pass | 13529ms |
| 3 | `cargo test f4se_scan_service` | 0 | ✅ pass (10 passed; 0 failed; 185 filtered out) | 33337ms |
| 4 | `cargo test f4se_dll_inspector` | 0 | ✅ pass (2 passed; 0 failed; 193 filtered out) | 8658ms |
| 5 | `cargo test` | 0 | ✅ pass (195 passed; 0 failed) | 8508ms |
| 6 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 16375ms |

## Deviations

The requested `Skill(...)` tool is not exposed in this harness, so I could not invoke the listed skills directly. I followed the relevant task guidance manually: TDD-style scoped tests first around the service contract, fail-closed parsing for malicious/malformed DLL bytes, no OS DLL loading, one-DLL-at-a-time memory behavior, and safe user-facing diagnostics.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml`
- `Cargo.lock`
- `src/services/f4se.rs`
- `src/services/mod.rs`
