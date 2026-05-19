# Collective Modding Toolkit Rust Port

## What This Is

This project ports the existing `CMT/` Collective Modding Toolkit desktop application to Rust using the Slint GUI framework. The Rust application preserves the original Tkinter application's workflows, tab structure, labels, defaults, validation behavior, and user-facing messages as closely as practical while keeping the implementation idiomatic, testable, and responsive.

The reference app is the Python source under `CMT/src/`; the new implementation lives outside `CMT/` in the Rust crate. Milestone M001 (Initial Port) established the buildable Rust/Slint foundation and ported the major reference surfaces as vertical slices.

## Core Value

Fallout 4 mod users can run a faithful Rust/Slint Collective Modding Toolkit that performs the same practical checks and utility workflows as the original CMT app without relying on the Python/Tkinter implementation.

## Current State After M001

M001 delivered a validated Initial Port across eleven completed slices:

- Slint shell with the `Collective Modding Toolkit` identity and tabs ordered `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, and `About`.
- Typed Settings defaults, persistence, repair behavior, save-failure rollback, scanner toggles, and downgrader preferences.
- Fakeable discovery, filesystem, registry, process, desktop, clipboard, and worker handoff seams.
- Overview diagnostics for game, mod manager, binaries, archives, modules, update state, problem feed, and safe open/link feedback.
- Tools and About tabs with reference groupings, attribution, Rust-owned image resources, safe link/copy actions, and live utility entrypoints.
- F4SE DLL compatibility diagnostics using fail-closed local PE inspection without loading plugin DLLs.
- Scanner read-only scan workflow with settings, progress, grouped results, details, copy/open actions, MO2/Vortex handling, and Overview issue handoff.
- Scanner Auto-Fix lifecycle plumbing with an empty production registry to match the current reference behavior and fail-closed future extension points.
- Downgrade Manager modal with read-only preview, explicit confirmation, digest-bound execution, pinned delta validation, backup/delta preferences, and live progress/log feedback.
- Archive Patcher modal with Overview-authoritative archive candidates, preview digest, latest-run restore manifest, bounded BA2 version-field writes, post-write validation, restore-last-run, and live progress/log feedback.
- Validation traceability remediation covering R001-R054, S01-S11 summary/assessment/UAT artifacts, and corrected cross-slice provenance.

Automated closeout evidence for M001 passed: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked` (365 passed), `cargo clippy --locked --all-targets --all-features`, targeted mutation-safety tests, the S11 artifact verifier, roadmap checkbox audit, and `git status --short CMT` with no output. Manual desktop visual comparison, live Fallout 4 install testing, live network provider testing, and destructive real-file UAT remain future release-candidate activities rather than hidden M001 claims.

## Requirements

### Validated

- [x] R001-R005 FOUND — Rust/Slint foundation, app identity, tab order, module separation, verification gates, and read-only `CMT/` discipline.
- [x] R006-R011 SET — Settings defaults, persistence keys, update/log labels including schema-supported `Warning`, scanner defaults, and repair/rollback behavior.
- [x] R012-R016 DISC — Fallout 4/mod-manager discovery, injectable filesystem/process/desktop adapters, PC specs representation, and non-blocking update checks.
- [x] R017-R024 OVR — Overview summary, binaries, archives, modules, update banner, helper actions, and Scanner-ready problem records.
- [x] R025-R029 F4SE — F4SE tab, plugin DLL scan, compatibility table, missing-folder guidance, and non-blocking worker-backed scanning.
- [x] R030-R039 SCAN — Scanner controls/settings, progress, MO2/Vortex attribution, reference classifications, grouped details, copy/open actions, Auto-Fix feedback seam, and Overview-derived issues.
- [x] R040-R045 TOOL — Tools groupings, live Downgrade Manager and Archive Patcher entrypoints, external links, failure reporting, downgrader preferences, fail-closed archive patching, and off-thread file-changing workflows.
- [x] R046-R049 ABOUT — About attribution, project/community links, Discord invite action, and visible link/copy failure feedback.
- [x] R050-R054 SAFE — Long-running work off the Slint UI thread, typed worker events, Slint-free testable domain logic, fail-closed file-changing workflows, and behavior/labels/defaults/messages checked against `CMT/src/`.

