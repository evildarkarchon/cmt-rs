---
id: S10
milestone: M001
status: ready
---

# S10: Archive Patcher Workflow — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver a faithful but safer Archive Patcher workflow that lets users patch eligible Fallout 4 BA2 archive headers between `v1 (OG)` and `v8 (NG)` through a validated, confirm-before-write, restore-capable modal flow.

## Why this Slice

Archive Patcher is one of the remaining mutation-heavy Toolkit Utilities and should be ported only after the read-only discovery, Overview archive classification, Tools entry points, and Downgrade Manager safety patterns are available. This slice replaces the currently deferred Overview and Tools Archive Patcher buttons with a real workflow while preserving S04/S05 safety boundaries and proving the archive write-plan pattern needed for trustworthy user-file mutations.

## Scope

### In Scope

- Open Archive Patcher from both Overview `Archive Patcher...` and Tools `Archive Patcher`.
- Present a reference-shaped modal/dialog titled `Archive Patcher`, closely mirroring the Python patcher structure:
  - `Desired Version` group with `v1 (OG)` and `v8 (NG)` radio choices.
  - Default desired version is `v1 (OG)`.
  - `Patch All` action.
  - `About` action using the reference `Bethesda Archive (BA2) Formats & Versions` title/text.
  - `Name Filter:` entry with case-insensitive substring filtering.
  - Candidate file list/tree.
  - Bottom log/status surface.
- Use the current Overview-enabled archive sets as the candidate source:
  - When targeting `v1 (OG)`, candidates come from the discovered enabled `v7/v8 (NG)` archive set.
  - When targeting `v8 (NG)`, candidates come from the discovered enabled `v1 (OG)` archive set.
  - The candidate set updates when desired version or name filter changes.
- Keep reference patch labels/messages where practical, including:
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
- Before writes, build a fail-closed write plan that verifies target path, `BTDX` magic, current archive version, desired target version, and restore-point feasibility.
- Require explicit user confirmation after showing the write plan and before any archive header changes.
- Create low-disk restore points by saving the original BA2 header/version bytes in an app-owned restore manifest, not by copying full BA2 archives by default.
- Include a simple user-facing restore action for the most recent patch run, using the saved header restore manifest.
- Execute patch and restore work off the Slint UI thread.
- While patching/restoring runs, keep the modal open, disable write controls, stream log/progress updates, and refresh Overview/archive candidates when the operation completes.
- Continue per file on validation, backup, or write failure: skip failed entries, patch valid entries, and report per-file messages plus final success/failure counts.
- If game/Data discovery or Overview archive data is missing, open a safe empty/error modal with write controls disabled and a clear message to refresh/fix discovery.

### Out of Scope

- Patching every `.ba2` under `Data` regardless of Overview-enabled state.
- Manual folder picking or creating an Archive Patcher-specific path authority outside shared discovery.
- Checked multi-selection as the primary patch scope; `Patch All` operates on the filtered reference candidate set.
- Exposing `v7` as a target version.
- Auto-selecting or hiding desired version based on detected game version.
- Full archive file copies by default.
- Full historical backup/restore manager or browsing arbitrary old restore manifests.
- Cancellation support during the initial parity slice; the reference patcher has no cancel button.
- New archive repair features beyond the reference header-version patch/restore behavior.
- New scanner/archive diagnostics not required to make Archive Patcher safe and usable.

## Constraints

- `CMT/` remains read-only; inspect it for reference behavior but do not edit or generate files under it.
- Preserve reference labels, control order, default desired version, candidate source semantics, and user-facing log messages unless a safety improvement is explicitly documented.
- Archive writes must fail closed: if magic bytes, current version, target version, restore-point creation, path ownership, or write permission is unclear, skip/report the file instead of writing.
- The workflow must protect user files without copying multi-GB archives by default; the agreed protection model is a header restore manifest plus a simple last-run restore action.
- Long-running or filesystem-mutating work must run through worker/event-loop handoff patterns and must not block the Slint UI thread.
- Slint should present state and emit callbacks; patch planning, validation, filesystem writes, restore manifests, and result aggregation belong in Rust domain/service/worker code.
- Candidate archives must come from the typed discovery/Overview archive state, not from ad hoc UI-side directory scans.
- After patch/restore completion, Overview archive diagnostics should refresh so counts and candidate lists reflect the new header versions.
- Unknown, missing, unreadable, already-patched, and partial-failure cases are expected user states and should be visible/logged rather than panics.

