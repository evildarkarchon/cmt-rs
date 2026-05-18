# S09: Downgrade Manager Workflow

**Goal:** Deliver the Downgrade Manager workflow as a faithful Slint modal opened from Overview or Tools, with reference file status, backup and delta cleanup preferences, inline plan confirmation, safe backup/download/xdelta execution, visible log/progress feedback, and non-blocking worker handoff.
**Demo:** User can open and run Downgrade Manager from Overview or Tools with backup and delta cleanup preferences respected and visible status/errors.

## Must-Haves

- Threat Surface Q3: This slice touches user-selected settings, discovered filesystem roots, network downloads, and destructive game-file mutation. It must reject path traversal or target escape, keep raw diagnostics out of UI text, avoid secrets entirely, and never mutate before explicit confirmation plus pre-mutation revalidation.
- Requirement Impact Q4: No active requirements are owned. Re-verify settings persistence, Overview and Tools entrypoint behavior, worker handoff, and scanner/auto-fix regressions because S09 changes shared settings, tools, worker, and runtime surfaces. Decisions D028, D029, and D030 apply.
- Verification before completion: targeted filters must pass (`cargo test downgrader_domain`, `cargo test downgrader_service_plan`, `cargo test downgrader_executor`, `cargo test downgrader_controller`, `cargo test downgrader_worker_payload`, `cargo test s09_downgrader_slint_contract`, `cargo test s09_downgrader_runtime_wiring`, plus `cargo test settings`, `cargo test overview`, `cargo test tools`, and `cargo test worker`). Closeout must also run `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features`.
- Negative Tests Q7: sandbox and fake-backed tests must cover missing game path, target path escape, missing files, already-target files, Anniversary/Obsolete/Unknown/unsupported CRCs, malformed desired target callback values, invalid backups, read/write/remove failures, failed download, failed xdelta apply, settings-save failure, stale worker events, and blocked close while running.
- Load Profile Q6: The workflow processes a fixed six-file set and patch files up to the reference 63 MB range. Execution should remain sequential or explicitly bounded, stream/download with progress where practical, avoid unbounded directory scans, and keep all file/network/delta work off the Slint UI thread.

## Proof Level

- This slice proves: Integration and operational safety proof. Real runtime wiring is required for opening the modal and scheduling workers; destructive semantics are proven with sandbox/fake filesystem fixtures before production writes are enabled. Human UAT is not required for slice completion, but the Slint source contract and runtime tests must prove the modal labels, callbacks, and blocked-close behavior.

## Integration Closure

Consumes S02 settings persistence, S03 discovery/platform seams, S04 Overview refresh/action projection, S05 Tools action contracts, S08 fail-closed mutation pattern, and worker event handoff. Produces a live Downgrade Manager entrypoint and service/controller/UI pattern that S10 Archive Patcher can reuse. The roadmap remains unchanged: Archive Patcher stays deferred until S10.

## Verification

- Adds structured tracing for downgrader open/status/plan/run phases, request ids, file-level plan and execution outcomes, settings-save failures, download/apply failures, stale-event rejection, and worker spawn failures. The modal log remains the primary user-visible surface with reference-style per-file messages; diagnostics stay in tests/logs rather than UI text.

## Tasks

- [x] **T01: Add Downgrader domain contract** `est:2h`
  ---
  estimated_steps: 6
  estimated_files: 2
  skills_used:
    - write-docs
    - tdd
  ---
  Why: The destructive workflow needs a Slint-free, IO-free source of truth for reference labels, file order, CRC maps, target names, backup names, patch names, status vocabulary, about copy, log messages, and plan/log row types before any service or UI code can rely on strings.
  Do: Create `src/domain/downgrader.rs` from the read-only Python references. Preserve the exact modal title `Downgrader`, group labels, desired-version labels `Old-Gen` and `Next-Gen`, button labels including `Patch\n All`, initial log line, about title/body, tooltip copy, six file definitions in reference order, install status display labels, CRC maps, backup filename helpers, patch URL/name helpers, and reference-style log message helpers. Add pure types for target, status rows, options snapshot, plan rows, execution log rows, and progress values that later services/controllers can reuse without Slint. Export the module from `src/domain/mod.rs` and add public-import assertions next to the existing domain visibility test.
  Done when: Domain unit/source-contract tests prove all reference strings, CRC mappings, file groups, target labels, backup names, patch URL names, and status labels without reading `.gsd`, `.planning`, or `.audits`.
  - Files: `src/domain/downgrader.rs`, `src/domain/mod.rs`
  - Verify: cargo test downgrader_domain

