# Phase 2: Settings & Defaults Parity - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 02-settings-defaults-parity
**Areas discussed:** Settings file placement, UI save behavior, Validation reporting, Test contract style

---

## Settings File Placement

| Question | Options Presented | User's Choice |
|----------|-------------------|---------------|
| Where should Phase 2 read/write `settings.json` by default? | Current directory; Config directory; Hybrid search; You decide | Current directory |
| Should Phase 2 add a path abstraction for settings IO even though the default is current-directory `settings.json`? | Yes, injectable path; No, direct path; You decide | Yes, injectable path |
| Should Phase 2 create `settings.json` on first launch when missing, matching the Python reference, or keep defaults in memory until the user changes a setting? | Create on load; Save on change; You decide | Create on load |
| How should `download-source.txt` be located for the `update_source` default? | Asset resolver; Current directory; Compile include; You decide | Asset resolver |

**Notes:** Brief research found `directories::ProjectDirs` is standard for OS-native config paths, but the user chose current-directory `settings.json` for reference parity in this phase.

---

## UI Save Behavior

| Question | Options Presented | User's Choice |
|----------|-------------------|---------------|
| Should the Settings-tab radio controls save immediately when changed, matching the Python reference? | Immediate save; Apply button; You decide | Immediate save |
| When a save succeeds, should the Settings tab show any visible confirmation? | No confirmation; Subtle status text; You decide | No confirmation |
| If a write to `settings.json` fails after the user changes a radio option, what should the visible UI do in Phase 2? | Revert and log; Keep UI selected; Show error dialog; You decide | Revert and log |
| Should settings changes update the active runtime logging level immediately, or only persist the value for use on next startup? | Persist only; Update runtime; You decide | Persist only |

**Notes:** Brief research found both instant-apply and explicit-save preference patterns. The user chose instant apply because it matches the reference Tkinter behavior.

---

## Validation Reporting

| Question | Options Presented | User's Choice |
|----------|-------------------|---------------|
| When `settings.json` is malformed JSON, should Phase 2 overwrite it with defaults immediately like the reference? | Reset and save; Keep bad file; Backup then reset; You decide | Reset and save |
| For partially invalid settings, should Phase 2 expose validation repairs anywhere in the UI? | Log only; Settings status text; Error dialog; You decide | Log only |
| Should Phase 2 preserve valid values from a malformed JSON file if possible, or treat malformed JSON as unrecoverable defaults-only? | Defaults only; Best effort salvage; You decide | Defaults only |
| How should unknown keys be handled when loading a syntactically valid `settings.json`? | Remove on resave; Preserve extras; Fail load; You decide | Remove on resave |

**Notes:** Brief research confirmed robust apps should recover from corrupt settings rather than crash. The user chose reference-like reset/repair behavior without adding visible UI notices.

---

## Test Contract Style

| Question | Options Presented | User's Choice |
|----------|-------------------|---------------|
| Should Phase 2 tests require exact JSON formatting, or just exact keys/values after parsing? | Keys and values; Exact formatting; You decide | Keys and values |
| Should tests assert that unknown keys disappear after save, or is it enough to ignore them in memory? | Assert removal; Ignore only; You decide | Assert removal |
| How should Settings-tab UI labels be verified in Phase 2? | Source assertions; Manual only; GUI automation; You decide | Source assertions |
| Should tests exercise Settings-tab selection callbacks, or only the Rust settings model plus source labels? | Model plus labels; Callback tests too; You decide | Model plus labels |

**Notes:** Brief research found snapshots/golden files are useful for serialized settings, but the user chose less brittle parsed key/value assertions plus source-level label checks.

---

## the agent's Discretion

- Exact Rust type/module names and Slint component structure are left to downstream research/planning as long as the locked decisions are preserved.
- Exact logging/error type choices are left to downstream agents as long as validation and save failures are observable enough for tests or diagnostics.

## Deferred Ideas

None.
