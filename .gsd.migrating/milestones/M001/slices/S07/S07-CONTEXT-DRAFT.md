---
id: S07
milestone: M001
status: draft
---

# S07: Scanner Read Only Results — Context Draft

## Goal

Deliver the Scanner tab's read-only scan settings, explicit scan flow, progress, grouped results, details, and safe copy/open actions without enabling Auto-Fix writes.

## Confirmed Human Decisions So Far

- Layout: use embedded Slint panes rather than floating Tk-style side/detail windows, while preserving reference labels and spatial intent.
- Partial failures: continue scanning safe/available areas, show visible warning/error result rows for skipped or broken areas, and avoid silent skips or whole-scan failure where partial results are useful.
- Auto-Fix boundary: hide Auto-Fix entirely in S07; include read-only details actions such as Copy Details, File List, path open, and URL open/copy. S08 will add Auto-Fix, Fixed!, and Fix Failed.
- Scan flow: match the reference explicit `Scan Game` workflow: disable during scan, clear old results/details, refresh Overview first, show `Refreshing Overview...`, `Building mod file index...`, `Scanning... n/N: folder`, then re-enable and show `N Results ~ Select an item for details`. No cancellation or auto-scan in S07.
- Rule scope: implement all reference read-only scanner categories behind the existing persisted toggles: Overview Issues, Errors, Wrong File Formats, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs. Auto-fix execution is the only deferred scanner capability.
- Ordering: group by problem type in a stable reference enum order, with a mod column only when MO2 staging attribution exists. Prefer deterministic Rust ordering over Python set-order instability.

## Open Items To Resolve

- Whether scanner checkbox changes persist immediately or only when `Scan Game` starts; current reference persists during scan settings construction.
- Exact empty/zero-result wording and whether to preserve the reference `0 Results ~ Select an item for details` text.
- How visible read-only action failures should be surfaced for path open, URL open, URL copy, and Copy Details.