- [x] **T02: Build status snapshot and preview plan service** `est:3h`
  ---
  estimated_steps: 8
  estimated_files: 3
  skills_used:
    - write-docs
    - tdd
    - security-review
  ---
  Why: The first `Patch All` action must classify the six reference files and build an inline plan while proving that no mutation occurs before confirmation.
  Do: Add `src/services/downgrader.rs` and export it from `src/services/mod.rs`. Implement a `DowngraderService` over the existing read-only `Filesystem` trait that validates the discovered Fallout 4 game root, resolves only the six constant relative paths under that root, rejects absolute or escaping paths, computes CRC32 status snapshots, applies the `steam_api64.dll` Next-Gen and Anniversary display rule, chooses the reference default target from `Fallout4.exe`, reads backup CRCs for plan accuracy, and builds plan rows for skip, invalid-backup cleanup, restore-from-backup, current-backup creation, delta download, patch apply, and optional cleanup. The service must return safe user-facing failures if the root is missing or unsafe and must not call any mutation, downloader, or applier during status or plan building.
  Failure Modes Q5: Missing or unsupported discovery returns a safe failure and disabled mutation. Permission/read errors classify the affected file as unknown or plan failure according to reference-safe behavior. Malformed relative-path definitions are rejected before plan output.
  Negative Tests Q7: Cover missing root, path escape attempts, missing files, already-target files, Anniversary/Obsolete/Unknown/unsupported CRCs, valid and invalid desired/current backups, and no mutation during plan generation.
  Done when: Fake-backed tests prove status classification, reference row order, target defaulting, plan contents, path safety, and zero mutation before confirmation.
  - Files: `src/services/downgrader.rs`, `src/services/mod.rs`, `src/domain/downgrader.rs`
  - Verify: cargo test downgrader_service_plan

- [x] **T03: Implement confirmed executor and write seams** `est:5h`
  ---
  estimated_steps: 9
  estimated_files: 6
  skills_used:
    - rust-async-patterns
    - tdd
    - security-review
    - verify-before-complete
  ---
  Why: After inline confirmation, S09 must safely execute the reference backup, restore, download, xdelta apply, and cleanup semantics against sandbox fixtures before real game paths are reachable.
  Do: Add a separate writable filesystem trait to `src/platform/filesystem.rs` instead of expanding the existing read-only `Filesystem` contract. Extend `PlatformOperation` with safe write/copy/rename/remove labels. Implement real filesystem mutation methods with typed `PlatformError` mapping and local fake/recording implementations for tests. In `src/services/downgrader.rs`, add `DeltaDownloader` and `DeltaApplier` traits, a reqwest-backed downloader with bounded progress callbacks, and a production xdelta applier only after adding/proving a compatible dependency with a tiny deterministic fixture. Add confirmed execution that revalidates current file and backup CRCs immediately before each mutation, processes the six files independently, creates/reuses/removes `_downgradeBackup` and `_upgradeBackup` files according to direction and `Keep Backups`, downloads only the needed `NG-to-OG-{file}.xdelta` or `OG-to-NG-{file}.xdelta`, applies deltas with the current backup as input, honors `Delete Patches`, continues after per-file failures where safe, and never deletes the only valid source backup after a failed apply.
  Failure Modes Q5: File write/copy/rename/remove errors log `Failed patching {file}` and continue where safe. Download timeout/request failure and malformed or failed xdelta apply log failure and preserve backups. If no proven production xdelta applier exists, fail the task and replan rather than shipping a silently non-functional production path.
  Load Profile Q6: Fixed six-file execution, bounded patch downloads, sequential mutation, and progress events per file/download prevent unbounded concurrency or memory growth.
  Negative Tests Q7: Cover valid desired-backup restore with and without `Keep Backups`, invalid backup deletion, current-backup creation/reuse, as-needed delta download, delete-deltas cleanup, failed download, failed apply, read-only/permission failures, unsupported source generation, and no downloader/applier calls for skipped files.
  Done when: Executor tests prove reference log rows and mutation/download/apply calls for success and failure paths, and the production applier has a fixture proof if a dependency is added.
  - Files: `src/platform/mod.rs`, `src/platform/filesystem.rs`, `src/services/downgrader.rs`, `Cargo.toml`, `Cargo.lock`
  - Verify: cargo test downgrader_executor

