# Requirements: Collective Modding Toolkit Rust Port

**Defined:** 2026-05-17
**Core Value:** Fallout 4 mod users can run a faithful Rust/Slint Collective Modding Toolkit that performs the same practical checks and utility workflows as the original CMT app without relying on the Python/Tkinter implementation.

## v1 Requirements

Requirements for the initial Rust/Slint port. Each maps to roadmap phases.

### Project Foundation

- [ ] **FOUND-01**: Developer can build and run a Slint desktop application from the Rust crate.
- [ ] **FOUND-02**: User sees the `Collective Modding Toolkit` application identity and tab order `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
- [ ] **FOUND-03**: Developer can add behavior through separated UI, app/controller, domain, platform, and worker modules without putting domain logic in Slint markup.
- [ ] **FOUND-04**: Developer can run core verification commands for the current slice: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.
- [ ] **FOUND-05**: Developer can verify that implementation changes do not modify files under `CMT/`.

### Settings

- [ ] **SET-01**: User settings load with reference-compatible defaults when no settings file exists.
- [ ] **SET-02**: User settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`.
- [ ] **SET-03**: User can choose update channel options matching the reference labels: `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, and `Never: Don't Check`.
- [ ] **SET-04**: User can choose log level options matching the reference labels: `Debug`, `Info`, and `Error`.
- [ ] **SET-05**: Scanner-related settings default to enabled for Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs.
- [ ] **SET-06**: Invalid or incomplete settings files fail safely by preserving valid values and falling back to documented defaults for invalid values.

### Platform And Discovery

- [ ] **DISC-01**: App can discover or represent the Fallout 4 game path needed by Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows.
- [ ] **DISC-02**: App can identify mod manager context and display Mod Manager, Game Path, Version, and PC Specs data in the Overview area.
- [ ] **DISC-03**: App can read the file and directory sources needed for archive, module, F4SE plugin, scanner, and settings workflows through injectable filesystem adapters.
- [ ] **DISC-04**: App can launch URLs, open paths, and run external tools through injectable process adapters with visible failure reporting.
- [ ] **DISC-05**: App can perform update checks according to `update_source` without blocking startup or the UI thread.

### Overview

- [ ] **OVR-01**: User sees Overview game/mod-manager summary labels matching the reference: `Mod Manager:`, `Game Path:`, `Version:`, and `PC Specs:`.
- [ ] **OVR-02**: User sees the Binaries `(EXE/DLL/BIN)` panel with game/F4SE/Creation Kit status data and Address Library status.
- [ ] **OVR-03**: User can open the Downgrade Manager action from the Overview binaries panel.
- [ ] **OVR-04**: User sees the Archives `(BA2)` panel with General, Texture, Total, Unreadable, OG, and NG archive counts.
- [ ] **OVR-05**: User can open the Archive Patcher action from the Overview archives panel.
- [ ] **OVR-06**: User sees the Modules `(ESM/ESL/ESP)` panel with Full, Light, Total, HEDR v1.00, HEDR v0.95, and HEDR unknown counts.
- [ ] **OVR-07**: Overview diagnostics produce typed problem records that can be included in Scanner results when Overview Issues are enabled.
- [ ] **OVR-08**: Overview refresh and update-banner behavior preserve the reference update source semantics and user-facing links.

### F4SE

- [ ] **F4SE-01**: User can open the F4SE tab and trigger or observe scanning of `Data/F4SE/Plugins` DLLs.
- [ ] **F4SE-02**: User sees F4SE table columns matching the reference: `DLL`, `OG`, `NG`, `AE`, and `Your Game`.
- [ ] **F4SE-03**: User sees reference-compatible status for known F4SE DLL compatibility across original, next-gen, anniversary, and current game versions.
- [ ] **F4SE-04**: User sees reference-compatible missing-folder guidance when the Data folder or `Data/F4SE/Plugins` folder is unavailable.
- [ ] **F4SE-05**: F4SE scanning runs without blocking the Slint UI thread.

### Scanner

- [ ] **SCAN-01**: User can open the Scanner tab and see `Scan Game`, `Scan Settings`, `Collapse All`, and `Expand All` actions matching the reference labels.
- [ ] **SCAN-02**: User can enable or disable scanner categories matching the reference defaults and settings keys.
- [ ] **SCAN-03**: User can start a game scan and see progress/status text such as `Scanning...` without blocking the UI.
- [ ] **SCAN-04**: Scanner can build a mod file list from the discovered game/mod-manager context while preserving mod attribution where the reference supports it.
- [ ] **SCAN-05**: Scanner can classify reference problem types: Junk File, Unexpected Format, Misplaced DLL, Loose Previs, Loose AnimTextData, Invalid Archive, Invalid Module, Invalid Archive Name, F4SE Script Override, File Not Found, and Wrong Version.
- [ ] **SCAN-06**: User sees scan results grouped and expandable in a tree/list model with `Problem` and `Files` style detail information.
- [ ] **SCAN-07**: User can select a result and see details for `Mod:`, `Problem:`, `Summary:`, and `Solution:`.
- [ ] **SCAN-08**: User can use detail actions equivalent to reference URL open/copy and `Copy Details` behavior.
- [ ] **SCAN-09**: User sees auto-fix actions only where supported and receives `Fixed!` or `Fix Failed` feedback.
- [ ] **SCAN-10**: Scanner results can include Overview-derived issues when the Overview Issues setting is enabled.

### Tools

