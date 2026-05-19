# Requirements

This file is the explicit capability and coverage contract for the project.

## Remediation Note

S11 rebuilt this Markdown traceability matrix directly from .planning/REQUIREMENTS.md and completed M001 slice evidence because this execution tool session did not expose a DB-backed requirement update capability. The source of truth for v1 requirement titles is the list order: R001-R005 FOUND, R006-R011 SET, R012-R016 DISC, R017-R024 OVR, R025-R029 F4SE, R030-R039 SCAN, R040-R045 TOOL, R046-R049 ABOUT, and R050-R054 SAFE.

All validated statuses below are evidence-backed by completed S01-S10 summaries, task summaries, source-contract/runtime tests, and recorded Cargo gates. This document does not claim manual desktop, live Fallout 4 install, live network, or destructive real-file UAT unless such evidence is separately recorded in a slice artifact.

## Active

No active v1 requirements remain after S01-S10 evidence reconciliation. Future validation should reopen or add a requirement instead of weakening proof text if new gaps are discovered.

## Validated

### R001 — FOUND-01: Developer can build and run a Slint desktop application from the Rust crate
- Class: launchability
- Status: validated
- Description: Developer can build and run a Slint desktop application from the Rust crate.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 Slint dependency/build pipeline, `build.rs`, `ui/main.slint`, `src/main.rs`, and final Cargo gates passed.

### R002 — FOUND-02: User sees `Collective Modding Toolkit` identity and tab order `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`
- Class: launchability
- Status: validated
- Description: User sees the `Collective Modding Toolkit` application identity and tab order `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: S02, S03, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 tab-order/source-contract tests and CMT references in `CMT/src/enums.py` / `cm_checker.py`; later slices preserved the shell.

### R003 — FOUND-03: Developer can add behavior through separated UI, app/controller, domain, platform, and worker modules
- Class: operability
- Status: validated
- Description: Developer can add behavior through separated UI, app/controller, domain, platform, and worker modules without putting domain logic in Slint markup.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: S03, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 module boundaries; later slices use app/domain/services/platform/workers without placing domain logic in Slint.

### R004 — FOUND-04: Developer can run core verification commands for the current slice
- Class: operability
- Status: validated
- Description: Developer can run core verification commands for the current slice: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: S02, S03, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 ran the four core gates; every completed slice summary records passing relevant Cargo gates.

### R005 — FOUND-05: Developer can verify implementation changes do not modify files under `CMT/`
- Class: constraint
- Status: validated
- Description: Developer can verify that implementation changes do not modify files under `CMT/`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: S02, S03, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 and later summaries treat CMT as read-only; research `git status --short CMT` returned no output.

### R006 — SET-01: Settings load with reference-compatible defaults when no settings file exists
- Class: continuity
- Status: validated
- Description: User settings load with reference-compatible defaults when no settings file exists.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 settings domain/store tests and full gates; S02 summary and recorded Cargo gates provide the completed evidence.

### R007 — SET-02: Settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`
- Class: continuity
- Status: validated
- Description: User settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: S07, S09
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 JSON wire tests; S07 consumes scanner settings and S09 consumes downgrader preferences.

### R008 — SET-03: Update channel choices match reference labels
- Class: continuity
- Status: validated
- Description: User can choose update channel options matching the reference labels: `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, and `Never: Don't Check`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: S04
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 Slint source contract tests; S04 update checks consume `update_source`.

### R009 — SET-04: Log level choices match the validated Settings contract, including schema-supported Warning
- Class: continuity
- Status: validated
- Description: User can choose log level options matching the reference labels: `Debug`, `Info`, and `Error`. S02 intentionally validates Warning in addition to the Python tab visible Debug/Info/Error labels because the reference settings schema accepts persisted WARNING.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 re-scoped to include `Warning`; source/domain tests and gates passed.
- Notes: S02 discrepancy note preserved: the Rust port intentionally validates Warning in addition to the Python tab visible Debug/Info/Error labels because the reference settings schema accepts persisted WARNING.

### R010 — SET-05: Scanner settings default to enabled for reference categories
- Class: continuity
- Status: validated
- Description: Scanner-related settings default to enabled for Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: S07
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 domain tests; S07 UI/controller consumes scanner settings.

