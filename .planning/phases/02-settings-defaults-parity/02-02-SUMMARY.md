---
phase: 02-settings-defaults-parity
plan: 02
subsystem: platform
tags: [rust, settings, filesystem, serde_json, asset-resolver]

requires:
  - phase: 02-settings-defaults-parity
    provides: Typed `AppSettings` defaults, JSON key contract, and repair diagnostics from Plan 01
provides:
  - Injectable settings store with current-directory production `settings.json` default
  - `download-source.txt` asset resolver abstraction with Nexus fallback
  - Load/create/repair/save behavior for reference-compatible settings JSON
  - Filesystem tests for missing, malformed, partial, key persistence, asset fallback, and save failure cases
affects: [settings-tab, app-startup, settings-controller, update-checks]

tech-stack:
  added: []
  patterns:
    - Platform store owns filesystem side effects while domain settings own repair rules
    - Generic asset resolver enables test-injected `download-source.txt` without touching repository files
    - TDD RED/GREEN commits for settings boundary and IO behavior

key-files:
  created: [src/platform/settings_store.rs]
  modified: [src/platform/mod.rs]

key-decisions:
  - "Keep production settings parity at current-directory `settings.json` while requiring tests to inject isolated paths."
  - "Resolve `download-source.txt` through an `AssetResolver` instead of tying assets to the settings file location."
  - "Return save errors from `SettingsStore::save` so later UI/controller code can revert failed radio selections."

patterns-established:
  - "SettingsStore::load creates defaults on missing files and resaves repaired valid JSON objects."
  - "FileAssetResolver::production uses `assets/download-source.txt`; StaticAssetResolver supports deterministic tests."
  - "LoadedSettings carries repaired settings, diagnostics, and reset-to-defaults state for later logging/UI wiring."

requirements-completed: [SET-01, SET-02, SET-06]

duration: 8min
completed: 2026-05-17
---

# Phase 02 Plan 02: Settings Store IO and Asset Fallback Summary

**Injectable Rust settings store that creates first-run defaults, repairs persisted JSON, resolves update-source assets, and returns save failures for UI recovery.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-17T04:00:08Z
- **Completed:** 2026-05-17T04:08:02Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `src/platform/settings_store.rs` with doc-commented `SettingsStore`, `SettingsPaths`, `AssetResolver`, `FileAssetResolver`, `StaticAssetResolver`, and `LoadedSettings` APIs.
- Preserved D-01 production parity by using current-directory `settings.json` while supporting injected settings and asset paths for tests.
- Implemented `download-source.txt` resolution through an asset resolver with `nexus` fallback for missing, unreadable, or invalid content.
- Implemented `load` and `save` behavior for missing files, malformed JSON defaults reset, partial valid JSON repair, unknown-key cleanup, and observable save errors.
- Added filesystem tests that exercise all plan-local settings store behaviors without touching `CMT/` or repository user settings.

## Task Commits

Each TDD task was committed atomically:

1. **Task 1 RED: Add injectable settings store boundary tests** - `4b8778c` (test)
2. **Task 1 GREEN: Add injectable settings store boundary** - `b1f670c` (feat)
3. **Task 2 RED: Add settings store IO tests** - `39659a4` (test)
4. **Task 2 GREEN: Implement settings store load and save** - `a8cb5c9` (feat)
5. **Post-task refactor/formatting** - `33979df` (refactor)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified

- `src/platform/mod.rs` - Exports the new settings store platform module.
- `src/platform/settings_store.rs` - Implements injectable settings paths, asset resolver abstractions, load/create/repair/save behavior, and filesystem tests.

## Decisions Made

- Kept production path as `PathBuf::from("settings.json")` through a named constant to match `CMT/src/app_settings.py` and D-01.
- Accepted all typed `UpdateSource` wire values from `download-source.txt` (`both`, `github`, `nexus`, `none`) and fell back to `nexus` for missing/invalid content per the plan's D-04 requirement.
- Returned raw `io::Error` values from `save` so Plan 04 can detect failures and revert Settings-tab state without treating failed persistence as success.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adjusted verification command for binary-only crate**
- **Found during:** Task 2 (Implement load, save, create-defaults, and repair persistence)
- **Issue:** Plan-local commands used `cargo test ... --lib`, but this crate currently has no library target.
- **Fix:** Ran equivalent named test filters without `--lib`, consistent with the Plan 01 adjustment.
- **Files modified:** None
- **Verification:** Named settings tests, `cargo test`, `cargo check`, and `cargo clippy --all-targets --all-features` passed.
- **Committed in:** N/A (command adjustment only)

**2. [Rule 1 - Bug] Accepted platform-specific save-failure error kind**
- **Found during:** Task 2 (Implement load, save, create-defaults, and repair persistence)
- **Issue:** The save-failure test expected `IsADirectory`, but Windows returned `PermissionDenied` when writing to a directory path.
- **Fix:** Kept the behavioral assertion focused on the plan requirement: `save` returns an observable error result instead of swallowing the failure.
- **Files modified:** `src/platform/settings_store.rs`
- **Verification:** `cargo test settings_save_failure_is_returned` passed.
- **Committed in:** `a8cb5c9`

**3. [Rule 1 - Bug] Applied rustfmt and removed clippy field reassignment warning**
- **Found during:** Plan-level verification
- **Issue:** `cargo fmt --check` reported formatting drift and clippy warned about assigning a field after `Default::default()`.
- **Fix:** Ran `cargo fmt` and initialized `AppSettings` with struct update syntax.
- **Files modified:** `src/platform/settings_store.rs`
- **Verification:** `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` passed.
- **Committed in:** `33979df`

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs)
**Impact on plan:** All fixes preserved the intended settings-store behavior and avoided scope expansion.

## Issues Encountered

- Cargo reported no library target for `--lib` plan commands; named test filters without `--lib` validated the same tests in the existing binary crate test harness.
- Windows reports directory-write save failures as `PermissionDenied` rather than `IsADirectory`; the test now asserts observable failure rather than platform-specific error kind.

## Known Stubs

None - the settings store behavior required by this plan is implemented. Settings-tab UI and immediate-save callback wiring remain intentionally deferred to Plans 03-04.

## Threat Flags

None - this plan implemented the planned local filesystem trust boundaries covered by T-02-05 through T-02-08.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo fmt --check` — passed
- `cargo check` — passed
- `cargo test` — passed (18 tests)
- `cargo clippy --all-targets --all-features` — passed
- `git status --short CMT` — clean

## Self-Check: PASSED

- Created file exists: `src/platform/settings_store.rs`
- Modified file exists: `src/platform/mod.rs`
- Task commits found: `4b8778c`, `b1f670c`, `39659a4`, `a8cb5c9`, `33979df`
- Requirements copied from plan frontmatter: `SET-01`, `SET-02`, `SET-06`

## Next Phase Readiness

Ready for Plan 02-03 to render the Settings-tab reference labels and source-level UI contract tests using the settings store and domain contract established in Plans 01-02.

---
*Phase: 02-settings-defaults-parity*
*Completed: 2026-05-17*
