# S09 Research: Downgrade Manager Workflow

## Summary

Depth: **deep research**. S09 is the first destructive workflow in the Rust port: it opens a new modal, reads large executable CRCs, writes settings, renames/copies/deletes game files, downloads GitHub xdelta files, applies binary deltas, updates progress/log UI, and refreshes Overview. Existing S04-S08 patterns are strong enough for the controller/worker/UI handoff, but the current platform filesystem seam is intentionally read-only and no Rust xdelta implementation has been proven yet.

Recommended implementation shape: build the Downgrader as a Slint-free domain/service/controller slice first, with a fake-backed plan/executor proving no mutation before confirmation. Add the Slint window and runtime wiring only after the write plan, backup semantics, and worker events are covered by tests. Keep real xdelta/download code behind narrow traits so the safety contract and UI can be completed independently of crate/tool selection risk.

## Active Requirements

No active `REQUIREMENTS.md` requirements were preloaded for this slice.

## Skills Discovered

- Installed and relevant from prompt: `write-docs` for stranger-readable research structure; `rust-async-patterns` may be useful for executor/runtime work because S09 mixes blocking file work and network progress.
- Skill registry search:
  - `npx skills find "Slint"` returned unrelated ESLint/accessibility skills, not a Slint/Rust GUI skill worth installing.
  - `npx skills find "xdelta"` returned no skills.
- No new skills were installed.

## Reference Behavior to Preserve

Source of truth: `CMT/src/downgrader.py`, plus `globals.py`, `enums.py`, `modal_window.py`, `_overview.py`, and `_tools.py`.

### Modal and UI shape

- Window: `Downgrader`, reference size `600x334`, non-resizable, transient/grabbed modal through `ModalWindow`.
- Close/Escape: `ModalWindow._ungrab_and_destroy` returns early while `processing_data` is true. Rust should block close/Escape while running.
- Groups and labels:
  - `Current Game`: `Fallout4.exe`, `Fallout4Launcher.exe`, `steam_api64.dll`.
  - `Current Creation Kit`: `CreationKit.exe`, `Archive2.exe`, `Archive2Interop.dll` (display uses `Path(f).name`, while internal paths for Archive2 files include `Tools\Archive2\...`).
  - `Desired Version`: radio labels `Old-Gen`, `Next-Gen`.
  - `Options`: checkboxes `Keep Backups`, `Delete Patches`.
  - Main button text is exactly `Patch\n All`.
  - Secondary button text is `About`.
  - Bottom log starts with `Patches will be downloaded and applied as-needed.` and progress range is 0-100.
- Desired-version default: reference sets `Old-Gen` selected only when current `Fallout4.exe` is classified `Old-Gen`; otherwise `Next-Gen` is selected. This means the default target usually matches the detected game generation instead of choosing the opposite direction.
- About title/text from `globals.py`:
  - Title: `About Downgrading Fallout 4 & Creation Kit`.
  - Text explains delta patches from GitHub, patch sizes, backups, Simple Downgrader backup reuse, backup names, and that game + Creation Kit must be patched together.

### Install-type and status vocabulary

Reference `InstallType` display values from `CMT/src/enums.py`:

- `Obsolete`, `Old-Gen`, `Down-Grade`, `Next-Gen`, `Anniversary`, `Next-Gen & Anniversary`, `Unknown`, `Not Found`.
- S09 scope only makes `Old-Gen` and `Next-Gen` selectable targets. `Anniversary`, `Obsolete`, `Unknown`, and `Not Found` are status/skip states.
- `steam_api64.dll` can classify as `Next-Gen & Anniversary`; reference `draw_versions` displays this as `Next-Gen` or `Anniversary` based on the current game type when possible, otherwise bad/unknown color.

### Downgrader-specific CRC map

Do not blindly reuse `OverviewCollector::BASE_FILES`: the reference Downgrader has its own CRC maps, includes `Tools\Archive2\Archive2Interop.dll`, and classifies `CreationKit.exe` by CRC rather than version metadata.

