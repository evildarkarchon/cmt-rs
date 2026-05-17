---
phase: 01
slug: slint-shell-port-architecture
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-17
---

# Phase 01 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness via `cargo test` |
| **Config file** | none - Wave 0 creates the first automated shell-contract test |
| **Quick run command** | `cargo test shell_tab_labels_match_reference_order` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds after dependencies are fetched |

---

## Sampling Rate

- **After every task commit:** Run `cargo test shell_tab_labels_match_reference_order` once the test exists; before then run `cargo check`.
- **After every plan wave:** Run `cargo fmt --check && cargo check && cargo test`.
- **Before `/gsd-verify-work`:** Full suite must be green, including `cargo clippy --all-targets --all-features`.
- **Max feedback latency:** 60 seconds after dependencies are fetched.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01-01 | 1 | FOUND-01, FOUND-04 | T-01-02 | Slint/build dependencies compile without adding scanner/archive parser crates | compile | `cargo check` | no - Wave 1 creates build setup | pending |
| 01-01-02 | 01-01 | 1 | FOUND-01, FOUND-03 | T-01-03 | External Slint UI is compiled through `build.rs`, not generated under `CMT/` | compile | `cargo check` | no - Wave 1 creates `build.rs` and `ui/main.slint` | pending |
| 01-01-03 | 01-01 | 1 | FOUND-01 | T-01-03 | Rust startup launches generated `MainWindow` instead of console-only Hello World | compile | `cargo check` | no - Wave 1 rewrites `src/main.rs` | pending |
| 01-02-01 | 01-02 | 2 | FOUND-02, SAFE-05 | T-01-02 | Six tab components are inert and contain no filesystem/network/process behavior | compile/review | `cargo check` | no - Wave 2 creates tab files | pending |
| 01-02-02 | 01-02 | 2 | FOUND-02 | T-01-03 | `ui/main.slint` exposes exact title and tab labels/order | compile/review | `cargo check` | no - Wave 2 updates `ui/main.slint` | pending |
| 01-02-03 | 01-02 | 2 | FOUND-05, SAFE-05 | T-01-01 | Reference source remains unchanged while labels are copied from CMT | git check | `git status --short CMT` | yes - git command available | pending |
| 01-03-01 | 01-03 | 3 | FOUND-02, FOUND-03 | T-01-03 | Canonical tab labels are asserted by Rust tests | unit | `cargo test shell_tab_labels_match_reference_order` | no - Wave 3 creates `src/app/mod.rs` tests | pending |
| 01-03-02 | 01-03 | 3 | FOUND-03, SAFE-05 | T-01-02 | Module stubs remain no-op boundaries without real CMT behavior | compile/review | `cargo check` | no - Wave 3 creates module stubs | pending |
| 01-03-03 | 01-03 | 3 | FOUND-04, FOUND-05, SAFE-05 | T-01-01 | Final gates prove cargo health and untouched CMT reference | command gate | `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features && git status --short CMT` | yes - commands available after implementation | pending |

*Status: pending · green · red · flaky*

---

## Wave 0 Requirements

- [ ] `build.rs` - compile `ui/main.slint` through `slint-build`.
- [ ] `ui/main.slint` - export `MainWindow` with `Collective Modding Toolkit` title and Slint `TabWidget` wiring.
- [ ] `ui/overview_tab.slint`, `ui/f4se_tab.slint`, `ui/scanner_tab.slint`, `ui/tools_tab.slint`, `ui/settings_tab.slint`, `ui/about_tab.slint` - one inert component per reference tab.
- [ ] `src/app/mod.rs` - app/controller-facing boundary with `SHELL_TAB_LABELS`, `shell_tab_labels()`, and Rust tab-order tests.
- [ ] `src/domain/mod.rs` - no-op domain boundary.
- [ ] `src/platform/mod.rs` - no-op platform boundary.
- [ ] `src/workers/mod.rs` - no-op worker boundary.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Desktop shell opens as a visible Slint window titled `Collective Modding Toolkit` | FOUND-01, FOUND-02 | Cargo tests can prove labels/order but not that the native desktop window is visually usable in this environment | Run the app after implementation and confirm the window opens with tabs `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in order. |
| Placeholder tabs are selectable and inert | SAFE-05 | GUI automation is out of Phase 1 scope | Select each tab manually and confirm only scope-note placeholder text appears; no scans, settings writes, network calls, or process launches occur. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all MISSING references.
- [x] No watch-mode flags.
- [x] Feedback latency < 60s after dependencies are fetched.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-05-17

---

## Threat References

| Threat | Description | Mitigation |
|--------|-------------|------------|
| T-01-01 | Accidental mutation of read-only `CMT/` reference files | Run `git status --short CMT` and never edit files under `CMT/`. |
| T-01-02 | UI-thread blocking or real behavior sneaks into shell placeholders | Keep Phase 1 tabs static and inert; no callbacks, filesystem, network, process, settings, scanner, or worker actions. |
| T-01-03 | Tab identity drifts from reference source | Assert canonical Rust labels in tests and cite `CMT/src/cm_checker.py` / `CMT/src/enums.py` in completion notes. |
