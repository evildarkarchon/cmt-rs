# S03: Platform Discovery Background Adapters

**Goal:** Create the pure Rust domain contracts for Phase 3: Fallout 4 installation state, optional derived paths, archive/module/INI representation, mod-manager context, semantic versions, MO2 configuration results, and typed safe errors.
**Demo:** Create the pure Rust domain contracts for Phase 3: Fallout 4 installation state, optional derived paths, archive/module/INI representation, mod-manager context, semantic versions, MO2 configuration results, and typed safe errors.

## Must-Haves


## Tasks

- [x] **T01: 03-platform-discovery-background-adapters 01**
  - Create the pure Rust domain contracts for Phase 3: Fallout 4 installation state, optional derived paths, archive/module/INI representation, mod-manager context, semantic versions, MO2 configuration results, and typed safe errors.

Purpose: Later adapters, discovery orchestration, Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows need stable typed data instead of ad hoc strings.
Output: Public domain modules and fake-free unit tests that require no Slint window and no real filesystem/process state.
- [x] **T02: 03-platform-discovery-background-adapters 02**
  - Create injectable platform seams for filesystem, registry, process/version metadata, and desktop launch/open operations, including fake-backed tests and focused production dependencies.

Purpose: Discovery and later UI actions must not depend directly on real OS state or silently fail; tests must fake platform inputs without launching windows or processes.
Output: Platform adapter modules exported from `src/platform/mod.rs`, dependencies in Cargo files, and unit tests for fake and typed failure behavior.
- [x] **T03: 03-platform-discovery-background-adapters 03**
  - Implement the reference-compatible discovery orchestration service over the domain contracts and platform adapter traits.

Purpose: Later user workflows need one tested service that discovers or represents Fallout 4 and mod-manager state without real OS dependencies in tests and without UI prompts.
Output: `src/services/discovery.rs`, service exports, and fake-backed unit tests for discovery ordering, path normalization, optional derived paths, registry errors, and manager-specific errors.
- [x] **T04: 03-platform-discovery-background-adapters 04**
  - Replace the inert worker boundary with reusable typed worker event contracts, cancellation states, recording/Slint handoff sinks, and a small off-UI-thread execution facade.

Purpose: Later scans, filesystem traversal, parsing, downloads, patching, and process monitoring need consistent progress/completion/cancellation/error events delivered through Slint-safe handoff.
Output: Worker event and handoff modules with unit tests that require no Slint window, plus compile-safe Slint event-loop sink code.

## Files Likely Touched

- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/domain/mod.rs`
- `Cargo.toml`
- `Cargo.lock`
- `src/platform/filesystem.rs`
- `src/platform/registry.rs`
- `src/platform/process.rs`
- `src/platform/desktop.rs`
- `src/platform/mod.rs`
- `src/services/mod.rs`
- `src/services/discovery.rs`
- `src/main.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
