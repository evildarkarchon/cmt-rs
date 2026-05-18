---
id: S07
milestone: M001
status: ready
---

# S07: Scanner Read Only Results — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver the Scanner tab's reference-shaped read-only scan settings, explicit scan execution flow, progress, grouped results, details pane, and safe copy/open actions while keeping Auto-Fix writes deferred.

## Why this Slice

S07 follows Overview and F4SE because it consumes their discovery, diagnostics, worker, settings, and problem-feed foundations, then turns them into the main user-facing scan workflow before S08 adds mutation-heavy Auto-Fix actions.

## Scope

### In Scope

- Replace the inert Scanner placeholder with a functional Scanner tab that preserves the reference tab label, scan setting labels, scan button text, progress text, result count text, grouping shape, details labels, and read-only actions.
- Render scan settings and result details as embedded Slint panes rather than Tk-style floating side/detail windows, while preserving the reference labels and spatial intent.
- Show all reference scanner checkboxes with persisted/default state from S02: `Overview Issues`, `Errors`, `Wrong File Formats`, `Loose Previs`, `Junk Files`, `Problem Overrides`, and `Race Subgraphs`.
- Disable `Scan Game` when all scanner checkboxes are off, matching the reference behavior.
- Persist scanner checkbox changes when `Scan Game` starts, matching the reference `ScanSettings` construction/save timing rather than saving every toggle immediately.
- Use the explicit reference scan flow: user clicks `Scan Game`; the button is disabled and changes to `Scanning...`; old results/details are cleared; Overview is refreshed first; progress shows `Refreshing Overview...`, `Building mod file index...`, and `Scanning... n/N: folder`; completion re-enables `Scan Game` and shows `N Results ~ Select an item for details`.
- Implement all reference read-only scanner categories behind their toggles: Overview Issues, Errors, Wrong File Formats, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs.
- Use the S04 Overview problem feed for the `Overview Issues` category, and map Overview problems into Scanner-compatible result rows/details.
- Preserve MO2 staged mod attribution when available, including a mod column only when staging/source attribution exists.
- Preserve Vortex partial-support behavior: scan Data only and do not invent Vortex staging/source-mod attribution in this slice.
- Continue scanning safe/available areas when prerequisites are incomplete or files/folders are unreadable; show visible warning/error result rows or inline feedback for skipped/broken areas rather than silently skipping or failing the whole scan where partial results are useful.
- Group results by problem type in a stable reference enum order, with deterministic row ordering suitable for tests and support screenshots.
- Show the reference empty result text `0 Results ~ Select an item for details` when a scan completes with no results.
- Provide read-only details for selected results with reference labels: `Mod:`, `Problem:`, `Summary:`, `Solution:`, `Copy Details`, and optional `File List`.
- Support safe read-only actions: open file/folder path when available, open URL, copy URL, copy details, and show file lists for supported simple results.
- Surface read-only action failures through safe inline feedback/banner text, keeping diagnostics in logs/tests, following the S05 action-feedback precedent.
- Run traversal, parsing, Overview refresh, MO2 file indexing, and race-subgraph counting off the Slint UI thread and marshal owned results/progress back through the worker/event-loop handoff pattern.
- Add fake-backed tests for settings toggles/persistence timing, progress/result state transitions, grouped row ordering, Overview problem mapping, MO2 attribution, Vortex/Data-only behavior, unreadable/missing paths, zero-result completion, details text, and read-only action feedback.

### Out of Scope

- Implementing, showing, or wiring `Auto-Fix`, `Fixed!`, or `Fix Failed` in S07; the Auto-Fix affordance should be hidden until S08 implements the write actions.
- Mutating user files in any way, including delete, rename, archive, patch, backup, or repair operations.
- Adding scan cancellation, live filesystem watching, automatic scan-on-tab-open, or background auto-rescan.
- Adding new scanner problem categories beyond the reference scanner and Overview problem feed.
- Adding better-than-reference Vortex staging/source-mod attribution.
- Replacing the explicit `Scan Game` workflow with a redesigned scanner UX.
- Implementing Downgrade Manager or Archive Patcher workflows.

## Constraints

