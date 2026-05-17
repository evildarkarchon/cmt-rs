# Phase 2: Settings & Defaults Parity - Specification

**Created:** 2026-05-17
**Ambiguity score:** 0.18 (gate: <= 0.20)
**Requirements:** 6 locked

## Goal

The Rust/Slint app loads, displays, validates, and persists the reference-compatible CMT settings needed by the Settings tab and later scanner/downgrader workflows.

## Background

Phase 1 produced a buildable Rust/Slint shell with the reference tab order and placeholder tab contents. `ui/settings_tab.slint` currently shows only a static `Settings` heading and reserved-behavior text; no Rust settings model, settings file loader, validation, persistence, or Settings-tab controls exist yet.

The Python reference in `CMT/src/app_settings.py` stores settings in `settings.json`, defaults `log_level` to `INFO`, reads the default `update_source` from `download-source.txt` with fallback to `nexus`, defaults seven scanner settings and two downgrader settings to `true`, preserves valid loaded values, and resets invalid or unknown values when resaving. The reference Settings tab in `CMT/src/tabs/_settings.py` exposes only two visible radio groups: `Update Channel` and `Log Level`. Scanner setting checkboxes live with scanner behavior in `CMT/src/tabs/_scanner.py`, so this phase persists scanner defaults but does not port the Scanner-tab controls.

## Requirements

1. **Reference settings file**: Settings load from and save to a reference-compatible `settings.json` shape.
   - Current: No Rust settings file loader or writer exists; settings are not persisted by the Rust app.
   - Target: The Rust app reads and writes JSON keys compatible with the Python reference: `log_level`, `update_source`, `scanner_OverviewIssues`, `scanner_Errors`, `scanner_WrongFormat`, `scanner_LoosePrevis`, `scanner_JunkFiles`, `scanner_ProblemOverrides`, `scanner_RaceSubgraphs`, `downgrader_keep_backups`, and `downgrader_delete_deltas`.
   - Acceptance: A test loads a JSON file containing all reference keys and verifies the Rust settings model preserves each value and writes the same key names back to disk.

2. **Reference defaults**: Missing settings load with reference-compatible defaults.
   - Current: The Rust app has no default settings model.
   - Target: With no settings file, `log_level` defaults to `INFO`; `update_source` defaults from `download-source.txt` when it contains `nexus` or `github`; invalid or missing `download-source.txt` falls back to `nexus`; all scanner booleans and both downgrader booleans default to `true`.
   - Acceptance: Tests cover no settings file, valid `download-source.txt` values, invalid/missing `download-source.txt`, and verify every default value matches the reference behavior.

3. **Settings validation**: Invalid or incomplete settings fail safely.
   - Current: The Rust app has no validation path for malformed, missing, unknown, or incorrectly typed settings.
   - Target: Loading settings preserves valid known values, fills missing known keys with defaults, rejects invalid enum values or wrong JSON types by using defaults, and removes unknown keys when the file is resaved.
   - Acceptance: Tests with partial JSON, malformed JSON, unknown keys, invalid `log_level`, invalid `update_source`, and wrong boolean types verify valid values remain intact and invalid values fall back to documented defaults.

4. **Update Channel controls**: The Settings tab exposes the reference Update Channel choices and persists selection changes.
   - Current: `ui/settings_tab.slint` is a static placeholder and has no selectable update-channel controls.
   - Target: The visible Settings tab contains an `Update Channel` group with choices labeled `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, and `Never: Don't Check`, backed by values `both`, `github`, `nexus`, and `none` respectively.
   - Acceptance: UI/source tests or smoke checks verify the four labels exist in the Settings tab, and a settings-model test verifies selecting each option persists the expected JSON value.

5. **Log Level controls**: The Settings tab exposes the reference Log Level choices and persists selection changes.
   - Current: `ui/settings_tab.slint` has no log-level controls.
   - Target: The visible Settings tab contains a `Log Level` group with choices labeled `Debug`, `Info`, and `Error`, backed by values `DEBUG`, `INFO`, and `ERROR` respectively.
   - Acceptance: UI/source tests or smoke checks verify the three labels exist in the Settings tab, and a settings-model test verifies selecting each option persists the expected JSON value.

6. **Settings boundary integration**: Settings state is connected through Rust app/domain boundaries without implementing later workflows.
   - Current: `src/app`, `src/domain`, `src/platform`, and `src/workers` contain Phase 1 no-op boundary markers only.
   - Target: Settings state is represented with typed Rust models and connected to the Slint Settings tab through app/controller-facing code, while scanner, downgrader, discovery, update-check, and tool behaviors continue to consume only persisted settings in later phases.
   - Acceptance: Source/tests show settings logic lives outside Slint markup, Slint UI files contain labels/control structure but not filesystem or JSON parsing logic, and no scanner scan, downgrader operation, platform discovery, network update check, or file-changing workflow is implemented in this phase.

