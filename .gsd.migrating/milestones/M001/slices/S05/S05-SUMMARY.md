---
id: S05
parent: M001
milestone: M001
provides:
  - Reference-shaped Tools and About tabs for downstream visual/workflow slices.
  - Safe external URL and clipboard action seams usable by future Scanner/F4SE/Downgrader/Patcher actions.
  - Rust-owned About image assets available to Slint.
  - Visible but non-destructive utility entry points for S09/S10.
  - Contract and runtime tests covering labels/order/URLs/resources/callbacks/deferred state/failure feedback.
requires:
  - slice: S03
    provides: Fakeable platform/process/desktop seams and worker handoff patterns consumed by Tools/About action wiring.
  - slice: S04
    provides: Precedent for safe visible last-action feedback and deferred workflow presentation consumed by S05.
affects:
  - S06 F4SE Diagnostics
  - S07 Scanner Read Only Results
  - S09 Downgrade Manager Workflow
  - S10 Archive Patcher Workflow
key_files:
  - Cargo.toml
  - Cargo.lock
  - src/domain/tools.rs
  - src/domain/mod.rs
  - src/platform/clipboard.rs
  - src/platform/desktop.rs
  - src/platform/mod.rs
  - src/services/tools.rs
  - src/services/mod.rs
  - src/app/tools_controller.rs
  - src/app/about_controller.rs
  - src/app/mod.rs
  - src/workers/events.rs
  - src/workers/mod.rs
  - src/main.rs
  - ui/tools_tab.slint
  - ui/about_tab.slint
  - ui/main.slint
  - resources/images/icon-256.png
  - resources/images/logo-nexusmods.png
  - resources/images/logo-discord.png
  - resources/images/logo-github.png
key_decisions:
  - D021: Copy the S05 reference images into Rust-owned `resources/images/` paths instead of depending on `CMT/` at runtime.
  - D022: Route Tools/About open and copy actions through fakeable controller/worker/platform boundaries instead of direct Slint callback side effects.
  - D023: Use `arboard` behind the fakeable `ClipboardActions` trait for production clipboard access.
  - Keep static URLs out of Slint; Slint emits stable Rust-owned action ids that are preflighted before adapter calls.
  - Keep Downgrade Manager and Archive Patcher visible but disabled/deferred until their destructive workflows are ported.
patterns_established:
  - Pure domain contract modules can lock reference labels, URLs, action ids, resources, and deferred-state metadata outside Slint.
  - Untrusted UI callback ids are parsed against known contracts and fail closed before invoking desktop or clipboard adapters.
  - Reducer-managed status/copy feedback keeps UI-visible messages testable without launching a browser or requiring a real clipboard.
  - Worker payloads carry owned safe action results and avoid carrying copied text.
  - Slint source-contract tests lock callback forwarding, image references, copy timers, disabled utility state, and absence of raw URLs in UI markup.
observability_surfaces:
  - Tools/About inline status/error banners for URL-open, copy, disabled utility, spawn, and worker failures.
  - Structured tracing around scheduling, rejection, spawn failure, worker event application/ignore paths, dropped events, copy reset, and poisoned controller locks.
  - Worker completion/failure payloads that map background action results back to safe user-visible controller feedback.
  - Operational readiness: health signal is successful action/copy status plus passing worker/test gates; failure signal is inline banner/status plus tracing; recovery is retry after restoring browser/clipboard/runtime availability; monitoring gaps are limited to no packaged-app telemetry or manual GUI telemetry yet.
drill_down_paths:
  - .gsd/milestones/M001/slices/S05/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T04-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T05-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-18T02:18:27.908Z
blocker_discovered: false
---

# S05: Tools Shell, Links & About

**Tools and About are now reference-shaped Slint tabs with Rust-owned static contracts, safe link/copy actions, deferred utility entries, visible failure feedback, and passing slice gates.**

## What Happened

