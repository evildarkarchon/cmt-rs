# Roadmap: Collective Modding Toolkit Rust Port

## Overview

This roadmap ports the reference `CMT/` application into a faithful Rust/Slint desktop app through fine-grained, sequential, vertical MVP slices. The path starts with a buildable Slint shell and architecture boundaries, then preserves settings, platform discovery, read-only diagnostics, shared link/tool behavior, scanner parity, and finally the file-changing downgrade/archive workflows with explicit safety gates. Every phase keeps `CMT/` read-only, compares user-facing behavior against `CMT/src/`, and remains buildable with the relevant Rust checks.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Slint Shell & Port Architecture** - Developer can run the CMT Slint shell with reference identity, tab order, and safe module boundaries.
- [ ] **Phase 2: Settings & Defaults Parity** - User settings load, validate, persist, and appear with reference-compatible labels/defaults.
- [ ] **Phase 3: Platform Discovery & Background Adapters** - Shared filesystem, process, discovery, and worker seams support later tabs without blocking the UI.
- [ ] **Phase 4: Overview Diagnostics & Updates** - User can see Overview summary panels, typed diagnostics, and update-banner behavior.
- [ ] **Phase 5: Tools Shell, Links & About** - User can use static Tools/About link workflows with reference labels and visible failures.
- [ ] **Phase 6: F4SE Diagnostics** - User can inspect F4SE plugin compatibility in the reference table shape without UI stalls.
- [ ] **Phase 7: Scanner Read-Only Results** - User can run Scanner, see grouped read-only results, inspect details, and use copy/open actions.
- [ ] **Phase 8: Scanner Auto-Fix Actions** - User sees and runs supported Scanner auto-fixes with clear success/failure feedback.
- [ ] **Phase 9: Downgrade Manager Workflow** - User can open and run the Downgrade Manager with backup/delta safety and responsive status.
- [ ] **Phase 10: Archive Patcher Workflow** - User can open and run Archive Patcher operations through validated, fail-closed write plans.

## Phase Details

