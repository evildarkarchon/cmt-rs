---
id: S05
milestone: M001
status: ready
---

# S05: Tools Shell, Links & About — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Deliver the reference-shaped Tools and About tabs with preserved tool/link groupings, attribution, image/logo identity, safe URL/copy actions, visible failure feedback, and clearly deferred utility workflow entries.

## Why this Slice

S05 follows S04 because Overview already established safe URL/path open patterns and disabled/deferred utility entry points; Tools and About now generalize those shared external-action and clipboard affordances before F4SE, Scanner details, and later Downgrade Manager/Archive Patcher workflows need the same behavior. The slice is deliberately non-destructive: it makes the static links, attribution, and utility entry locations visible and testable while preserving S09/S10 as the only slices that add live downgrade/archive mutation behavior.

## Scope

### In Scope

- Replace the inert `Tools` tab with the reference group structure from `CMT/src/tabs/_tools.py`: `Toolkit Utilities`, `Other CM Authors' Tools`, and `Other Useful Tools`.
- Preserve the reference Tools button labels, ordering, URLs, multi-line label formatting where practical, and tooltip/help text semantics for the external tools.
- Show `Downgrade Manager` and `Archive Patcher` in the Tools `Toolkit Utilities` group, but keep them visibly deferred/disabled in S05; users should not be able to start downloads, backups, patch plans, archive writes, or modal utility workflows yet.
- Replace the inert `About` tab with the reference attribution content from `CMT/src/tabs/_about.py`: CMT title, version/credit text, Nexus Mods link actions, Discord invite actions, and GitHub link actions.
- Use the reference About imagery now if Slint resource handling can do so safely: `icon-256.png`, `logo-nexusmods.png`, `logo-discord.png`, and `logo-github.png`. Assets must be brought into or referenced through Rust-owned resource paths without modifying `CMT/`; missing image resources should degrade safely with visible text/buttons rather than breaking the tab.
- Implement open-link and copy-link actions through shared, fakeable platform/action boundaries instead of direct `webbrowser`/`Command`-style calls in UI callbacks.
- Preserve reference-style copy success feedback: copy buttons should briefly become `Copied!` when the copy succeeds, and then return to their original label.
- Surface safe visible failure feedback for failed URL opens or copy attempts, using an inline banner/status pattern consistent with S04's safe last-action error behavior.
- Add source-level/UI contract tests for group labels, button labels, URL constants, About text/actions, callback wiring, and deferred utility state; add Rust tests for action definitions and success/failure feedback mapping without launching a browser or requiring a real clipboard.

### Out of Scope

- Live `Downgrade Manager` behavior, modal shell, download orchestration, backup handling, delta cleanup, version switching, or any game/Creation Kit file mutation; this remains S09.
- Live `Archive Patcher` behavior, BA2 parsing/write plans, backup handling, patch execution, or any archive mutation; this remains S10.
- Adding new external tools, changing reference URLs, redesigning the Tools/About layouts, or rewording attribution/link text beyond practical Slint formatting constraints.
- F4SE diagnostics, Scanner result details, Scanner copy/open/file-list actions, and auto-fix behavior; S05 may establish shared action helpers, but those tabs remain later slices.
- Final installer/package resource validation beyond ensuring the app builds and the S05 resources are resolved in the current Rust/Slint project layout.
- Editing, formatting, moving, or generating files under `CMT/`.

## Constraints

- `CMT/` remains read-only and is the source of truth for S05 labels, group ordering, URLs, tooltips/help text, attribution copy, and image names.
- Reference files to inspect during execution include `CMT/src/tabs/_tools.py`, `CMT/src/tabs/_about.py`, `CMT/src/globals.py`, and `CMT/src/utils.py`.
- Utility entries must be visible but fail closed in S05; a user must not be able to trigger destructive downgrade/archive behavior before S09/S10.
- Link/copy callbacks must not panic or silently fail. Failures need safe user-facing feedback, while raw adapter diagnostics belong in tracing/logs/tests.
- Keep Slint declarative: UI emits typed callbacks/properties; Rust domain/controller/platform code owns action definitions, state transitions, desktop opens, and clipboard work.
- Do not block the Slint event loop for external actions. Reuse the existing worker/event-loop handoff pattern or an equivalently safe controller boundary for operations that can stall or fail.
- Use a conservative visual port: close to the Tkinter grouping, button text, logo placement, and attribution feel, not a redesigned landing page.
- Run the relevant verification gates before completing the implementation slice: `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`, and a `CMT/` cleanliness check when allowed by the active GSD unit.