| File path | Old-Gen CRC(s) | Next-Gen CRC(s) | Anniversary CRC(s) | Obsolete CRC(s) |
| --- | --- | --- | --- | --- |
| `Fallout4.exe` | `C6053902` | `C5965A2E` | `CF47788D` | `97DA3E03`, `2ED2A242`, `A0100017`, `9ABC94F0`, `B61675B1`, `0AEB19A7`, `1E90BE57`, `0481725D`, `0E176ABC` |
| `Fallout4Launcher.exe` | `02445570` | `F6A06FF5` | `720BB9C3` | `0E696744`, `D15C6A49`, `8C52BE93`, `591009C9` |
| `steam_api64.dll` | `BBD912FC` | `E36E7B4D` as NG/AE | `E36E7B4D` as NG/AE | none |
| `CreationKit.exe` | `0F5C065B` | `481CCE95` | `49E45284` | none |
| `Tools\Archive2\Archive2.exe` | `4CDFC7B5` | `71A5240B` | `C867674F` | none |
| `Tools\Archive2\Archive2Interop.dll` | `850D36A9` | `EFBE3622` | `7B893B0D` | none |

### Patch and backup semantics

For each of the six reference files, the reference independently skips or patches:

- Already target: `Skipped {file}: Already {desired_version}.`
- Missing: `Skipped {file}: Not Found.`
- `Anniversary` or `Obsolete`: `Skipped {file}: Unsupported Version.`
- Current CRC not in the expected source generation: `Skipped {file}: Unsupported Version.`
- Successful restore or delta apply: `Patched {file}`.
- OSError or failed delta: `Failed patching {file}`.

Backup names and direction:

- Current Next-Gen file being downgraded to Old-Gen:
  - current backup name: `{stem}_downgradeBackup{suffix}`
  - desired backup name: `{stem}_upgradeBackup{suffix}`
  - patch file name: `NG-to-OG-{file}.xdelta`
- Current Old-Gen file being upgraded to Next-Gen:
  - current backup name: `{stem}_upgradeBackup{suffix}`
  - desired backup name: `{stem}_downgradeBackup{suffix}`
  - patch file name: `OG-to-NG-{file}.xdelta`
- Patch URL base: `https://github.com/wxMichael/Collective-Modding-Toolkit/releases/download/delta-patches/`.
- If a backup of the current version exists and its CRC matches the current file, reference deletes the current file and reuses the backup as the xdelta input. If it exists but CRC differs, reference deletes the bad backup.
- If current file still exists, reference renames it to the current backup name.
- If a desired-version backup exists and CRC matches the desired generation, reference restores from it: copy when `Keep Backups` is true, move/replace when false.
- If a desired-version backup exists but CRC is wrong, reference deletes it.
- If output file still does not exist after backup restore attempts, reference downloads the xdelta as-needed and applies it to the current backup.
- If `Keep Backups` is false, reference deletes the current backup after successful restore/patch paths.
- If `Delete Patches` is true, reference deletes the downloaded xdelta after patch application.

Research surprise: reference `patch_files` uses a single `patch_needed` variable reset inside the loop, so a later skipped file can re-enable the button even if an earlier file queued download/patch work. The Rust implementation should not preserve that UI race; the S09 context explicitly requires the patch action disabled and close blocked while work runs.

## Existing Rust Landscape

### Useful existing seams

- `src/domain/settings.rs` already has `DowngraderSettings { keep_backups, delete_deltas }`, defaulting both to true and preserving JSON keys `downgrader_keep_backups` and `downgrader_delete_deltas`.
- `src/app/settings_controller.rs` owns last-persisted settings and has the scanner save-at-start pattern. S09 should add an analogous `save_downgrader_settings_for_workflow` instead of writing settings directly from UI callbacks.
- `src/domain/discovery.rs` has `Fallout4Installation`, `Fallout4InstallType`, and optional `data_path`; use `game_path` as the safe anchor for all target paths.
- `src/services/discovery.rs` and `src/services/overview_collector.rs` already show the production pattern for discovering the installation and collecting binary facts off the UI thread.
- `src/workers/mod.rs` has `WorkerRuntime::spawn_blocking_task`, `WorkerTaskContext::emit_progress`, cancellation tokens, and safe failure events.
- `src/workers/events.rs` already has typed payload variants for Overview, Scanner, F4SE, Tools, and About. Add a Downgrader-specific payload rather than overloading display strings.
- `src/app/overview_controller.rs` and `src/app/scanner_controller.rs` are the patterns to copy: monotonic/stale-safe worker IDs, Slint-free state, safe worker-failure mapping, and UI projection in `main.rs`.
- `src/domain/tools.rs`, `src/services/tools.rs`, `src/app/tools_controller.rs`, and `ui/tools_tab.slint` currently model `Downgrade Manager` as a disabled/deferred utility. S09 must turn only this entry live while leaving `Archive Patcher` deferred until S10.

