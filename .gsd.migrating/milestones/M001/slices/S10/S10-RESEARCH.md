# S10 Research: Archive Patcher Workflow

## Summary

S10 is a medium-risk mutation workflow, not a pure UI slice. The Python reference `ArchivePatcher` is small and direct: it opens a `700x600` modal titled `Archive Patcher`, defaults `Desired Version` to `v1 (OG)`, shows a `Patch All` and `About` button, filters candidates by case-folded basename substring, lists candidate files sorted by path/name, and patches BA2 header version bytes in place. The Rust port should preserve this visible workflow but must implement the slice safety improvements: read-only plan first, explicit confirmation, digest-bound revalidation, header restore manifest, per-file fail-closed writes, live log/progress, and Overview refresh after completion.

The strongest local pattern to reuse is S09 Downgrader: Slint-free domain contract, adapter-backed service, controller lifecycle with request IDs and stale-event rejection, `WorkerEvent` payloads, modal projection helpers in `src/main.rs`, and post-run Overview refresh. S10 should not introduce a separate architecture. It should add an Archive Patcher sibling to the Downgrader layers.

## Requirements and Scope Notes

No advanced/validated requirements were preloaded for this slice.

Slice-owned behaviors from context:

- Open Archive Patcher from Overview `Archive Patcher...` and Tools `Archive Patcher`.
- Candidate source is the current Overview/discovery enabled archive state, not an ad hoc UI scan:
  - Target `v1 (OG)` patches enabled `v7/v8 (NG)` archives.
  - Target `v8 (NG)` patches enabled `v1 (OG)` archives.
- Preserve reference labels/messages where practical, including `Showing N files to be patched.`, `Nothing to do!`, `Unrecognized format: <file>`, `Unrecognized version [<hex>]: <file>`, `Patched to v<target>: <file>`, and final `Patching complete. N Successful, M Failed.`.
- Mutation must fail closed with validation, confirmation, restore manifest, worker execution, log streaming, disabled controls while running, and Overview refresh after completion.

## Skills Discovered

- `write-docs`: requested by auto-mode, but no callable `Skill` tool was exposed in this tool namespace. I used the documented output rule anyway: write for a fresh planner, prioritize file map, seams, first proof, and verification.
- Rust: installed skill `rust-async-patterns` is already available and relevant because S10 uses background workers/Tokio/Slint handoff. No new install needed.
- Slint: no installed Slint-specific skill was present. `npx skills find "Slint"` returned unrelated ESLint/accessibility results, so no skill was installed.
- Additional `npx skills find "Rust"` results included external Rust best-practice/testing skills, but the project already has Rust-specific local instructions and an installed Rust async skill; no global skill was added.

## Reference Behavior

### Files

- `CMT/src/patcher/_archives.py`
  - `ArchivePatcher.__init__` sets `desired_version = IntVar(value=ArchiveVersion.OG)` and passes title `Archive Patcher`.
  - `filter_text` displays `PATCHER_FILTER_NG` when targeting OG, otherwise `PATCHER_FILTER_OG`.
  - `files_to_patch` selects `cmc.game.archives_ng` for target OG and `cmc.game.archives_og` for target NG, then filters on `self.name_filter in file.name.casefold()`.
  - `build_gui_secondary` creates `Desired Version` radios `v1 (OG)` and `v8 (NG)`, the dynamic filter explanation label, `Name Filter:` label, and an entry that clears log and repopulates on key release.
  - `patch_files` opens each candidate `r+b`, validates first four bytes are `BTDX`, reads one byte at offset `4`, and writes one target byte (`0x01` or `0x08`). It catches `FileNotFoundError`, `PermissionError`, and generic `OSError` with specific log messages.