### R011 — SET-06: Invalid/incomplete settings fail safely, preserving valid values and repairing defaults
- Class: failure-visibility
- Status: validated
- Description: Invalid or incomplete settings files fail safely by preserving valid values and falling back to documented defaults for invalid values.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S02
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 domain/store/controller repair and rollback tests; S02 summary and recorded Cargo gates provide the completed evidence.

### R012 — DISC-01: App can discover or represent the Fallout 4 game path for downstream workflows
- Class: integration
- Status: validated
- Description: App can discover or represent the Fallout 4 game path needed by Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S03
- Supporting slices: S04, S06, S07, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 DiscoveryService tests; downstream slices consume discovery/game path.

### R013 — DISC-02: App can identify mod-manager context and display Mod Manager, Game Path, Version, PC Specs
- Class: integration
- Status: validated
- Description: App can identify mod manager context and display Mod Manager, Game Path, Version, and PC Specs data in the Overview area.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 discovery/mod-manager/system metadata; S04 Overview displays the summary rows.

### R014 — DISC-03: App reads file/directory sources through injectable filesystem adapters
- Class: integration
- Status: validated
- Description: App can read the file and directory sources needed for archive, module, F4SE plugin, scanner, and settings workflows through injectable filesystem adapters.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S03
- Supporting slices: S04, S06, S07, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 platform filesystem seam; all later read/mutation slices use fakeable adapters.

### R015 — DISC-04: App launches URLs/paths/tools through injectable adapters with visible failure reporting
- Class: integration
- Status: validated
- Description: App can launch URLs, open paths, and run external tools through injectable process adapters with visible failure reporting.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: S03, S04, S07
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 desktop/tool action seam; S05 Tools/About visible link/copy failures; S04/S07 safe open/copy actions.

### R016 — DISC-05: App performs update checks according to `update_source` without blocking UI/startup
- Class: integration
- Status: validated
- Description: App can perform update checks according to `update_source` without blocking startup or the UI thread.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S02, S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 setting, S03 worker handoff, S04 update-check service and Overview banner tests.

### R017 — OVR-01: Overview shows `Mod Manager:`, `Game Path:`, `Version:`, `PC Specs:`
- Class: core-capability
- Status: validated
- Description: User sees Overview game/mod-manager summary labels matching the reference: `Mod Manager:`, `Game Path:`, `Version:`, and `PC Specs:`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 source contract/projection tests and Overview summary rows.

### R018 — OVR-02: Overview Binaries panel shows game/F4SE/Creation Kit and Address Library status
- Class: core-capability
- Status: validated
- Description: User sees the Binaries `(EXE/DLL/BIN)` panel with game/F4SE/Creation Kit status data and Address Library status.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 collector/diagnostics tests and summary; S04 summary and recorded Cargo gates provide the completed evidence.

### R019 — OVR-03: User can open Downgrade Manager from Overview binaries panel
- Class: core-capability
- Status: validated
- Description: User can open the Downgrade Manager action from the Overview binaries panel.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S09
- Supporting slices: S04
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 deferred control; S09 live modal from Overview and Tools with runtime tests.

### R020 — OVR-04: Overview Archives panel shows General, Texture, Total, Unreadable, OG, NG counts
- Class: core-capability
- Status: validated
- Description: User sees the Archives `(BA2)` panel with General, Texture, Total, Unreadable, OG, and NG archive counts.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 archive diagnostics; S10 consumes archive records; S04 summary and recorded Cargo gates provide the completed evidence.

### R021 — OVR-05: User can open Archive Patcher from Overview archives panel
- Class: core-capability
- Status: validated
- Description: User can open the Archive Patcher action from the Overview archives panel.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S10
- Supporting slices: S04, S05
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 deferred control; S10 live Archive Patcher from Overview and Tools with runtime tests.

### R022 — OVR-06: Overview Modules panel shows Full, Light, Total, HEDR v1.00, v0.95, unknown counts
- Class: core-capability
- Status: validated
- Description: User sees the Modules `(ESM/ESL/ESP)` panel with Full, Light, Total, HEDR v1.00, HEDR v0.95, and HEDR unknown counts.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 module collector/diagnostics and UI source contract tests.

### R023 — OVR-07: Overview diagnostics produce typed problem records for Scanner Overview Issues
- Class: core-capability
- Status: validated
- Description: Overview diagnostics produce typed problem records that can be included in Scanner results when Overview Issues are enabled.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S07
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 `OverviewProblem` feed; S07 consumes Overview-derived issues.

