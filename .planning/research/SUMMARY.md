# Project Research Summary

**Project:** CMT Rust/Slint port  
**Domain:** Faithful native desktop port of the Collective Modding Toolkit Fallout 4 utility  
**Researched:** 2026-05-16  
**Confidence:** HIGH

## Executive Summary

This project is a behavior-preserving Rust/Slint port of the Python/Tkinter Collective Modding Toolkit for Fallout 4. Experts should build it as a vertical-slice desktop port, not as a redesign: keep the original tab order, labels, defaults, validation rules, status messages, and workflows, while replacing Python/Tkinter internals with typed Rust domain services and declarative Slint UI components.

The recommended approach is to establish the Slint shell and Rust architecture first, then port shared settings, platform adapters, game/mod-manager discovery, and typed domain models before building feature tabs. Slint should only render state and emit callbacks; Rust controllers, domain services, filesystem/process adapters, and workers should own behavior. Long scans, update checks, downloads, patching, and process work must run off the UI thread and marshal owned results back through Slint-safe event-loop handoffs.

The largest risks are fidelity drift, incorrect Fallout 4/mod-manager discovery, scanner taxonomy loss, UI-thread blocking, and unsafe destructive file operations. Mitigate them by treating `CMT/` as read-only reference material, adding parity tests/fixtures for settings/discovery/scanner/archive behavior, preserving typed problem/action enums instead of strings, and isolating auto-fix/downgrade/patcher workflows behind dry-run plans, backups, and fail-closed write checks.

## Key Findings

### Recommended Stack

Use Rust stable with edition 2024, Slint 1.16.1, and `slint-build` 1.16.1 as the UI foundation. The crate already targets Rust 2024, and Slint's `.slint` files plus `build.rs`/`slint::include_modules!()` fit the desired separation between UI structure and Rust behavior. Tokio 1.52.3 should provide background orchestration for scans, downloads, update checks, and subprocess monitoring, with blocking filesystem/patch work moved to worker threads.

Supporting crates should stay focused: `serde`/`toml` or similar for settings persistence, `directories` for app/user paths, `walkdir` for traversal, `tracing`/`tracing-subscriber` for diagnostics, `rfd` for dialogs, and BA2/module parsing support where it directly matches reference behavior. Avoid broad GUI abstraction layers, webview rewrites, Python interop, or stringly typed state maps.

**Core technologies:**
- Rust stable, edition 2024: application language/runtime — single native binary, strong typing, and predictable filesystem/process behavior.
- Slint 1.16.1: native desktop UI — supports a faithful tabbed UI with external `.slint` files.
- `slint-build` 1.16.1: build-time UI compilation — keeps generated Slint modules out of hand-written Rust.
- Tokio 1.52.3: background orchestration — keeps scans, downloads, and process work off the UI thread.
- `slint::Weak`/`upgrade_in_event_loop`/`invoke_from_event_loop`: UI-thread handoff — required for safe background-to-Slint updates.

### Expected Features

The initial launch must recreate the reference application's visible shape and read-oriented diagnostics before expanding mutation-heavy workflows. Users expect the same window identity, tab order (`Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`), lazy/refresh behavior, settings defaults, game and mod-manager discovery, overview summaries, F4SE DLL compatibility table, scanner settings/execution/results/details, and link/tool actions.

**Must have (table stakes):**
- Fixed desktop shell and tab order — establishes this as CMT, not a new tool.
- Settings persistence/defaults — update source, log level, scanner toggles, and downgrader backup/cleanup settings must match the reference.
- Startup game, PC, and mod-manager discovery — Overview, F4SE, Scanner, Downgrader, and Patcher all depend on it.
- Overview diagnostics — binaries, archives, modules, update notification, and problem aggregation.
- F4SE DLL scan table — preserve compatibility/status semantics and display behavior.
- Scanner settings, execution, results, details, and actions — preserve problem taxonomy, solution types, progress/status behavior, and detail panes.
- Tools/About links and attribution — preserve labels, URLs, and visible launch failures.
- Responsive worker architecture and typed domain/test fixtures — required for a reliable Rust port, even if invisible to users.

**Should have (competitive):**
- Responsive scans and parsing — improves perceived quality without changing workflows.
- Typed domain models for settings, discovery, scan results, and tools — reduces parity drift and enables tests.
- Golden/reference tests for scanner/archive/module classification — prevents regressions.
- Better error containment for unreadable files, invalid settings, and failed process/link launches — preserve messages while making failures explicit.
- Conservative Slint visual fidelity — close layout, grouping, spacing, labels, and enabled states.

