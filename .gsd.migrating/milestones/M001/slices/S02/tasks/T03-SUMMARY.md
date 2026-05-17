---
id: T03
parent: S02
milestone: M001
key_files:
  - ui/settings_tab.slint
  - src/main.rs
key_decisions:
  - Preserved the intentional `Warning` log-level radio in the Rust Settings tab because it derives from the reference settings schema and the user confirmed the addition.
duration: 
verification_result: passed
completed_at: 2026-05-17T08:27:57.919Z
blocker_discovered: false
---

# T03: Added exact source-level Settings-tab label/order coverage while preserving the intentional schema-derived Warning log-level option.

**Added exact source-level Settings-tab label/order coverage while preserving the intentional schema-derived Warning log-level option.**

## What Happened

Inspected the existing Slint Settings tab, main window forwarding, Rust settings domain/controller, and Python reference files `CMT/src/tabs/_settings.py`, `CMT/src/app_settings.py`, and `CMT/src/globals.py`. The Slint Settings tab already exposed the Update Channel and Log Level radio groups with callbacks and reference-compatible settings values. I added a source-level contract test that extracts the Settings tab group titles and option labels from `ui/settings_tab.slint` and asserts the exact display order and radio-option count without GUI automation. During reference comparison I briefly removed the Warning radio because the Python Settings tab omits it, then restored it after the user clarified that the Warning addition was an intentional derivation from the reference settings schema, which accepts `WARNING`. I also captured that discrepancy as durable project memory.

## Verification

Verified the Settings-tab source contract and overall crate health with `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`. All commands passed; `cargo test` reported 31 passed and 0 failed, including the new exact Settings-tab label/order test.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 253ms |
| 2 | `cargo check` | 0 | ✅ pass | 10717ms |
| 3 | `cargo test` | 0 | ✅ pass — 31 passed | 25292ms |
| 4 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 10933ms |

## Deviations

Preserved the pre-existing `Warning` Log Level option as a user-confirmed schema-derived Settings UI option: the Python Settings tab lists Debug/Info/Error, while the reference settings schema accepts persisted `WARNING`. Otherwise none.

## Known Issues

None.

## Files Created/Modified

- `ui/settings_tab.slint`
- `src/main.rs`