- `CMT/` remains read-only; inspect it for parity but implement all Rust/Slint behavior outside the submodule.
- Preserve reference scanner labels, checkbox order, button text, status strings, result count wording, details labels, problem labels, and solution text unless an intentional difference is documented.
- The known intentional UI difference is embedded Slint panes instead of separate undecorated Tk windows for settings/details.
- S07 is read-only: no scanner result may trigger destructive filesystem operations.
- Do not expose disabled or no-op Auto-Fix controls in S07; hiding Auto-Fix is clearer than implying an unavailable write action.
- Slint must not perform filesystem traversal, parser work, MO2 indexing, or Overview refresh logic directly; callbacks should dispatch typed commands and render typed state.
- Background workers must send owned progress/results and update Slint only through the event-loop handoff pattern; do not move Slint handles or models into worker threads.
- Partial failures should be visible and actionable enough for users to understand missing/skipped coverage without losing valid scan results.
- Unknown mod attribution is acceptable for unmanaged/Data-only results; do not fabricate mod names.
- Deterministic result order is preferred over exactly reproducing Python set-order instability.

## Integration Points

### Consumes

- `CMT/src/tabs/_scanner.py` — Source of truth for Scanner UI labels, explicit scan flow, progress messages, result grouping/details behavior, read-only actions, and data traversal logic.
- `CMT/src/scan_settings.py` — Source of truth for scanner setting labels, default-enabled categories, Data whitelist, junk files, proper-format mappings, ignored folders, skip suffixes, and settings-save timing.
- `CMT/src/enums.py` — Source of truth for `ProblemType` and `SolutionType` labels/text used in results and details.
- `CMT/src/globals.py` — Source of truth for scanner tooltip/help text, race-subgraph threshold/info, archive suffix/name allow-lists, F4SE script CRC metadata, and user-facing scanner messages.
- `src/domain/settings.rs` and `src/platform/settings_store.rs` — Provide persisted scanner toggle defaults and reference-compatible `scanner_*` keys.
- `src/domain/overview.rs` and `src/services/overview.rs` — Provide the scanner-ready Overview problem feed consumed when `Overview Issues` is enabled.
- `src/domain/discovery.rs`, `src/domain/mod_manager.rs`, and `src/services/discovery.rs` — Provide Data path, enabled modules/archives, install state, MO2 context, Vortex identity-only context, and manager-specific failure messages.
- `src/platform/filesystem.rs` — Provides fakeable directory traversal, metadata, byte/text reads, and unreadable/not-found errors for scanner fixtures and production scans.
- `src/platform/desktop.rs`, `src/platform/clipboard.rs`, and `src/services/tools.rs` — Provide safe open/copy action patterns and failure feedback for read-only Scanner details actions.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — Provide the owned worker progress/result/failure envelope and Slint-safe handoff pattern used by scans.
- `ui/main.slint` and `ui/scanner_tab.slint` — Existing shell and placeholder surface to replace with Scanner properties/models/callback forwarding.
- `S06-CONTEXT.md` — Precedent for read-only diagnostics, partial parser failures, and non-blocking tab work immediately before Scanner.

### Produces

- `src/domain/scanner.rs` — Typed scanner settings snapshot, problem types, solution text, result records, mod attribution, file-list/detail metadata, and read-only action descriptors.
- `src/services/scanner.rs` — Adapter-backed scanner engine for Overview Issues integration, MO2 mod-file indexing, Data traversal, rule classification, race-subgraph counting, and partial-failure reporting.
- `src/app/scanner_controller.rs` — Slint-free reducer/controller state for checkbox state, scan lifecycle, progress, grouped results, selected details, read-only action feedback, and stale-result handling.
- `src/workers/events.rs` — Scanner-specific worker payloads for progress, completion, action feedback, and safe failures.
- `ui/scanner_tab.slint` — Reference-shaped embedded Scanner UI with scan settings, progress, grouped results, selected-result details, file-list affordance, copy/open actions, and inline failure feedback.
- `ui/main.slint` — Scanner property/model/callback forwarding through `MainWindow`.
- Rust tests and Slint source-contract tests — Coverage for labels/order, settings persistence timing, worker progress, grouped result ordering, scanner rules, partial failures, zero results, details actions, and read-only boundaries.

## Open Questions

- None at discussion closeout. If cancellation, auto-scan, Vortex staging attribution, or Auto-Fix visibility is requested later, treat those as future scoped changes rather than part of S07.
