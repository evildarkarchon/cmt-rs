---
phase: 02
slug: settings-defaults-parity
status: complete
nyquist_compliant: true
wave_0_complete: true
updated: 2026-05-17
---

# Phase 02 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test settings` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

Note: this crate currently has only a binary target, so plan-local `--lib` examples are intentionally represented here as named `cargo test` filters without `--lib`.

---

## Sampling Rate

- **After every task commit:** Run the relevant named `cargo test` filter for the touched settings behavior.
- **After every plan wave:** Run `cargo test`.
- **Before `/gsd-verify-work`:** Full suite must be green.
- **Max feedback latency:** 30 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 0 | SET-01 | T-02-01 | Missing settings/default model creates defaults without crashing. | unit | `cargo test settings_missing_file_defaults` | yes | green |
| 02-01-02 | 01 | 0 | SET-02 | T-02-02 | Required JSON keys persist with reference-compatible names and wire values, including `WARNING`. | unit/filesystem | `cargo test settings_persist_reference_keys` | yes | green |
| 02-01-03 | 01 | 0 | SET-05 | T-02-03 | Scanner defaults remain enabled for all seven categories. | unit | `cargo test scanner_settings_defaults_enabled` | yes | green |
| 02-02-01 | 02 | 1 | SET-06 | T-02-04 | Malformed and partial invalid settings repair safely, preserve valid `WARNING`, and remove unknown keys. | unit/filesystem | `cargo test settings_repair` | yes | green |
| 02-02-02 | 02 | 1 | SET-01 | T-02-06 | Invalid/missing `download-source.txt` falls back to `nexus`. | unit/filesystem | `cargo test download_source` | yes | green |
| 02-02-03 | 02 | 1 | SET-01 | T-02-06 | Real `FileAssetResolver` reads valid `download-source.txt` file contents `github` and `nexus` as default update sources. | unit/filesystem | `cargo test file_asset_resolver_reads_valid_download_source_file_values` | yes | green |
| 02-02-04 | 02 | 1 | SET-02 | T-02-07 | Save failures are returned to callers rather than swallowed. | unit/filesystem | `cargo test settings_save_failure_is_returned` | yes | green |
| 02-03-01 | 03 | 1 | SET-03 | T-02-09 | Update Channel labels and values match reference order. | source-level Slint contract | `cargo test settings_tab_update_channel_labels` | yes | green |
| 02-03-02 | 03 | 1 | SET-04 | T-02-10 | Log Level labels and values match verified reference/UI order, including Warning coverage. | source-level Slint contract | `cargo test settings_tab_log_level_labels` | yes | green |
| 02-03-03 | 03 | 1 | SET-04 | T-02-10 | Settings tab keeps the dark Settings palette regression fixed. | source-level Slint contract | `cargo test settings_tab_uses_dark_mode_palette` | yes | green |
| 02-04-01 | 04 | 3 | SET-03 | T-02-13 | Update Channel selections save immediately and persist expected JSON values. | unit/filesystem | `cargo test settings_controller_saves_update_source_immediately` | yes | green |
| 02-04-02 | 04 | 3 | SET-04 | T-02-13 | Log Level selections save immediately as uppercase persisted wire values, including `WARNING`. | unit/filesystem | `cargo test settings_controller_saves` | yes | green |
| 02-04-03 | 04 | 3 | SET-04 | T-02-14 | Save failure reverts the visible log-level value, including Warning reversion. | unit/filesystem | `cargo test settings_controller_reverts` | yes | green |
| 02-04-04 | 04 | 3 | SET-03/SET-04 | T-02-15 | MainWindow forwards SettingsTab properties and callbacks. | source-level Slint contract | `cargo test main_window_forwards_settings_tab_api` | yes | green |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [x] `src/domain/settings.rs` tests cover defaults, persisted keys, scanner defaults, and repair behavior.
- [x] `src/platform/settings_store.rs` tests cover missing file, save failure, invalid/missing asset fallback, and real-file `download-source.txt` values for `github` and `nexus`.
- [x] Source-level Slint tests cover Settings-tab labels and dark-palette regression without requiring GUI automation.

---

## Manual-Only Verifications

All phase behaviors have automated verification or source-level contract checks. Human UAT additionally approved end-to-end desktop persistence in `02-HUMAN-UAT.md` as summarized by `02-VERIFICATION.md`.

---

## Validation Sign-Off

- [x] All tasks have automated verify coverage or documented source-level contract checks.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all MISSING references.
- [x] No watch-mode flags.
- [x] Feedback latency < 30s.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** green

---

## Validation Audit 2026-05-17

| Metric | Count |
|--------|-------|
| Gaps found | 2 |
| Resolved | 2 |
| Escalated | 0 |

### Audit Notes

- Existing validation rows were stale after Phase 2 execution and still marked tests as pending/missing.
- Added automated coverage for real `download-source.txt` file resolution through `FileAssetResolver` with valid `github` and `nexus` values.
- Updated the validation map to include post-verification Warning log-level, dark-palette, controller reversion, and MainWindow forwarding coverage.
