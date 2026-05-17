---
phase: 02
slug: settings-defaults-parity
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-16
---

# Phase 02 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test settings --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test settings --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 0 | SET-01 | T-02-01 | Missing settings file creates defaults without crashing. | unit/filesystem | `cargo test settings_missing_file_defaults --lib` | no | pending |
| 02-01-02 | 01 | 0 | SET-02 | T-02-02 | Required JSON keys persist with reference-compatible names. | unit/filesystem | `cargo test settings_persist_reference_keys --lib` | no | pending |
| 02-01-03 | 01 | 0 | SET-05 | T-02-03 | Scanner defaults remain enabled for all seven categories. | unit | `cargo test scanner_settings_defaults_enabled --lib` | no | pending |
| 02-02-01 | 02 | 1 | SET-06 | T-02-04 | Malformed and partial invalid settings repair safely. | unit/filesystem | `cargo test settings_repair --lib` | no | pending |
| 02-03-01 | 03 | 1 | SET-03 | T-02-05 | Update Channel labels and values match reference order. | source-level Slint contract | `cargo test settings_tab_update_channel_labels --lib` | no | pending |
| 02-03-02 | 03 | 1 | SET-04 | T-02-06 | Log Level labels and values match reference order. | source-level Slint contract | `cargo test settings_tab_log_level_labels --lib` | no | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `src/domain/settings.rs` tests or equivalent domain test module covers defaults, persisted keys, scanner defaults, and repair behavior.
- [ ] `src/platform/settings_store.rs` tests or equivalent injectable IO tests covers missing file, save failure, and asset resolver fallback.
- [ ] Source-level Slint tests cover Settings-tab labels without requiring GUI automation.

---

## Manual-Only Verifications

All phase behaviors have automated verification or source-level contract checks.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