- `CMT/src/patcher/_base.py`
  - Modal window size uses `WINDOW_WIDTH_PATCHER = 700`, `WINDOW_HEIGHT_PATCHER = 600`.
  - Primary layout order: top frame, middle frame, bottom log; `Patch All` and `About` are packed top-right; tree is middle; logger is bottom; secondary controls are added by subclass.
  - `_patch_wrapper` sets `processing_data = True`, runs `patch_files`, refreshes Overview, repopulates tree, then clears processing.
  - `populate_tree` sorts `files_to_patch`, inserts basenames, and logs `Showing {len(self.files_to_patch)} files to be patched.`.
- `CMT/src/globals.py`
  - `PATCHER_FILTER_OG = "Showing all v1\n(Includes Base Game/DLC/CC)"`.
  - `PATCHER_FILTER_NG = "Showing all v7 & v8\n(Includes Base Game/DLC/CC)"`.
  - About title: `Bethesda Archive (BA2) Formats & Versions`.
  - About body explains BA2 formats `GNRL`/`DX10`, versions `v1`, `v7/8`, and why the version byte can be patched.
- `CMT/src/enums.py`
  - `Magic.BTDX = b"BTDX"`.
  - `ArchiveVersion.OG = 1`, `ArchiveVersion.NG7 = 7`, `ArchiveVersion.NG = 8`.
  - `LogType.Info = "info"`, `Good = "good"`, `Bad = "bad"`.
- `CMT/src/tabs/_overview.py`
  - Overview Archive panel button text is `Archive Patcher...` and command is `ArchivePatcher(self.cmc.root, self.cmc)`.
  - Enabled archive classification reads 12-byte BA2 header; `head[:4] == BTDX`, `head[4]` version, `head[8:]` format; enabled `v7/v8` go to `game.archives_ng`, enabled `v1` go to `game.archives_og`.
- `CMT/src/tabs/_tools.py`
  - Tools `Toolkit Utilities` contains `Downgrade Manager` followed by `Archive Patcher`; Archive Patcher opens `ArchivePatcher(self.cmc.root, self.cmc)`.

### Reference messages to freeze in Rust domain

- `Showing all v1\n(Includes Base Game/DLC/CC)`.
- `Showing all v7 & v8\n(Includes Base Game/DLC/CC)`.
- `Showing N files to be patched.`.
- `Nothing to do!`.
- `Unrecognized format: <file>`.
- `Skipping already-patched archive: <file>`.
- `Unrecognized version [<hex>]: <file>`.
- `Failed patching (File Not Found): <file>`.
- `Failed patching (Permissions/In-Use): <file>`.
- `Failed patching (Unknown OS Error): <file>`.
- `Patched to v<target>: <file>`.
- `Patching complete. N Successful, M Failed.`.

## Existing Rust Implementation Landscape

### Domain and services already available

- `src/domain/discovery.rs`
  - Has `ArchiveFormat::{General, DirectX10, Unknown}`, `ArchiveVersion::{OldGen, NextGen7, NextGen8, Unknown(u32)}`, and `ArchiveRecord { path, format, version, enabled, readable }`.
  - `ArchiveVersion::as_header_value()` already returns `1`, `7`, `8`, or unknown value.
  - Use these types for Archive Patcher candidates; avoid a duplicate archive version enum unless a small `ArchivePatcherTarget` enum is needed for UI target semantics.
- `src/services/overview_collector.rs`
  - Defines BA2 header length `12` and reads headers through `Filesystem::read_prefix`.
  - Current Rust collector interprets the version field as `u32::from_le_bytes([header[4], header[5], header[6], header[7]])`, then recognizes `1`, `7`, and `8`. This is safer/more correct than the Python single-byte read and should be the patcher validator contract.
  - Candidate freshness can be rebuilt by running discovery + `OverviewCollector`, as Overview and Scanner workers already do.
- `src/services/overview.rs`
  - `OverviewDiagnostics::build` consumes injected `ArchiveRecord` facts; it does not read files.
  - `append_archive_problems` treats enabled/unreadable archives and unknown versions/formats as Overview problems, and old-gen/down-grade installs reject NG archive versions.
- `src/domain/overview.rs`
  - `ArchivePanelSummary::from_records` counts records where `record.enabled || !record.readable`.
  - The existing Overview action label constant is `ACTION_ARCHIVE_PATCHER_LABEL = "Archive Patcher..."`.
