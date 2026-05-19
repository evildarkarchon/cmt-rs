# S10: Archive Patcher Workflow

**Goal:** Deliver the Archive Patcher workflow as a faithful Rust and Slint modal opened from Overview and Tools, using Overview-enabled BA2 archive records as the candidate source and fail-closed patch and restore execution to protect user files.
**Demo:** User can open and run Archive Patcher operations through validated, fail-closed write plans that protect user files.

## Must-Haves

- ## Must-Haves
- Overview `Archive Patcher...` and Tools `Archive Patcher` open a live Archive Patcher modal instead of deferred feedback.
- The modal preserves the reference title, default desired version `v1 (OG)`, desired-version radios, `Patch All`, `About`, `Name Filter:`, candidate list, and bottom log/status surface.
- Candidate lists come from the current Overview/discovery enabled archive records: target `v1 (OG)` shows enabled `v7` and `v8` archives; target `v8 (NG)` shows enabled `v1` archives; name filtering is case-insensitive on basename; no ad hoc UI directory scan is introduced.
- Patch execution is gated by a read-only preview plan and explicit confirmation. The confirmed worker revalidates path containment, BTDX magic, current version, desired target, and restore manifest feasibility before writing.
- Restore protection uses the D033 app-owned latest header manifest and supports a simple restore-last-run action that skips stale, moved, malformed, or changed files safely.
- Patching and restore run off the Slint UI thread, disable write controls while running, stream log/progress events, keep the modal open, and refresh Overview when complete.
- Reference-visible messages are preserved where practical, including `Showing N files to be patched.`, `Nothing to do!`, `Unrecognized format: <file>`, `Unrecognized version [<hex>]: <file>`, `Patched to v<target>: <file>`, and `Patching complete. N Successful, M Failed.`.
- ## Threat Surface Q3
- Abuse: User-controlled game paths and archive filenames reach filesystem mutation. Mitigation is current Overview/discovery source authority, canonical containment checks, per-file BTDX and version validation, digest-bound confirmation, disabled controls while running, and skip-on-ambiguity behavior.
- Data exposure: No tokens or PII are handled. Logs should expose only local paths already selected by the user's game install/discovery state.
- Input trust: Name filter is display-only; archive paths are untrusted until revalidated immediately before mutation and restore.
- ## Requirement Impact Q4
- Requirements touched: no advanced or validated requirement IDs were preloaded for S10. This slice supports the project-level UI fidelity, responsiveness, and user-file safety promises.
- Re-verify: Overview archive diagnostics, Tools routing, worker handoff, settings-independent launch, and mutation failure visibility.
- Decisions revisited: D033 governs the low-disk restore manifest plus byte-range write safety model for this slice.
- ## Verification
- `cargo test archive_patcher_domain --quiet`
- `cargo test archive_patcher_service_plan --quiet`
- `cargo test archive_patcher_executor --quiet`
- `cargo test archive_patcher_controller --quiet`
- `cargo test archive_patcher_worker_payload --quiet`
- `cargo test s10_archive_patcher_slint_contract --quiet`
- `cargo test s10_archive_patcher_runtime_wiring --quiet`
- `cargo test overview --quiet`
- `cargo test tools --quiet`
- `cargo test worker --quiet`
- `cargo fmt --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo clippy --all-targets --all-features --quiet`
- ## Failure Modes Q5
- Missing discovery or empty Overview archive state opens a safe modal with write controls disabled and an actionable refresh/fix-discovery message.
- Missing file, denied file, bad magic, unknown version, already-patched file, stale manifest, and write failure are logged per file and do not prevent later files from being processed.
- Worker spawn failure becomes visible modal status/log state and leaves controls safe to retry.
- ## Load Profile Q6
- Shared resources: filesystem reads and byte-range writes against the user's Data archives, one app-owned manifest file, Tokio blocking worker pool, and Slint event-loop handoff.
- Per-operation cost: O(number of filtered candidates) header probes and at most one byte-range write per valid patch or restore entry. No full-archive copies by default.
- 10x breakpoint: very large archive counts stress log/model rendering and filesystem metadata latency first; bounded header probes avoid memory blow-up from multi-GB BA2 files.
- ## Negative Tests Q7
- Malformed inputs: empty filter, mixed-case filter, non-BA2 path, short header, non-BTDX magic, unknown version byte, unknown format marker.
- Error paths: missing file, permission denied, manifest write failure, byte-write failure after manifest creation, worker spawn failure, stale worker event, plan digest mismatch.
- Boundary conditions: no candidates, all candidates already target version, partial success with failures, restore after file changed, restore after file moved, close attempted while running.

## Proof Level

- This slice proves: Operational integration proof with fake-backed Rust tests, source-level Slint contract tests, and full cargo quality gates. A real Fallout 4 install is not required for completion; destructive behavior is proven in sandbox and fake filesystem tests.

## Integration Closure

T05 closes the integration loop by wiring the new modal to both Overview and Tools, extending Overview state to retain the enabled archive records needed by the patcher, scheduling patch and restore workers through the existing WorkerRuntime and SlintEventLoopSink handoff, and refreshing Overview after patch or restore completion. Nothing remains in the milestone for Archive Patcher once T06 gates pass, except optional manual UAT on a real game install.

## Verification

