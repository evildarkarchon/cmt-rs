# S02: Settings Defaults Parity

**Goal:** Create the typed domain settings contract and its automated parity tests.
**Demo:** Create the typed domain settings contract and its automated parity tests.

## Must-Haves


## Tasks

- [x] **T01: Plan 01** `est:49min`
  - Create the typed domain settings contract and its automated parity tests.

Purpose: Phase 2 depends on a reference-compatible Rust model before file IO or UI callbacks can safely persist user choices.
Output: `src/domain/settings.rs` with typed settings, defaults, JSON key/value handling, repair behavior, and tests for SET-01, SET-02, SET-05, and SET-06.
- [x] **T02: Plan 02** `est:8min`
  - Implement reference-compatible settings file IO around the domain model.

Purpose: Users need first-run defaults, safe repair, persistence, and test-injectable paths before the Settings tab can save choices.
Output: A platform settings store with current-directory production path, asset resolver for `download-source.txt`, and tests for missing, malformed, partial, and save-failure cases.
- [x] **T03: Plan 03** `est:25min`
  - Replace the inert Settings placeholder with reference-labeled Slint controls and source-level contract tests.

Purpose: Users must see the same Settings-tab choices as the Python reference before Rust callbacks persist choices.
Output: `ui/settings_tab.slint` radio groups for Update Channel and Log Level plus tests proving exact labels/order without GUI automation.
- [x] **T04: Plan 04** `est:28min`
  - Wire the Settings UI to persisted settings with immediate save and fail-safe UI state.

Purpose: Phase 2 is complete only when visible Settings choices are backed by `settings.json` persistence and save failures do not leave the UI lying about persisted state.
Output: App/controller wiring that loads settings at startup, binds Slint callbacks, persists `update_source` and `log_level`, and reverts UI properties on save errors.

## Files Likely Touched

