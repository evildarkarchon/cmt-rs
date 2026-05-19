---
id: S09
parent: M001
milestone: M001
provides:
  - A live Downgrade Manager utility entrypoint from Overview and Tools.
  - A tested modal utility/controller/worker pattern for S10 Archive Patcher.
  - A hardened write-plan and confirmed-run pattern for destructive filesystem workflows.
  - Pinned delta integrity and bounded VCDIFF application infrastructure.
requires:
  - slice: S02
    provides: Settings defaults/persistence for Downgrader options.
  - slice: S03
    provides: Discovery/platform seams and worker event handoff foundations.
  - slice: S04
    provides: Overview state projection and refresh patterns.
  - slice: S05
    provides: Tools action routing and deferred utility contracts.
  - slice: S08
    provides: Fail-closed mutation and user-visible action feedback patterns.
affects:
  - S10: Archive Patcher Workflow
key_files:
  - src/domain/downgrader.rs
  - src/services/downgrader.rs
  - src/app/downgrader_controller.rs
  - src/workers/events.rs
  - src/platform/filesystem.rs
  - src/main.rs
  - ui/downgrader_window.slint
  - ui/main.slint
  - ui/overview_tab.slint
  - ui/tools_tab.slint
  - Cargo.toml
  - Cargo.lock
key_decisions:
  - Downgrader preview and confirmed execution are separated: the first Patch All builds a read-only inline plan, and confirmation runs only after the reviewed plan is bound by stable digest.
  - Confirmed execution preserves active game files until replacement bytes are ready, integrity-checked, CRC-checked, and ready for same-directory replacement.
  - Delta assets are validated with pinned size, SHA-256, and expected VCDIFF output-size metadata before use.
  - Runtime modal state lives in a Slint-free controller with owned worker payloads and request-id based stale-event rejection.
  - The Downgrader About action is a real modal overlay with preserved reference copy, not a deferred utility log message.
patterns_established:
  - Fail-closed destructive workflow pattern: status -> read-only plan -> explicit confirmation -> digest re-preview -> bounded mutation.
  - Use fakeable `Filesystem`, `DeltaDownloader`, and `DeltaApplier` seams for sandbox-proven file mutation and network/patch failures.
  - Emit live progress/log callbacks from long-running services into typed worker payloads while keeping Slint handles on the UI thread.
  - Keep modal close/Escape blocking as controller/runtime state rather than relying on Slint markup alone.
observability_surfaces:
  - Reference-style per-file Downgrader log rows remain the primary user-visible completion/failure surface.
  - Progress percent/text is projected live while downloads and patch application run.
  - Safe failure messages are surfaced through controller state while diagnostic details remain in service results/tracing/tests.
  - Stale worker events, settings-save failures, worker spawn failures, plan mismatches, and download/apply failures are covered by runtime/service tests.
drill_down_paths:
  - .gsd/milestones/M001/slices/S09/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S09/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S09/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S09/tasks/T04-SUMMARY.md
  - .gsd/milestones/M001/slices/S09/tasks/T05-SUMMARY.md
  - .gsd/milestones/M001/slices/S09/tasks/T06-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-19T00:45:03.969Z
blocker_discovered: false
---

# S09: Downgrade Manager Workflow

**Delivered a live Downgrade Manager modal opened from Overview or Tools with reference-shaped status, options, inline plan confirmation, safe digest-bound execution, live progress/log feedback, and sandbox-proven mutation safeguards.**

## What Happened

S09 ports the reference Downgrader workflow into the Rust/Slint application as a separate `Downgrader` window surfaced from both Overview and Tools. The slice added a Slint-free `domain::downgrader` contract for the reference labels, desired targets (`Old-Gen`, `Next-Gen`), managed Fallout 4 and Creation Kit file set, CRC/status vocabulary, backup naming, patch URL naming, about copy, plan rows, log rows, and progress payloads. `services::downgrader` classifies the six managed files through the existing filesystem seam, validates that the discovered game root and managed targets remain contained, builds read-only preview plans, and keeps mutation, download, and patch application behind fakeable traits.

After security closeout findings, the confirmed executor was hardened before slice completion. Confirmed runs now revalidate canonical containment immediately before mutation, reject symlink/reparse-point escapes where exposed by the platform seam, preserve the active file until replacement bytes are produced and verified, check local and downloaded xdelta assets against a pinned size/SHA-256/output-size manifest, and cap VCDIFF output to the expected target size. Runs are bound to the inline preview through `DowngraderPreviewPlan::stable_digest()`; if a fresh re-preview differs from the confirmed plan, execution fails closed before mutation and asks the user to preview again.

