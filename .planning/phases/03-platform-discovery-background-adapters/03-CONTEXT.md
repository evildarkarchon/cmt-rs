# Phase 3: platform-discovery-background-adapters - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 delivers the typed platform foundation that later user-visible tabs consume: reference-compatible Fallout 4 discovery, running mod-manager detection, injectable filesystem/process/desktop seams, typed worker events, and a Slint-safe event-loop handoff boundary. It does not render final Overview diagnostics, run Scanner/F4SE workflows, perform downloads, or mutate game files.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**7 requirements are locked.** See `03-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `03-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Fallout 4 installation discovery and representation, including registry-style discovery, `Fallout4.exe` validation, game path, `Data` path, F4SE path, settings/preferences INI paths, archive sets, enabled archive set, module set, and reference-compatible not-found error text.
- Running Mod Organizer and Vortex process detection with manager name, executable path, parsed version, and `0.0.0` version fallback.
- Injectable filesystem abstractions used by discovery and prepared for future scanner traversal.
- Injectable process/desktop abstractions for process listing, version metadata, launch/open URL, and launch/open path operations.
- Typed background command/result/progress/cancellation/error events reusable by later Overview, F4SE, Scanner, Downgrader, and Archive Patcher phases.
- A Slint-safe event-loop handoff seam for owned domain events from worker code.
- Fake-backed tests for discovery, filesystem/process adapters, and background event routing.

**Out of scope (from SPEC.md):**
- Rendering final Overview diagnostics panels - Phase 4 owns the user-facing Overview diagnostics UI and counts.
- Implementing Scanner tab scanning, result grouping, details, URL actions, or scanner problem classification - Phases 7 and 8 own scanner workflows.
- Implementing F4SE plugin scanning or F4SE-specific compatibility results - Phase 6 owns F4SE behavior.
- Implementing downgrader downloads, archive patching, or mutation workflows - later Tools/Downgrader/Archive Patcher phases own those behaviors.
- Fully wiring a user-visible UI button to a real background scan - Phase 3 defines the shared seam and event contract; later workflow phases consume it.
- Adding new product behavior not present in the Python reference - this phase exists to make faithful later ports possible, not to redesign CMT.

</spec_lock>

<decisions>
## Implementation Decisions

### Discovery Fallback
- **D-01:** Mirror the Python reference discovery order: use running manager game path first, then current working directory if it is a Fallout 4 directory, then Bethesda/GOG registry paths.
- **D-02:** Phase 3 should not show manual file-picker UI. When no valid Fallout 4 directory is found, return recoverable typed discovery results with reference-compatible messages so later UI phases can decide how to prompt.
- **D-03:** Accept either a Fallout 4 directory or a `Fallout4.exe` path as input; normalize executable paths to the parent game directory, matching the reference manual-selection behavior.
- **D-04:** A valid game directory can produce partial derived state. Missing `Data` or `Data/F4SE/Plugins` should be represented as missing/`None` fields rather than making discovery fail.

### Mod Organizer And Vortex Depth
- **D-05:** Parse Mod Organizer configuration deeply enough for later phases: `gamePath`, `selected_profile`, `mod_directory`, `overwrite_directory`, `profiles_directory`, profile-local flags, and skip suffix/directory rules.
- **D-06:** Mirror reference MO2 discovery for portable and instance-based installs: check `portable.txt`/`ModOrganizer.ini` beside the executable first, then `HKCU\Software\Mod Organizer Team\Mod Organizer` `CurrentInstance` under `LOCALAPPDATA`.
- **D-07:** If MO2 is running but its INI is missing, incomplete, or points to a non-Fallout game, return a manager-specific typed error with the reference message text rather than panicking or silently falling through.
- **D-08:** Vortex scope in Phase 3 is detection only: manager display name, executable path, parsed version, and `0.0.0` fallback. Do not add Vortex staging/config parsing beyond the current Python reference placeholder.

### Worker Event Shape
- **D-09:** Use a shared worker event envelope plus typed payload variants. The envelope should carry task identity/kind/status metadata; payload variants can carry discovery, scan, patch, download, external-process, cancellation, and error data.
- **D-10:** Define typed task kinds for discovery, scan, patch, download, external process, and generic/unknown even though most workflows are implemented later.
- **D-11:** Progress events should support optional human-readable progress text plus optional current/total counts. Do not require percentages, rates, or ETA in Phase 3.
- **D-12:** Cancellation should distinguish a cancellation request/acknowledgement from final cancelled completion so later UI can show pending cancellation separately from stopped work.

