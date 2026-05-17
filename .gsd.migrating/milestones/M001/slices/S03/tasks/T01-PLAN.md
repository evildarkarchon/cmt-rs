# T01: 03-platform-discovery-background-adapters 01

**Slice:** S03 — **Milestone:** M001

## Description

Create the pure Rust domain contracts for Phase 3: Fallout 4 installation state, optional derived paths, archive/module/INI representation, mod-manager context, semantic versions, MO2 configuration results, and typed safe errors.

Purpose: Later adapters, discovery orchestration, Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows need stable typed data instead of ad hoc strings.
Output: Public domain modules and fake-free unit tests that require no Slint window and no real filesystem/process state.

## Must-Haves

- [ ] "D-02: Rust code can represent recoverable typed discovery failures with reference-compatible not-found messages and no manual file-picker UI requirement."
- [ ] "D-04: Rust code can represent a valid Fallout 4 installation without requiring Data or F4SE folders to exist."
- [ ] "D-05: Rust code can represent MO2 context fields for gamePath, selected_profile, mod_directory, overwrite_directory, profiles_directory, profile-local flags, and skip rules."
- [ ] "D-07: Known MO2 parse failures carry manager-specific typed kinds plus reference-compatible user-facing messages."
- [ ] "D-08: Rust code can represent Vortex detection scope with exact display name, executable path, semantic version fallback 0.0.0, and no staging/config parsing."
- [ ] "D-13: Known discovery and MO2 parse failures carry typed error kinds plus safe user-facing messages."
- [ ] "D-14: User-facing messages use known/reference text and keep raw OS details in diagnostics unless the reference intentionally includes the path."

## Files

- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/domain/mod.rs`