The app layer added a Slint-free `DowngraderController` with request-id based lifecycle states for loading, ready, planning, plan-ready, running, completed, and safe-error phases. It gates the first `Patch All` click into preview generation, requires explicit confirmation before running, blocks close/Escape while running, ignores stale worker events, persists the existing `downgrader_keep_backups` and `downgrader_delete_deltas` options when starting work, and projects owned status/progress/log payloads back to the modal. Runtime wiring opens the modal from Overview or Tools, implements the preserved About Downgrading copy as a real modal overlay, emits live worker progress/log rows while execution is active, and refreshes Overview after completion using the current shared settings snapshot rather than defaults. Archive Patcher remains deferred for S10.

## Verification

Fresh closeout verification was run through `gsd_exec` id `62a79917-b91a-4a94-9c84-b5d979e46132`. All required slice filters and quality gates exited 0: `cargo test downgrader_domain --quiet` (8 passed), `cargo test downgrader_service_plan --quiet` (8 passed), `cargo test downgrader_executor --quiet` (6 passed), `cargo test downgrader_controller --quiet` (10 passed), `cargo test downgrader_worker_payload --quiet` (1 passed), `cargo test s09_downgrader_slint_contract --quiet` (2 passed), `cargo test s09_downgrader_runtime_wiring --quiet` (6 passed), `cargo test settings --quiet` (35 passed), `cargo test overview --quiet` (59 passed), `cargo test tools --quiet` (21 passed), `cargo test worker --quiet` (36 passed), `cargo fmt --check`, `cargo check --quiet`, full `cargo test --quiet` (326 passed), and `cargo clippy --all-targets --all-features --quiet`. Clippy exited 0 while reporting warning-level findings, including existing `too_many_arguments`-style warnings and test-code `field_reassign_with_default`; no gate failed. The direct runtime wiring filter now passes without the earlier warning-suppression workaround.

## Requirements Advanced

None.

## Requirements Validated

None.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

None.

## Operational Readiness

None.

## Deviations

S09 intentionally adds an inline preview/confirmation gate before mutation, diverging from the reference one-click mutation flow as a safety improvement. T03 also added `sha2` plus pinned delta asset integrity metadata, and production xdelta application is routed through the `vcdiff-decoder`-backed `DeltaApplier` trait rather than shelling out to xdelta. A prior rustc warning/ICE workaround noted in T06 is no longer needed because the final direct `s09_downgrader_runtime_wiring` filter passes.

## Known Limitations

Manual UAT against a real Fallout 4 install was not performed in this closeout; destructive behavior is proven through fake/sandbox-backed automated tests. Clippy exits 0 but still emits warning-level diagnostics, including some in `src/main.rs` tests and existing broad callback functions. The workflow remains sequential, has no cancellation control, and does not add Anniversary/AE as a selectable target or implement Archive Patcher.

## Follow-ups

S10 should reuse the S09 patterns for fail-closed write plans, inline confirmation, plan digests, active-file-preserving replacement, fakeable filesystem seams, live worker feedback, and close-blocked modal behavior. Consider reducing warning-level clippy noise in a cleanup slice so future closeout logs are easier to scan.

## Files Created/Modified

- `src/domain/downgrader.rs` — Added the Slint-free Downgrader domain contract, reference strings, CRC/status mappings, file definitions, plan/log/progress payloads, backup helpers, and patch metadata helpers.
- `src/services/downgrader.rs` — Added read-only status/plan generation, safe containment validation, digest-bound confirmed execution, delta integrity checks, bounded VCDIFF application, live event callbacks, and executor tests.
- `src/app/downgrader_controller.rs` — Added the Slint-free modal lifecycle reducer, option persistence workflow state, plan confirmation gating, stale-event rejection, close blocking, and safe failure/progress/log projection.
- `src/workers/events.rs` — Added owned Downgrader worker payloads for status, plan, log, progress, completion, and safe failure events.
- `src/platform/filesystem.rs` — Extended the filesystem seam with metadata/canonicalization/replacement capabilities required for safe mutation tests and execution.
- `src/main.rs` — Wired Overview/Tools Downgrader entrypoints, modal projection, About overlay, settings snapshots, worker scheduling, Overview refresh, and source/runtime contract tests.
- `ui/downgrader_window.slint` — Added the faithful Downgrader window UI with status panels, desired target controls, options, inline plan confirmation, log, progress, about overlay, and callbacks.
- `ui/main.slint` — Imported/exported the Downgrader component and forwarded entrypoint callbacks.
- `ui/overview_tab.slint` — Enabled Overview Downgrade Manager callback forwarding while preserving reference placement.
- `ui/tools_tab.slint` — Enabled the Tools Downgrade Manager utility action while keeping Archive Patcher deferred.
- `Cargo.toml` — Added focused dependencies needed for pinned hash verification and delta application.
- `Cargo.lock` — Recorded dependency lockfile updates for S09.