- [x] **T04: Add controller and worker payloads** `est:4h`
  ---
  estimated_steps: 8
  estimated_files: 4
  skills_used:
    - rust-async-patterns
    - tdd
    - write-docs
  ---
  Why: The Downgrader modal needs a Slint-free state machine and owned worker payloads so status, plan, and execution events can cross background boundaries without moving Slint handles or models into worker threads.
  Do: Add `src/app/downgrader_controller.rs` and export it from `src/app/mod.rs`. Model phases such as closed, loading status, ready, planning, plan ready, running, completed, and safe error. Implement transitions for open from settings snapshot, status-loaded default target selection, target/option changes, first `Patch All` plan request, second explicit confirmation run request, log/progress updates, completion status refresh, worker failure, stale request rejection, and close/Escape blocking while running. Extend `src/workers/events.rs` with `DowngraderWorkerPayload` variants for status loaded, plan ready, log row, progress, run completed, and safe failure data; re-export from `src/workers/mod.rs`. Keep request ids monotonic so stale plan/run/status events fail closed.
  Failure Modes Q5: Worker spawn failure maps to safe visible error and unblocks close unless a run is still active. Stale events are ignored and traced. Malformed UI target/option values revert to the last controller state.
  Negative Tests Q7: Cover open/loading, default target from status, settings option changes, plan confirmation gating, no execution request on first click, blocked close while running, stale status/plan/run events, worker failure recovery, and completion re-enabling patch action.
  Done when: Controller and worker tests prove the lifecycle and payload round trips through `RecordingEventSink` without Slint types.
  - Files: `src/app/downgrader_controller.rs`, `src/app/mod.rs`, `src/workers/events.rs`, `src/workers/mod.rs`
  - Verify: cargo test downgrader_controller
cargo test downgrader_worker_payload