**Defer (v2+):**
- Workflow redesigns or product-direction changes — violates the port goal.
- Live filesystem monitoring/auto-rescan — adds race/cancellation complexity absent from the reference.
- Cross-game support — CMT is Fallout 4-specific.
- Full Vortex staging support beyond existing behavior — useful but high-risk and not core parity.
- New scanner checks, CLI/headless mode, plugin installers, or Python compatibility layers — not essential for faithful launch.

### Architecture Approach

Use a layered architecture: Slint owns layout and view callbacks, a Rust controller bridges UI and application state, typed domain services implement settings/discovery/scanner/archive/module/tool behavior, platform adapters isolate filesystem/process/OS dependencies, and workers run slow jobs before returning typed events to the UI thread. The key rule is to port behavior into testable Rust services first, then project those services into Slint-friendly view models.

**Major components:**
1. `ui/main.slint` and per-tab `.slint` components — window identity, tab order, labels, layout, enabled states, and Slint callbacks/properties.
2. `src/app/controller.rs` — the only Slint bridge; wires callbacks, owns weak handles, and updates models on the UI thread.
3. `src/app/state.rs` and `src/app/view_models.rs` — typed application snapshot plus mappings to Slint row structs/enums/strings.
4. `src/domain/settings.rs` — reference-compatible defaults, validation, migration, and persistence.
5. `src/domain/discovery.rs` — Fallout 4 install, executable/version, MO2/Vortex, archive, and module discovery.
6. `src/domain/scanner.rs`, `archive.rs`, and `module.rs` — pure diagnostic/classification logic and metadata parsing.
7. `src/domain/tools.rs` — external tools, URLs, process specs, downgrade, auto-fix, and patcher contracts.
8. `src/platform/fs.rs`, `process.rs`, and `logging.rs` — injectable OS boundaries for tests and reliable failures.
9. `src/workers/mod.rs` — background work, progress/completion/failure events, and Slint-safe handoff.

### Critical Pitfalls

1. **Treating the port as a redesign** — avoid by comparing every tab/dialog/workflow against `CMT/src/` before implementation and preserving labels, order, defaults, and messages.
2. **Editing or generating files under `CMT/`** — avoid by making the submodule read-only and adding a `git status --short CMT` safety check during planning/review.
3. **Scanner correctness drift from lossy path/mod-manager modeling** — avoid by preserving typed scan result/problem/solution models and testing relative paths, mod attribution, enabled state, and unreadable cases.
4. **Misreading Fallout 4 install/settings/enabled-state sources** — avoid a dedicated discovery phase with injectable registry/environment/filesystem seams and fixtures for Steam, GOG, MO2, Vortex, `plugins.txt`, INI, and Creation Club list handling.
5. **Blocking or mutating Slint UI state from worker threads** — avoid by using Tokio/background workers and `Weak::upgrade_in_event_loop`/`invoke_from_event_loop` for all UI updates.
6. **Unsafe destructive operations** — avoid by isolating auto-fix, downgrade, and archive patcher workflows behind dry-run operation plans, default backups, explicit byte/version checks, sandbox tests, and fail-closed writes.
7. **Settings/update behavior drift** — avoid by porting defaults and validated loader before feature consumers, including all update-source enum values (`All`, `Early`, `Stable`, `Never`).

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Project/Slint Shell Foundation
**Rationale:** Every later slice depends on a stable Slint compile pipeline, module layout, tab order, and UI-thread handoff convention.  
**Delivers:** `slint`/`slint-build`, `build.rs`, `ui/main.slint`, generated `MainWindow`, exact reference tab order/window identity, and module skeletons for `app`, `domain`, `platform`, and `workers`.  
**Addresses:** Shell/tab order/window identity; conservative visual fidelity baseline.  
**Avoids:** Redesign drift, CMT submodule mutation, and Slint-first business logic.

### Phase 2: Typed Foundation, Settings, and Platform Adapters
**Rationale:** Settings and OS seams are shared dependencies for discovery, scanner, tools, update behavior, and destructive workflows.  
**Delivers:** Typed settings/defaults/persistence, app paths, filesystem/process/logging traits, fake adapters, and tests for defaults/round trips.  
**Addresses:** Settings persistence/defaults, typed domain model/test fixtures, reliable link/process failures.  
**Avoids:** Wrong defaults, update-source drift, untestable filesystem/process code, and generic panic-based launch failures.

### Phase 3: Game and Mod-Manager Discovery Core
**Rationale:** Overview, F4SE, Scanner, Downgrader, Archive Patcher, and update/problem aggregation all inherit their truth from discovery.  
**Delivers:** Reference-compatible game path/version detection, Documents/AppData parsing, registry/environment abstraction, MO2/Vortex state, archive/module inventories, warning states, and fixtures.  
**Addresses:** Startup game/PC/mod-manager discovery and shared state for diagnostics.  
**Avoids:** Wrong install/profile paths, missing `plugins.txt`/CC warnings, bad enabled-state counts, and unsafe operations in the wrong directory.