- `src/services/scanner.rs`
  - Already consumes `with_enabled_archives(&collected.archives)`. This reinforces using `ArchiveRecord.enabled` as the shared enabled-archive source.

### Existing deferred entry points to replace

- `ui/main.slint`
  - Imports/exports `DowngraderWindow` only.
  - Properties currently set `overview-archive-patcher-enabled: false` and status `Deferred until the Archive Patcher workflow is ported.`.
  - Callback already exists: `overview-open-archive-patcher-requested()`.
- `ui/overview_tab.slint`
  - `overview-archive-patcher-enabled` defaults false, status deferred.
  - `open-archive-patcher-requested` callback is already wired from the Archive panel.
- `ui/tools_tab.slint`
  - `Archive Patcher` row uses action id `tools.archive_patcher`, `button-enabled: false`, helper `Deferred until S10 Archive Patcher workflow is ported.`.
  - It forwards to `tool-action-requested(action_id)` only if enabled; S10 should make it a live internal utility path like Downgrader.
- `src/main.rs`
  - `bind_overview_callbacks` currently handles `on_overview_open_archive_patcher_requested` by applying an Overview error: `Archive Patcher is reserved for a later port phase.`.
  - `bind_tools_callbacks` special-cases only `ToolActionId::DowngradeManager`; Archive Patcher falls to `request_tools_action` and is rejected as deferred.
  - Tests at around `s09_downgrader_runtime_wiring_open_entrypoints_and_archive_patcher_deferred` and `s09_downgrader_slint_contract_entrypoints_forward_downgrader_but_keep_archive_patcher_deferred` intentionally assert the current deferred state. S10 must update these or add S10-specific replacement tests.
- `src/domain/tools.rs`
  - `ToolActionId::ArchivePatcher` exists.
  - `TOOLKIT_UTILITIES` currently marks Archive Patcher as `ToolEntryAction::DeferredUtility` with status `Archive Patcher is not available in this Rust port yet.`. Change to `InternalUtility` once runtime routing exists.
- `src/services/tools.rs`
  - `tools_action_for_id` and `ToolsActionService` already route `InternalUtility` without desktop handoff. After changing `domain/tools.rs`, Archive Patcher will parse as `ToolsActionKind::InternalUtility(ToolActionId::ArchivePatcher)`.
- `src/app/tools_controller.rs`
  - Default disabled status is `Archive Patcher is deferred until S10.`; this can be removed/changed once Tools row is live.

### S09 patterns to copy for S10

- `src/domain/downgrader.rs`
  - Pattern: reference strings and UI row/domain payload types live in a Slint-free domain module.
  - S10 should add `src/domain/archive_patcher.rs` for target labels, modal dimensions, filter/about text, log levels, candidate/log/plan/restore payloads, target parse/display helpers, and message formatting.
- `src/services/downgrader.rs`
  - Pattern: `DowngraderService<'a, F: Filesystem + ?Sized>` performs read-only status/plan work over `Filesystem`; confirmed mutation is available only on `impl<F: Filesystem + WritableFilesystem>`.
  - Pattern: preview plan has `stable_digest()` excluding request IDs; confirmed execution re-previews, compares digest, then mutates.
  - Pattern: detailed diagnostics stay in service results/tracing; modal rows use safe/reference-style messages.
- `src/app/downgrader_controller.rs`
  - Pattern: closed/loading/ready/planning/plan-ready/running/completed/safe-error lifecycle, monotonic request ids, stale-event rejection, close blocked while running, and pending status refresh after run.
  - S10 should create `src/app/archive_patcher_controller.rs` rather than overloading Tools/Overview controllers.
- `src/workers/events.rs`
  - Pattern: task-specific `DowngraderWorkerPayload` carries status, plan, progress, log, completion, and safe failure. S10 needs analogous `ArchivePatcherWorkerPayload` or a generalized patcher payload if desired; sibling is simpler and matches current style.
