---
phase: 01
slug: slint-shell-port-architecture
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-17
updated: 2026-05-17
---

# Phase 01 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness via `cargo test` |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test shell_contract` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds after dependencies are fetched |

---

## Sampling Rate

- **After every task commit:** Run `cargo test shell_contract` once the shell-contract tests exist; before then run `cargo check`.
- **After every plan wave:** Run `cargo fmt --check && cargo check && cargo test`.
- **Before `/gsd-verify-work`:** Full suite must be green, including `cargo clippy --all-targets --all-features`.
- **Max feedback latency:** 60 seconds after dependencies are fetched.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01-01 | 1 | FOUND-01, FOUND-04 | T-01-02 | Slint/build dependencies compile without adding scanner/archive parser crates | compile | `cargo check` | yes - `Cargo.toml`, `build.rs`, `ui/main.slint` | green |
| 01-01-02 | 01-01 | 1 | FOUND-01, FOUND-03 | T-01-03 | External Slint UI is compiled through `build.rs`, not generated under `CMT/` | compile | `cargo check` | yes - `build.rs` and `ui/main.slint` | green |
| 01-01-03 | 01-01 | 1 | FOUND-01 | T-01-03 | Rust startup launches generated `MainWindow` instead of console-only Hello World | compile | `cargo check` | yes - `src/main.rs` | green |
| 01-02-01 | 01-02 | 2 | FOUND-02, SAFE-05 | T-01-02 | Six tab components are inert and contain no filesystem/network/process behavior | unit | `cargo test shell_contract` | yes - `src/main.rs` includes inert tab component test | green |
| 01-02-02 | 01-02 | 2 | FOUND-02 | T-01-03 | `ui/main.slint` exposes exact title and tab labels/order | unit | `cargo test shell_contract_main_slint_title_and_tabs_match_rust_contract` | yes - `src/main.rs` includes Slint title/tab parser test | green |
| 01-02-03 | 01-02 | 2 | FOUND-05, SAFE-05 | T-01-01 | Reference source remains unchanged while labels are copied from CMT | git check | `git status --short CMT` | yes - git command available | green |
| 01-03-01 | 01-03 | 3 | FOUND-02, FOUND-03 | T-01-03 | Canonical tab labels are asserted by Rust tests | unit | `cargo test shell_tab_labels_match_reference_order` | yes - `src/main.rs` tests `shell_tab_labels()` | green |
| 01-03-02 | 01-03 | 3 | FOUND-03, SAFE-05 | T-01-02 | Module stubs remain no-op boundaries without real CMT behavior | unit | `cargo test shell_contract_boundary_markers_construct_as_no_ops` | yes - `src/main.rs` constructs no-op boundary markers | green |
| 01-03-03 | 01-03 | 3 | FOUND-04, FOUND-05, SAFE-05 | T-01-01 | Final gates prove cargo health and untouched CMT reference | command gate | `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features && git status --short CMT` | yes - commands passed during audit | green |

*Status: pending · green · red · flaky*

---

## Wave 0 Requirements

- [x] `build.rs` - compile `ui/main.slint` through `slint-build`.
- [x] `ui/main.slint` - export `MainWindow` with `Collective Modding Toolkit` title and Slint `TabWidget` wiring.
- [x] `ui/overview_tab.slint`, `ui/f4se_tab.slint`, `ui/scanner_tab.slint`, `ui/tools_tab.slint`, `ui/settings_tab.slint`, `ui/about_tab.slint` - one inert component per reference tab.
- [x] `src/app/mod.rs` - app/controller-facing boundary with `SHELL_TAB_LABELS`, `shell_tab_labels()`, and Rust tab-order tests.
- [x] `src/domain/mod.rs` - no-op domain boundary.
- [x] `src/platform/mod.rs` - no-op platform boundary.
- [x] `src/workers/mod.rs` - no-op worker boundary.

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

## Validation Audit 2026-05-17

| Metric | Count |
|--------|-------|
| Gaps found | 4 |
| Resolved | 4 |
| Escalated | 0 |

| Added Check | Covers |
|-------------|--------|
| `shell_contract_main_slint_title_and_tabs_match_rust_contract` | `ui/main.slint` title and reference tab order match `SHELL_TAB_LABELS`. |
| `shell_contract_inert_tab_components_are_static_placeholders` | Each tab component remains a one-component inert placeholder without callback, filesystem, network, or process markers. |
| `shell_contract_boundary_markers_construct_as_no_ops` | Phase 1 app/domain/platform/worker boundary markers construct without side effects. |

Final gate rerun: `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`, and `git status --short CMT` all passed during the Nyquist audit.

---

## Threat References

| Threat | Description | Mitigation |
|--------|-------------|------------|
| T-01-01 | Accidental mutation of read-only `CMT/` reference files | Run `git status --short CMT` and never edit files under `CMT/`. |
| T-01-02 | UI-thread blocking or real behavior sneaks into shell placeholders | Keep Phase 1 tabs static and inert; no callbacks, filesystem, network, process, settings, scanner, or worker actions. |
| T-01-03 | Tab identity drifts from reference source | Assert canonical Rust labels in tests and cite `CMT/src/cm_checker.py` / `CMT/src/enums.py` in completion notes. |
