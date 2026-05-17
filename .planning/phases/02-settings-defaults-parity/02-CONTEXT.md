# Phase 2: Settings & Defaults Parity - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 delivers reference-compatible settings state for the Rust/Slint port: `settings.json` loading, defaulting, validation, persistence, and the visible Settings-tab radio controls for Update Channel and Log Level.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**6 requirements are locked.** See `02-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `02-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Typed Rust settings model for all Phase 2 `SET-*` keys.
- Reference-compatible `settings.json` load, validation, defaulting, and save behavior.
- `download-source.txt` default detection for `update_source` with fallback to `nexus`.
- Settings-tab Update Channel and Log Level visible controls with reference labels and persisted values.
- Tests or source-level checks that prove defaults, validation, persistence keys, and Settings-tab labels match the reference.
- Confirmation that `CMT/` remains read-only during the implementation.

**Out of scope (from SPEC.md):**
- Scanner-tab checkbox UI for scanner settings - scanner UI behavior belongs to the scanner phase; Phase 2 only persists the values and defaults.
- Running scanner diagnostics - this phase defines settings consumed by later scanner behavior only.
- Platform/game/mod-manager discovery - Phase 3 owns discovery and background adapter seams.
- Performing update checks or downloads - Phase 2 stores `update_source`; later phases act on it.
- Downgrader/archive patching behavior - Phase 2 stores backup and delta cleanup preferences only.
- Migrating to a new TOML settings format - this phase explicitly uses reference-compatible `settings.json`.
- Adding new settings not present in the reference app - this phase preserves reference parity rather than expanding product behavior.

</spec_lock>

<decisions>
## Implementation Decisions

### Settings File Placement
- **D-01:** Default production settings path is current-directory `settings.json`, matching the Python reference `Path("settings.json")` behavior.
- **D-02:** Settings IO must still accept injectable paths so tests can use temp files and avoid touching the repository or user settings.
- **D-03:** Missing `settings.json` should create/save defaults during load, matching the reference first-run behavior.
- **D-04:** `download-source.txt` should be resolved through an asset resolver abstraction rather than assumed to sit beside `settings.json` or embedded at compile time.

### UI Save Behavior
- **D-05:** Settings-tab radio changes save immediately when selected, matching the reference `Radiobutton` command behavior.
- **D-06:** Successful saves do not show visible confirmation text or dialogs; the UI remains quiet like the reference.
- **D-07:** If saving fails after a radio selection, the UI should revert to the last persisted value and log the failure.
- **D-08:** Changing `Log Level` in Phase 2 only persists the value. It does not need to reconfigure active runtime logging until a later app-wiring phase chooses to do so.

### Validation Reporting
- **D-09:** Malformed JSON is treated as unrecoverable: reset to defaults, save the default file, log the parse failure, and continue.
- **D-10:** Partially invalid but syntactically valid settings are repaired silently from the UI perspective and logged only.
- **D-11:** The implementation should not attempt best-effort salvage from malformed JSON; defaults-only reset is sufficient.
- **D-12:** Unknown keys in valid JSON are ignored in memory and removed on resave, matching reference cleanup behavior.

### Test Contract Style
- **D-13:** Tests should assert parsed JSON keys and values, not exact whitespace, indentation, or object key ordering.
- **D-14:** Tests should explicitly assert unknown keys are removed after save.
- **D-15:** Settings-tab labels should be verified with source-level assertions against `ui/settings_tab.slint`, following the low-cost Phase 1 shell contract style.
- **D-16:** Phase 2 tests should cover the Rust settings model and Slint source labels, not full GUI automation or callback-driving tests.

### the agent's Discretion
- Downstream agents may choose exact Rust type/module names and Slint component structure as long as the decisions above, SPEC.md, and project module-boundary rules remain satisfied.
- Downstream agents may choose the logging facade or error type style that best fits the current crate, provided invalid/repair events are observable in logs or testable diagnostics.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked Phase Requirements
- `.planning/phases/02-settings-defaults-parity/02-SPEC.md` - Locked Phase 2 requirements, boundaries, constraints, and acceptance criteria. MUST read before planning.
- `.planning/ROADMAP.md` - Phase 2 goal, success criteria, dependency on Phase 1, and adjacent phase boundaries.
- `.planning/REQUIREMENTS.md` - `SET-01` through `SET-06` definitions and downstream settings consumers.

### Project Direction
- `.planning/PROJECT.md` - Project goal, Rust/Slint direction, CMT read-only rule, UI fidelity requirements, and verification gates.
- `.planning/STATE.md` - Current decisions and phase status context.
- `.planning/phases/01-slint-shell-port-architecture/01-CONTEXT.md` - Prior decisions about Slint shell, module boundaries, inert tab placeholders, and reference-label traceability.

### Reference App Sources
- `CMT/src/app_settings.py` - Reference `settings.json` path, default settings, `download-source.txt` handling, validation/repair behavior, and save behavior.
- `CMT/src/tabs/_settings.py` - Reference Settings-tab radio groups, labels, option ordering, and immediate-save callback behavior.
- `CMT/src/scan_settings.py` - Reference scanner setting names and labels for persisted scanner defaults.
- `CMT/src/tabs/_scanner.py` - Confirms scanner checkbox UI belongs to scanner behavior and is not part of Phase 2 UI scope.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ui/settings_tab.slint`: Current placeholder component to replace with reference Settings-tab radio groups.
- `src/app/mod.rs`: Existing app-facing shell contract module; can host settings UI contracts or app-controller glue if that remains the established boundary.
- `src/domain/mod.rs`: Current no-op domain boundary; likely destination for typed settings model and validation behavior.
- `src/platform/mod.rs`: Current no-op platform boundary; likely destination for file/asset path abstractions if planning keeps IO outside domain logic.

### Established Patterns
- Phase 1 uses source-level tests with `include_str!` against Slint files to lock UI labels and structural contracts without GUI automation.
- Slint files currently contain structural UI only; filesystem, parsing, and settings validation should remain in Rust modules.
- `CMT/` is reference-only and must remain unmodified; every implemented label/default must cite the relevant `CMT/src/` file.

### Integration Points
- `ui/main.slint` already instantiates `SettingsTab {}` under the reference-order `Settings` tab.
- `src/main.rs` includes generated Slint modules and launches `MainWindow`; settings wiring will connect near this startup/app-controller boundary.
- Tests should use injectable settings and asset paths so they can run without writing a real top-level `settings.json` or relying on packaged assets.

</code_context>

<specifics>
## Specific Ideas

- Preserve the reference's quiet settings behavior: immediate radio-save, no success status, log-only repair reporting.
- Treat current-directory `settings.json` as an intentional reference-parity choice for this phase, even though OS-native config directories may be preferable for future packaging.
- Keep runtime logging reconfiguration out of Phase 2; persisting `log_level` is enough.

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope.

</deferred>

---

*Phase: 02-settings-defaults-parity*
*Context gathered: 2026-05-17*
