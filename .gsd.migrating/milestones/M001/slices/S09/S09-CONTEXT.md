---
id: S09
milestone: M001
status: ready
---

# S09: Downgrade Manager Workflow — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver a faithful, modal Downgrade Manager workflow that opens from Overview or Tools, shows current Fallout 4 and Creation Kit version status, previews and confirms the patch plan, then runs the reference OG/NG backup, restore, delta-download, patch, cleanup, log, and progress flow without blocking the main Slint UI thread.

## Why this Slice

S09 comes after the read-only diagnostics and Scanner Auto-Fix plumbing because downgrade operations are destructive: they rename, copy, delete, download, and patch game/Creation Kit files and therefore need the settings, discovery, worker, safe-action, and fail-closed patterns from earlier slices before users can run them safely. It also replaces the disabled/deferred `Downgrade Manager` utility entries established in S04/S05 and unblocks S10 by proving the modal utility, write-plan, backup, cleanup, progress, and non-blocking mutation workflow pattern that Archive Patcher will reuse.

## Scope

### In Scope

- Enable the existing Overview `Downgrade Manager...` and Tools `Downgrade Manager` entry points and open a separate faithful modal/window titled `Downgrader`, rather than embedding the workflow into an existing tab.
- Preserve the reference modal shape as closely as Slint allows: `Current Game`, `Current Creation Kit`, `Desired Version`, `Options`, `Patch\n All`, `About`, bottom log, and progress bar.
- Preserve reference desired-version options only: `Old-Gen` and `Next-Gen`. Anniversary/AE is not a target in S09.
- Include the same reference file groups in one workflow: Fallout 4 runtime files (`Fallout4.exe`, `Fallout4Launcher.exe`, `steam_api64.dll`) and Creation Kit/Archive2 files (`CreationKit.exe`, `Tools\Archive2\Archive2.exe`, `Tools\Archive2\Archive2Interop.dll`).
- Show current file status using the reference install-type vocabulary where applicable: `Old-Gen`, `Next-Gen`, `Anniversary`, `Obsolete`, `Unknown`, and `Not Found`.
- Default and persist `Keep Backups` and `Delete Patches` from the existing `downgrader_keep_backups` and `downgrader_delete_deltas` settings, saving changed options when the user starts the patch workflow.
- Add an intentional safety improvement before mutation: the first `Patch All` action builds and displays an inline plan inside the same Downgrader modal, showing files to skip, restore from backup, back up, download deltas for, patch, and clean up.
- Require explicit confirmation from the inline plan before any file is renamed, copied, deleted, downloaded, or patched.
- After confirmation, keep the batch behavior close to the reference: process the reference file set independently, use backup files when valid, download GitHub xdelta patches only when a valid backup cannot restore the desired version, apply patches, and continue the reference queue flow as practical.
- Preserve reference backup naming and semantics: `_downgradeBackup` for current NG files when downgrading, `_upgradeBackup` for current OG files when upgrading, and reuse compatible existing backups including Simple Downgrader-style names.
- Preserve reference delta download semantics: use the CMT GitHub `delta-patches` release base, construct `NG-to-OG-{file}.xdelta` or `OG-to-NG-{file}.xdelta`, show per-download progress, and honor `Delete Patches` cleanup after patching.
- Preserve reference-style log feedback as the primary completion surface: `Skipped {file}: Already ...`, `Skipped {file}: Not Found.`, `Skipped {file}: Unsupported Version.`, `Patched {file}`, and `Failed patching {file}`.
- On completion, refresh/redraw the version status, re-enable the patch action, and rely on the per-file log rows rather than adding a new success/partial/failure summary banner.
- While patch/download work is running, disable the patch action and block close/Escape for the Downgrader modal like the reference `processing_data` behavior.
- Keep downloads, CRC checks, file operations, xdelta application, progress updates, and Overview refresh work off the Slint UI thread and marshal owned status/progress/results back through worker/event-loop handoff.
- Preserve the reference `About Downgrading Fallout 4 & Creation Kit` text and provide an `About` action from the modal.
- Add sandbox/fake-backed tests for status classification, plan generation, confirmation gating, backup creation/reuse/removal, restore-from-backup, as-needed delta download, xdelta apply success/failure, `Keep Backups`, `Delete Patches`, not-found/unsupported/unknown files, read-only files, failed downloads, and worker/UI state transitions.

### Out of Scope

- Adding Anniversary/AE as a selectable downgrade/upgrade target.
- Adding a game-only mode that omits Creation Kit and Archive2 files.
- Replacing the faithful modal with a wizard, main-tab embedded panel, or redesigned workflow.
- Starting destructive work immediately on the first `Patch All` click without a preview/confirmation step.
- Pre-downloading every xdelta before any mutation; S09 uses the reference as-needed download flow after confirmation.
- Requiring users to manually provide xdelta patches or making S09 offline-only.
- Adding cancellation controls or letting the Downgrader modal close while work continues in the background.
- Adding a final summary line or partial-failure banner beyond the reference-style per-file log and refreshed version display.
- Implementing Archive Patcher, BA2 mutation, Scanner Auto-Fix production mutations, or new scanner/downgrader product features not present in the reference.
- Adding better-than-reference support for Vortex staging or cross-game downgrade workflows.