### R024 — OVR-08: Overview refresh/update-banner preserves update source semantics and links
- Class: core-capability
- Status: validated
- Description: Overview refresh and update-banner behavior preserve the reference update source semantics and user-facing links.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S04
- Supporting slices: S02, S05
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 update service/banner/link behavior; S05 desktop link failure pattern.

### R025 — F4SE-01: User can open F4SE tab and scan `Data/F4SE/Plugins` DLLs
- Class: core-capability
- Status: validated
- Description: User can open F4SE tab and scan `Data/F4SE/Plugins` DLLs.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S06
- Supporting slices: S03, S04
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S06 lazy/read-only F4SE tab and scan service tests; S06 summary and recorded Cargo gates provide the completed evidence.

### R026 — F4SE-02: F4SE table columns are `DLL`, `OG`, `NG`, `AE`, `Your Game`
- Class: core-capability
- Status: validated
- Description: F4SE table columns are `DLL`, `OG`, `NG`, `AE`, `Your Game`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S06
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S06 Slint/source contract tests; S06 summary and recorded Cargo gates provide the completed evidence.

### R027 — F4SE-03: F4SE compatibility status is reference-compatible across game generations
- Class: core-capability
- Status: validated
- Description: F4SE compatibility status is reference-compatible across game generations.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S06
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S06 domain/service/DLL inspector tests; S06 summary and recorded Cargo gates provide the completed evidence.

### R028 — F4SE-04: Missing Data or plugin folder guidance is reference-compatible
- Class: core-capability
- Status: validated
- Description: Missing Data or plugin folder guidance is reference-compatible.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S06
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S06 safe missing-folder status/error tests; S06 summary and recorded Cargo gates provide the completed evidence.

### R029 — F4SE-05: F4SE scanning runs without blocking the Slint UI thread
- Class: quality-attribute
- Status: validated
- Description: F4SE scanning runs without blocking the Slint UI thread.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S06
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S06 worker payload/lazy activation/runtime wiring tests; S06 summary and recorded Cargo gates provide the completed evidence.

### R030 — SCAN-01: Scanner tab shows `Scan Game`, `Scan Settings`, `Collapse All`, `Expand All`
- Class: core-capability
- Status: validated
- Description: User can open the Scanner tab and see `Scan Game`, `Scan Settings`, `Collapse All`, and `Expand All` actions matching the reference labels.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S01
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 Slint source contract and UI tests; S07 summary and recorded Cargo gates provide the completed evidence.

### R031 — SCAN-02: User can enable/disable scanner categories matching defaults/settings keys
- Class: core-capability
- Status: validated
- Description: User can enable or disable scanner categories matching the reference defaults and settings keys.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S02
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 settings contract; S07 scanner settings UI/controller persistence-on-scan.

### R032 — SCAN-03: User can start scan and see progress/status without blocking UI
- Class: core-capability
- Status: validated
- Description: User can start a game scan and see progress/status text such as `Scanning...` without blocking the UI.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 controller/worker/progress tests and runtime wiring; S07 summary and recorded Cargo gates provide the completed evidence.

### R033 — SCAN-04: Scanner builds mod file list from game/mod-manager context with supported attribution
- Class: core-capability
- Status: validated
- Description: Scanner can build a mod file list from the discovered game/mod-manager context while preserving mod attribution where the reference supports it.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 scan service MO2 attribution and Vortex Data-only handling tests.

### R034 — SCAN-05: Scanner classifies reference problem types
- Class: core-capability
- Status: validated
- Description: Scanner can classify reference problem types: Junk File, Unexpected Format, Misplaced DLL, Loose Previs, Loose AnimTextData, Invalid Archive, Invalid Module, Invalid Archive Name, F4SE Script Override, File Not Found, and Wrong Version.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S04, S06
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 scanner domain/service tests for taxonomy and overview/F4SE-related inputs.

### R035 — SCAN-06: Scan results are grouped/expandable with problem/files detail information
- Class: core-capability
- Status: validated
- Description: User sees scan results grouped and expandable in a tree/list model with `Problem` and `Files` style detail information.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 grouped row model/details tests and Slint contract; S07 summary and recorded Cargo gates provide the completed evidence.

