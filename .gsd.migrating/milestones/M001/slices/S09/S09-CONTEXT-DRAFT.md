---
id: S09
milestone: M001
status: draft
---

# S09: Downgrade Manager Workflow — Context Draft

## Captured Decisions So Far

- Open Downgrade Manager from Overview or Tools as a faithful separate modal/window shaped like the reference `Downgrader` window rather than embedding it into the main tab.
- Preserve the reference visible structure: `Current Game`, `Current Creation Kit`, `Desired Version`, `Options`, `Patch\n All`, `About`, bottom log, and progress bar.
- Add a safety preview/confirmation before mutation even though the reference starts patching immediately. The preview should show the write/download/restore plan and backup/cleanup effects before any files change.
- After confirmation, keep batch behavior close to the reference: process each file independently, log reference-style per-file skip/success/failure messages, and avoid redesigning the patch queue flow.
- Support only the reference `Old-Gen` and `Next-Gen` targets in S09. Anniversary/AE is not a selectable target and remains skipped as unsupported when encountered.
- Patch the same reference file groups together: Fallout 4 runtime files and Creation Kit/Archive2 files, including `steam_api64.dll` matching behavior.
- Download xdelta patches from the CMT GitHub delta-patches release as-needed when a valid backup cannot restore the desired version; keep per-download progress and `Delete Patches` cleanup semantics.
- While patch/download work is running, keep the modal open, disable `Patch All`, and block close/Escape like the reference; refresh/redraw state when work completes.

## Likely In Scope

- Replace disabled/deferred Downgrade Manager entries in Overview and Tools with a live modal workflow.
- Respect persisted `downgrader_keep_backups` and `downgrader_delete_deltas` settings, including reference labels `Keep Backups` and `Delete Patches`.
- Preserve reference version labels/colors/statuses as practical in Slint: `Old-Gen`, `Next-Gen`, `Anniversary`, `Obsolete`, `Unknown`, and `Not Found`.
- Preserve reference About text for downgrading.
- Keep filesystem/download/patching work off the Slint UI thread with visible log/progress feedback.
- Use sandbox/fake-backed tests for backups, restore-from-backup, delta download/apply, cleanup, unsupported/not-found files, and read-only/locked failure handling.

## Likely Out of Scope

- Anniversary/AE target patching.
- Game-only mode that omits Creation Kit files.
- Archive Patcher behavior, BA2 mutation, or scanner auto-fix work.
- Background closing while patches continue.
- New product features such as live cancellation unless explicitly chosen later.

## Open Items

- Exact shape of the preview/confirmation surface: inline plan in the modal vs a second confirmation dialog vs changing `Patch All` into a two-step button.
- Whether to add a final explicit summary beyond the reference per-file log messages.