## Constraints

- `CMT/` remains read-only; inspect the Python reference for parity but implement all Rust/Slint behavior outside the submodule.
- Preserve reference labels, group names, option names, button text, about copy, install-type labels, backup names, patch URL naming, and log messages unless an intentional difference is documented.
- The inline plan/confirmation is an intentional safety divergence from the Python one-click mutation flow; it must still feel like the same Downgrader modal rather than a redesigned wizard.
- The workflow must fail closed before mutation if discovery cannot establish the game path or if a target path would be outside the discovered Fallout 4 installation.
- Unsupported, unknown, obsolete, Anniversary, and not-found files should be skipped with reference-style log entries rather than force-patched.
- All mutating operations must be tested against temporary/sandbox fixtures before real-path writes are enabled.
- Do not move Slint handles, models, or UI-owned objects into worker threads; worker payloads must be owned Rust data.
- Downloads and patch application must surface user-safe progress/log feedback and keep diagnostic details in structured logs/tests.
- Settings round-trip must preserve the existing `downgrader_keep_backups` and `downgrader_delete_deltas` JSON keys and defaults.
- The modal should remain non-resizable/fixed-shape as practical and block close/Escape while work is running, matching the reference `ModalWindow` behavior.

## Integration Points

### Consumes

- `CMT/src/downgrader.py` — Source of truth for modal title/size, current-version panels, desired-version options, settings usage, CRC maps, backup naming, patch direction, delta URL construction, queue/progress flow, skip/success/failure log messages, and completion refresh behavior.
- `CMT/src/globals.py` — Source of truth for `ABOUT_DOWNGRADING_TITLE`, `ABOUT_DOWNGRADING`, `TOOLTIP_DOWNGRADER_BACKUPS`, and `TOOLTIP_DOWNGRADER_DELTAS` copy.
- `CMT/src/enums.py` — Source of truth for `InstallType` labels and `LogType` categories used by the reference Downgrader log.
- `CMT/src/modal_window.py` — Source of truth for modal close/Escape behavior and `processing_data` close blocking.
- `CMT/src/tabs/_overview.py` — Source of truth for the Overview `Downgrade Manager...` entry point placement and label.
- `CMT/src/tabs/_tools.py` — Source of truth for the Tools `Toolkit Utilities` `Downgrade Manager` entry point.
- `src/domain/settings.rs` and `src/platform/settings_store.rs` — Existing typed settings/defaults/persistence for `downgrader_keep_backups` and `downgrader_delete_deltas`.
- `src/domain/discovery.rs` and `src/services/discovery.rs` — Existing discovered Fallout 4 installation, game path, install-type, and mod-manager context used to anchor safe target paths.
- `src/services/overview_collector.rs` and `src/domain/overview.rs` — Existing binary classification/downgrade-detection patterns and Overview refresh state to reuse or extend after patch completion.
- `src/platform/filesystem.rs` — Fakeable file metadata/read/write/rename/remove/copy boundary for sandboxed downgrade operations.
- `src/platform/process.rs` — Existing executable-version/version-metadata boundary, and potential seam for external patch application if xdelta is implemented as a process rather than a library.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — Owned event, progress, cancellation-token, spawn-blocking, and Slint-safe event-loop handoff patterns used for long-running downgrade work.
- `ui/overview_tab.slint`, `ui/tools_tab.slint`, and `ui/main.slint` — Existing disabled/deferred utility entry surfaces to replace with live Downgrade Manager callbacks/properties.

### Produces

- `src/domain/downgrader.rs` or equivalent domain additions — Typed downgrade file definitions, CRC/version classification, desired target, options snapshot, backup names, operation-plan records, safe log/status labels, and result summaries independent of Slint.
- `src/services/downgrader.rs` — Adapter-backed plan builder and executor for backups, restore-from-backup, as-needed delta download, xdelta apply, cleanup, and completion refresh facts.
- `src/app/downgrader_controller.rs` — Slint-free reducer/controller state for modal open/close, selected desired version, option state, inline plan, confirmation, running/blocked-close state, log rows, progress, completion redraw, and failure handling.
- `src/workers/events.rs` — Downgrader-specific worker payloads for plan-ready, progress, log row, file result, download progress, completion, and safe failure events.
- `ui/downgrader_window.slint` or equivalent Slint component — Faithful Downgrader modal/window UI with current status panels, desired-version radio buttons, options, inline plan/confirmation, `Patch\n All`, `About`, log, and progress bar.
- `ui/overview_tab.slint`, `ui/tools_tab.slint`, and `ui/main.slint` — Live callback forwarding/enabled states for opening the Downgrade Manager from Overview and Tools.
- Rust tests and Slint source-contract tests — Coverage for labels/order/copy fidelity, settings persistence timing, plan-confirm gating, no mutation before confirmation, sandboxed write behavior, download/progress events, completion redraw, blocked close while running, and reference-style log output.

## Open Questions

- None at discussion closeout. If cancellation, AE targeting, game-only mode, stronger completion summaries, or offline/manual patch handling are requested later, treat them as future scoped changes rather than part of S09.