- `src/main.rs`
  - Pattern: create modal window once, bind callbacks/sink, schedule blocking workers through `WorkerRuntime::spawn_blocking_task`, project controller state into Slint model rows, refresh Overview after run completion using current shared settings snapshot.
- `ui/downgrader_window.slint`
  - Pattern: modal as separate exported component, exported row structs, custom radio/check components, About overlay, plan panel, log panel, progress/blocked-close UI.
  - S10 should add `ui/archive_patcher_window.slint` and import/export it from `ui/main.slint`.

### Filesystem seam status

- `src/platform/filesystem.rs`
  - `Filesystem` supports `metadata`, `symlink_metadata`, `canonicalize_path`, `read_bytes`, `read_prefix`, `read_to_string`, `read_dir`, and `walk_dir`.
  - `WritableFilesystem` supports whole-file writes/replacement/copy/rename/remove only. It does **not** yet support small offset writes.
  - For S10, add a focused mutation method such as `write_bytes_at(&self, path: &Path, offset: u64, bytes: &[u8]) -> PlatformResult<()>`, or implement service mutation by reading the full file and using `replace_file_bytes`. Because BA2 files can be multi-GB and slice scope calls for low-disk header patching, prefer adding `write_bytes_at` and a companion fake implementation for tests.
  - Keep write trait separate from read trait so preview code cannot mutate accidentally.
- `src/platform/mod.rs`
  - Has `PlatformOperation::WriteFile`; no new operation is required unless planners want `PersistManifest` separated. Existing `WriteFile` is enough for restore manifest and offset writes.

### Restore manifest storage

Current settings persistence is intentionally current-directory `settings.json` via `src/platform/settings_store.rs`; `directories` is a dependency but not used for settings paths. Slice context wants an app-owned restore manifest, not a file inside `Data`. Practical S10 options:

1. **Current-directory app-owned path**: `archive-patcher-restore.json` or `.cmt-rs/archive-patcher-restore.json` next to `settings.json`.
   - Fits existing settings-store behavior and is easiest to test.
   - Less OS-correct than ProjectDirs, but consistent with current app-local settings.
2. **ProjectDirs app data/config path** using the already-present `directories` crate.
   - More aligned with slice wording “app-owned data/config area”.
   - Adds a new path policy when settings currently do not use ProjectDirs.

Recommendation: use a small `ArchivePatcherManifestStore` seam over `Filesystem + WritableFilesystem` with injectable manifest path. Production can choose a current-directory `archive-patcher-restore.json` initially for consistency unless the planner wants to record a decision to adopt ProjectDirs for this workflow. The manifest path must be outside the game `Data` tree by construction and covered by tests.

## Recommended Architecture

Add a dedicated Archive Patcher vertical stack:

1. `src/domain/archive_patcher.rs`
   - Constants: modal title/dimensions, labels, about copy, filter text, status messages.
   - Types:
     - `ArchivePatcherTarget::{OldGen, NextGen}` mapping to desired header values `1` and `8` and UI values (`old_gen`, `next_gen`).
     - `ArchivePatcherCandidate { path, display_name, current_version, format, enabled }`.
     - `ArchivePatcherLogLevel::{Info, Good, Bad}` matching reference strings.
     - `ArchivePatcherLogRow` with constructors for reference messages.
     - `ArchivePatcherPlanRow` and `ArchivePatcherPreviewPlan` with `stable_digest()`.
     - `ArchiveHeaderRestoreEntry` / `ArchiveHeaderRestoreManifest` capturing original first 12 bytes, expected post-patch header bytes, target version, timestamp/run id if needed.
     - `ArchivePatcherExecutionResult` and `ArchivePatcherRestoreResult` summaries.
   - Keep all strings here so UI/tests do not duplicate reference copy.