### Active

No active M001 v1 requirements remain. Future milestones should add or reopen explicit requirements before expanding scope.

### Candidate Follow-ups

- Run manual desktop visual/UAT passes against the Python reference on a representative Windows setup.
- Run real-install or disposable-copy Fallout 4 validation for discovery, Scanner, Downgrade Manager, and Archive Patcher workflows.
- Exercise live update providers/network failure modes before release-candidate distribution.
- Optionally harden Archive Patcher restore-manifest path handling so config-directory creation failure becomes a visible error instead of falling back to `archive-patcher-latest.json` in the current working directory.
- Optionally clean warning-level Clippy noise before adopting a stricter `-D warnings` policy.

### Out of Scope

- Redesigning the application or modernizing workflows before the original behavior is faithfully ported — UI fidelity remains the priority for this project stage.
- Editing, formatting, moving, deleting, or generating files under `CMT/` — the reference submodule remains read-only.
- Adding new CMT product features not present in the reference app unless explicitly requested later.
- Shipping a web, mobile, or non-Slint UI — the target is a Rust desktop application using Slint.
- Requiring a Python runtime for new Rust application behavior — Python remains reference material only.

## Context

The repository now contains a Rust crate named `cmt-rs` with Slint UI files under `ui/`, Rust domain/controller/service/platform/worker modules under `src/`, Rust-owned image resources under `resources/images/`, and the read-only `CMT/` reference submodule. The app identifies itself as `Collective Modding Toolkit`, uses the reference tab order, and implements the major M001 workflows through typed Rust contracts and Slint projection.

Important reference files remain `CMT/src/main.py`, `CMT/src/cm_checker.py`, `CMT/src/tabs/*.py`, `CMT/src/app_settings.py`, `CMT/src/scan_settings.py`, `CMT/src/game_info.py`, `CMT/src/downgrader.py`, `CMT/src/autofixes.py`, and `CMT/src/patcher/`. Future work should continue inspecting the relevant reference files before changing behavior.

## Constraints

- **Reference source**: `CMT/` is read-only and must be inspected before porting or changing behavior; it is the source of truth for labels, ordering, defaults, validation, and messages.
- **Tech stack**: Rust with Slint for UI; Rust handles application state, filesystem work, parsing, and command execution.
- **UI fidelity**: Match the original layout, tab names, grouping, button text, enabled/disabled states, and conservative visual language as closely as Slint allows.
- **Responsiveness**: Slow scans, archive parsing, filesystem traversal, downloads, patching, and process work must run off the Slint UI thread and marshal results back safely.
- **Quality gates**: Relevant checks are `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` before considering implementation slices complete.
- **Scope control**: Port in vertical slices and avoid broad refactors or new dependencies unless they directly support the port.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Use Rust + Slint for the port | Matches the project goal and enables a native desktop UI without Tkinter/Python runtime dependence | Validated in M001 through the running Slint shell and all completed workflow surfaces. |
| Treat `CMT/` as read-only reference material | Prevents accidental divergence from the source application and protects the submodule | Validated by repeated clean `git status --short CMT` evidence, including final closeout. |
| Port by vertical tab/workflow slices | Keeps the app buildable while preserving behavior one user-facing area at a time | Validated by completed slices S01-S11 and cross-slice integration review. |
| Preserve original behavior before redesigning | The project goal is fidelity, not a new product direction | Mostly validated; deviations are explicitly safety-oriented and documented, such as Downgrader confirmation and Archive Patcher restore support. |
| Keep UI/domain/platform/workers separated | Allows behavior to be tested without GUI automation and keeps slow or OS-dependent work fakeable | Validated by Slint-free domain/service/controller tests and worker/event-loop handoff patterns across M001. |
| Use fail-closed plans for file-changing workflows | Fallout 4 mod files can be valuable; mutation must be previewed, bounded, validated, and recoverable | Validated by S08 Auto-Fix gating, S09 Downgrader digest-bound execution, and S10 Archive Patcher manifest/digest workflow. |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check - still the right priority?
3. Audit Out of Scope - reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-19 during M001 closeout after final verification gates passed*
