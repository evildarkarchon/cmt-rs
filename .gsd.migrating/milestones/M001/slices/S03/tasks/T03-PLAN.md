# T03: 03-platform-discovery-background-adapters 03

**Slice:** S03 — **Milestone:** M001

## Description

Implement the reference-compatible discovery orchestration service over the domain contracts and platform adapter traits.

Purpose: Later user workflows need one tested service that discovers or represents Fallout 4 and mod-manager state without real OS dependencies in tests and without UI prompts.
Output: `src/services/discovery.rs`, service exports, and fake-backed unit tests for discovery ordering, path normalization, optional derived paths, registry errors, and manager-specific errors.

## Must-Haves

- [ ] "D-01: Discovery executes the locked reference order: running manager game path, current working directory, then Bethesda/GOG registry paths."
- [ ] "D-02: Discovery returns recoverable typed not-found results with reference-compatible messages and does not show manual file-picker UI."
- [ ] "D-03: A direct `Fallout4.exe` candidate normalizes to its parent game directory."
- [ ] "D-04: A valid game directory can produce partial derived state with missing Data or Data/F4SE/Plugins represented as missing/None fields."
- [ ] "D-05: MO2 discovery parses gamePath, selected_profile, mod/overwrite/profiles directories, profile-local flags, and skip rules for later phases."
- [ ] "D-06: MO2 portable/instance discovery checks adjacent portable files first, then HKCU CurrentInstance under LOCALAPPDATA."
- [ ] "D-07: MO2 incomplete, missing, or non-Fallout state returns manager-specific typed errors without panicking or silently falling through."
- [ ] "D-08: Vortex detection returns display name, executable path, parsed/fallback version, and no staging/config parsing."
- [ ] "Discovery results include fake-backed PC Specs/system metadata so later Overview can display DISC-02 data without querying real host state in tests."

## Files

- `src/services/mod.rs`
- `src/services/discovery.rs`
- `src/main.rs`