## Integration Points

### Consumes

- `CMT/src/tabs/_tools.py` — source of truth for Tools group labels, button labels/order, external URLs, tooltip/help text, and utility entries.
- `CMT/src/tabs/_about.py` — source of truth for About layout intent, attribution text, link/copy button labels, and link action ordering.
- `CMT/src/globals.py` — source of truth for `APP_TITLE`, `APP_VERSION`, `NEXUS_LINK`, `DISCORD_INVITE`, `GITHUB_LINK`, and referenced image names.
- `CMT/src/utils.py` — reference copy behavior, especially the `Copied!` temporary button-label feedback.
- `CMT/src/assets/images/*` — read-only source assets for the CMT icon and Nexus/Discord/GitHub logos; implementation must use Rust-owned resource copies or safe Slint resource configuration.
- `ui/main.slint` — existing shell/tab wiring that must forward Tools/About properties and callbacks.
- `ui/tools_tab.slint` and `ui/about_tab.slint` — inert placeholders to replace with reference-shaped Slint components.
- `src/platform/desktop.rs` — existing fakeable URL/path/tool launch boundary and safe action-result pattern from S03/S04.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — existing owned worker/action event and Slint event-loop handoff surfaces available for safe external-action feedback.
- `src/app/overview_controller.rs` and S04 Overview wiring in `src/main.rs` — precedent for safe last-action error feedback, deferred action presentation, and source-level Slint contract testing.

### Produces

- `ui/tools_tab.slint` — reference-shaped Tools tab with grouped buttons, deferred utility entries, action callbacks, and visible feedback surface.
- `ui/about_tab.slint` — reference-shaped About tab with title/version/credit content, logos where available, link/copy buttons, and visible feedback surface.
- `ui/main.slint` — forwarded Tools/About properties and callbacks from the main window to the tab components.
- `src/domain/tools.rs` or equivalent domain module — typed definitions for Tools/About groups, static URLs, utility entries, labels, help text, and deferred-state metadata.
- `src/app/tools_controller.rs`, `src/app/about_controller.rs`, or equivalent controller code — Slint-free reducers/action handlers for open/copy success/failure feedback and temporary copy-label state.
- `src/platform/clipboard.rs` or equivalent fakeable clipboard boundary — copy-link behavior with typed success/failure results suitable for tests and user-visible errors.
- Rust-owned asset/resource files or resource configuration — CMT icon and logo assets available to Slint without modifying or depending on mutable `CMT/` paths at runtime.
- Tests/contract checks — coverage for reference labels/order/URLs/text, deferred utility state, callback forwarding, copy feedback, and failed open/copy feedback.

## Open Questions

- Exact Slint resource packaging path for About images — Current thinking: copy or include the minimal reference images into a Rust-owned asset/resource location and assert they are referenced by the Slint UI; do not make the running app depend on mutable files under `CMT/`.
- Tooltip/help affordance fidelity for Tools entries — Current thinking: preserve tooltip text in typed definitions and surface it through the closest practical Slint UI pattern, such as adjacent info text/icons, if exact Tk tooltip behavior is unavailable.
- Clipboard implementation details — Current thinking: add a small fakeable clipboard adapter; if the platform clipboard is unavailable or fails, show a safe banner/status rather than pretending the copy succeeded.
- About version source — Current thinking: preserve the reference-visible `v0.6.1` About text for parity unless a later product/versioning decision explicitly distinguishes the Rust port package version from the original CMT app version.