- [ ] **TOOL-01**: User sees Tools tab groupings matching the reference: Toolkit Utilities, Other CM Authors' Tools, and Other Useful Tools.
- [ ] **TOOL-02**: User can launch Toolkit Utilities for `Downgrade Manager` and `Archive Patcher`.
- [ ] **TOOL-03**: User can open external tool links from the Tools tab with reference labels and visible failure reporting.
- [ ] **TOOL-04**: Downgrade Manager honors backup and delta cleanup settings before performing file-changing operations.
- [ ] **TOOL-05**: Archive Patcher performs archive-changing operations through fail-closed plans that validate inputs before writing.
- [ ] **TOOL-06**: Destructive or file-changing tool operations run off the UI thread and preserve responsive status/error reporting.

### About And Links

- [ ] **ABOUT-01**: User sees About tab attribution matching the reference, including `Created by wxMichael for the Collective Modding Community`.
- [ ] **ABOUT-02**: User can open and copy relevant project/community links from the About tab.
- [ ] **ABOUT-03**: User can open and copy the Discord invite action from the About tab.
- [ ] **ABOUT-04**: Link actions report failures visibly instead of silently failing.

### Responsiveness And Safety

- [ ] **SAFE-01**: Long-running scans, filesystem traversal, parsing, downloads, patching, and process monitoring run off the Slint UI thread.
- [ ] **SAFE-02**: Background work returns typed progress, completion, cancellation, and error events to the UI through Slint-safe event-loop handoff.
- [ ] **SAFE-03**: Domain logic can be tested without launching a window by using fake filesystem and process adapters.
- [ ] **SAFE-04**: File-changing workflows use backups, dry-run plans, validation, or fail-closed behavior where the reference workflow can alter user files.
- [ ] **SAFE-05**: User-facing labels, tab ordering, default states, and messages are compared against `CMT/src/` before completing each ported slice.

## v2 Requirements

Deferred to future releases. Tracked but not in current roadmap.

### Enhancements

- **ENH-01**: User can opt into live filesystem watching or auto-rescan behavior.
- **ENH-02**: User can run scanner/domain workflows from a CLI or headless mode.
- **ENH-03**: User can use scanner checks beyond the reference problem taxonomy.
- **ENH-04**: User can use broader Vortex staging support beyond reference behavior.
- **ENH-05**: User can use cross-game support beyond Fallout 4.
- **ENH-06**: User can use redesigned workflows or UI once parity with the original app is validated.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Editing files under `CMT/` | `CMT/` is a read-only reference submodule for this port. |
| Python runtime dependency for new app behavior | The target is a native Rust/Slint port, not a Python wrapper. |
| Web, mobile, or non-Slint UI | The project target is a desktop app built with Slint. |
| Product redesign before parity | UI fidelity and workflow preservation are the current project goal. |
| New scanner categories in v1 | Adds false-positive risk and makes parity harder to validate. |
| Automatic update installation | The reference checks for updates and opens links; auto-install adds trust and permission risk. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUND-01 | TBD | Pending |
| FOUND-02 | TBD | Pending |
| FOUND-03 | TBD | Pending |
| FOUND-04 | TBD | Pending |
| FOUND-05 | TBD | Pending |
| SET-01 | TBD | Pending |
| SET-02 | TBD | Pending |
| SET-03 | TBD | Pending |
| SET-04 | TBD | Pending |
| SET-05 | TBD | Pending |
| SET-06 | TBD | Pending |
| DISC-01 | TBD | Pending |
| DISC-02 | TBD | Pending |
| DISC-03 | TBD | Pending |
| DISC-04 | TBD | Pending |
| DISC-05 | TBD | Pending |
| OVR-01 | TBD | Pending |
| OVR-02 | TBD | Pending |
| OVR-03 | TBD | Pending |
| OVR-04 | TBD | Pending |
| OVR-05 | TBD | Pending |
| OVR-06 | TBD | Pending |
| OVR-07 | TBD | Pending |
| OVR-08 | TBD | Pending |
| F4SE-01 | TBD | Pending |
| F4SE-02 | TBD | Pending |
| F4SE-03 | TBD | Pending |
| F4SE-04 | TBD | Pending |
| F4SE-05 | TBD | Pending |
| SCAN-01 | TBD | Pending |
| SCAN-02 | TBD | Pending |
| SCAN-03 | TBD | Pending |
| SCAN-04 | TBD | Pending |
| SCAN-05 | TBD | Pending |
| SCAN-06 | TBD | Pending |
| SCAN-07 | TBD | Pending |
| SCAN-08 | TBD | Pending |
| SCAN-09 | TBD | Pending |
| SCAN-10 | TBD | Pending |
| TOOL-01 | TBD | Pending |
| TOOL-02 | TBD | Pending |
| TOOL-03 | TBD | Pending |
| TOOL-04 | TBD | Pending |
| TOOL-05 | TBD | Pending |
| TOOL-06 | TBD | Pending |
| ABOUT-01 | TBD | Pending |
| ABOUT-02 | TBD | Pending |
| ABOUT-03 | TBD | Pending |
| ABOUT-04 | TBD | Pending |
| SAFE-01 | TBD | Pending |
| SAFE-02 | TBD | Pending |
| SAFE-03 | TBD | Pending |
| SAFE-04 | TBD | Pending |
| SAFE-05 | TBD | Pending |

**Coverage:**
- v1 requirements: 54 total
- Mapped to phases: 0
- Unmapped: 54

---
*Requirements defined: 2026-05-17*
*Last updated: 2026-05-17 after initial definition*
