---
estimated_steps: 20
estimated_files: 2
skills_used: []
---

# T01: Define Overview snapshot contracts

---
estimated_steps: 7
estimated_files: 2
skills_used:
  - design-an-interface
  - tdd
  - verify-before-complete
---
Why: S04 needs a typed state boundary before any Slint or OS wiring so tests can assert Overview behavior without host filesystem, registry, process, or network access.

Do:
1. Inspect the reference inputs listed below for labels, panel names, count row order, problem fields, update banner semantics, and deferred utility placement; do not edit anything under CMT/.
2. Add `src/domain/overview.rs` with doc-commented domain/view contracts for `OverviewSnapshot`, refresh state, top status rows, binary/archive/module panel summaries, count rows, status severity, deferred actions, update banner state, and scanner-ready `OverviewProblem` records.
3. Reuse existing discovery and settings types where possible, including `Fallout4InstallType`, archive/module records, `SystemMetadata`, `DiscoveredModManager`, and `UpdateSource`; do not introduce Slint types into the domain module.
4. Include reference constants or enum labels needed by later projection: Mod Manager, Game Path, Version, PC Specs, Binaries (EXE/DLL/BIN), Archives (BA2), Modules (ESM/ESL/ESP), General, Texture, Total, Unreadable, v1 (OG), v7/8 (NG), Full, Light, HEDR v1.00, HEDR v0.95, HEDR v????, Downgrade Manager..., and Archive Patcher....
5. Export the new module from `src/domain/mod.rs`.
6. Add unit tests in the new module for label order, default/loading/partial/error snapshot states, update banner state, and problem-feed records carrying source, path, summary, solution, and optional link/detail metadata.
7. Keep tests pure and fake-backed; no .gsd, .planning, .audits, real registry, real network, or real user profile reads.

Done when: Overview state can be constructed entirely in Rust tests with no UI and no OS access, and the labels/problem fields needed by later tasks are locked by tests.

Failure Modes Q5: malformed or incomplete input should map to `Unknown`, `Not Found`, warning, or error snapshot states rather than panics.
Negative Tests Q7: include empty snapshot, missing game path, missing Data marker, disabled update checking, and problem records without paths.

## Inputs

- `CMT/src/tabs/_overview.py`
- `CMT/src/cm_checker.py`
- `CMT/src/globals.py`
- `CMT/src/enums.py`
- `CMT/src/helpers.py`
- `CMT/src/utils.py`
- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/domain/settings.rs`

## Expected Output

- `src/domain/overview.rs`
- `src/domain/mod.rs`

## Verification

cargo test overview_domain

## Observability Impact

Defines the typed refresh, status, severity, and problem fields that later logs, worker events, and UI diagnostics will expose.
