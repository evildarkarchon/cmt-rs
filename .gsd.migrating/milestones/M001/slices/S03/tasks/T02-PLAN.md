# T02: 03-platform-discovery-background-adapters 02

**Slice:** S03 — **Milestone:** M001

## Description

Create injectable platform seams for filesystem, registry, process/version metadata, and desktop launch/open operations, including fake-backed tests and focused production dependencies.

Purpose: Discovery and later UI actions must not depend directly on real OS state or silently fail; tests must fake platform inputs without launching windows or processes.
Output: Platform adapter modules exported from `src/platform/mod.rs`, dependencies in Cargo files, and unit tests for fake and typed failure behavior.

## Must-Haves

- [ ] "Discovery and later scan code can read file/directory inputs through fakeable filesystem traits."
- [ ] "D-13: Registry, process list, version metadata, PC Specs/system metadata, URL open, path open, and tool launch operations are injectable and return typed failure kinds plus safe user-facing messages."
- [ ] "D-15: Non-Windows real platform operations return explicit typed UnsupportedPlatform-style errors while fake-backed tests and public domain models remain usable cross-platform."
- [ ] "D-16: Process/desktop launch and open failures surface as typed action result values with operation kind, target, success/failure, and safe message; adapters never show dialogs directly or log-only failures."
- [ ] "Adapter tests run without a real Fallout 4 install, registry state, running manager, external process launch, or visible desktop handler."

## Files

- `Cargo.toml`
- `Cargo.lock`
- `src/platform/filesystem.rs`
- `src/platform/registry.rs`
- `src/platform/process.rs`
- `src/platform/desktop.rs`
- `src/platform/mod.rs`
