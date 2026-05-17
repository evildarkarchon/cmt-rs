---
phase: 02-settings-defaults-parity
plan: 01
subsystem: domain
tags: [rust, settings, serde_json, validation, repair]

requires:
  - phase: 01-slint-shell-port-architecture
    provides: Rust crate module boundaries and buildable shell
provides:
  - Typed settings model for reference-compatible `settings.json` keys
  - Default settings for log level, update source, scanner categories, and downgrader toggles
  - Per-key JSON validation and repair diagnostics for syntactically valid settings objects
affects: [settings-store, settings-tab, scanner, downgrader]

tech-stack:
  added: [serde_json]
  patterns:
    - Defaults-first settings model
    - Per-key JSON repair from `serde_json::Value`
    - Non-sensitive repair diagnostics for later logging

key-files:
  created: [src/domain/settings.rs]
  modified: [Cargo.toml, src/domain/mod.rs]

key-decisions:
  - "Use `serde_json::Value` object inspection instead of direct struct deserialization to preserve CMT's per-key repair semantics."
  - "Treat malformed JSON and non-object roots as unrecoverable at the domain boundary so the settings store can perform a defaults-only reset."
  - "Keep `WARNING` unsupported for Rust Phase 02 log-level loading, repairing it to the documented `INFO` default."

patterns-established:
  - "Reference key constants: settings serialization uses exact CMT key names, including mixed-case scanner keys."
  - "Repair diagnostics include setting keys and failure classes only, avoiding value echoing."

requirements-completed: [SET-01, SET-02, SET-05, SET-06]

duration: 49min
completed: 2026-05-17
---

# Phase 02 Plan 01: Typed Settings Defaults and Repair Summary

**Reference-compatible Rust settings domain contract with defaults, JSON key parity, scanner toggle defaults, and tested per-key repair diagnostics.**

## Performance

- **Duration:** 49 min
- **Started:** 2026-05-17T03:10:00Z
- **Completed:** 2026-05-17T03:59:17Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Added `serde_json` and exported `crate::domain::settings` for downstream settings store, controller, scanner, and downgrader plans.
- Implemented typed `AppSettings`, `LogLevel`, `UpdateSource`, `ScannerSettings`, and `DowngraderSettings` with doc-commented public API.
- Locked exact reference JSON keys and wire values with tests that parse/assert objects rather than formatting.
- Implemented per-key repair for valid JSON objects while rejecting malformed JSON and non-object roots for defaults-only reset by the future store.

## Task Commits

Each TDD task was committed atomically:

1. **Task 1 RED: Add JSON settings dependencies and domain exports** - `61854a6` (test)
2. **Task 1 GREEN: Add JSON settings dependencies and domain exports** - `0fade10` (feat)
3. **Task 2 RED: Implement defaults and reference JSON key contract** - `676c38e` (test)
4. **Task 2 GREEN: Implement defaults and reference JSON key contract** - `672710d` (feat)
5. **Task 3 RED: Implement per-key validation and repair semantics** - `c4814fd` (test)
6. **Task 3 GREEN: Implement per-key validation and repair semantics** - `5fcc525` (feat)
7. **Post-task formatting** - `7b7c504` (style)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified

- `Cargo.toml` - Added `serde_json = "1.0.149"` for reference-compatible settings JSON parsing and serialization.
- `src/domain/mod.rs` - Exported the settings module and retained existing Phase 1 domain marker/tests.
- `src/domain/settings.rs` - Added typed settings model, defaults, JSON serialization, per-key repair, diagnostics, and unit tests.

## Decisions Made

- Used `serde_json::Value` object inspection because direct Serde struct deserialization would not match CMT's per-key repair behavior.
- Rejected malformed JSON and non-object roots at the domain boundary rather than attempting salvage, preserving D-09/D-11 defaults-only reset semantics for the future store.
- Kept repair diagnostics non-sensitive by recording keys and failure classes only.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adjusted verification command for binary-only crate**
- **Found during:** Task 1 (Add JSON settings dependencies and domain exports)
- **Issue:** Plan-local commands used `cargo test ... --lib`, but this crate currently has no library target.
- **Fix:** Ran equivalent named test filters without `--lib` while keeping the tests in the crate's existing Rust test harness.
- **Files modified:** None
- **Verification:** `cargo test settings`, `cargo test`, `cargo check`, and `cargo clippy --all-targets --all-features` passed.
- **Committed in:** N/A (command adjustment only)

**2. [Rule 1 - Bug] Applied rustfmt after final verification exposed formatting drift**
- **Found during:** Plan-level verification
- **Issue:** `cargo fmt --check` reported formatting differences in `src/domain/settings.rs` after the repair implementation.
- **Fix:** Ran `cargo fmt` and committed the formatting-only change.
- **Files modified:** `src/domain/settings.rs`
- **Verification:** `cargo fmt --check` passed after formatting.
- **Committed in:** `7b7c504`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes preserved the planned behavior and kept the crate buildable; no scope was added.

## Issues Encountered

- Cargo reported no library target for `--lib` plan commands; named test filters without `--lib` validated the same test functions in the existing binary crate test harness.

## Known Stubs

None - the settings domain contract is functional for this plan. File IO, asset fallback, Settings-tab UI, and callback persistence are intentionally deferred to Plans 02-04.

## Threat Flags

None - this plan implemented the planned local settings parsing trust boundary already covered by T-02-01 through T-02-04.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo fmt --check` — passed
- `cargo check` — passed
- `cargo test` — passed (10 tests)
- `cargo clippy --all-targets --all-features` — passed
- `git status --short CMT` — clean

## Self-Check: PASSED

- Created file exists: `src/domain/settings.rs`
- Modified files exist: `Cargo.toml`, `src/domain/mod.rs`
- Task commits found: `61854a6`, `0fade10`, `676c38e`, `672710d`, `c4814fd`, `5fcc525`, `7b7c504`
- Requirements copied from plan frontmatter: `SET-01`, `SET-02`, `SET-05`, `SET-06`

## Next Phase Readiness

Ready for Plan 02-02 to implement injectable settings file IO, `download-source.txt` asset fallback, and safe persistence using the domain contract from this plan.

---
*Phase: 02-settings-defaults-parity*
*Completed: 2026-05-17*
