# Phase 3: Platform Discovery & Background Adapters - Specification

**Created:** 2026-05-17
**Ambiguity score:** 0.18 (gate: <= 0.20)
**Requirements:** 7 locked

## Goal

Later user workflows can rely on reference-compatible Fallout 4 discovery, running mod-manager detection, injectable filesystem/process seams, and typed background task events without blocking the Slint UI thread.

## Background

Phase 1 established the Slint shell, tab structure, and inert module seams. Phase 2 completed reference-compatible settings defaults and persistence, including scanner toggles and downgrader options. The current Rust code has placeholder tab behavior, typed settings, filesystem-backed settings IO, and a `WorkerRuntime` marker documenting where slow work will live, but it does not yet discover Fallout 4 paths, detect Mod Organizer or Vortex, expose fakeable filesystem/process adapters, or define real progress/completion/cancellation/error events for background work. The Python reference provides this behavior across `CMT/src/game_info.py`, `CMT/src/utils.py`, and later scanner/overview workflows, including registry-based Fallout 4 path lookup, `Fallout4.exe` validation, `Data`-related path state, running mod-manager detection, and user-facing game-not-found guidance.

## Requirements

1. **Reference-compatible Fallout 4 discovery**: The Rust domain/platform layer discovers or represents the same core Fallout 4 installation state that later Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows need.
   - Current: No Rust game-discovery model or lookup behavior exists; later tabs only have placeholder UI and cannot rely on a typed game path or derived paths.
   - Target: The app can perform reference-style discovery, validate the discovered or configured game directory with `Fallout4.exe`, expose the game path, `Data` path, F4SE path, settings/preferences INI paths, archive sets, enabled archive set, and module set as typed state, and represent a not-found result with reference-compatible error text.
   - Acceptance: Fake-backed discovery tests cover a valid Fallout 4 directory, a registry path that does not contain `Fallout4.exe`, and a missing discovery result; the not-found case includes `A Fallout 4 installation could not be found.` and the invalid-registry case includes the registry path guidance used by the reference.

2. **Running mod-manager detection**: The Rust platform layer detects the running mod manager context needed by later diagnostics.
   - Current: No Rust mod-manager discovery or process inspection exists.
   - Target: The app can detect a running `ModOrganizer.exe` or `Vortex.exe` process, return a typed manager value with display name (`Mod Organizer` or `Vortex`), executable path, and parsed semantic version, and fall back to version `0.0.0` when version metadata cannot be read.
   - Acceptance: Fake-backed process tests verify Mod Organizer detection, Vortex detection, unknown/no-manager detection, version parsing, and `0.0.0` fallback when version data is unavailable.

3. **Injectable filesystem seam**: Filesystem reads needed by discovery and future scans are behind a testable adapter boundary.
   - Current: Settings persistence has filesystem IO, but there is no general discovery/scan filesystem adapter that tests can fake without touching the real game directory.
   - Target: Game discovery and related path/file checks use injectable filesystem abstractions for existence checks, directory/file classification, text reads needed for INI-style sources, and deterministic directory enumeration needed by later archive/module collection.
   - Acceptance: Unit tests prove discovery behavior using fake filesystem data only, including existing files, missing files, directories, and deterministic archive/module enumeration inputs.

4. **Injectable process and desktop seam**: Process inspection, external command launch, path opening, and URL opening are behind a testable adapter boundary with visible failure values.
   - Current: There is no Rust process/desktop adapter for running-manager detection or later Tools/About/Scanner detail actions.
   - Target: The app exposes typed interfaces for listing candidate processes, reading executable version metadata, launching external tools, opening filesystem paths, and opening URLs; failures are returned as typed errors rather than panics or silent no-ops.
   - Acceptance: Fake-backed tests verify process listing, version metadata success/failure, launch/open success, and launch/open failure reporting without invoking real processes or desktop handlers.

5. **Typed background task events**: Long-running work has a shared event contract that later phases can reuse.
   - Current: `src/workers` contains only an inert marker and explicitly creates no runtimes, channels, tasks, or UI-thread handoffs.
   - Target: The worker layer defines typed command/result/progress events covering task start, progress text, progress counts when available, completion, cancellation, and errors, with enough metadata to distinguish discovery, scan, patch, download, and external-process tasks.
   - Acceptance: Tests construct and route representative discovery, scan, patch, download, cancellation, and error events without requiring a Slint window.

6. **Slint-safe handoff seam**: Background results can be marshalled back to UI state without mutating Slint objects off the UI thread.
   - Current: No real UI-thread handoff or event-loop adapter exists; future slow workflows would otherwise risk blocking or cross-thread UI mutation.
   - Target: The app provides a small Slint-safe handoff boundary for later callbacks to enqueue owned domain events from background work and apply them on the Slint event loop.
   - Acceptance: A testable adapter or integration check verifies that background-produced owned events are delivered through the handoff seam, and code review can confirm no Slint models or UI handles are directly mutated from worker threads.