2. `src/services/archive_patcher.rs`
   - Read-only service over `Filesystem`:
     - Build candidate snapshot from `ArchiveRecord` inputs (filter by `enabled && readable` and target source version).
     - Apply name filter via basename `.to_string_lossy().casefold-ish` (`to_lowercase()` is acceptable; for exact Unicode casefold, Rust std has no full casefold, but archive names are expected ASCII-ish).
     - Build preview plan from candidate paths by re-reading header prefix and validating canonical containment/data root if provided.
     - Compute plan digest over target, filter, sorted path list, header bytes/current versions, and manifest feasibility.
   - Confirmed service over `Filesystem + WritableFilesystem`:
     - Rebuild/revalidate plan and compare digest before any writes.
     - Persist restore manifest before patching. If manifest write fails, abort or mark rows failed before header writes. Slice says restore-point creation uncertainty must skip/report rather than write; simplest fail-closed behavior is no patching until manifest is saved.
     - Patch each file by validating `BTDX`, known current version, not already target, still contained under Data/game root, then write only version bytes at offset `4..8` (recommended) to the little-endian target `u32`.
     - Continue per file on validation or write failures and emit reference log rows.
   - Restore service:
     - Load most recent manifest.
     - For each entry, verify path still exists, magic still `BTDX`, current header matches expected post-patch header, then restore original header bytes (or at least original version field plus magic/format sanity).
     - Skip/log moved/changed/unreadable entries.

3. `src/app/archive_patcher_controller.rs`
   - Lifecycle similar to Downgrader but with target/filter candidate updates:
     - `Closed`, `LoadingCandidates`, `Ready`, `Planning`, `PlanReady`, `RunningPatch`, `RunningRestore`, `Completed`, `SafeError`.
   - State: current target default `OldGen`, name filter, candidates, plan, log rows, progress, manifest availability, safe error, active request IDs.
   - UI intents:
     - `open(settings/installation/candidate facts)` returns candidate-load/status request.
     - `set_target_from_ui_value`, `set_name_filter` recompute visible candidates and clear plan/log like reference.
     - `request_patch_all` first click prepares plan, second confirms run (or use explicit confirm callback like Downgrader).
     - `request_restore_last_run` returns restore worker only when manifest exists and not running.
     - `request_about`, `request_close`/close blocking can stay runtime/UI if simpler; controller should expose `close_enabled`.

4. `src/workers/events.rs`
   - Add `ArchivePatcherWorkerStage::{Candidates, Plan, Run, Restore}` and `ArchivePatcherWorkerPayload::{CandidatesLoaded, PlanReady, LogRow, Progress, PatchCompleted, RestoreCompleted, SafeFailure}`.
   - Add constructors and request-id/stage helpers; update `WorkerPayload` enum.

5. `ui/archive_patcher_window.slint`
   - Separate exported component.
   - Suggested exported structs:
     - `ArchivePatcherCandidateUiRow { name: string, version: string, format: string, path: string, severity: string }`.
     - `ArchivePatcherPlanUiRow { name: string, action: string, detail: string, severity: string }`.
     - `ArchivePatcherLogUiRow { level: string, message: string }`.
   - Layout should mirror reference:
     - Top row: `Desired Version` group with `v1 (OG)` then `v8 (NG)` radios; dynamic filter explanation label; right-side `Patch All`, `Restore Last Run` (new safety action), `About`.
     - Middle: `Name Filter:` label + entry, candidate list/tree in scroll view.
     - Bottom: log/status surface and progress/blocked-close text.
     - About overlay title/body from domain constants.
     - Plan confirmation panel can be inserted above log, matching S09 safety pattern while preserving reference controls.

6. `src/main.rs`
   - Instantiate `ArchivePatcherWindow`, `ArchivePatcherController`, and sink in `main` alongside Downgrader.
   - Wire Overview and Tools entrypoints to `request_open_archive_patcher_modal` instead of deferred error.
   - For candidate load, production worker should run discovery + `OverviewCollector` like `build_overview_snapshot`, then pass collected `ArchiveRecord`s and installation/data path to the archive patcher service/controller.
   - After patch/restore completion, schedule Overview refresh using the same helper pattern as Downgrader completion.