### Missing or insufficient seams

- `src/platform/filesystem.rs` is read-only (`metadata`, `read_bytes`, `read_prefix`, `read_to_string`, `read_dir`, `walk_dir`). S09 needs mutation operations but extending `Filesystem` directly would force many existing fakes/tests to implement write methods. Prefer a new trait such as `FileMutation`/`WritableFilesystem` with real and fake implementations.
- `src/platform/mod.rs::PlatformOperation` lacks write operation variants. Add safe labels for operations like write file, copy file, rename/replace file, remove file, and set writable/read-only handling.
- There is no HTTP download client with streaming progress. `src/services/update.rs` has an async update-check client over `reqwest`, but it returns whole response bodies and is not file/progress oriented.
- There is no xdelta applier seam. Cargo search found:
  - `xdelta3 = 0.1.5`: Rust binding for xdelta3; unknown MSRV/build implications, needs proof.
  - `oxidelta = 0.1.4`: pure Rust VCDIFF/xdelta-style implementation, but `rust-version = 1.90`, incompatible with this crate's MSRV 1.85.
- Slint multiple-window/modal details need a compile proof. The likely shape is a second `export component DowngraderWindow inherits Window`; close blocking probably needs Slint's close-request callback/response. Verify early with `cargo check` because true Tk-style grab/transient modality may not be fully supported.

## Recommended Implementation Landscape

### 1. Domain/reference contract (`src/domain/downgrader.rs`)

Purpose: freeze all reference constants, labels, file definitions, target enums, plan/log/status structs, and display strings without Slint or IO.

Suggested contents:

- Constants:
  - `DOWNGRADER_TITLE = "Downgrader"`
  - `CURRENT_GAME_TITLE`, `CURRENT_CREATION_KIT_TITLE`, `DESIRED_VERSION_TITLE`, `OPTIONS_TITLE`
  - `PATCH_ALL_LABEL = "Patch\n All"`, `ABOUT_LABEL = "About"`
  - `PATCH_URL_BASE`
  - `ABOUT_DOWNGRADING_TITLE`, `ABOUT_DOWNGRADING`, `TOOLTIP_DOWNGRADER_BACKUPS`, `TOOLTIP_DOWNGRADER_DELTAS`
  - initial log message and skip/success/failure format helpers.
- Types:
  - `DowngradeTarget::{OldGen, NextGen}` with exact display labels.
  - `DowngradeInstallStatus` or reuse/convert `Fallout4InstallType` carefully; include `NextGenAnniversary` internally for `steam_api64.dll`.
  - `DowngradeFileDefinition { relative_path, display_name, group, crc_map }` in reference order.
  - `DowngradeStatusRow { definition, status, crc, path, severity }`.
  - `DowngraderOptions { keep_backups, delete_deltas }` (or reuse `DowngraderSettings` at boundaries).
  - `DowngradePlan`, `DowngradePlanFile`, `DowngradePlanStep`, `DowngradeExecutionLogRow`, `DowngradeProgress`.
- Unit/source-contract tests:
  - exact file order/group labels/CRC map.
  - exact about/tooltips/log strings.
  - target labels only `Old-Gen` and `Next-Gen`.
  - backup/patch filename helper outputs for representative files.

### 2. Plan/status/execution service (`src/services/downgrader.rs`)

Purpose: all CRC/status reading, safe path validation, plan generation, backup semantics, downloader/applier orchestration, and per-file results. No Slint.