### R036 — SCAN-07: Selecting a result shows `Mod:`, `Problem:`, `Summary:`, `Solution:` details
- Class: core-capability
- Status: validated
- Description: User can select a result and see details for `Mod:`, `Problem:`, `Summary:`, and `Solution:`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 controller/detail projection and UI tests; S07 summary and recorded Cargo gates provide the completed evidence.

### R037 — SCAN-08: Detail actions support URL open/copy and `Copy Details`
- Class: core-capability
- Status: validated
- Description: User can use detail actions equivalent to reference URL open/copy and `Copy Details` behavior.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S05
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S07 read-only actions through desktop/clipboard adapters; S07 summary and recorded Cargo gates provide the completed evidence.

### R038 — SCAN-09: Auto-fix actions appear only where supported and show `Fixed!` / `Fix Failed`
- Class: core-capability
- Status: validated
- Description: User sees auto-fix actions only where supported and receives `Fixed!` or `Fix Failed` feedback.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S08
- Supporting slices: S07
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S08 fail-closed Auto-Fix domain/service/controller/worker/Slint/runtime tests.

### R039 — SCAN-10: Scanner results can include Overview-derived issues when enabled
- Class: core-capability
- Status: validated
- Description: Scanner results can include Overview-derived issues when the Overview Issues setting is enabled.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S07
- Supporting slices: S02, S04
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S04 problem feed; S07 overview handoff and settings-driven scan tests.

### R040 — TOOL-01: Tools tab groupings match reference
- Class: core-capability
- Status: validated
- Description: User sees Tools tab groupings matching the reference: Toolkit Utilities, Other CM Authors' Tools, and Other Useful Tools.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 static contract tests and summary; S05 summary and recorded Cargo gates provide the completed evidence.

### R041 — TOOL-02: User can launch Toolkit Utilities for Downgrade Manager and Archive Patcher
- Class: core-capability
- Status: validated
- Description: User can launch Toolkit Utilities for `Downgrade Manager` and `Archive Patcher`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S10
- Supporting slices: S05, S09
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 utility entries; S09 live Downgrade Manager; S10 live Archive Patcher completes both.

### R042 — TOOL-03: External tool links open with reference labels and visible failure reporting
- Class: core-capability
- Status: validated
- Description: User can open external tool links from the Tools tab with reference labels and visible failure reporting.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 Tools service/controller tests; S05 summary and recorded Cargo gates provide the completed evidence.

### R043 — TOOL-04: Downgrade Manager honors backup and delta cleanup settings before file changes
- Class: core-capability
- Status: validated
- Description: Downgrade Manager honors backup and delta cleanup settings before performing file-changing operations.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S09
- Supporting slices: S02
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S02 persisted preferences; S09 modal/executor tests; S09 summary and recorded Cargo gates provide the completed evidence.

### R044 — TOOL-05: Archive Patcher uses fail-closed plans validating inputs before writing
- Class: compliance/security
- Status: validated
- Description: Archive Patcher performs archive-changing operations through fail-closed plans that validate inputs before writing.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S10
- Supporting slices: S03, S04
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S10 planner/executor tests: digest confirmation, BA2 validation, manifest-before-write.

### R045 — TOOL-06: File-changing tool operations run off UI thread with responsive status/errors
- Class: quality-attribute
- Status: validated
- Description: Destructive or file-changing tool operations run off the UI thread and preserve responsive status/error reporting.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S10
- Supporting slices: S03, S09
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S09 and S10 modal/controller/worker event tests; S10 summary and recorded Cargo gates provide the completed evidence.

### R046 — ABOUT-01: About attribution matches reference including created-by text
- Class: core-capability
- Status: validated
- Description: User sees About tab attribution matching the reference, including `Created by wxMichael for the Collective Modding Community`.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: -
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 About source contract tests; S05 summary and recorded Cargo gates provide the completed evidence.

### R047 — ABOUT-02: User can open/copy project/community links from About tab
- Class: core-capability
- Status: validated
- Description: User can open and copy relevant project/community links from the About tab.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 About controller/service tests; S05 summary and recorded Cargo gates provide the completed evidence.

### R048 — ABOUT-03: User can open/copy Discord invite action
- Class: core-capability
- Status: validated
- Description: User can open and copy the Discord invite action from the About tab.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 About link/copy tests; S05 summary and recorded Cargo gates provide the completed evidence.