## Boundaries

**In scope:**
- Typed Rust settings model for all Phase 2 `SET-*` keys.
- Reference-compatible `settings.json` load, validation, defaulting, and save behavior.
- `download-source.txt` default detection for `update_source` with fallback to `nexus`.
- Settings-tab Update Channel and Log Level visible controls with reference labels and persisted values.
- Tests or source-level checks that prove defaults, validation, persistence keys, and Settings-tab labels match the reference.
- Confirmation that `CMT/` remains read-only during the implementation.

**Out of scope:**
- Scanner-tab checkbox UI for scanner settings - scanner UI behavior belongs to the scanner phase; Phase 2 only persists the values and defaults.
- Running scanner diagnostics - this phase defines settings consumed by later scanner behavior only.
- Platform/game/mod-manager discovery - Phase 3 owns discovery and background adapter seams.
- Performing update checks or downloads - Phase 2 stores `update_source`; later phases act on it.
- Downgrader/archive patching behavior - Phase 2 stores backup and delta cleanup preferences only.
- Migrating to a new TOML settings format - this phase explicitly uses reference-compatible `settings.json`.
- Adding new settings not present in the reference app - this phase preserves reference parity rather than expanding product behavior.

## Constraints

- `CMT/` remains read-only; reference behavior is inspected but never modified.
- Settings persistence must use JSON keys and string/boolean values compatible with `CMT/src/app_settings.py`.
- Valid `log_level` persisted values are `DEBUG`, `INFO`, and `ERROR` for the visible Settings-tab choices; unsupported loaded values fall back to `INFO` unless a later requirement explicitly adds `WARNING` UI parity.
- Valid `update_source` persisted values are `both`, `github`, `nexus`, and `none`; `download-source.txt` only supplies `github` or `nexus`, matching the reference default detection.
- Settings file parsing and filesystem access must stay in Rust logic outside Slint markup.
- Standard verification remains `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.

## Acceptance Criteria

- [ ] Loading with no `settings.json` creates or yields defaults matching the reference values for all locked settings keys.
- [ ] Valid `download-source.txt` values `github` and `nexus` set the default `update_source`; invalid or missing source-file data falls back to `nexus`.
- [ ] Loading a complete valid `settings.json` preserves all reference settings values and saves the same reference key names.
- [ ] Partial, malformed, unknown-key, wrong-type, and invalid-enum settings inputs preserve valid known values and fall back safely for invalid values.
- [ ] The Settings tab contains `Update Channel` choices labeled `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, and `Never: Don't Check`.
- [ ] The Settings tab contains `Log Level` choices labeled `Debug`, `Info`, and `Error`.
- [ ] Changing each Settings-tab radio option updates the typed settings state and persists the expected JSON value.
- [ ] Scanner and downgrader settings are persisted with defaults but no scanner scan UI, scanner execution, downgrader execution, update check, or platform discovery behavior is implemented.
- [ ] `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` are run for the completed implementation slice.
- [ ] `git status --short CMT` confirms no reference submodule files were modified.

## Ambiguity Report

| Dimension           | Score | Min   | Status | Notes |
|---------------------|-------|-------|--------|-------|
| Goal Clarity        | 0.88  | 0.75  | met    | Phase changes from placeholder Settings tab/no settings model to reference-compatible settings load/display/persist behavior. |
| Boundary Clarity    | 0.80  | 0.70  | met    | Scanner checkbox UI, scanner execution, discovery, update checks, and downgrader behavior are explicitly out of scope. |
| Constraint Clarity  | 0.78  | 0.65  | met    | `settings.json`, `download-source.txt`, valid enum values, CMT read-only, and Rust/Slint separation are locked. |
| Acceptance Criteria | 0.76  | 0.70  | met    | Pass/fail checks cover defaults, validation, labels, persistence, boundaries, and verification commands. |
| **Ambiguity**       | 0.18  | <=0.20| met    | Gate passed after round 1. |

Status: met = dimension meets minimum; below minimum = planner treats as assumption.

## Interview Log

| Round | Perspective | Question summary | Decision locked |
|-------|-------------|------------------|-----------------|
| 1 | Researcher | Should Phase 2 show Scanner-tab checkbox controls or only persist scanner settings? | Persist scanner settings/defaults only; Scanner-tab checkbox UI is out of scope. |
| 1 | Researcher | Should Rust settings use reference `settings.json` or a new TOML format? | Use reference-compatible `settings.json` for Phase 2. |
| 1 | Researcher | Should `update_source` default read `download-source.txt` or be fixed? | Read `download-source.txt` when valid and fall back to `nexus`. |

---

*Phase: 02-settings-defaults-parity*
*Spec created: 2026-05-17*
*Next step: /gsd-discuss-phase 2 - implementation decisions (how to build what's specified above)*
