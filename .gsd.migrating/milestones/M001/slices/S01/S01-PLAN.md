# S01: Slint Shell Port Architecture

**Goal:** Establish the buildable Rust/Slint walking skeleton: Cargo knows about Slint, build.
**Demo:** Establish the buildable Rust/Slint walking skeleton: Cargo knows about Slint, build.

## Must-Haves


## Tasks

- [x] **T01: Plan 01**
  - Establish the buildable Rust/Slint walking skeleton: Cargo knows about Slint, build.rs compiles an external Slint file, and src/main.rs launches a generated MainWindow instead of printing Hello World.

Purpose: This creates the first executable vertical slice for the port foundation while keeping the UI inert and deferring tab behavior to later plans and phases.
Output: Updated Cargo dependency baseline, Slint build script, Rust entry point, and minimal external MainWindow file.
- [x] **T02: Plan 02**
  - Turn the Slint window into the faithful inert shell: one component file per tab, exact reference tab labels/order, and static placeholder notes that do not imply implemented behavior.

Purpose: This is the user-visible walking skeleton slice for Phase 1: the desktop shell looks like the port target while real tab behavior remains explicitly deferred.
Output: ui/main.slint wired to six tab component files.
- [x] **T03: Plan 03**
  - Add the Rust architecture boundary stubs and automated shell-contract test that make Phase 1 safe to extend without moving domain behavior into Slint markup.

Purpose: This closes the walking skeleton by proving the tab identity contract in Rust and creating empty, documented module seams for future vertical slices.
Output: app/domain/platform/workers modules with no-op public types/functions, canonical tab-label test, and final verification gates.

## Files Likely Touched