- [ ] **T05: Create Slint modal source contract** `est:3h`
  ---
  estimated_steps: 7
  estimated_files: 5
  skills_used:
    - write-docs
    - tdd
    - make-interfaces-feel-better
  ---
  Why: The user-facing Downgrader must look and behave like the reference modal before runtime code can wire real actions to it.
  Do: Add `ui/downgrader_window.slint` and import it through `ui/main.slint` so Slint generates the component. Build a conservative fixed-shape window titled `Downgrader` near the reference 600x334 proportions, with `Current Game`, `Current Creation Kit`, `Desired Version`, `Options`, `Patch\n All`, `About`, bottom log, progress bar, and an inline plan/confirmation area that stays within the same modal rather than becoming a redesigned wizard. Preserve row labels and display names including Archive2 basename display. Expose properties and callbacks needed by the controller projection: grouped status rows, selected target, `Keep Backups`, `Delete Patches`, plan rows, plan visibility, confirmation state, log rows/text, progress percent/text, patch/about enabled state, close blocked state, target/option callbacks, patch requested, confirm requested if needed, about requested, and close requested. Update Overview and Tools Slint surfaces so they can forward Downgrade Manager open requests while Archive Patcher remains deferred. Add source-contract tests in `src/main.rs` or the nearest existing source-contract test module for labels, titles, callback names, deferred Archive Patcher text, and no accidental Anniversary target option.
  Failure Modes Q5: Slint compile errors or unsupported close interception must be surfaced early with `cargo check`; if exact Tk-style modality is unavailable, document the practical difference in code comments/tests and still block close/Escape while running through available Slint callbacks.
  Negative Tests Q7: Source tests must reject missing required labels, accidental `Anniversary` target selection, absent inline plan confirmation copy, or re-enabled Archive Patcher.
  Done when: Slint compiles and source-contract tests prove the reference-shaped modal and entrypoint callback surfaces.
  - Files: `ui/downgrader_window.slint`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`, `src/main.rs`
  - Verify: cargo test s09_downgrader_slint_contract
cargo check

- [ ] **T06: Wire runtime entrypoints and closeout checks** `est:6h`
  ---
  estimated_steps: 10
  estimated_files: 10
  skills_used:
    - rust-async-patterns
    - tdd
    - verify-before-complete
  ---
  Why: The slice is only complete when the Overview and Tools entrypoints open the real Downgrader modal, persist workflow options safely, schedule background status/plan/run workers, refresh Overview after completion, and leave Archive Patcher deferred.
  Do: Wire `src/main.rs` to own a `DowngraderController`, create/show the generated `DowngraderWindow`, bind modal callbacks, project controller state into Slint properties/models, and route worker events back through the existing event-loop sink pattern. Add `SettingsController::save_downgrader_settings_for_workflow` so `Keep Backups` and `Delete Patches` are persisted at workflow start; on save failure, revert visible options and do not plan or run with unpersisted preferences. Update Tools domain/service/controller behavior so `tools.downgrade_manager` opens the modal instead of returning a deferred rejection, while `tools.archive_patcher` remains disabled/deferred until S10. Update Overview downgrade projection so the button is enabled when the workflow can be opened and still fails closed in the modal if discovery cannot establish a safe game root. Schedule status, plan, and confirmed run work on worker threads using owned request payloads, real discovery and filesystem adapters, the downloader/applier seams from T03, and no Slint handles in closures. On completion, refresh/redraw Downgrader status and request an Overview refresh; keep `Patch\n All` disabled and close/Escape blocked while running. Add runtime wiring tests that use fakes or source-level assertions to prove callback routing, settings-save failure, worker spawn failure, stale completion, Overview refresh request after completion, and Archive Patcher still deferred.
  Failure Modes Q5: Discovery unsupported/missing root shows safe modal log/status and disables mutation. Settings save failure reverts options and suppresses workers. Worker spawn failure shows a safe error. Download/apply failures remain per-file log rows. Off-Windows real discovery/platform failures stay safe and fake-backed tests remain cross-platform.
  Load Profile Q6: Background workers own fixed-size plan/status payloads and emit bounded log/progress events; no UI-thread blocking is allowed for CRC, filesystem, network, or xdelta work.
  Negative Tests Q7: Cover Overview open, Tools open, unknown Tools action, Archive Patcher deferred, missing game path, settings-save failure, close blocked while running, run completion refreshing status, and no Slint handle/model captured by worker closures.
  Done when: Targeted runtime tests pass, full closeout checks pass, and the modal is live from both entrypoints without regressing settings, overview, tools, worker, scanner, or auto-fix behavior.
  - Files: `src/main.rs`, `src/app/settings_controller.rs`, `src/domain/tools.rs`, `src/services/tools.rs`, `src/app/tools_controller.rs`, `src/services/downgrader.rs`, `src/workers/events.rs`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`
  - Verify: cargo test s09_downgrader_runtime_wiring
cargo test settings
cargo test overview
cargo test tools
cargo test worker
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Files Likely Touched

- src/domain/downgrader.rs
- src/domain/mod.rs
- src/services/downgrader.rs
- src/services/mod.rs
- src/platform/mod.rs
- src/platform/filesystem.rs
- Cargo.toml
- Cargo.lock
- src/app/downgrader_controller.rs
- src/app/mod.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/downgrader_window.slint
- ui/main.slint
- ui/overview_tab.slint
- ui/tools_tab.slint
- src/main.rs
- src/app/settings_controller.rs
- src/domain/tools.rs
- src/services/tools.rs
- src/app/tools_controller.rs
