---
id: S10
milestone: M001
status: draft
---

# S10: Archive Patcher Workflow — Context Draft

## Interview Signals So Far

- Opening UX: use a reference-shaped modal/dialog opened from both Overview `Archive Patcher...` and Tools `Archive Patcher`.
- Write safety: show a verified fail-closed write plan, require confirmation, and create backups/restore points before changing headers.
- Candidate scope: patch only the current Overview-enabled OG/NG archive sets, matching the reference source (`archives_ng` when patching to v1, `archives_og` when patching to v8).
- Backup kind: use a header restore manifest/restore point for the original BA2 header/version bytes rather than copying full archives by default.
- Partial failures: continue per file, skip failed/invalid entries, and show per-file messages plus final success/failure counts.
- Run feedback: execute off the UI thread with the modal open, write controls disabled, live log/progress updates, refreshed candidate list, and Overview refresh on completion.