S05 replaced the inert Tools and About placeholders with a faithful, non-destructive Rust/Slint shell for the reference app's utility and attribution surfaces. T01 established `src/domain/tools.rs` as the pure Rust source of truth for ordered Tools groups, button labels, URLs, help text, About attribution/link definitions, copy-feedback constants, disabled utility metadata, and Rust-owned image resource paths; the required reference images were copied into `resources/images/` without modifying `CMT/`. T02 added the Slint-free action layer: typed open/copy services, fail-closed parsing of untrusted callback ids, an `arboard`-backed clipboard adapter behind a fakeable `ClipboardActions` trait, pure Tools/About reducers, and owned worker payloads that expose safe UI messages without carrying copied text. T03 implemented the Slint UI surfaces: Tools now shows `Toolkit Utilities`, `Other CM Authors' Tools`, and `Other Useful Tools` with reference button ordering and helper text, while About shows the CMT identity, version/credit text, logo resources, open/copy actions, inline failure banners, and copy-label reset timers. T04 wired those callbacks into `MainWindow` through preflight checks, background workers, production desktop/clipboard adapters, state projection, spawn/worker-failure mapping, and structured tracing. T05 and closeout verification confirmed the full Rust gates pass and reused the recorded CMT cleanliness evidence from the completed task because the closeout unit prohibits running git commands directly. The Downgrade Manager and Archive Patcher entries are visible but intentionally disabled/deferred so S05 cannot trigger downloads, backups, patch plans, or archive writes before S09/S10.

## Verification

Closeout verification was run with `gsd_exec`: `cargo fmt --check` passed with exit 0 in 463ms (6574d641-bd28-4bd5-bb83-b3e8e446f5e7); `cargo check` passed with exit 0 in 8513ms (485cec21-c96b-4b82-bd1a-a45fa46966cb); `cargo test` passed with exit 0 in 8268ms and reported `174 passed; 0 failed; 0 ignored` (473dd5d3-7978-4676-954a-db4d2a9c174b); `cargo clippy --all-targets --all-features` passed with exit 0 in 8481ms (e96e3a1c-5c3c-4832-acc1-f62b49305f52). Prior S05 T05 evidence was reused for the read-only reference check: `gsd_exec_search` found `git status --short CMT` exit 0 with empty stdout in run 9db91d35-dedd-4c12-a7af-c6b5ee5047fb. Focused task evidence also covered `s05_reference_contract`, `s05_actions`, `s05_slint_contract`, and `s05_runtime_wiring` tests before closeout.

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

No scope deviations. Exact Tk tooltip widgets were represented through visible Slint helper/status text, matching the planned practical Slint affordance.

## Known Limitations

Downgrade Manager and Archive Patcher are intentionally disabled/deferred until S09/S10. Manual GUI UAT was drafted but not executed by the closeout automation; closeout proof is automated cargo/test evidence plus task-level contract coverage.

## Follow-ups

S06 can build on the safe callback/state projection patterns for diagnostics. S07 can reuse the action service and visible failure feedback pattern for Scanner copy/open actions. S09 and S10 should replace the current deferred utility entries with validated, fail-closed live workflows.

## Files Created/Modified

- `Cargo.toml` — Added the focused production clipboard dependency used behind a fakeable boundary.
- `Cargo.lock` — Updated dependency lockfile for clipboard support.
- `src/domain/tools.rs` — Added the pure Tools/About reference contract with labels, ids, URLs, help text, image paths, copy constants, deferred utility metadata, and tests.
- `src/domain/mod.rs` — Exported the Tools/About domain contract module.
- `src/platform/clipboard.rs` — Added fakeable and production clipboard adapter boundaries.
- `src/platform/desktop.rs` — Extended platform operation/result surfaces for safe copy/open feedback.
- `src/platform/mod.rs` — Exported clipboard/platform additions.
- `src/services/tools.rs` — Added action services and preflight logic for Tools/About open/copy requests.
- `src/services/mod.rs` — Exported Tools action service module.
- `src/app/tools_controller.rs` — Added pure Tools feedback reducer/state projection.
- `src/app/about_controller.rs` — Added pure About feedback and copy-label reducer/state projection.
- `src/app/mod.rs` — Exported Tools/About controllers.
- `src/workers/events.rs` — Added owned Tools/About action completion payloads.
- `src/workers/mod.rs` — Exported worker event additions.
- `src/main.rs` — Wired MainWindow Tools/About callbacks, worker sinks, state projection, reset handling, and source-contract/runtime tests.
- `ui/tools_tab.slint` — Replaced the placeholder with the reference-shaped Tools tab, grouped actions, deferred utility entries, and visible feedback surface.
- `ui/about_tab.slint` — Replaced the placeholder with the reference-shaped About tab, image resources, link/copy actions, status banner, and copy reset timers.
- `ui/main.slint` — Forwarded Tools/About properties and callbacks through the main shell.
- `resources/images/icon-256.png` — Copied Rust-owned CMT icon resource for Slint.
- `resources/images/logo-nexusmods.png` — Copied Rust-owned Nexus Mods logo resource for Slint.
- `resources/images/logo-discord.png` — Copied Rust-owned Discord logo resource for Slint.
- `resources/images/logo-github.png` — Copied Rust-owned GitHub logo resource for Slint.