### Phase 4: Overview and Update Notification Vertical Slice
**Rationale:** Overview validates discovery models, status aggregation, update-source handling, and Slint view-model projection before scanner complexity.  
**Delivers:** Overview panels for game/mod-manager/PC specs, binaries, archives, modules, problem aggregation, helper actions, and non-blocking update checks respecting update source.  
**Addresses:** Overview diagnostics and update notification banner.  
**Avoids:** Incomplete discovery-driven counts, UI blocking during update checks, and link/open failure silence.

### Phase 5: Settings UI and Shared User Actions
**Rationale:** Settings consumers need a canonical persisted model, and Tools/About share external action behavior with Overview/Scanner details.  
**Delivers:** Reference-compatible Settings tab layout/labels/options, Tools tab links/actions, About text/attribution/URLs, clipboard/open helpers, and user-visible failures.  
**Addresses:** Settings UI, Tools/About links, attribution fidelity.  
**Avoids:** Settings migration/default drift, About text drift, and direct `Command::spawn().unwrap()` in callbacks.

### Phase 6: F4SE Diagnostics
**Rationale:** F4SE is a focused diagnostic slice that depends on discovery/module models but is simpler than full Scanner execution.  
**Delivers:** F4SE DLL scan table, compatibility/status rows, detail affordances, and tests for version/status classification.  
**Addresses:** F4SE DLL compatibility scan table.  
**Avoids:** DLL compatibility drift and generic string-only status rendering.

### Phase 7: Scanner Engine, Results, Details, and Actions
**Rationale:** Scanner is the highest-value read-only parity feature and needs all prior foundations: settings, discovery, typed taxonomy, workers, and view models.  
**Delivers:** Scanner toggles, explicit `Scan Game` execution, disabled/progress/reenable behavior, result table, details panes, problem/solution enums, URL/details actions, and golden tests.  
**Addresses:** Scanner settings/execution/results/details and reference problem taxonomy.  
**Avoids:** `Vec<String>` scan results, auto-fix eligibility based on message text, UI stalls, stale Overview-derived problem data, and lost mod attribution.

### Phase 8: Mutation Workflows — Auto-Fix, Downgrade Manager, Archive Patcher
**Rationale:** Destructive workflows should come after diagnostic parity is stable because they require stronger safety, backup, dry-run, and fixture coverage.  
**Delivers:** Existing auto-fix actions, Downgrade Manager, Archive Patcher metadata parsing/byte patching, backup/cleanup settings, progress/status feedback, and sandboxed tests.  
**Addresses:** P2 features from the prioritization matrix.  
**Avoids:** Unrecoverable writes, wrong-directory patching, BA2 byte corruption, blocking downloads, and update/download source divergence.

### Phase 9: Packaging, Verification, and Deferred Enhancements
**Rationale:** Once parity is complete, harden distribution and explicitly decide which v1.x/v2 items are worth adding.  
**Delivers:** Resource lookup validation, packaging smoke tests, complete verification gates, and scoped backlog entries for Vortex improvements, new scanner checks, cancellation, or CLI/headless mode.  
**Addresses:** Packaging/resource parity and future roadmap hygiene.  
**Avoids:** Shipping after only `cargo check`, resource path drift, and accidental expansion beyond faithful parity.

### Phase Ordering Rationale

- Build shell and handoff conventions first so every vertical slice uses the same Slint/Rust boundary.
- Port settings and platform seams before discovery because every real workflow needs configuration, app paths, filesystem traversal, process launching, and test fakes.
- Port discovery before feature tabs because incorrect install/profile/module state poisons Overview, F4SE, Scanner, Downgrader, and Patcher.
- Ship read-only diagnostics before mutation workflows so destructive operations are based on trusted typed state and tested classification.
- Group Tools/About/Settings near the foundation because they are low-risk parity work and establish shared URL/folder/clipboard behavior used elsewhere.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3: Game and Mod-Manager Discovery Core** — Windows registry, Steam/GOG path detection, MO2/Vortex layouts, `plugins.txt`, INI, Creation Club, and enabled-state behavior need per-slice source review and fixtures.
- **Phase 7: Scanner Engine, Results, Details, and Actions** — reference scanner taxonomy, mod attribution, details/actions, and auto-fix eligibility are complex enough to warrant focused research before implementation.
- **Phase 8: Mutation Workflows** — downgrade, auto-fix, downloads, and BA2 patching are destructive and need exact source behavior, safety contracts, and byte-level fixture validation.
- **Phase 9: Packaging** — Slint resource lookup and Windows distribution details may need targeted research once the app shape is known.