## Integration Points

### Consumes

- `CMT/src/patcher/_archives.py` — Source of truth for Archive Patcher desired-version choices, filter behavior, candidate source, byte-patching semantics, and log messages.
- `CMT/src/patcher/_base.py` — Source of truth for the modal layout shape, `Patch All`, `About`, file tree, logger, close-while-processing behavior, and refresh-after-patch pattern.
- `CMT/src/globals.py` — Source of truth for patcher window size constants, `PATCHER_FILTER_OG`, `PATCHER_FILTER_NG`, `ABOUT_ARCHIVES_TITLE`, and `ABOUT_ARCHIVES`.
- `CMT/src/tabs/_overview.py` — Source of truth for the Overview `Archive Patcher...` entry point and the enabled archive sets that feed the patcher.
- `CMT/src/tabs/_tools.py` — Source of truth for the Tools `Archive Patcher` entry point under `Toolkit Utilities`.
- `src/domain/discovery.rs` — Existing archive record/version/domain types to reuse or extend for patch-plan inputs.
- `src/services/overview_collector.rs` — Existing BA2 header classification logic and tests that should align with patcher validation.
- `src/domain/overview.rs` and `src/app/overview_controller.rs` — Existing Overview deferred action and archive-state surfaces currently used by the disabled `Archive Patcher...` control.
- `src/domain/tools.rs`, `src/services/tools.rs`, and `src/app/tools_controller.rs` — Existing Tools action ids, deferred utility metadata, and safe action patterns to replace with live Archive Patcher opening.
- `src/platform/filesystem.rs` — Filesystem adapter boundary to extend for safe byte writes and restore-manifest persistence.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — Worker event and Slint event-loop handoff patterns for progress, completion, failure, and UI-safe refresh.
- `ui/overview_tab.slint`, `ui/tools_tab.slint`, and `ui/main.slint` — Existing disabled/deferred Archive Patcher callbacks/properties to replace with live modal-opening wiring.

### Produces

- Archive Patcher domain/service contracts — Typed desired-version, candidate, write-plan, restore-point, per-file result, summary, and error models.
- Archive Patcher Slint modal/dialog component — Reference-shaped UI for desired version, name filter, candidate list, About, confirmation, live log/progress, Patch All, and Restore last run.
- Archive write/restore worker flow — Off-UI-thread patch and restore execution with progress events, final summary, and Overview refresh trigger.
- Header restore manifest — App-owned record of the most recent patch run's original BA2 header/version bytes, used by the simple restore action.
- Platform filesystem write extensions/fakes — Testable byte-write and manifest persistence seams that do not touch real game files in tests.
- Fixture tests — Byte-level tests for BTDX validation, v1/v7/v8 transitions, already-patched skip, unknown version skip, bad magic skip, missing file, permission failure, manifest creation, restore success, restore skip/failure, filtered candidates, and partial-success summaries.
- Updated Overview/Tools wiring — The existing Archive Patcher controls become live entry points instead of deferred disabled actions.

## Open Questions

- Exact restore manifest storage path, file name, and retention policy — Current thinking: store a single most-recent Archive Patcher manifest in the app-owned data/config area rather than inside `Data`, so game/scanner behavior is not affected and the UI can offer a simple `Restore` without a full backup manager.
- Exact restore button label and placement — Current thinking: add a small `Restore Previous Headers`/`Restore Last Run` action in the modal near `Patch All`, enabled only when a valid manifest exists.
- Byte-write granularity — Current thinking: validate the BA2 version as the little-endian header field, but preserve reference-compatible behavior by changing only known `v1`/`v7`/`v8` header values to the chosen `v1`/`v8` target and failing closed if surrounding version bytes are unexpected.
- Restore if files moved or changed after patching — Current thinking: skip and log the affected file if path, magic, or current header no longer matches the manifest's expected post-patch state.
- Whether to surface disk/free-space checks for header restore manifests — Current thinking: manifest size is tiny, so ordinary manifest write failure handling is sufficient unless implementation uncovers a platform-specific issue.
