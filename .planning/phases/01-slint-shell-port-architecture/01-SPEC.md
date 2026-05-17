# Phase 1: Slint Shell & Port Architecture - Specification

**Created:** 2026-05-17
**Ambiguity score:** 0.11 (gate: <= 0.20)
**Requirements:** 6 locked

## Goal

Developer can build and run a Rust/Slint CMT shell that shows the reference window identity and six tab labels in order, with safe module boundaries ready for later port slices.

## Background

The current Rust crate is a minimal bootstrap: `Cargo.toml` has no dependencies and `src/main.rs` only prints `Hello, world!`. There is no Slint compile pipeline, no `build.rs`, no `ui/` directory, no generated `MainWindow`, and no Rust module structure for app/controller, domain, platform, or worker boundaries.

The reference application in `CMT/src/cm_checker.py` creates the Tk notebook tabs in this order: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`. Phase 1 establishes that visible shell and porting boundary without implementing any real CMT tab behavior yet.

## Requirements

1. **Buildable Slint app**: The Rust crate builds a Slint desktop application instead of a console-only `Hello, world!` binary.
   - Current: `src/main.rs` prints `Hello, world!`; there are no Slint dependencies, build script, or UI files.
   - Target: The crate includes the Slint build pipeline and launches a desktop window from Rust.
   - Acceptance: `cargo check` succeeds and running the app opens a Slint window rather than printing only to stdout.

2. **Reference shell identity**: The visible shell identifies itself as Collective Modding Toolkit and exposes the six reference tabs in the original order.
   - Current: No GUI shell or tabs exist.
   - Target: The window title/identity uses `Collective Modding Toolkit`, and the tab labels appear as `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in that exact order.
   - Acceptance: A manual launch or UI smoke check can confirm the title/identity and exact tab order.

3. **Tabs-only placeholder scope**: Phase 1 provides only inert tab placeholders, not domain behavior or workflow logic.
   - Current: No UI exists.
   - Target: Each tab can be selected and displays placeholder content sufficient to prove the tab exists; no scanner, settings, discovery, tools, update, F4SE, overview, or about behavior is implemented.
   - Acceptance: Selecting each tab does not trigger filesystem scans, network calls, settings writes, process launches, or CMT domain actions.

4. **Recommended module boundaries**: The crate contains the recommended layer skeleton for future slices.
   - Current: Only `src/main.rs` exists.
   - Target: Source modules exist for app/controller-facing code, domain logic, platform adapters, and workers, while Slint markup remains in UI files.
   - Acceptance: The source tree contains separate `app`, `domain`, `platform`, and `workers` module entry points or equivalent files, and Slint UI files do not contain domain logic.

5. **Verification gate**: The phase defines and passes the current project verification commands.
   - Current: No Slint-specific verification has been run for this phase.
   - Target: Formatting, compilation, tests, and clippy are runnable for the current crate state.
   - Acceptance: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` are run; any failure is either fixed or explicitly documented with the blocking reason.

6. **Reference safety check**: The phase proves the reference source was not changed and the visible shell labels were checked against `CMT/src/`.
   - Current: `CMT/` is a read-only reference submodule, but Phase 1 has not yet produced an implementation to compare.
   - Target: Completion notes identify the relevant reference files used for shell labels and confirm no files under `CMT/` changed.
   - Acceptance: `git status --short CMT` produces no modified/untracked reference files, and the Phase 1 verification notes cite `CMT/src/cm_checker.py` or `CMT/src/enums.py` for tab labels/order.

## Boundaries

**In scope:**
- Add Slint runtime/build dependencies needed for a buildable desktop shell.
- Add the build script and Slint UI file(s) needed to compile and launch a `MainWindow`.
- Show the `Collective Modding Toolkit` shell with tabs `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in that order.
- Provide inert placeholder content for each tab.
- Create recommended Rust module skeletons for `app`, `domain`, `platform`, and `workers` boundaries.
- Run and report the Rust verification commands for this slice.
- Verify `CMT/` remains unmodified and shell labels were checked against reference source.

**Out of scope:**
- Implementing Overview diagnostics, settings persistence, game discovery, F4SE scanning, scanner results, tool launching, or About link behavior - those are later phases.
- Performing a full tab layout audit - later tab phases inspect their own reference files in detail.
- Adding real background jobs or Slint UI-thread handoff behavior beyond the skeleton boundary - Phase 3 owns background adapters.
- Adding new product features, scanner categories, or redesigns - this phase only establishes the faithful shell foundation.
- Editing any file under `CMT/` - the reference submodule is read-only.

## Constraints

- The shell must preserve the reference tab labels and order from `CMT/src/cm_checker.py` / `CMT/src/enums.py`.
- Slint UI files may define layout and placeholders, but domain behavior must stay out of Slint markup.
- The module skeleton should be minimal and compile cleanly; empty/future modules are acceptable only when they establish the agreed boundary.
- No implementation or generated output may modify files under `CMT/`.

## Acceptance Criteria

- [ ] `cargo check` succeeds with a Slint desktop app wired from Rust.
- [ ] Launching the app opens a window identified as `Collective Modding Toolkit`.
- [ ] The visible tabs are exactly `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in that order.
- [ ] Each tab can be selected and shows inert placeholder content only.
- [ ] Source modules exist for `app`, `domain`, `platform`, and `workers` boundaries or clearly equivalent layer entry points.
- [ ] Slint markup contains UI structure/placeholders only and no CMT domain logic.
- [ ] `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` are run and pass or have documented blockers.
- [ ] `git status --short CMT` confirms no reference submodule files were modified.
- [ ] Completion notes cite the reference file(s) used to confirm tab labels/order.

## Ambiguity Report

| Dimension           | Score | Min   | Status | Notes |
|---------------------|-------|-------|--------|-------|
| Goal Clarity        | 0.94  | 0.75  | met    | Tabs-only visible shell is locked. |
| Boundary Clarity    | 0.90  | 0.70  | met    | Real tab behavior and full tab audits are excluded. |
| Constraint Clarity  | 0.82  | 0.65  | met    | Reference labels, module boundaries, and CMT read-only constraint are explicit. |
| Acceptance Criteria | 0.88  | 0.70  | met    | Pass/fail checks cover build, UI shell, boundaries, checks, and CMT cleanliness. |
| **Ambiguity**       | 0.11  | <=0.20| met    | Gate passed after round 1. |

Status: met = dimension meets minimum; below minimum = planner treats as assumption.

## Interview Log

| Round | Perspective | Question summary | Decision locked |
|-------|-------------|------------------|-----------------|
| 1 | Researcher | What should the visible shell prove? | Tabs only: window identity plus six reference tab labels in order; placeholder content is enough. |
| 1 | Researcher | What architecture skeleton must exist? | Recommended layers: app, domain, platform, workers modules plus UI files and build script. |
| 1 | Researcher | What should the CMT reference check require? | Labels and CMT clean: verify tab labels/window identity against CMT and prove no CMT files changed. |

---

*Phase: 01-slint-shell-port-architecture*
*Spec created: 2026-05-17*
*Next step: /gsd-discuss-phase 1 - implementation decisions (how to build what's specified above)*