Phases with standard patterns (skip research-phase unless surprises appear):
- **Phase 1: Project/Slint Shell Foundation** — Slint build setup, `TabWidget`, and `include_modules!()` patterns are well documented.
- **Phase 2: Typed Foundation, Settings, and Platform Adapters** — standard Rust settings/adapter/testing patterns, with source review focused on defaults rather than external research.
- **Phase 5: Settings UI and Shared User Actions** — mostly direct reference UI parity plus standard platform-open abstractions.
- **Phase 6: F4SE Diagnostics** — likely standard once discovery/module parsing is in place, though classification fixtures still matter.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Slint, Tokio, Walkdir, Serde, and tracing guidance came from official docs/Context7 plus crates.io version checks and current repo constraints. |
| Features | HIGH | Feature inventory is grounded in `.planning/PROJECT.md`, `AGENTS.md`, and the read-only Python/Tkinter reference files under `CMT/src/`. |
| Architecture | HIGH | Component boundaries follow project instructions, Slint threading rules, and direct reference-app responsibilities; implementation details still require per-slice source review. |
| Pitfalls | HIGH | Risks are derived from explicit project constraints plus source files for settings, discovery, scanner, tabs, auto-fixes, downgrade, and patcher workflows. |

**Overall confidence:** HIGH

### Gaps to Address

- **Exact UI geometry/styling:** Research identifies labels/order/workflows, but each tab needs side-by-side source review and Slint implementation checks for spacing, grouping, and enabled states.
- **Windows registry and real install edge cases:** Discovery should be validated with fixtures and, where possible, manual UAT against Steam/GOG/MO2/Vortex environments.
- **BA2/module parser fidelity:** Confirm crate behavior and byte-level parsing against reference expectations before enabling writes.
- **Network/update semantics:** Preserve update source behavior and failure handling, but exact Nexus/GitHub response handling should be rechecked during the update slice.
- **Packaging/resource lookup:** Verify final Windows packaging and asset lookup after Slint UI/assets exist.

## Sources

### Primary (HIGH confidence)
- `.planning/PROJECT.md` — project goal, scope, out-of-scope constraints, and roadmap inputs.
- `AGENTS.md` — read-only `CMT/`, Rust/Slint direction, UI fidelity, threading, verification, and git safety rules.
- `CMT/src/cm_checker.py` and `CMT/src/main.py` — window identity, tab construction, lifecycle, and update behavior.
- `CMT/src/tabs/*.py` — Overview, F4SE, Scanner, Tools, Settings, and About UI/workflow parity requirements.
- `CMT/src/app_settings.py` and `CMT/src/scan_settings.py` — settings keys/defaults, validation, persistence, and scanner toggles.
- `CMT/src/game_info.py` and `CMT/src/mod_manager_info.py` — Fallout 4 path/version, registry/INI/AppData handling, MO2/Vortex detection, archives, modules, and warnings.
- `CMT/src/autofixes.py`, `CMT/src/downgrader.py`, and `CMT/src/patcher/*.py` — destructive workflow and archive patching risks.
- Slint Rust docs via Context7 (`/websites/slint_dev_rust_slint`) — `slint-build`, external `.slint` compilation, generated modules, `Weak::upgrade_in_event_loop`, and `invoke_from_event_loop`.
- Slint widget docs via Context7 (`/websites/slint_dev_slint`) — `TabWidget` and standard widget guidance.
- Tokio docs via Context7 (`/websites/rs_tokio_1_49_0`) — `spawn_blocking`, channel bridging, and filesystem caveats.
- Walkdir docs via Context7 (`/burntsushi/walkdir`) — traversal, filtering, symlink handling, and errors.
- Serde docs via Context7 (`/websites/serde_rs`) — `Serialize`/`Deserialize` derive and dependency setup.
- tracing-subscriber docs via Context7 (`/websites/rs_tracing-subscriber`) — `EnvFilter`, formatting, and logging layers.
- crates.io checks on 2026-05-16 — current crate versions including `slint 1.16.1`, `slint-build 1.16.1`, `tokio 1.52.3`, `walkdir 2.5.0`, `serde 1.0.228`, `toml 1.1.2`, `tracing 0.1.44`, `rfd 0.17.2`, and `ba2 3.0.1`.

### Secondary (MEDIUM confidence)
- Current Rust crate files (`Cargo.toml`, `src/main.rs`) — useful for bootstrapping constraints, but the port is still early and subject to planned refactoring.
- Slint language/editor tooling guidance — helpful for implementation ergonomics, not central to behavior.

### Tertiary (LOW confidence)
- None currently identified. Most conclusions come from first-party project/reference files or official library documentation.

---
*Research completed: 2026-05-16*  
*Ready for roadmap: yes*
