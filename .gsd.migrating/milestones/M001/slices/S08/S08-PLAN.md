# S08: Scanner Auto Fix Actions

**Goal:** Add Scanner Auto-Fix eligibility, lifecycle, fail-closed execution plumbing, and inline result feedback while keeping the production registry empty so normal users see no Auto-Fix buttons.
**Demo:** User sees supported auto-fix actions on Scanner results and receives Fixed or Fix Failed feedback without blocking the UI.

## Must-Haves

- Must-haves:
- Production Auto-Fix registry is empty and unsupported Scanner results render no Auto-Fix button or disabled placeholder.
- Eligibility is keyed by typed ScannerSolutionKind or operation metadata, never by display-string matching in Slint.
- Reference lifecycle labels are preserved: Auto-Fix, Fixing..., Fixed!, Fix Failed, Auto-Fix Results.
- Tampered, stale, unsupported, missing-target, unconfirmed, and failed-precondition requests return safe failure feedback and do not call a mutating operation.
- Future real-operation safety contract is represented in typed plan preview, confirmation, and pre-mutation revalidation types.
- Auto-Fix execution uses owned worker payloads off the Slint UI thread and applies results through the existing Scanner reducer and event-loop sink.
- Inline details replace the reference modal while retaining the Auto-Fix Results heading and copy.
- Threat Surface Q3:
- Abuse: UI callbacks can be invoked with stale selections or unsupported results; controller and service must validate scan id, result identity, registered operation, target requirements, confirmation, and preconditions before any operation runs.
- Data exposure: No secrets or PII are introduced; raw filesystem diagnostics remain in diagnostic fields/logs, not primary user text.
- Input trust: Scan results and paths are untrusted snapshots of the user filesystem; future write operations must revalidate immediately before mutation.
- Requirement Impact Q4:
- Requirements touched: slice-scoped S08 promises only; no root REQUIREMENTS.md IDs are active.
- Re-verify: S07 read-only Scanner selection/actions, Scanner worker stale-event handling, Slint Scanner source contract, and Settings save-on-scan behavior.
- Decisions revisited: D027 read-only scanner architecture remains valid; D028 records the S08 typed empty-registry Auto-Fix architecture.
- Verification:
- cargo test scanner_autofix_domain
- cargo test scanner_autofix_service
- cargo test scanner_controller_autofix
- cargo test scanner_worker_payload_autofix
- cargo test s08_scanner_autofix_slint_contract
- cargo test s08_scanner_autofix_runtime_wiring
- cargo fmt --check
- cargo check
- cargo test
- cargo clippy --all-targets --all-features

## Proof Level

- This slice proves: Contract and runtime integration proof. Real user filesystem mutation is explicitly not required or allowed in this slice; fake registered operations prove lifecycle and worker handoff, and the empty production registry proves normal-user parity.

## Integration Closure

Consumes the completed S07 Scanner domain, service, controller, worker, Slint, and runtime wiring. Introduces typed Auto-Fix domain and service contracts, controller state, worker payloads, Slint gates, and MainWindow callback forwarding. The slice closes when the real app path remains hidden with the empty production registry and fake-backed tests exercise success, failure, stale, and tampered requests. S09 Downgrade Manager and S10 Archive Patcher remain deferred and must not gain real mutation behavior from this slice.

## Verification

- Adds Scanner Auto-Fix state transitions and safe failure surfaces: visible button labels, inline Auto-Fix Results details, row fixed/check state, safe status feedback, and structured tracing for requested, rejected, scheduled, completed, failed, stale, and worker-spawn-failed events. Diagnostics may include raw adapter errors for tests/logs but must not be the primary UI text.

## Tasks

- [x] **T01: Add typed Auto Fix domain contract and solution identities** `est:2h`
  Expected executor skills_used: tdd, design-an-interface, verify-before-complete.
  - Files: `src/domain/autofix.rs`, `src/domain/scanner.rs`, `src/domain/mod.rs`, `src/services/scanner.rs`
  - Verify: cargo test scanner_autofix_domain

- [x] **T02: Implement empty production Auto Fix service with fake registry** `est:2h`
  Expected executor skills_used: tdd, observability, verify-before-complete.
  - Files: `src/services/autofix.rs`, `src/services/mod.rs`, `src/domain/autofix.rs`
  - Verify: cargo test scanner_autofix_service

- [x] **T03: Wire Auto Fix lifecycle through Scanner controller and workers** `est:2.5h`
  Expected executor skills_used: rust-async-patterns, tdd, observability, verify-before-complete.
  - Files: `src/app/scanner_controller.rs`, `src/workers/events.rs`, `src/workers/mod.rs`
  - Verify: cargo test scanner_controller_autofix
cargo test scanner_worker_payload_autofix

- [x] **T04: Expose gated Auto Fix UI and runtime scheduling** `est:2.5h`
  Expected executor skills_used: rust-async-patterns, tdd, observability, verify-before-complete.
  - Files: `ui/scanner_tab.slint`, `ui/main.slint`, `src/main.rs`
  - Verify: cargo test s08_scanner_autofix_slint_contract
cargo test s08_scanner_autofix_runtime_wiring
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Files Likely Touched

- src/domain/autofix.rs
- src/domain/scanner.rs
- src/domain/mod.rs
- src/services/scanner.rs
- src/services/autofix.rs
- src/services/mod.rs
- src/app/scanner_controller.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/scanner_tab.slint
- ui/main.slint
- src/main.rs
