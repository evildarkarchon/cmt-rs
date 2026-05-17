# Collective Modding Toolkit Rust Port

## What This Is

This project ports the existing `CMT/` Collective Modding Toolkit desktop application to Rust using the Slint GUI framework. The Rust application should preserve the original Tkinter application's workflows, tab structure, labels, defaults, validation behavior, and user-facing messages as closely as practical while keeping the implementation idiomatic, testable, and responsive.

The reference app is the Python source under `CMT/src/`; the new implementation lives outside `CMT/` in the Rust crate. The initial milestone is an "Initial Port" that establishes a faithful, buildable Rust/Slint foundation and then ports the original app in narrow vertical slices.

## Core Value

Fallout 4 mod users can run a faithful Rust/Slint Collective Modding Toolkit that performs the same practical checks and utility workflows as the original CMT app without relying on the Python/Tkinter implementation.

## Requirements

### Validated

- [x] Phase 02 validated Settings persistence and defaults, including update channel, log level, scanner toggles, and downgrader options. The Settings tab now uses the dark-only UI palette, exposes `Debug`, `Info`, `Warning`, and `Error`, and persists/repairs settings through typed Rust domain and platform boundaries.

### Active

- [ ] Create a Rust/Slint desktop shell that matches the original `Collective Modding Toolkit` window identity and tab order: Overview, F4SE, Scanner, Tools, Settings, About.
- [ ] Preserve `CMT/` as a read-only reference submodule and implement new behavior only in the Rust project outside `CMT/`.
- [ ] Port the Overview tab's game/mod-manager status summaries for binaries, BA2 archives, modules, update prompts, and related helper actions.
- [ ] Port the F4SE tab's DLL scanning table and compatibility status behavior.
- [ ] Port the Scanner tab's selectable scan settings, scan execution flow, tree results, details pane, URL/details actions, and auto-fix result feedback.
- [ ] Port Toolkit Utilities and external tool links from the Tools tab, including Downgrade Manager and Archive Patcher workflows.
- [ ] Port the About tab's attribution, links, Discord invite actions, and original user-facing text.
- [ ] Keep long-running filesystem scans, parsing, and process work off the Slint UI thread.
- [ ] Use typed Rust domain models for settings, game/mod-manager discovery, scan results, archive/module metadata, and tool execution state.
- [ ] Keep the application buildable and covered by relevant Rust checks after each vertical slice.

### Out of Scope

- Redesigning the application or modernizing workflows before the original behavior is faithfully ported - UI fidelity is the priority for this project stage.
- Editing, formatting, moving, deleting, or generating files under `CMT/` - the reference submodule remains read-only.
- Adding new CMT product features not present in the reference app unless explicitly requested later.
- Shipping a web, mobile, or non-Slint UI - the target is a Rust desktop application using Slint.
- Requiring a Python runtime for new Rust application behavior - Python remains reference material only.

## Context

The repository currently contains a bootstrapped Rust crate named `cmt-rs` with a minimal `src/main.rs`, an existing `Cargo.lock`, and the `CMT/` reference submodule. The project instruction file already defines the porting direction: use Rust and Slint, keep UI/domain logic separated, avoid blocking the UI thread, and compare each ported tab against `CMT/src/tabs/`.

The reference app identifies itself as `Collective Modding Toolkit` v0.6.1 and builds a Tkinter notebook with the tabs `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, and `About`. Important reference files include `CMT/src/main.py`, `CMT/src/cm_checker.py`, `CMT/src/tabs/*.py`, `CMT/src/app_settings.py`, `CMT/src/scan_settings.py`, `CMT/src/game_info.py`, `CMT/src/downgrader.py`, `CMT/src/autofixes.py`, and `CMT/src/patcher/`.

The original settings default to `log_level = INFO`, an update source derived from `download-source.txt` with fallback to Nexus, all major scanner toggles enabled, and downgrader backup/delta cleanup options enabled. The scanner covers problem classes such as junk files, unexpected formats, misplaced DLLs, loose previs, loose AnimTextData, invalid archives/modules/archive names, F4SE overrides, missing files, and wrong versions.

## Constraints

- **Reference source**: `CMT/` is read-only and must be inspected before porting any behavior - it is the source of truth for labels, ordering, defaults, validation, and messages.
- **Tech stack**: Rust with Slint for UI; Rust handles application state, filesystem work, parsing, and command execution.
- **UI fidelity**: Match the original layout, tab names, grouping, button text, enabled/disabled states, and conservative visual language as closely as Slint allows.
- **Responsiveness**: Slow scans, archive parsing, filesystem traversal, and process work must run off the Slint UI thread and marshal results back safely.
- **Quality gates**: Relevant checks are `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` before considering implementation slices complete.
- **Scope control**: Port in vertical slices and avoid broad refactors or new dependencies unless they directly support the port.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Use Rust + Slint for the port | Matches the project goal and enables a native desktop UI without Tkinter/Python runtime dependence | - Pending |
| Treat `CMT/` as read-only reference material | Prevents accidental divergence from the source application and protects the submodule | - Pending |
| Port by vertical tab/workflow slices | Keeps the app buildable while preserving behavior one user-facing area at a time | - Pending |
| Preserve original behavior before redesigning | The project goal is fidelity, not a new product direction | - Pending |
| Use fine-grained sequential GSD planning | The port has many behavior-sensitive slices and the user selected fine granularity with sequential execution | - Pending |

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
*Last updated: 2026-05-17 after Phase 02 completion*