### R049 — ABOUT-04: Link actions report failures visibly
- Class: failure-visibility
- Status: validated
- Description: Link actions report failures visibly instead of silently failing.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S05
- Supporting slices: S03
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S05 visible failure feedback tests; S04/S07 reuse safe action pattern.

### R050 — SAFE-01: Long-running scans/traversal/parsing/downloads/patching/process monitoring run off Slint UI thread
- Class: quality-attribute
- Status: validated
- Description: Long-running scans, filesystem traversal, parsing, downloads, patching, and process monitoring run off the Slint UI thread.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S03
- Supporting slices: S04, S06, S07, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 worker foundation plus later worker-backed workflow tests.

### R051 — SAFE-02: Background work returns typed progress/completion/cancellation/error events via Slint-safe handoff
- Class: quality-attribute
- Status: validated
- Description: Background work returns typed progress, completion, cancellation, and error events to the UI through Slint-safe event-loop handoff.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S03
- Supporting slices: S04, S06, S07, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 WorkerEvent/Handoff tests; later slices add owned payloads.

### R052 — SAFE-03: Domain logic is testable without launching a window using fake adapters
- Class: quality-attribute
- Status: validated
- Description: Domain logic can be tested without launching a window by using fake filesystem and process adapters.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S03
- Supporting slices: S02, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S03 fake platform adapters; later services/controllers are Slint-free and test-backed.

### R053 — SAFE-04: File-changing workflows use backups/dry-run plans/validation/fail-closed behavior
- Class: compliance/security
- Status: validated
- Description: File-changing workflows use backups, dry-run plans, validation, or fail-closed behavior where the reference workflow can alter user files.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S10
- Supporting slices: S08, S09
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S08 fail-closed Auto-Fix registry, S09 digest-bound downgrader, S10 manifest/digest/byte-range patcher.

### R054 — SAFE-05: Labels, tab ordering, defaults, and messages are compared against `CMT/src/` before completing slices
- Class: constraint
- Status: validated
- Description: User-facing labels, tab ordering, default states, and messages are compared against `CMT/src/` before completing each ported slice.
- Source: .planning/REQUIREMENTS.md v1 requirement list, reconciled by S11 against completed M001 evidence.
- Primary owning slice: S01
- Supporting slices: S02, S03, S04, S05, S06, S07, S08, S09, S10
- Evidence class: completed slice/task summaries, automated Rust tests, source-contract/runtime wiring checks, and recorded Cargo gates; no unrecorded manual UAT is claimed.
- Validation: S01 CMT tab references; each behavior slice cites source-contract/fidelity tests and known deviations.

## Deferred

No v1 requirements are deferred in M001 after S01-S10 completion evidence was reconciled. v2 enhancements remain documented in .planning/REQUIREMENTS.md.

## Out of Scope

No v1 requirements were moved out of scope during S11. The original out-of-scope product boundaries remain documented in .planning/REQUIREMENTS.md.

## Traceability