### Failure Reporting
- **D-13:** Known discovery and adapter failures should return typed error kinds plus user-facing messages. Reference-compatible messages are required for known Fallout 4 discovery failures.
- **D-14:** User-facing output should use known/reference messages where available. Raw OS errors, stack details, and incidental paths should stay in diagnostics/logging unless a reference message intentionally includes a path, such as invalid registry guidance.
- **D-15:** Non-Windows real platform operations should return explicit typed `UnsupportedPlatform`-style errors while fake-backed tests and public domain models remain usable cross-platform.
- **D-16:** Process/desktop launch/open failures should surface as typed action result events with operation kind, target, success/failure, and safe message. Adapters must not show dialogs directly or log-only failures.

### the agent's Discretion
No selected area was delegated to the agent. The planner still owns exact module names, trait signatures, dependency choices, and test layout, as long as the decisions above and `03-SPEC.md` are satisfied.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Planning And Requirements
- `.planning/ROADMAP.md` — Defines Phase 3 goal, dependencies, requirements, and success criteria.
- `.planning/REQUIREMENTS.md` — Defines DISC/SAFE requirement families and downstream phase dependencies.
- `.planning/PROJECT.md` — Defines project-wide port goals, read-only `CMT/` constraint, Rust/Slint direction, and quality gates.
- `.planning/phases/03-platform-discovery-background-adapters/03-SPEC.md` — Locked Phase 3 requirements, boundaries, constraints, and acceptance criteria. MUST read before planning.
- `.planning/phases/01-slint-shell-port-architecture/01-CONTEXT.md` — Prior decisions for Slint shell layout and module boundaries.
- `.planning/phases/02-settings-defaults-parity/02-CONTEXT.md` — Prior decisions for settings path, controller behavior, injectable IO, and reference-compatible persistence.

### Python Reference Source
- `CMT/src/game_info.py` — Source of truth for Fallout 4 discovery order, game path normalization, `Data`/F4SE derived paths, game INI loading, registry paths, and not-found messages.
- `CMT/src/utils.py` — Source of truth for `is_fo4_dir`, file/directory compatibility helpers, parent-process mod-manager detection, version fallback, environment paths, registry reads, and desktop/open helper behavior.
- `CMT/src/mod_manager_info.py` — Source of truth for MO2 INI parsing, `%BASE_DIR%` substitution, profile/staging/overwrite paths, skip rules, and reference error messages.
- `CMT/src/cm_checker.py` — Source of truth for where `GameInfo` is constructed, tab lifecycle integration, update checks, and current app-level state expectations.

### Existing Rust Code
- `src/app/mod.rs` — Existing application shell contracts and tab-label traceability pattern.
- `src/app/settings_controller.rs` — Existing controller pattern for UI-facing state, immediate persistence, and save-failure reversion.
- `src/domain/mod.rs` — Existing public domain module boundary where typed discovery/manager state should be added.
- `src/platform/settings_store.rs` — Existing injectable platform IO pattern and fake-backed test style to mirror for filesystem/process/desktop seams.
- `src/workers/mod.rs` — Existing inert worker boundary that Phase 3 should replace/extend with typed worker event contracts and handoff seams.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SettingsStore` and `AssetResolver` in `src/platform/settings_store.rs`: useful pattern for injectable adapters with production and fake/static implementations.
- `SettingsController` in `src/app/settings_controller.rs`: useful pattern for UI-facing controller state that keeps domain/platform behavior outside Slint markup.
- `WorkerRuntime` marker in `src/workers/mod.rs`: placeholder seam explicitly reserved for scan, patch, download, subprocess orchestration, and Slint-safe handoff.

### Established Patterns
- Rust code uses typed domain models and adapter boundaries rather than passing unstructured strings through Slint.
- Public functions/types and non-obvious behavior are documented with Rust doc comments.
- Tests use fake or isolated filesystem inputs rather than touching real user state.
- Existing UI callbacks are routed through Rust controllers instead of embedding domain behavior in `.slint` files.

### Integration Points
- Add discovery and manager domain models under `src/domain/` and expose them through the module boundary currently used by settings.
- Add filesystem/process/desktop adapters under `src/platform/`, following the injected production/fake pattern from settings IO.
- Extend `src/workers/` from an inert marker into the shared event contract and event-loop handoff seam.
- Later UI phases should consume owned domain events/results through app/controller code, not mutate Slint models from worker threads.

</code_context>

<specifics>
## Specific Ideas

- Keep Phase 3 non-visual even though the roadmap has `UI hint: yes`; this phase defines contracts and seams that later UI phases use.
- Favor reference parity over platform redesign: known reference text such as `A Fallout 4 installation could not be found.` and invalid registry path guidance must be preserved for known discovery failures.
- Slint handoff should pass owned domain events into the event loop; Slint handles/models should not cross worker boundaries.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-platform-discovery-background-adapters*
*Context gathered: 2026-05-17*
