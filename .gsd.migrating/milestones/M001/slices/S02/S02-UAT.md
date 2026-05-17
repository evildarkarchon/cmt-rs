# S02: Settings Defaults Parity — UAT

**Milestone:** M001
**Written:** 2026-05-17T08:48:30.938Z

# UAT: S02 Settings Defaults Parity

## UAT Type
Desktop manual smoke test backed by automated Rust parity tests.

## Preconditions
1. Build the Rust application successfully.
2. Start from a temporary or disposable working directory when testing first-run behavior so any existing `settings.json` does not affect results.
3. If testing failure handling, make `settings.json` read-only or otherwise use an unwritable settings path in a controlled test environment.

## Steps and Expected Outcomes
1. Launch the Rust Slint application.
   - Expected: the window opens as `Collective Modding Toolkit` and includes the Settings tab in the established tab order.
2. Open the Settings tab.
   - Expected: an `Update Channel` group is visible with choices in this order: `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, `Never: Don't Check`.
   - Expected: a `Log Level` group is visible with choices `Debug`, `Info`, `Warning`, and `Error`.
3. On first run with no `settings.json`, inspect the selected controls and generated file.
   - Expected: reference-compatible defaults are selected and a valid `settings.json` can be written with `update_source`, `log_level`, scanner toggle keys, `downgrader_keep_backups`, and `downgrader_delete_deltas`.
4. Change the update channel, then close and relaunch the app.
   - Expected: the changed update channel persists and is selected after relaunch.
5. Change the log level, then close and relaunch the app.
   - Expected: the changed log level persists and is selected after relaunch.
6. Replace `settings.json` with a valid but partial file that contains at least one valid setting and omits other settings.
   - Expected: the valid value is preserved, missing values fall back to documented defaults, and the repaired file remains usable.
7. Replace `settings.json` with malformed JSON.
   - Expected: loading fails safely and defaults are used rather than crashing the app.
8. Simulate a save failure, then change a Settings radio option.
   - Expected: the failed write is reported through the Rust error/log path and the visible selected option reverts to the last successfully persisted setting.

## Edge Cases
- Unknown JSON keys should not prevent loading valid known keys.
- Invalid enum values such as an unrecognized `log_level` or `update_source` should be repaired to defaults.
- Asset-derived `download-source.txt` defaults should be honored when present and valid.

## Not Proven By This UAT
- Full update-check network behavior for each update source.
- Scanner, downgrader, or overview behavior that consumes these settings in later slices.
- Cross-platform installer/package behavior outside the working-directory `settings.json` contract.