## Natural Work Units for Planner

1. **Domain contract first**
   - Add `src/domain/archive_patcher.rs` and `pub mod archive_patcher`.
   - Freeze labels/messages/about copy, target enum, log rows, candidate rows, plan/manifest/result payloads.
   - Tests: message formatting, default target, target source-version semantics, filter text, digest changes when header/path/target changes.

2. **Filesystem write seam**
   - Extend `WritableFilesystem` with `write_bytes_at` (or equivalent header write method).
   - Implement in `RealFilesystem` with `OpenOptions::new().write(true).open`, `seek`, `write_all`, and sync where practical.
   - Extend service-local fakes in new tests; existing test fakes in other modules may compile-break because trait methods need implementations. Use a default trait method only if it can remain safe; otherwise update fakes explicitly.
   - Tests: writes only expected offset bytes; maps not-found/permission/io to typed `PlatformErrorKind`.

3. **Service planning/execution**
   - Add `src/services/archive_patcher.rs` and module export.
   - Implement candidate selection from `ArchiveRecord`s, read-only plan, digest, manifest persist/load, confirmed patch, restore.
   - Tests should be sandbox/fake-backed and cover the slice fixture list: valid v1/v7/v8 transitions, already-patched skip, unknown version, bad magic, missing file, permission failure, manifest write failure, partial success, restore success and restore changed-file skip.

4. **Controller and worker payloads**
   - Add `src/app/archive_patcher_controller.rs` and module export.
   - Add worker payload variants in `src/workers/events.rs`.
   - Tests: open/load, target/filter recompute, first Patch All -> plan, second -> run with digest, stale events ignored, close blocked while patch/restore running, spawn failures safe, restore enabled only with manifest.

5. **Slint modal**
   - Add `ui/archive_patcher_window.slint`; import/export from `ui/main.slint`.
   - Update source-contract tests in `src/main.rs` or add S10 tests for labels/control order/properties/callbacks.
   - Keep UI declarative only; no domain decisions in `.slint`.

6. **Runtime wiring and entrypoints**
   - Update `src/domain/tools.rs` Archive Patcher to `InternalUtility`.
   - Update `ui/tools_tab.slint` button enabled/helper/callback, `ui/overview_tab.slint` default enabled/status, `ui/main.slint` imports/properties/callbacks as needed.
   - Update `src/main.rs` to bind open callbacks, schedule workers, project modal state, and refresh Overview after patch/restore completion.
   - Replace S09 deferred tests with live-entrypoint tests.

## First Proof / Highest-Risk Spike

The first proof should be the service-level byte/manifest contract, before Slint wiring:

- Given fake enabled `ArchiveRecord`s and fake files with 12-byte BA2 headers, `ArchivePatcherService::preview_plan` selects the correct target candidates, validates `BTDX`, validates current version, creates a digest, and reports plan rows without mutation.
- Given a confirmed digest and fake writable filesystem, `execute_confirmed` writes only the little-endian version field bytes at offset `4..8`, persists a restore manifest before writing, continues after one file failure, emits reference-style log rows, and returns `Patching complete. N Successful, M Failed.`.
- Given the manifest, `restore_last_run` restores the original header only when the current header still matches the expected post-patch state.

This proof unblocks UI/controller work and catches the dangerous ambiguity: Python writes one byte, but Rust collector reads a 32-bit little-endian field. The S10 implementation should patch the 32-bit field (`1u32.to_le_bytes()` / `8u32.to_le_bytes()`) while preserving the reference user-facing `v1`/`v8` messages.

## Risks and Constraints

