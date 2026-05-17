---
phase: 03
slug: platform-discovery-background-adapters
status: ready
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-16
updated: 2026-05-17
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness through Cargo (`#[cfg(test)]` unit tests). |
| **Config file** | `Cargo.toml`; no separate test configuration. |
| **Quick run command** | Use the task-specific `cargo test` command in the verification map below. |
| **Full suite command** | `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features` |
| **Estimated runtime** | Target under 60 seconds for each task-level filtered command; full suite runtime depends on dependency compilation. |

---

## Sampling Rate

- **After every task commit:** Run the task-specific automated command in the map below; if one task touches multiple independent module filters, run the listed commands sequentially with `&&` rather than passing multiple cargo filters before `--`.
- **After every plan wave:** Run `cargo check && cargo test`.
- **Before `/gsd-verify-work`:** Run `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features` and `git status --short CMT`.
- **Max feedback latency:** One filtered command per task should stay under 60 seconds after dependencies are built; no three consecutive tasks may proceed without automated feedback.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 03-01 | 1 | DISC-01, SAFE-03 | T-03-01-02 / T-03-01-03 | Discovery messages separate safe user text from diagnostics; missing Data/F4SE remains optional instead of panicking. | unit | `cargo test discovery -- --nocapture` | ❌ W0 creates `src/domain/discovery.rs` | ⬜ pending |
| 03-01-02 | 03-01 | 1 | DISC-02, SAFE-03 | T-03-01-01 | MO2 parser returns typed errors for malformed/non-Fallout config and PC Specs/system metadata preserves unavailable values without placeholders. | unit | `cargo test mod_manager -- --nocapture` | ❌ W0 creates `src/domain/mod_manager.rs` | ⬜ pending |
| 03-02-01 | 03-02 | 2 | DISC-03, SAFE-03 | T-03-02-01 | Filesystem reads/enumeration return typed results and deterministic ordering without real user filesystem state. | unit | `cargo test filesystem_adapter -- --nocapture` | ❌ W0 creates `src/platform/filesystem.rs` | ⬜ pending |
| 03-02-02 | 03-02 | 2 | DISC-02, DISC-04, SAFE-03 | T-03-02-02 | Registry/process/system metadata OS data stays behind fakeable traits; unsupported platforms return typed errors and fake PC Specs tests do not read real host state. | unit | `cargo test registry_adapter -- --nocapture && cargo test process_adapter -- --nocapture` | ❌ W0 creates `src/platform/registry.rs` and `src/platform/process.rs` | ⬜ pending |
| 03-02-03 | 03-02 | 2 | DISC-04 | T-03-02-03 | URL/path/tool launch results are typed values with safe failure messages and no shell-string construction. | unit | `cargo test desktop_adapter -- --nocapture` | ❌ W0 creates `src/platform/desktop.rs` | ⬜ pending |
| 03-03-01 | 03-03 | 3 | DISC-01, DISC-03, SAFE-03 | T-03-03-01 | Candidate paths are validated via injected filesystem and direct executable inputs normalize to parent directory. | unit | `cargo test discovery_service_candidate -- --nocapture` | ❌ W0 creates `src/services/discovery.rs` | ⬜ pending |
| 03-03-02 | 03-03 | 3 | DISC-01, DISC-03 | T-03-03-02 | Discovery order and not-found messages match reference strings without UI prompts. | unit | `cargo test discovery_service_order -- --nocapture` | ❌ W0 extends `src/services/discovery.rs` | ⬜ pending |
| 03-03-03 | 03-03 | 3 | DISC-02, DISC-03, SAFE-03 | T-03-03-03 | MO2 manager failures are typed, do not silently fall through to registry discovery, and discovery output carries fake-backed PC Specs/system metadata. | unit | `cargo test discovery_service_manager -- --nocapture` | ❌ W0 extends `src/services/discovery.rs` | ⬜ pending |
| 03-04-01 | 03-04 | 4 | DISC-04, SAFE-02 | T-03-04-03 | Worker events expose safe messages and no generated Slint component/model types. | unit/source | `cargo test worker_events -- --nocapture` | ❌ W0 creates `src/workers/events.rs` | ⬜ pending |
| 03-04-02 | 03-04 | 4 | SAFE-02, SAFE-03 | T-03-04-02 | Owned events are routed through recording/Slint event-loop sinks without cross-thread UI mutation. | unit/source | `cargo test handoff -- --nocapture` | ❌ W0 creates `src/workers/handoff.rs` | ⬜ pending |
| 03-04-03 | 03-04 | 4 | SAFE-01, SAFE-02 | T-03-04-01 | Blocking work uses an off-UI-thread worker facade and emits progress/completion/error events after discovery-service implementation plans are complete. | unit | `cargo test workers:: -- --nocapture` | ✅ existing `src/workers/mod.rs`, W0 extends it | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/domain/discovery.rs` — create tests for DISC-01 reference messages, direct path representation, and optional `Data`/F4SE paths.
- [ ] `src/domain/mod_manager.rs` — create tests for DISC-02 display names, `0.0.0` fallback, PC Specs/system metadata populated/unavailable values, MO2 defaults, `%BASE_DIR%`, selected profile, and exact error text.
- [ ] `src/platform/filesystem.rs` — create fake filesystem tests for DISC-03 file/dir/text/sorted enumeration behavior.
- [ ] `src/platform/registry.rs` — create fake registry and non-Windows unsupported tests supporting DISC-01/DISC-02.
- [ ] `src/platform/process.rs` — create fake process/version/system metadata tests for DISC-02/DISC-04 manager detection inputs, fallback version behavior, and PC Specs populated/unavailable behavior.
- [ ] `src/platform/desktop.rs` — create fake desktop action tests for DISC-04 success/failure messages.
- [ ] `src/services/discovery.rs` — create fake-backed discovery orchestration tests for DISC-01/DISC-02/DISC-03, including PC Specs/system metadata on discovery output.
- [ ] `src/workers/events.rs` — create worker event shape tests for SAFE-02.
- [ ] `src/workers/handoff.rs` — create recording sink and Slint-boundary source tests for SAFE-02/SAFE-03.
- [ ] `src/workers/mod.rs` — extend worker facade tests for SAFE-01.

Existing Cargo/Rust test infrastructure is present; Wave 0 means the first behavior-adding task in each plan creates the required module tests before or with implementation.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real local Fallout 4 / MO2 / Vortex discovery against the developer machine | DISC-01, DISC-02 | Environment-dependent; automated acceptance uses fake filesystem/registry/process fixtures because installs and running managers may be absent. | Optional after automated green: run the app or a temporary dev harness on a machine with the relevant install/manager and compare paths/messages against the Python reference. Do not require this for Phase 3 automated completion. |

All required Phase 3 behaviors have automated fake-backed verification; the row above is optional production smoke coverage only.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands or Wave 0 test-creation dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing Phase 3 test modules and fake fixtures.
- [x] No watch-mode flags.
- [x] Feedback latency target documented as < 60 seconds per filtered command after dependencies are built.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-05-17 for planning; execution updates statuses as tasks run.