Recommended traits:

- `DowngradeReadFs` can be the existing `Filesystem`.
- New mutation trait in `src/platform/filesystem.rs`, e.g.:
  - `copy_file(from, to)`, `rename_file(from, to)`, `replace_file(from, to)`, `remove_file(path)`, `write_file(path, bytes)`, `set_writable(path)`.
  - Keep it separate from `Filesystem` to avoid broad test churn.
- `DeltaDownloader`: `download(url, destination, progress_callback)` or iterator/chunk method. Production can use `reqwest`; tests fake deterministic success/failure/progress.
- `DeltaApplier`: `apply(patch, input, output) -> Result<(), DeltaApplyError>`. Tests fake; production can be implemented by `xdelta3` after a compile/runtime proof.
- Optional `DeltaStore`: computes patch file destination. Keep patch names exactly `NG-to-OG-{file}.xdelta`/`OG-to-NG-{file}.xdelta`; isolate the directory decision behind this seam.

Service operations:

- `classify_installation(installation) -> DowngradeStatusSnapshot` reads CRCs for the six files and maps NotFound/Unknown/Obsolete/AE/OG/NG.
- `build_plan(installation, status_snapshot, target, options) -> DowngradePlan`:
  - fail closed if game path is unavailable.
  - validate every target path is under `installation.game_path`.
  - include skip rows, backup rows, restore rows, download/patch rows, cleanup rows.
  - read backup CRCs while planning so the inline plan can say restore/delete/download accurately.
  - do **not** mutate.
- `execute_plan(plan, sink/context)`:
  - immediately revalidate current CRC/backup assumptions before first mutation.
  - emit reference-style log rows.
  - continue per-file where practical after failures, as reference queue behavior does.
  - never delete the only valid source backup after a failed delta apply.
  - return final status snapshot or enough information for a status refresh.

### 3. Controller (`src/app/downgrader_controller.rs`)

Purpose: Slint-free modal state machine.

Suggested state:

- `closed/open`, `phase: Idle | LoadingStatus | Ready | Planning | PlanReady | Running | Completed | Error`.
- current status rows grouped as game/CK.
- selected target and options.
- inline plan rows and confirmation-required flag.
- log rows/multiline log text.
- progress percent/text.
- patch button label/enabled, about enabled, close_blocked.
- latest request id/task id so stale status/plan/run events are ignored.

Suggested transitions:

- `open(settings_snapshot)` -> visible with options from settings, schedule status worker.
- `status_loaded(snapshot)` -> rows updated; default target selected using reference default rule.
- `patch_all_requested()` when Ready/Completed -> returns plan request, disables button while planning.
- `plan_ready(plan)` -> shows inline plan; no mutation has occurred; requires a second explicit confirmation.
- `patch_all_requested()` when PlanReady -> returns execution request if confirmed/current plan still active.
- `execution_event(log/progress)` -> append visible log/progress.
- `execution_completed(final_status)` -> refresh status, re-enable patch, unblock close, clear/retain plan per UI choice.
- `worker_failed` -> safe log/status, unblock close unless an operation is still running.

### 4. Worker events (`src/workers/events.rs`)

Add typed payloads such as:

- `WorkerPayload::Downgrader(DowngraderWorkerPayload)`.
- `DowngraderWorkerPayload::StatusLoaded { request_id, snapshot }`.
- `PlanReady { request_id, plan }`.
- `LogRow { request_id, row }`.
- `Progress { request_id, progress }` or use existing `WorkerPayload::Progress` plus typed request id in task id; typed is cleaner.
- `RunCompleted { request_id, final_status }`.

Use `WorkerTaskKind::Patch` for the execution task. A plan/status task can use `Patch` or `Generic`; keep labels clear (`s09-downgrader-status`, `s09-downgrader-plan`, `s09-downgrader-run`).

### 5. Slint UI (`ui/downgrader_window.slint`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`)

Suggested Slint contracts:

- New structs: `DowngraderStatusUiRow`, `DowngraderPlanUiRow`, `DowngraderLogUiRow`.
- `DowngraderWindow inherits Window` with fixed-ish width/height near reference, background/style matching current app.
- Expose properties for all controller projections: rows, selected target, options, plan visible, log rows/text, progress, button states, close_blocked.
- Expose callbacks: target selected, option toggled, patch all requested, about requested, close requested.
- Main callbacks:
  - `overview-open-downgrade-manager-requested` should open/show Downgrader instead of applying deferred error.
  - `tool-action-requested("tools.downgrade_manager")` should open/show Downgrader instead of being rejected as disabled.
- `ToolsTab`: enable `Downgrade Manager`, remove S09 deferred helper text/status for that row; keep `Archive Patcher` disabled until S10.
- `OverviewTab`: enable `Downgrade Manager...` when an action is available and not busy/running; status text should become `Ready.` or similar. If no game path is known, either keep disabled with safe text or allow opening a modal that fails closed.

### 6. Runtime wiring (`src/main.rs`)

Main is currently monolithic, but S09 can still follow existing patterns:

- Instantiate/own `DowngraderWindow` and `Arc<Mutex<DowngraderController>>` near other controllers.
- Add `bind_downgrader_worker_sink`, `bind_downgrader_callbacks`, `apply_current_downgrader_state`, and projection helpers.
- On open from Overview/Tools:
  - show window.
  - schedule discovery/status worker using `DiscoveryService`, `RealFilesystem`, `RealRegistry`, `RealProcessInspector`.
  - if discovery fails, controller shows safe failure/log and leaves mutation disabled.
- Before planning/execution, call new `SettingsController::save_downgrader_settings_for_workflow`; if save fails, revert option checkboxes and do not schedule a plan/run with unpersisted preferences.
- On execution completion, schedule/request Overview refresh to redraw main status.

## Natural Seams / Task Boundaries

1. **Reference contract/domain constants**: `src/domain/downgrader.rs` plus `src/domain/mod.rs`; no platform/UI changes.
2. **Read/status + plan builder**: `src/services/downgrader.rs` using existing read-only `Filesystem`; fake-backed tests for classification, path safety, plan generation, and no mutation.
3. **Mutation boundary + executor**: new write trait in `src/platform/filesystem.rs`, `PlatformOperation` variants, fake mutation recorder, executor tests for backup/restore/download/apply/cleanup semantics.
4. **Delta dependency proof**: behind `DeltaApplier`; test `xdelta3` compile and tiny fixture application before committing production use. Avoid `oxidelta` under current MSRV.
5. **Controller/event reducer**: `src/app/downgrader_controller.rs` and worker payloads; tests for open/loading/plan-ready/running/completed/blocked-close/stale events.
6. **Slint window source contract**: `ui/downgrader_window.slint`, `ui/main.slint` imports/properties/callbacks; source tests for labels/order/copy.
7. **Entry point enablement**: change Overview and Tools deferred behavior for Downgrade Manager only; keep Archive Patcher deferred.
8. **Runtime adapters/wiring**: discovery/status/plan/run workers, real filesystem/downloader/applier, settings save, Overview refresh after completion.

Several of these can proceed in parallel after the domain contract lands: UI source-contract work, controller tests, and fake service/executor tests do not need real xdelta.

## First Proof

Highest-value first proof: **fake-backed write-plan/executor safety before any UI**.

Minimum proof tests:

1. Given a fake Fallout 4 root and six fake files with known CRC bytes, `classify_installation` returns the exact reference statuses and display row order.
2. First `Patch All` / `build_plan` produces plan rows and performs **zero** fake mutation calls.
3. Plan generation fails closed when the game path is missing or any derived target path would escape the discovered Fallout 4 root.
4. Confirmed execution with a valid desired backup restores/copies/moves according to `Keep Backups` and logs `Patched {file}`.
5. Confirmed execution with no desired backup downloads exactly the expected URL/file name, calls the fake delta applier with current backup input, logs success/failure, and honors `Delete Patches`.
6. Missing, already-desired, obsolete, anniversary, unknown, and unsupported-current files log the reference skip messages and never call downloader/applier.
7. Read-only/current-backup collision paths are explicit in the plan and fake mutation log.
8. Worker events for log/progress/completion round-trip through `RecordingEventSink` without Slint types.