7. **Quality-gate preservation**: Phase 3 keeps the Rust/Slint port buildable and does not modify the reference submodule.
   - Current: Phase 2 completed with the expected Rust checks and `CMT/` remains the read-only source of truth.
   - Target: Phase 3 changes are confined outside `CMT/`, keep the app buildable, and preserve the completed settings behavior from Phase 2.
   - Acceptance: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` pass after implementation, and `git status` shows no modifications under `CMT/`.

## Boundaries

**In scope:**
- Fallout 4 installation discovery and representation, including registry-style discovery, `Fallout4.exe` validation, game path, `Data` path, F4SE path, settings/preferences INI paths, archive sets, enabled archive set, module set, and reference-compatible not-found error text.
- Running Mod Organizer and Vortex process detection with manager name, executable path, parsed version, and `0.0.0` version fallback.
- Injectable filesystem abstractions used by discovery and prepared for future scanner traversal.
- Injectable process/desktop abstractions for process listing, version metadata, launch/open URL, and launch/open path operations.
- Typed background command/result/progress/cancellation/error events reusable by later Overview, F4SE, Scanner, Downgrader, and Archive Patcher phases.
- A Slint-safe event-loop handoff seam for owned domain events from worker code.
- Fake-backed tests for discovery, filesystem/process adapters, and background event routing.

**Out of scope:**
- Rendering final Overview diagnostics panels - Phase 4 owns the user-facing Overview diagnostics UI and counts.
- Implementing Scanner tab scanning, result grouping, details, URL actions, or scanner problem classification - Phases 7 and 8 own scanner workflows.
- Implementing F4SE plugin scanning or F4SE-specific compatibility results - Phase 6 owns F4SE behavior.
- Implementing downgrader downloads, archive patching, or mutation workflows - later Tools/Downgrader/Archive Patcher phases own those behaviors.
- Fully wiring a user-visible UI button to a real background scan - Phase 3 defines the shared seam and event contract; later workflow phases consume it.
- Adding new product behavior not present in the Python reference - this phase exists to make faithful later ports possible, not to redesign CMT.

## Constraints

- `CMT/` remains read-only and must only be inspected as reference material.
- Slow discovery, traversal, parsing, process inspection, downloads, patching, and process monitoring must not run directly on the Slint UI thread.
- Slint UI objects and models must only be mutated through Slint-safe event-loop handoff mechanisms.
- Filesystem and process behavior must be injectable so tests can run without a Fallout 4 install, running mod manager, real Windows registry state, external process launch, or visible desktop handler.
- Windows-specific discovery may use Windows-only implementations, but public domain models and tests must remain structured so non-Windows builds can report unsupported discovery cleanly rather than panicking.
- Production discovery and adapter paths must avoid `unwrap()`/`expect()` for missing, locked, malformed, or non-UTF-8 filesystem/process data unless the invariant is documented and impossible for user-controlled inputs.

## Acceptance Criteria

- [ ] Fake-backed game discovery returns typed game path, `Data` path, F4SE path, INI paths, archive sets, enabled archive set, and module set for a valid Fallout 4 fixture.
- [ ] Invalid-registry and missing-install discovery tests return reference-compatible not-found messages, including `A Fallout 4 installation could not be found.` and invalid registry path guidance where applicable.
- [ ] Fake-backed process tests detect `ModOrganizer.exe` as `Mod Organizer` with path/version, detect `Vortex.exe` as `Vortex` with path/version, return no-manager for unrelated processes, and use `0.0.0` when version data is unavailable.
- [ ] Discovery code uses injectable filesystem/process seams in tests rather than the real filesystem, real process list, registry, or desktop handlers.
- [ ] Process/desktop adapter tests cover successful and failed launch/open URL/open path operations with typed error results.
- [ ] Worker event tests cover start, progress text, optional progress counts, completion, cancellation, and error events for representative discovery, scan, patch, download, and external-process task kinds.
- [ ] The Slint handoff seam accepts owned background events and applies them only through an event-loop-safe boundary; worker code does not directly mutate Slint models or UI handles.
- [ ] No files under `CMT/` are modified.
- [ ] `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` pass after implementation.

## Ambiguity Report

| Dimension          | Score | Min   | Status | Notes |
|--------------------|-------|-------|--------|-------|
| Goal Clarity       | 0.90  | 0.75  | ✓      | Phase goal is locked to discovery, adapters, and typed worker events for later workflows. |
| Boundary Clarity   | 0.78  | 0.70  | ✓      | Scanner/F4SE/Overview/Downgrader behavior is explicitly deferred; platform seams and discovery are included. |
| Constraint Clarity | 0.76  | 0.65  | ✓      | Read-only reference, fakeable IO/process seams, Slint-thread safety, and quality gates are explicit. |
| Acceptance Criteria| 0.80  | 0.70  | ✓      | Acceptance is expressed as fake-backed tests and pass/fail quality checks. |
| **Ambiguity**      | 0.18  | <=0.20| ✓      | Gate passed after round 2. |

Status: ✓ = met minimum, ⚠ = below minimum (planner treats as assumption)

## Interview Log

| Round | Perspective | Question summary | Decision locked |
|-------|-------------|------------------|-----------------|
| 1 | Researcher | What is the required minimum target for Fallout 4 path detection? | Use reference-full discovery as far as practical, rather than manual-only path representation. |
| 1 | Researcher | What mod-manager context must Phase 3 establish? | Detect running Mod Organizer/Vortex managers rather than only defining a placeholder model. |
| 1 | Researcher | What background-adapter capability is irreducible? | Typed task events for progress, completion, cancellation, errors, and Slint-safe handoff are required. |
| 2 | Researcher + Simplifier | Which reference discovery behaviors must be included now? | Include all path caches needed by later workflows, not only registry validation or INI paths. |
| 2 | Researcher + Simplifier | What is the minimum mod-manager detection output? | Return manager name, executable path, and parsed version with fallback. |
| 2 | Researcher + Simplifier | What is the smallest proof adapters are usable? | Define injectable traits/seams with fake-backed tests; no user-visible UI workflow is required in this phase. |

---

*Phase: 03-platform-discovery-background-adapters*
*Spec created: 2026-05-17*
*Next step: /gsd-discuss-phase 3 - implementation decisions (module layout, adapter traits, Windows registry/process implementation, and Slint handoff mechanics)*