### Phase 1: Slint Shell & Port Architecture
**Goal:** Developer can build and run the Rust/Slint CMT shell while preserving the reference app identity and creating safe porting boundaries.
**Mode:** standard
**Depends on:** Nothing (first phase)
**Requirements:** FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, SAFE-05
**Success Criteria** (what must be TRUE):
  1. Developer can run a Slint desktop app from the Rust crate and see `Collective Modding Toolkit` with tabs ordered `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
  2. Developer can add behavior through separated UI, controller/app, domain, platform, and worker modules without placing domain logic in Slint markup.
  3. Developer can run the core verification commands for the slice: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.
  4. Developer can verify that no implementation change modifies files under `CMT/` and that user-facing labels/defaults are checked against `CMT/src/` before completing each slice.
**Plans:** 3 plans
Plans:
- [x] 01-01-PLAN.md — Establish Slint dependency/build pipeline and launch generated MainWindow.
- [x] 01-02-PLAN.md — Wire inert reference-order tab components in the Slint shell.
- [x] 01-03-PLAN.md — Add no-op Rust module boundaries, tab-order test, and final verification gates.
**UI hint:** yes

### Phase 2: Settings & Defaults Parity
**Goal:** User settings behave like the reference app, including defaults, validation, persistence, and visible Settings-tab controls.
**Mode:** standard
**Depends on:** Phase 1
**Requirements:** SET-01, SET-02, SET-03, SET-04, SET-05, SET-06
**Success Criteria** (what must be TRUE):
  1. User settings load with reference-compatible defaults when no settings file exists.
  2. User can choose update channel options labeled `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, and `Never: Don't Check`.
  3. User can choose log levels labeled `Debug`, `Info`, and `Error`, and settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`.
  4. Scanner-related settings default to enabled for Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs.
  5. Invalid or incomplete settings preserve valid values and safely fall back to documented defaults for invalid values.
**Plans:** 4 plans
Plans:
- [x] 02-01-PLAN.md — Create typed settings defaults, JSON key contract, and repair tests.
- [x] 02-02-PLAN.md — Implement injectable settings file IO, asset fallback, and safe persistence.
- [x] 02-03-PLAN.md — Render Settings-tab reference labels and source-level UI contract tests.
- [x] 02-04-PLAN.md — Wire Settings callbacks to immediate persistence with save-failure reversion.
**UI hint:** yes

### Phase 3: Platform Discovery & Background Adapters
**Goal:** Later user workflows can rely on typed game/mod-manager discovery, filesystem/process seams, and Slint-safe background event handoff.
**Mode:** standard
**Depends on:** Phase 2
**Requirements:** DISC-01, DISC-02, DISC-03, DISC-04, SAFE-01, SAFE-02, SAFE-03
**Success Criteria** (what must be TRUE):
  1. App can discover or represent the Fallout 4 game path and mod-manager context needed by Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows.
  2. App can read required file/directory sources through injectable filesystem adapters that tests can fake without launching a window.
  3. App can launch URLs, open paths, and run external tools through injectable process adapters with visible failure reporting.
  4. Long-running scans, filesystem traversal, parsing, downloads, patching, and process monitoring run off the Slint UI thread.
  5. Background work returns typed progress, completion, cancellation, and error events through Slint-safe event-loop handoff.
**Plans:** TBD
**UI hint:** yes

### Phase 4: Overview Diagnostics & Updates
**Goal:** User can use the Overview tab to understand game/mod-manager state, binary/archive/module diagnostics, and update availability.
**Mode:** standard
**Depends on:** Phase 3
**Requirements:** DISC-05, OVR-01, OVR-02, OVR-04, OVR-06, OVR-07, OVR-08
**Success Criteria** (what must be TRUE):
  1. User sees Overview summary labels matching the reference: `Mod Manager:`, `Game Path:`, `Version:`, and `PC Specs:`.
  2. User sees Binaries `(EXE/DLL/BIN)`, Archives `(BA2)`, and Modules `(ESM/ESL/ESP)` panels with the reference count/status categories.
  3. Overview diagnostics produce typed problem records that can later be included in Scanner results when Overview Issues are enabled.
  4. Overview refresh and update-banner behavior preserve reference update-source semantics and user-facing links without blocking startup or the UI thread.
**Plans:** TBD
**UI hint:** yes

### Phase 5: Tools Shell, Links & About
**Goal:** User can access non-mutating Tools and About workflows with reference labels, attribution, copy/open actions, and failure feedback.
**Mode:** standard
**Depends on:** Phase 4
**Requirements:** TOOL-01, TOOL-02, TOOL-03, ABOUT-01, ABOUT-02, ABOUT-03, ABOUT-04
**Success Criteria** (what must be TRUE):
  1. User sees Tools tab groupings matching the reference: Toolkit Utilities, Other CM Authors' Tools, and Other Useful Tools.
  2. User can open Toolkit Utility entry points for `Downgrade Manager` and `Archive Patcher` without performing file-changing work yet.
  3. User can open external tool links from the Tools tab with reference labels and visible failure reporting.
  4. User sees About attribution matching the reference, including `Created by wxMichael for the Collective Modding Community`.
  5. User can open and copy relevant project/community links and the Discord invite from the About tab, with visible failure reporting.
**Plans:** TBD
**UI hint:** yes

### Phase 6: F4SE Diagnostics
**Goal:** User can inspect F4SE plugin DLL compatibility and missing-folder guidance in a responsive F4SE tab.
**Mode:** standard
**Depends on:** Phase 5
**Requirements:** F4SE-01, F4SE-02, F4SE-03, F4SE-04, F4SE-05
**Success Criteria** (what must be TRUE):
  1. User can open the F4SE tab and trigger or observe scanning of `Data/F4SE/Plugins` DLLs.
  2. User sees F4SE table columns matching the reference: `DLL`, `OG`, `NG`, `AE`, and `Your Game`.
  3. User sees reference-compatible known DLL compatibility statuses for original, next-gen, anniversary, and current game versions.
  4. User sees reference-compatible missing-folder guidance when the Data folder or `Data/F4SE/Plugins` folder is unavailable.
  5. F4SE scanning runs without blocking the Slint UI thread.
**Plans:** TBD
**UI hint:** yes

### Phase 7: Scanner Read-Only Results
**Goal:** User can run the Scanner in read-only mode, see progress and grouped results, inspect details, and use safe copy/open actions.
**Mode:** standard
**Depends on:** Phase 6
**Requirements:** SCAN-01, SCAN-02, SCAN-03, SCAN-04, SCAN-05, SCAN-06, SCAN-07, SCAN-08, SCAN-10
**Success Criteria** (what must be TRUE):
  1. User can open the Scanner tab and see `Scan Game`, `Scan Settings`, `Collapse All`, and `Expand All` actions with reference labels and category defaults.
  2. User can start a game scan, see `Scanning...` style progress/status, and continue using the UI while scanning runs.
  3. Scanner builds a mod-attributed file list from discovered game/mod-manager context and classifies all reference problem types in read-only mode.
  4. User sees scan results grouped and expandable with `Problem` and `Files` style detail information.
  5. User can select a result, see `Mod:`, `Problem:`, `Summary:`, and `Solution:`, use URL open/copy actions, `Copy Details`, and receive Overview-derived issues when enabled.
**Plans:** TBD
**UI hint:** yes

### Phase 8: Scanner Auto-Fix Actions
**Goal:** User can identify and run supported Scanner auto-fix actions with reference-compatible availability and feedback.
**Mode:** standard
**Depends on:** Phase 7
**Requirements:** SCAN-09
**Success Criteria** (what must be TRUE):
  1. User sees auto-fix actions only on Scanner results where the reference supports an automatic fix.
  2. User can run a supported auto-fix without blocking the UI.
  3. User receives clear `Fixed!` or `Fix Failed` feedback matching the reference workflow semantics.
**Plans:** TBD
**UI hint:** yes

### Phase 9: Downgrade Manager Workflow
**Goal:** User can open and run the Downgrade Manager from Overview/Tools with backup and delta cleanup settings respected.
**Mode:** standard
**Depends on:** Phase 8
**Requirements:** OVR-03, TOOL-04, TOOL-06
**Success Criteria** (what must be TRUE):
  1. User can open the Downgrade Manager action from the Overview binaries panel and the Tools tab entry point.
  2. Downgrade Manager honors backup and delta cleanup settings before performing file-changing operations.
  3. Destructive or file-changing downgrade operations run off the UI thread and preserve responsive status/error reporting.
**Plans:** TBD
**UI hint:** yes

### Phase 10: Archive Patcher Workflow
**Goal:** User can open and run Archive Patcher operations through validated, fail-closed plans that protect user files.
**Mode:** standard
**Depends on:** Phase 9
**Requirements:** OVR-05, TOOL-05, SAFE-04
**Success Criteria** (what must be TRUE):
  1. User can open the Archive Patcher action from the Overview archives panel and the Tools tab entry point.
  2. Archive Patcher validates inputs before writing and fails closed when required metadata or files are unsafe or unavailable.
  3. File-changing patch workflows use backups, dry-run plans, validation, or fail-closed behavior where the reference workflow can alter user files.
**Plans:** TBD
**UI hint:** yes

## Progress

**Execution Order:**
Phases execute sequentially in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Slint Shell & Port Architecture | 3/3 | Complete | 2026-05-17 |
| 2. Settings & Defaults Parity | 0/TBD | Not started | - |
| 3. Platform Discovery & Background Adapters | 0/TBD | Not started | - |
| 4. Overview Diagnostics & Updates | 0/TBD | Not started | - |
| 5. Tools Shell, Links & About | 0/TBD | Not started | - |
| 6. F4SE Diagnostics | 0/TBD | Not started | - |
| 7. Scanner Read-Only Results | 0/TBD | Not started | - |
| 8. Scanner Auto-Fix Actions | 0/TBD | Not started | - |
| 9. Downgrade Manager Workflow | 0/TBD | Not started | - |
| 10. Archive Patcher Workflow | 0/TBD | Not started | - |
