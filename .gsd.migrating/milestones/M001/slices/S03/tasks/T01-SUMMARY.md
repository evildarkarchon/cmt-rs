---
id: T01
parent: S03
milestone: M001
key_files:
  - src/domain/discovery.rs
  - src/domain/mod_manager.rs
  - src/domain/mod.rs
key_decisions:
  - Kept discovery/mod-manager modules pure: no filesystem, registry, process, or Slint access in domain contracts.
  - Separated safe `user_message()` text from diagnostic details, preserving raw paths in user text only where the Python reference intentionally includes them.
  - Represented Vortex as identity-only detection scope with no staging/config parsing and semantic version fallback `0.0.0`.
duration: 
verification_result: passed
completed_at: 2026-05-17T08:59:58.848Z
blocker_discovered: false
---

# T01: Added pure Rust discovery and mod-manager domain contracts with reference-compatible messages and unit coverage.

**Added pure Rust discovery and mod-manager domain contracts with reference-compatible messages and unit coverage.**

## What Happened

Implemented `src/domain/discovery.rs` with pure data contracts for semantic versions, Fallout 4 install type labels, valid installation state with optional Data/F4SE paths, archive records, module records, INI documents, and recoverable discovery errors. Implemented `src/domain/mod_manager.rs` with exact Mod Organizer/Vortex identity contracts, Vortex identity-only scope and 0.0.0 version fallback, MO2 directory/profile/skip-rule context, MO2 configuration result shape, and typed MO2 parse errors. Updated `src/domain/mod.rs` to export the new modules and keep public import smoke coverage. Reference text was checked against `CMT/src/game_info.py`, `CMT/src/mod_manager_info.py`, `CMT/src/utils.py`, `CMT/src/tabs/_f4se.py`, `CMT/src/tabs/_scanner.py`, and `CMT/src/enums.py`. User-facing errors now expose known/reference text while diagnostics retain raw details unless the Python reference intentionally includes paths.

## Verification

Ran the required Rust gates after implementation and warning cleanup. `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` all completed successfully. Unit tests verify the must-have contracts: Fallout 4 installations can be represented without Data/F4SE folders, discovery failures are typed/recoverable and do not require a file-picker UI, MO2 context carries required paths/profile-local flags/skip rules, MO2 parse errors carry typed kinds and reference-compatible messages, Vortex detection is identity-only with exact display name and 0.0.0 fallback, and raw path details stay out of user messages except reference path exceptions.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 244ms |
| 2 | `cargo check` | 0 | ✅ pass | 10631ms |
| 3 | `cargo test` | 0 | ✅ pass (46 passed) | 25199ms |
| 4 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 11046ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/domain/mod.rs`
