---
id: S02
parent: M001
milestone: M001
provides:
  - Reference-compatible Settings defaults, JSON keys, and repair behavior.
  - Production `settings.json` persistence with test-injectable paths and asset resolution.
  - Settings-tab UI contract for update channel and log-level choices.
  - Immediate-save callbacks with rollback semantics for failed persistence.
requires:
  []
affects:
  []
key_files:
  - src/domain/settings.rs
  - src/platform/settings_store.rs
  - src/app/settings_controller.rs
  - src/app/mod.rs
  - src/main.rs
  - ui/settings_tab.slint
  - Cargo.toml
  - .gsd/REQUIREMENTS.md
  - .gsd/DECISIONS.md
key_decisions:
  - D013 — Settings state is split between typed domain semantics and a platform settings store with injectable paths/assets.
  - D014 — The Settings tab exposes the schema-supported `Warning` log-level option.
patterns_established:
  - Represent reference settings as typed Rust domain models before Slint projection.
  - Keep filesystem side effects behind a platform store so tests can inject paths and assets.
  - Use `SettingsController` to preserve a last-successfully-persisted snapshot and return the UI value that Slint should display after each save attempt.
  - Use source-level Slint contract tests for label/order fidelity where launching a GUI is unnecessary.
observability_surfaces:
  - Settings repair diagnostics for missing, invalid, and unknown keys.
  - Tracing error logs for invalid Settings UI selections and save failures.
  - Rollback-on-failure UI state as an immediate user-visible failure signal.
drill_down_paths:
  []
duration: ""
verification_result: passed
completed_at: 2026-05-17T08:48:30.938Z
blocker_discovered: false
---

# S02: Settings Defaults Parity

**Settings defaults, persistence, UI labels, and immediate-save rollback now have a typed Rust/Slint contract with automated parity coverage.**

## What Happened

S02 established the Settings foundation for later port slices. The Rust domain now owns typed `AppSettings`, `UpdateSource`, `LogLevel`, scanner toggles, and downgrader preferences with reference-compatible JSON keys, defaults, wire values, diagnostics, and repair behavior. A platform settings store loads first-run defaults from the production `settings.json` path, uses an asset resolver for `download-source.txt`, supports injectable paths for tests, repairs valid-but-incomplete files, and reports save failures. The Settings Slint tab now presents reference-shaped Update Channel and Log Level radio groups, including the intentionally preserved `Warning` log-level option supported by the reference settings schema. `SettingsController` wires startup load and callback persistence so UI selections save immediately and revert to the last persisted value when a save fails, keeping the UI from misrepresenting durable state. Requirement R009 was re-scoped to document the validated `Warning` option.

## Verification

All four S02 tasks are complete in GSD state. Prior closeout-safe verification evidence from this slice passed: `cargo fmt --check` exit 0 (gsd_exec e7c22043-947d-49cf-a291-e84da874e368, 263ms), `cargo check` exit 0 (2143d6ff-5143-47ab-af57-6acf9c214106, 8143ms), `cargo test` exit 0 with 31 tests passed (b3fc1048-23b4-4d29-91a9-050236a3b5a6, 7685ms), and `cargo clippy --all-targets --all-features` exit 0 (d4afdceb-e3dd-4764-b4b1-0fb24761b290, 8174ms). Automated coverage includes settings defaults, JSON key/value round-trips, malformed and partial repair behavior, asset-derived update-source fallback, save-failure reporting, Slint Settings label/order contract tests, and controller rollback behavior. Operational readiness: health signal is successful startup load plus passing settings tests; failure signals are repair diagnostics and tracing errors for invalid UI selections/save failures; recovery is automatic repair/resave or visible property rollback to persisted state; monitoring gap is that richer in-app diagnostics/log viewing remains for later slices.

## Requirements Advanced

- R014 — Settings file access now goes through an injectable platform settings store, establishing the pattern for broader filesystem adapter work.
- R016 — `update_source` now has persisted typed semantics that later non-blocking update checks can consume.
- R031 — Scanner setting keys and defaults now exist in the shared settings contract, preparing Scanner category toggles for later UI work.
- R043 — Downgrader backup and delta cleanup preferences now persist in the shared settings contract for later tool workflows.

## Requirements Validated

- R006 — First-run defaults are covered by settings domain/store tests and full Rust verification.
- R007 — JSON persistence keys and wire values for log level, update source, scanner toggles, and downgrader preferences are covered by domain/store tests and full Rust verification.
- R008 — Settings-tab source-level contract tests verify update-channel labels and order.
- R009 — Requirement was re-scoped to include `Warning`; Settings-tab source-level contract tests and settings-domain tests verify the log-level options and wire values.
- R010 — Scanner default toggles are covered by domain settings tests.
- R011 — Malformed, invalid, partial, and save-failure settings cases are covered by domain/store/controller tests.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

- R009 — Re-scoped from Debug/Info/Error-only wording to include `Warning`, because the persisted reference settings schema accepts `WARNING` and the Rust UI preserves that valid state.

## Operational Readiness

None.

## Deviations

The implementation intentionally exposes a `Warning` log-level radio because the reference settings schema accepts `WARNING`, even though the Python Settings tab source visibly listed only Debug/Info/Error. Requirement R009 was updated to capture this confirmed parity interpretation. The closeout resumed after task artifacts had already been repaired; this completion canonicalizes the slice summary and UAT through GSD.

## Known Limitations

No live GUI automation was added for Settings interactions; S02 relies on source-level Slint contract tests, controller tests, and manual UAT steps. Rich user-facing diagnostics beyond rollback/logging are deferred to later observability and workflow slices.

## Follow-ups

S03 should reuse the injectable settings/filesystem boundary pattern for platform discovery adapters. Later slices should consume `update_source`, scanner toggles, and downgrader preferences rather than duplicating settings parsing.

## Files Created/Modified

- `src/domain/settings.rs` — Added typed settings models, defaults, JSON wire handling, diagnostics, repair behavior, and tests.
- `src/platform/settings_store.rs` — Added filesystem-backed settings load/save behavior, production path handling, asset resolver support, repair/resave semantics, and IO tests.
- `src/app/settings_controller.rs` — Added controller state for startup load, immediate persistence, invalid selection repair, and save-failure rollback.
- `src/app/mod.rs` — Exposes Settings controller/app wiring modules.
- `src/main.rs` — Binds Slint Settings properties and callbacks to persisted settings state.
- `ui/settings_tab.slint` — Replaced the placeholder with Update Channel and Log Level radio groups and source-level contract test coverage.
- `Cargo.toml` — Carries dependencies/test setup required for typed settings and Slint contract verification.
- `.gsd/REQUIREMENTS.md` — Documents S02 validation and R009 re-scope.
- `.gsd/DECISIONS.md` — Records S02 settings architecture and UI fidelity decisions.
