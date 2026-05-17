---
id: T03
parent: S03
milestone: M001
key_files:
  - src/services/mod.rs
  - src/services/discovery.rs
  - src/main.rs
key_decisions:
  - Discovery orchestration remains UI-prompt-free and depends only on injected filesystem, registry, and process/system metadata adapters.
  - Manager-specific MO2 failures block silent fallback so incomplete or non-Fallout manager state is visible instead of being hidden by registry/CWD probing.
  - Vortex remains identity-only in this phase, with version fallback support but no staging/config parsing.
duration: 
verification_result: passed
completed_at: 2026-05-17T10:33:20.771Z
blocker_discovered: false
---

# T03: Added a fake-backed discovery orchestration service for Fallout 4, MO2, Vortex, and system metadata.

**Added a fake-backed discovery orchestration service for Fallout 4, MO2, Vortex, and system metadata.**

## What Happened

Implemented `src/services/discovery.rs` and exported it through `src/services/mod.rs`. The service coordinates the pure discovery and mod-manager domain contracts over injected filesystem, registry, and process/system metadata adapter traits, preserving the reference game-path search order: running manager `gamePath`, current working directory, Bethesda registry path, then GOG registry path. It records ordered discovery attempts, normalizes a direct `Fallout4.exe` candidate to its parent game directory, represents partial derived state when `Data` or `Data/F4SE/Plugins` are absent, and returns recoverable typed not-found/invalid-registry errors instead of prompting for a manual file picker.

The service also detects running Mod Organizer 2 and Vortex from the process ancestor chain. MO2 discovery checks adjacent portable files before HKCU `CurrentInstance`/`LOCALAPPDATA` instance configuration, then falls back to the adjacent portable INI. MO2 parsing captures `gamePath`, `selected_profile`, mod/overwrite/profiles/cache/download directories, profile-local flags, skip rules, and supported custom executable paths, while incomplete, missing, or non-Fallout configurations return manager-specific typed errors and block silent game-path fallback. Vortex detection remains identity-only, returning display name, executable path, parsed or `0.0.0` fallback version, and no staging/config parsing. Discovery reports also include fake-backed `SystemMetadata` so later Overview work can display PC Specs/DISC-02 data without querying the real host in tests.

Fake-backed unit tests cover discovery ordering, executable-path normalization, optional derived paths, registry error behavior, MO2 portable/instance ordering and typed failures, Vortex version fallback, and system metadata inclusion.

## Verification

Fresh verification ran the required Rust gates successfully. `cargo fmt --check` passed with no formatting changes required. `cargo check` passed. `cargo test` passed with 87 tests, including 16 discovery-service tests for the T03 must-haves. `cargo clippy --all-targets --all-features` passed with no warnings.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 320ms |
| 2 | `cargo check` | 0 | ✅ pass | 7923ms |
| 3 | `cargo test` | 0 | ✅ pass (87 passed) | 7501ms |
| 4 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 7825ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/services/mod.rs`
- `src/services/discovery.rs`
- `src/main.rs`