| ID | Requirement | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|---|
| R001 | FOUND-01 | launchability | validated | S01 | - | S01 Slint dependency/build pipeline, `build.rs`, `ui/main.slint`, `src/main.rs`, and final Cargo gates passed. |
| R002 | FOUND-02 | launchability | validated | S01 | S02, S03, S04, S05, S06, S07, S08, S09, S10 | S01 tab-order/source-contract tests and CMT references in `CMT/src/enums.py` / `cm_checker.py`; later slices preserved the shell. |
| R003 | FOUND-03 | operability | validated | S01 | S03, S04, S05, S06, S07, S08, S09, S10 | S01 module boundaries; later slices use app/domain/services/platform/workers without placing domain logic in Slint. |
| R004 | FOUND-04 | operability | validated | S01 | S02, S03, S04, S05, S06, S07, S08, S09, S10 | S01 ran the four core gates; every completed slice summary records passing relevant Cargo gates. |
| R005 | FOUND-05 | constraint | validated | S01 | S02, S03, S04, S05, S06, S07, S08, S09, S10 | S01 and later summaries treat CMT as read-only; research `git status --short CMT` returned no output. |
| R006 | SET-01 | continuity | validated | S02 | - | S02 settings domain/store tests and full gates; S02 summary and recorded Cargo gates provide the completed evidence. |
| R007 | SET-02 | continuity | validated | S02 | S07, S09 | S02 JSON wire tests; S07 consumes scanner settings and S09 consumes downgrader preferences. |
| R008 | SET-03 | continuity | validated | S02 | S04 | S02 Slint source contract tests; S04 update checks consume `update_source`. |
| R009 | SET-04 | continuity | validated | S02 | - | S02 re-scoped to include `Warning`; source/domain tests and gates passed. |
| R010 | SET-05 | continuity | validated | S02 | S07 | S02 domain tests; S07 UI/controller consumes scanner settings. |
| R011 | SET-06 | failure-visibility | validated | S02 | - | S02 domain/store/controller repair and rollback tests; S02 summary and recorded Cargo gates provide the completed evidence. |
| R012 | DISC-01 | integration | validated | S03 | S04, S06, S07, S09, S10 | S03 DiscoveryService tests; downstream slices consume discovery/game path. |
| R013 | DISC-02 | integration | validated | S04 | S03 | S03 discovery/mod-manager/system metadata; S04 Overview displays the summary rows. |
| R014 | DISC-03 | integration | validated | S03 | S04, S06, S07, S09, S10 | S03 platform filesystem seam; all later read/mutation slices use fakeable adapters. |
| R015 | DISC-04 | integration | validated | S05 | S03, S04, S07 | S03 desktop/tool action seam; S05 Tools/About visible link/copy failures; S04/S07 safe open/copy actions. |
| R016 | DISC-05 | integration | validated | S04 | S02, S03 | S02 setting, S03 worker handoff, S04 update-check service and Overview banner tests. |
| R017 | OVR-01 | core-capability | validated | S04 | S03 | S04 source contract/projection tests and Overview summary rows. |
| R018 | OVR-02 | core-capability | validated | S04 | S03 | S04 collector/diagnostics tests and summary; S04 summary and recorded Cargo gates provide the completed evidence. |
| R019 | OVR-03 | core-capability | validated | S09 | S04 | S04 deferred control; S09 live modal from Overview and Tools with runtime tests. |
| R020 | OVR-04 | core-capability | validated | S04 | S10 | S04 archive diagnostics; S10 consumes archive records; S04 summary and recorded Cargo gates provide the completed evidence. |
| R021 | OVR-05 | core-capability | validated | S10 | S04, S05 | S04 deferred control; S10 live Archive Patcher from Overview and Tools with runtime tests. |
| R022 | OVR-06 | core-capability | validated | S04 | - | S04 module collector/diagnostics and UI source contract tests. |
| R023 | OVR-07 | core-capability | validated | S04 | S07 | S04 `OverviewProblem` feed; S07 consumes Overview-derived issues. |
| R024 | OVR-08 | core-capability | validated | S04 | S02, S05 | S04 update service/banner/link behavior; S05 desktop link failure pattern. |
| R025 | F4SE-01 | core-capability | validated | S06 | S03, S04 | S06 lazy/read-only F4SE tab and scan service tests; S06 summary and recorded Cargo gates provide the completed evidence. |
| R026 | F4SE-02 | core-capability | validated | S06 | - | S06 Slint/source contract tests; S06 summary and recorded Cargo gates provide the completed evidence. |
| R027 | F4SE-03 | core-capability | validated | S06 | S03 | S06 domain/service/DLL inspector tests; S06 summary and recorded Cargo gates provide the completed evidence. |
| R028 | F4SE-04 | core-capability | validated | S06 | S03 | S06 safe missing-folder status/error tests; S06 summary and recorded Cargo gates provide the completed evidence. |
| R029 | F4SE-05 | quality-attribute | validated | S06 | S03 | S06 worker payload/lazy activation/runtime wiring tests; S06 summary and recorded Cargo gates provide the completed evidence. |
| R030 | SCAN-01 | core-capability | validated | S07 | S01 | S07 Slint source contract and UI tests; S07 summary and recorded Cargo gates provide the completed evidence. |
| R031 | SCAN-02 | core-capability | validated | S07 | S02 | S02 settings contract; S07 scanner settings UI/controller persistence-on-scan. |
| R032 | SCAN-03 | core-capability | validated | S07 | S03 | S07 controller/worker/progress tests and runtime wiring; S07 summary and recorded Cargo gates provide the completed evidence. |
| R033 | SCAN-04 | core-capability | validated | S07 | S03 | S07 scan service MO2 attribution and Vortex Data-only handling tests. |
| R034 | SCAN-05 | core-capability | validated | S07 | S04, S06 | S07 scanner domain/service tests for taxonomy and overview/F4SE-related inputs. |
| R035 | SCAN-06 | core-capability | validated | S07 | - | S07 grouped row model/details tests and Slint contract; S07 summary and recorded Cargo gates provide the completed evidence. |
| R036 | SCAN-07 | core-capability | validated | S07 | - | S07 controller/detail projection and UI tests; S07 summary and recorded Cargo gates provide the completed evidence. |
| R037 | SCAN-08 | core-capability | validated | S07 | S05 | S07 read-only actions through desktop/clipboard adapters; S07 summary and recorded Cargo gates provide the completed evidence. |
| R038 | SCAN-09 | core-capability | validated | S08 | S07 | S08 fail-closed Auto-Fix domain/service/controller/worker/Slint/runtime tests. |
| R039 | SCAN-10 | core-capability | validated | S07 | S02, S04 | S04 problem feed; S07 overview handoff and settings-driven scan tests. |
| R040 | TOOL-01 | core-capability | validated | S05 | - | S05 static contract tests and summary; S05 summary and recorded Cargo gates provide the completed evidence. |
| R041 | TOOL-02 | core-capability | validated | S10 | S05, S09 | S05 utility entries; S09 live Downgrade Manager; S10 live Archive Patcher completes both. |
| R042 | TOOL-03 | core-capability | validated | S05 | S03 | S05 Tools service/controller tests; S05 summary and recorded Cargo gates provide the completed evidence. |
| R043 | TOOL-04 | core-capability | validated | S09 | S02 | S02 persisted preferences; S09 modal/executor tests; S09 summary and recorded Cargo gates provide the completed evidence. |
| R044 | TOOL-05 | compliance/security | validated | S10 | S03, S04 | S10 planner/executor tests: digest confirmation, BA2 validation, manifest-before-write. |
| R045 | TOOL-06 | quality-attribute | validated | S10 | S03, S09 | S09 and S10 modal/controller/worker event tests; S10 summary and recorded Cargo gates provide the completed evidence. |
| R046 | ABOUT-01 | core-capability | validated | S05 | - | S05 About source contract tests; S05 summary and recorded Cargo gates provide the completed evidence. |
| R047 | ABOUT-02 | core-capability | validated | S05 | S03 | S05 About controller/service tests; S05 summary and recorded Cargo gates provide the completed evidence. |
| R048 | ABOUT-03 | core-capability | validated | S05 | S03 | S05 About link/copy tests; S05 summary and recorded Cargo gates provide the completed evidence. |
| R049 | ABOUT-04 | failure-visibility | validated | S05 | S03 | S05 visible failure feedback tests; S04/S07 reuse safe action pattern. |
| R050 | SAFE-01 | quality-attribute | validated | S03 | S04, S06, S07, S09, S10 | S03 worker foundation plus later worker-backed workflow tests. |
| R051 | SAFE-02 | quality-attribute | validated | S03 | S04, S06, S07, S09, S10 | S03 WorkerEvent/Handoff tests; later slices add owned payloads. |
| R052 | SAFE-03 | quality-attribute | validated | S03 | S02, S04, S05, S06, S07, S08, S09, S10 | S03 fake platform adapters; later services/controllers are Slint-free and test-backed. |
| R053 | SAFE-04 | compliance/security | validated | S10 | S08, S09 | S08 fail-closed Auto-Fix registry, S09 digest-bound downgrader, S10 manifest/digest/byte-range patcher. |
| R054 | SAFE-05 | constraint | validated | S01 | S02, S03, S04, S05, S06, S07, S08, S09, S10 | S01 CMT tab references; each behavior slice cites source-contract/fidelity tests and known deviations. |

## Coverage Summary

- Total v1 requirements: 54
- Active requirements: 0
- Validated requirements: 54
- Deferred requirements: 0
- Out-of-scope v1 requirements: 0
- Requirements with primary owner: 54
- Requirements with proof text: 54
- Requirements without primary owner: 0
- Requirements without proof text: 0

---
*Requirements traceability remediated by S11 T01 from completed M001 evidence.*