- Archive Patcher adds request-id tagged worker payloads, controller phases, user-visible log rows, progress text/percent, safe failure messages, stale-event tracing, and final success/failure summaries so future agents can inspect whether failure occurred during load, plan, patch, restore, manifest write, or Overview refresh.

## Tasks

- [x] **T01: Define Archive Patcher domain and preview plans** `est:2h`
  ---
  estimated_steps: 8
  estimated_files: 4
  skills_used:
    - write-docs
    - tdd
    - verify-before-complete
  ---
  Why: S10 needs a Slint-free contract before UI or mutation code so the reference labels, candidate semantics, log vocabulary, and confirmation model are testable without launching a window or touching real game files.
  - Files: `src/domain/archive_patcher.rs`, `src/domain/mod.rs`, `src/services/archive_patcher.rs`, `src/services/mod.rs`
  - Verify: cargo test archive_patcher_domain --quiet
cargo test archive_patcher_service_plan --quiet

- [x] **T02: Implement safe header patch and restore execution** `est:3h`
  ---
  estimated_steps: 9
  estimated_files: 3
  skills_used:
    - tdd
    - rust-async-patterns
    - security-review
    - verify-before-complete
  ---
  Why: Archive Patcher is mutation-heavy and must fail closed while avoiding full multi-GB archive copies. This task turns the read-only plan into a byte-level executor and restore path behind fakeable filesystem seams.
  - Files: `src/platform/filesystem.rs`, `src/services/archive_patcher.rs`, `src/domain/archive_patcher.rs`
  - Verify: cargo test archive_patcher_executor --quiet
cargo test platform::filesystem --quiet

- [x] **T03: Add controller and worker payload lifecycle** `est:2h`
  ---
  estimated_steps: 8
  estimated_files: 4
  skills_used:
    - rust-async-patterns
    - tdd
    - verify-before-complete
  ---
  Why: The modal needs the same Slint-free lifecycle discipline as S09 Downgrader: request ids, stale-event rejection, disabled write controls while running, close blocking during mutation, and owned worker payloads crossing the handoff boundary.
  - Files: `src/app/archive_patcher_controller.rs`, `src/app/mod.rs`, `src/workers/events.rs`, `src/workers/mod.rs`
  - Verify: cargo test archive_patcher_controller --quiet
cargo test archive_patcher_worker_payload --quiet
cargo test worker --quiet

- [x] **T04: Build Archive Patcher Slint modal contract** `est:2h`
  ---
  estimated_steps: 7
  estimated_files: 3
  skills_used:
    - write-docs
    - tdd
    - verify-before-complete
  ---
  Why: The visible workflow must be reference-shaped before runtime wiring so source contract tests can lock labels, control order, exported UI row types, and callbacks for the Overview and Tools entrypoints.
  - Files: `ui/archive_patcher_window.slint`, `ui/main.slint`, `src/main.rs`
  - Verify: cargo test s10_archive_patcher_slint_contract --quiet
cargo check --quiet

- [x] **T05: Wire entrypoints workers and Overview refresh** `est:3h`
  ---
  estimated_steps: 10
  estimated_files: 9
  skills_used:
    - rust-async-patterns
    - tdd
    - security-review
    - verify-before-complete
  ---
  Why: The slice is only complete when the modal is reachable from both existing UI entrypoints, consumes current Overview archive records, runs patch/restore workers off-thread, and refreshes Overview after completion.
  - Files: `src/main.rs`, `src/domain/tools.rs`, `src/services/tools.rs`, `src/domain/overview.rs`, `src/services/overview.rs`, `src/app/overview_controller.rs`, `ui/overview_tab.slint`, `ui/tools_tab.slint`, `ui/main.slint`
  - Verify: cargo test s10_archive_patcher_runtime_wiring --quiet
cargo test overview --quiet
cargo test tools --quiet
cargo test worker --quiet

- [x] **T06: Close with safety regression and quality gates** `est:1h`
  ---
  estimated_steps: 5
  estimated_files: 0
  skills_used:
    - verify-before-complete
    - review
    - security-review
  ---
  Why: This mutation workflow should not be marked complete until all focused tests, adjacent regression tests, and required Rust quality gates have fresh evidence in the current execution context.
  - Verify: cargo test archive_patcher_domain --quiet
cargo test archive_patcher_service_plan --quiet
cargo test archive_patcher_executor --quiet
cargo test archive_patcher_controller --quiet
cargo test archive_patcher_worker_payload --quiet
cargo test s10_archive_patcher_slint_contract --quiet
cargo test s10_archive_patcher_runtime_wiring --quiet
cargo test overview --quiet
cargo test tools --quiet
cargo test worker --quiet
cargo fmt --check
cargo check --quiet
cargo test --quiet
cargo clippy --all-targets --all-features --quiet

## Files Likely Touched

- src/domain/archive_patcher.rs
- src/domain/mod.rs
- src/services/archive_patcher.rs
- src/services/mod.rs
- src/platform/filesystem.rs
- src/app/archive_patcher_controller.rs
- src/app/mod.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/archive_patcher_window.slint
- ui/main.slint
- src/main.rs
- src/domain/tools.rs
- src/services/tools.rs
- src/domain/overview.rs
- src/services/overview.rs
- src/app/overview_controller.rs
- ui/overview_tab.slint
- ui/tools_tab.slint