- **Version byte width mismatch**: Python reads/writes one byte at offset 4; Rust overview parses four bytes as little-endian `u32`. To keep Overview correct after patching, S10 should write the full four-byte version field while recognizing reference inputs where upper bytes are zero. Tests must pin this.
- **Candidate freshness**: `OverviewSnapshot` currently does not carry raw `ArchiveRecord`s; only panel counts/actions. Do not derive candidates from UI rows. Runtime should recollect `OverviewCollectedFacts` for the modal or introduce a shared app-level archive facts cache. Recollection is simpler and consistent with Scanner/Overview workers.
- **Manifest before mutation**: The restore manifest must be saved before header writes. If saving fails, skip/abort patching rather than writing un-restorable changes.
- **Containment and links**: Reuse S09 containment thinking. Validate game/data root, canonicalize target path, reject path traversal/symlink/reparse ambiguity where exposed by `Filesystem::symlink_metadata`/`canonicalize_path`.
- **Large files**: Avoid whole-file read/replace for BA2 patching. Add offset write support to avoid multi-GB memory/disk usage.
- **Partial failures are expected**: Missing, permission denied/in-use, malformed magic/version, unknown OS errors, and already-patched files should become log rows and counts, not panics.
- **Close while running**: Reference `ModalWindow` blocks close while `processing_data`; S10 should expose close-blocked state like S09.
- **Clippy noise**: S09 closeout noted existing warning-level clippy output even with exit 0. S10 should keep new code clean but may still see unrelated warnings.

## Verification Plan

Targeted tests to add/run during implementation:

- `cargo test archive_patcher_domain --quiet`
- `cargo test archive_patcher_service --quiet`
- `cargo test archive_patcher_controller --quiet`
- `cargo test archive_patcher_worker_payload --quiet`
- `cargo test s10_archive_patcher_slint_contract --quiet`
- `cargo test s10_archive_patcher_runtime_wiring --quiet`
- Regression filters:
  - `cargo test overview --quiet`
  - `cargo test tools --quiet`
  - `cargo test worker --quiet`
  - `cargo test downgrader --quiet` if shared worker/filesystem patterns are touched.
- Required closeout gates:
  - `cargo fmt --check`
  - `cargo check --quiet`
  - `cargo test --quiet`
  - `cargo clippy --all-targets --all-features --quiet`

## Key Files for Planner

- Add: `src/domain/archive_patcher.rs` — reference strings, target/candidate/plan/log/manifest/result types.
- Add: `src/services/archive_patcher.rs` — candidate filtering, read-only plan, digest, confirmed patch, restore.
- Add: `src/app/archive_patcher_controller.rs` — modal lifecycle, target/filter state, request IDs, stale-event handling.
- Add: `ui/archive_patcher_window.slint` — reference-shaped modal.
- Modify: `src/domain/mod.rs`, `src/services/mod.rs`, `src/app/mod.rs` — export new modules/types.
- Modify: `src/platform/filesystem.rs` — offset header write seam and real adapter implementation.
- Modify: `src/workers/events.rs` — Archive Patcher worker payloads.
- Modify: `src/domain/tools.rs`, `src/services/tools.rs` tests, `src/app/tools_controller.rs` if disabled status is removed — make Archive Patcher live internal utility.
- Modify: `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint` — modal export and live callbacks/properties.
- Modify: `src/main.rs` — instantiate modal/controller, bind callbacks/sink, schedule workers, project UI state, refresh Overview after patch/restore, update contract/runtime tests.

## Sources

- Local reference: `CMT/src/patcher/_archives.py`, `CMT/src/patcher/_base.py`, `CMT/src/globals.py`, `CMT/src/enums.py`, `CMT/src/tabs/_overview.py`, `CMT/src/tabs/_tools.py`.
- Local Rust: `src/domain/discovery.rs`, `src/domain/overview.rs`, `src/services/overview_collector.rs`, `src/services/overview.rs`, `src/domain/tools.rs`, `src/services/tools.rs`, `src/app/tools_controller.rs`, `src/domain/downgrader.rs`, `src/services/downgrader.rs`, `src/app/downgrader_controller.rs`, `src/workers/events.rs`, `src/platform/filesystem.rs`, `src/main.rs`, `ui/downgrader_window.slint`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`.
- Project memories: Overview data-flow and worker handoff patterns, S09 Downgrader fail-closed plan/digest/execution pattern, writable filesystem separation.
