---
phase: 01-slint-shell-port-architecture
plan: 01
subsystem: ui
tags: [rust, slint, slint-build, cargo, desktop-shell]

requires: []
provides:
  - Aligned Slint runtime/build dependency baseline for the Rust desktop shell
  - Cargo build script that compiles an external Slint MainWindow source file
  - Rust entry point that launches the generated Slint MainWindow
affects: [phase-01-slint-shell-port-architecture, phase-01-plan-02-tabs, phase-01-plan-03-architecture-boundaries]

tech-stack:
  added: [slint 1.16.1, slint-build 1.16.1, anyhow 1.0.102, directories 6.0.0, serde 1.0.228, thiserror 2.0.18, tokio 1.52.3, toml 1.1.2, tracing 0.1.44, tracing-subscriber 0.3.23]
  patterns: [external-slint-build-script, generated-slint-module-startup, inert-shell-placeholder]

key-files:
  created: [build.rs, ui/main.slint]
  modified: [Cargo.toml, src/main.rs]

key-decisions:
  - "Use external Slint compilation through build.rs and ui/main.slint for the first GUI shell slice."
  - "Keep plan 01 UI inert: only a MainWindow placeholder, with tabs and behavior deferred to later plans."
  - "Add only foundation dependencies and defer scanner/archive/Fallout parser crates."

patterns-established:
  - "Slint build pipeline: Cargo build script calls slint_build::compile on a repository-local UI file."
  - "Startup pattern: src/main.rs includes generated modules and runs MainWindow through Slint's PlatformError result path."

requirements-completed: [FOUND-01, FOUND-03, FOUND-04]

duration: 24min
completed: 2026-05-17
---

# Phase 01 Plan 01: Slint Dependency/Build Pipeline Summary

**External Slint MainWindow build pipeline with aligned runtime/build crates and a Rust entry point that launches the generated window.**

## Performance

- **Duration:** 24 min
- **Started:** 2026-05-17T01:45:00Z
- **Completed:** 2026-05-17T02:09:17Z
- **Tasks:** 3 completed
- **Files modified:** 4

## Accomplishments

- Added the Phase 1 foundation dependency baseline in `Cargo.toml`, with `slint` and `slint-build` aligned at `1.16.1`.
- Created `build.rs` so Cargo compiles the external `ui/main.slint` source via `slint_build::compile("ui/main.slint")`.
- Added a minimal inert `MainWindow` Slint shell titled exactly `Collective Modding Toolkit`.
- Replaced the old console `Hello, world!` entry point with generated Slint module inclusion and `MainWindow::new()?.run()` startup.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Slint build/runtime dependency baseline** - `3ce30b9` (chore)
2. **Task 2: Compile the external Slint MainWindow** - `422c1f7` (feat)
3. **Task 3: Replace console startup with Slint window startup** - `5565798` (feat)

**Plan metadata:** pending final docs commit

## Files Created/Modified

- `Cargo.toml` - Declares `build = "build.rs"`, aligned Slint dependencies, and focused foundation crates.
- `build.rs` - Compiles only the repository-local `ui/main.slint` path through `slint-build`.
- `ui/main.slint` - Defines the first inert `MainWindow` shell with the required title.
- `src/main.rs` - Includes generated Slint modules and runs the generated `MainWindow`.

## Decisions Made

- Used the official external `.slint` build-script path from Slint documentation rather than inline `slint!` markup.
- Kept `src/main.rs` direct and simple, so no new public startup helper or extra doc comment was needed.
- Deferred tab components, Rust boundary modules, tab-order tests, and full shell fidelity checks to the remaining Phase 1 plans per the plan split.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added a temporary no-op build script during Task 1**
- **Found during:** Task 1 (Add Slint build/runtime dependency baseline)
- **Issue:** Adding `build = "build.rs"` to `Cargo.toml` before Task 2 would make `cargo check` fail if `build.rs` did not exist yet.
- **Fix:** Created a temporary no-op `build.rs` in Task 1, then replaced it with the planned `slint_build::compile("ui/main.slint")` implementation in Task 2.
- **Files modified:** `build.rs`
- **Verification:** `cargo check` passed after Task 1 and again after Task 2.
- **Committed in:** `3ce30b9` and completed in `422c1f7`

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** The deviation preserved the per-task cargo-check gate without changing final architecture or scope.

## Issues Encountered

None - all planned checks passed after each task.

## Known Stubs

| File | Line | Stub | Reason |
|------|------|------|--------|
| `ui/main.slint` | 7 | `Collective Modding Toolkit Slint shell placeholder...` | Intentional inert shell body for Plan 01; Plan 02 adds reference-order tabs and per-tab placeholders. |

## Threat Flags

None - the new build-script filesystem surface is the planned `build.rs -> ui/main.slint` trust boundary covered by `T-01-01-01`, and startup remains limited to `MainWindow` creation/run per `T-01-01-02`.

## Verification

- `cargo check` after Task 1: passed
- `cargo check` after Task 2: passed
- `cargo check` after Task 3: passed
- `cargo fmt --check`: passed
- `cargo check`: passed
- `cargo test`: passed
- `cargo clippy --all-targets --all-features`: passed
- `git status --short CMT`: passed with no output

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `01-02-PLAN.md` to replace the single inert shell body with the reference-order tab components.

## Self-Check: PASSED

- Found expected files: `Cargo.toml`, `build.rs`, `src/main.rs`, `ui/main.slint`.
- Found task commits in git history: `3ce30b9`, `422c1f7`, `5565798`.
- Verified `CMT/` remained untouched with `git status --short CMT`.

---
*Phase: 01-slint-shell-port-architecture*
*Completed: 2026-05-17*
