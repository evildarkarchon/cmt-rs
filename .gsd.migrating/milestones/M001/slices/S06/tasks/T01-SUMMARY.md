---
id: T01
parent: S06
milestone: M001
key_files:
  - src/domain/f4se.rs
  - src/domain/mod.rs
key_decisions:
  - D026: Map combined or non-concrete discovery install states to `F4seGameTarget::Unknown` so the F4SE Your Game column preserves uncertainty with a warning instead of pretending the target is concrete NG or AE.
duration: 
verification_result: passed
completed_at: 2026-05-18T03:46:19.744Z
blocker_discovered: false
---

# T01: Added a Slint-free F4SE domain contract that locks reference strings, icons, row classifications, scan snapshots, and current-game warning behavior.

**Added a Slint-free F4SE domain contract that locks reference strings, icons, row classifications, scan snapshots, and current-game warning behavior.**

## What Happened

Inspected the read-only Python reference for the F4SE tab (`CMT/src/tabs/_f4se.py`), shared strings (`CMT/src/globals.py`), DLL fact shape (`CMT/src/helpers.py`), DLL parsing behavior (`CMT/src/utils.py`), current-game generation helpers (`CMT/src/game_info.py`), and the existing Rust discovery domain. Added `src/domain/f4se.rs` with reference-locked tab/loading/table/legend/missing-folder constants, typed `F4seGameTarget`, `F4seDllFacts`, `F4seCompatibilityCell`, `F4seDllRow`, `F4seScanSnapshot`, `F4seScanStatus`, row tags, severity, and pure render helpers. The render logic matches the reference behavior: non-F4SE DLLs show question marks for OG/NG/AE and blank Your Game, OG support comes from `F4SEPlugin_Query`, NG/AE support comes from `F4SEPlugin_Version` plus `compatibleVersions`, unsupported reference columns stay blank, unsupported Your Game shows the cross mark, ambiguous NGAE support uses the warning icon, unknown current-game state remains a warning rather than confirmed incompatibility, and inspection failures remain visible with safe details instead of panicking. Exported the module from `src/domain/mod.rs` and added public import coverage. Followed a TDD loop: the first scoped `cargo test f4se_domain` failed on missing contract symbols before implementation, then passed after implementing and formatting.

## Verification

Verified with the task-scoped test and broader Rust gates after the final code changes: `cargo test f4se_domain` passed 9 F4SE domain tests, `cargo fmt --check` passed, `cargo check` passed, full `cargo test` passed 183 tests, and `cargo clippy --all-targets --all-features` passed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 445ms |
| 2 | `cargo test f4se_domain` | 0 | ✅ pass (9 passed; 0 failed; 174 filtered out) | 34519ms |
| 3 | `cargo check` | 0 | ✅ pass | 16720ms |
| 4 | `cargo test` | 0 | ✅ pass (183 passed; 0 failed) | 8476ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 18222ms |

## Deviations

The requested `Skill(...)` tool is not exposed in this harness, so I read the required skill files directly before editing and followed their guidance. No implementation-scope deviations.

## Known Issues

None.

## Files Created/Modified

- `src/domain/f4se.rs`
- `src/domain/mod.rs`