This proof gives the planner/executors permission to make the UI live without risking accidental mutation semantics.

## Risks and Watch-outs

- **xdelta crate risk**: `xdelta3` is plausible but unproven. `oxidelta` currently requires Rust 1.90 and conflicts with this crate's `rust-version = 1.85.0`. Keep `DeltaApplier` injectable and delay dependency commitment until a compile/tiny-fixture proof passes.
- **Filesystem trait churn**: do not add write methods to the existing `Filesystem` trait unless the executor is ready to update every fake. A separate mutation trait keeps S03-S08 read-only tests stable.
- **Path safety**: all game-file targets are constants, but still validate joins. Reject absolute paths, `..`, prefix escapes, and symlink/canonicalization ambiguity where possible. Fail closed before mutation.
- **Reference vs safety divergence**: inline plan/confirmation is intentional. Also, the Rust port should fix the reference button re-enable race while running.
- **Delta storage location**: reference downloads patch files by name into the process current directory. For the Rust port, isolate this behind `DeltaStore` so the planner can choose strict CWD parity or a safer app-cache directory while preserving URL/file-name semantics and `Delete Patches` cleanup.
- **Slint modal behavior**: separate windows are likely straightforward; true Tk-style grab/transient modality and close interception need early `cargo check` proof. If full OS modality is not available, document the practical difference and at least block the Downgrader close/Escape while running.
- **Settings timing**: because first click now builds a plan instead of mutating, decide and test that downgrader settings are persisted at first workflow start. If save fails, prefer reverting options and not planning/executing with unpersisted preferences.
- **Overview reuse trap**: Overview binary facts are useful for main-tab display, but S09 must classify with Downgrader CRC maps and include `Archive2Interop.dll`.
- **Non-Windows builds**: process/version registry adapters already return UnsupportedPlatform off Windows. Downgrader domain/service tests should remain cross-platform with fakes; real discovery/status workers should show safe failure off unsupported platforms.

## Verification Plan

Targeted tests to add/run during S09:

- `cargo test downgrader_domain`
- `cargo test downgrader_service`
- `cargo test downgrader_controller`
- `cargo test downgrader_worker_payload`
- `cargo test s09_downgrader_slint_contract`
- `cargo test s09_downgrader_runtime_wiring`
- Existing regression groups that S09 touches:
  - `cargo test settings`
  - `cargo test overview`
  - `cargo test tools`
  - `cargo test worker`

Closeout checks required by project instructions:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`

If a real xdelta dependency is added, include a tiny deterministic fixture test that applies a known delta and verifies output bytes before enabling production execution.

## Sources

- Reference files read: `CMT/src/downgrader.py`, `CMT/src/enums.py`, `CMT/src/globals.py`, `CMT/src/modal_window.py`, `CMT/src/app_settings.py`, `CMT/src/tabs/_overview.py`, `CMT/src/tabs/_tools.py`, `CMT/src/utils.py`.
- Rust files read: `Cargo.toml`, `src/domain/settings.rs`, `src/domain/discovery.rs`, `src/domain/overview.rs`, `src/domain/tools.rs`, `src/platform/mod.rs`, `src/platform/filesystem.rs`, `src/platform/process.rs`, `src/app/settings_controller.rs`, `src/app/overview_controller.rs`, `src/app/tools_controller.rs`, `src/services/tools.rs`, `src/services/overview.rs`, `src/services/overview_collector.rs`, `src/services/update.rs`, `src/workers/events.rs`, `src/workers/handoff.rs`, `src/workers/mod.rs`, `src/main.rs`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`.
- Tool outputs: `.gsd/exec/1fc305dc-6df0-4930-aa8f-7bb790ba66b2.stdout` for skill search, `.gsd/exec/41d53117-9c3d-4905-a50f-ec8a451bf666.stdout` and `.gsd/exec/afb41de0-8c97-48ba-b067-26d2c7718053.stdout` for xdelta crate search/info, `.gsd/exec/0928996d-64e6-4e83-8d6d-59126c1e487e.stdout` for reference downgrader constants/log extraction.