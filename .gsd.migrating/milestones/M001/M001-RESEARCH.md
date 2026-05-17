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

# Architecture Patterns

**Domain:** Rust/Slint desktop port of Collective Modding Toolkit  
**Researched:** 2026-05-16  
**Overall confidence:** HIGH for Slint threading/model guidance and repository constraints; MEDIUM for detailed reference behavior until each tab is ported line-by-line.

## Recommended Architecture

Build the port as a layered desktop application with Slint responsible only for view structure and presentation, Rust controllers responsible for UI callback wiring and state projection, and pure Rust domain services responsible for all reference-compatible behavior.

```
ui/*.slint
  ↓ callbacks / typed properties / models
src/app/* controller + state projection
  ↓ commands, events, immutable snapshots
src/domain/* reference-compatible logic
  ↓ filesystem/process traits
src/workers/* background execution + UI-thread handoff
  ↓
OS filesystem, game install, MO2/Vortex metadata, external tools
```

The key rule is one-way ownership: UI actions create typed commands; controllers validate and dispatch commands; domain services produce typed results; workers return events; the controller updates Slint properties/models on the Slint event loop. Domain code must not know Slint exists, and Slint callbacks must not do filesystem scans, archive parsing, binary reads, subprocess execution, or long JSON/settings writes directly.

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| `ui/main.slint` and per-tab `.slint` components | Window identity, tab order (`Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`), labels, layout, enabled states, result tables/details panes | Exposes callbacks and typed properties/models to Rust controller |
| `src/main.rs` | Start app, include Slint modules, create services/state/controller, run event loop | `app::Controller`, generated Slint `MainWindow` |
| `src/app/controller.rs` | Own Slint weak handle wiring, translate UI callbacks into commands, update UI models on the UI thread | Slint UI, `AppState`, workers, domain services |
| `src/app/state.rs` | Typed application snapshot: settings, discovered game/mod-manager state, scan progress/results, selected result details, tool execution state | Controller, tab view-model mappers |
| `src/app/view_models.rs` | Convert domain structs into Slint-friendly row structs/enums/strings without losing reference labels/messages | `AppState`, generated Slint types |
| `src/domain/settings.rs` | Reference-compatible defaults and persistence for log level, update source, scanner toggles, downgrader backup/cleanup options | Filesystem adapter, controller/settings tab |
| `src/domain/discovery.rs` | Game path, executable/version discovery, MO2/Vortex detection, archive/module inventories corresponding to `game_info.py` and `mod_manager_info.py` | Filesystem adapter, Overview/F4SE/Scanner services |
| `src/domain/scanner.rs` | Pure scan orchestration and problem classification corresponding to `scan_settings.py`, `cm_checker.py`, and `tabs/_scanner.py` | Discovery results, filesystem adapter, worker events |
| `src/domain/archive.rs` | BA2/archive naming, counts, suffix/format validation, archive patcher metadata parsing | Overview, Scanner, Patcher services |
| `src/domain/module.rs` | ESM/ESL/ESP parsing, module version/status/counts, SADD scanning, plugin metadata | Overview, Scanner, F4SE-adjacent checks |
| `src/domain/tools.rs` | External tool definitions, URL/open actions, process launch specs, downgrade/archive patcher workflow contracts | Tools tab, process adapter |
| `src/workers/mod.rs` | Spawn cancellable background work, stream progress, marshal completed results back to the Slint event loop | Controller, domain services |
| `src/platform/fs.rs` | Small trait surface for path walking, reading bytes/text, metadata, environment variables, appdata paths | Domain services; fake implementation in tests |
| `src/platform/process.rs` | Launch external tools/URLs and run helper processes without blocking UI | Tools/downgrader/patcher services; fake implementation in tests |
| `src/platform/logging.rs` | Logging initialization and file/console policy from settings | Main/controller/domain diagnostics |

## Data Flow

### UI Command Flow

```
[User clicks Scan / Refresh / Tool button]
    ↓ Slint callback
[Controller]
    ↓ creates typed Command with current AppState snapshot
[Domain service or Worker]
    ↓ returns Result<Event, DomainError>
[Controller on Slint event loop]
    ↓ mutates AppState and Slint properties/models
[UI re-renders]
```

### Background Work Flow

```
[Controller]
    ↓ clones Send-safe settings/path snapshot
[Worker thread]
    ↓ runs discovery/scan/parser/process work with no Slint handles
[WorkerEvent::Progress / WorkerEvent::Finished / WorkerEvent::Failed]
    ↓ slint::Weak::upgrade_in_event_loop or slint::invoke_from_event_loop
[Controller]
    ↓ updates AppState and VecModel/ModelRc on UI thread
[Slint UI]
```

Slint documentation explicitly requires worker threads to hand UI updates back to the main event loop via `invoke_from_event_loop` or `Weak::upgrade_in_event_loop`; `ModelRc`/`VecModel` updates should happen on the UI thread because Slint models are not a general Send data structure. Therefore workers should return plain Rust data (`Vec<ScanProblem>`, `DiscoverySnapshot`, `ToolResult`) and the controller should replace or update Slint models inside the event-loop closure.

### State Management

Use a single Rust-owned `AppState` as the source of truth. Slint stores display state only: selected tab, current text, booleans for enabled/loading states, and model rows. Do not make Slint properties the canonical settings or scan-result store.

```
AppState
  ├─ SettingsState
  ├─ DiscoveryState
  ├─ OverviewState
  ├─ F4seState
  ├─ ScannerState
  ├─ ToolsState
  └─ AboutState/static content
```

Prefer immutable snapshots for worker input so a scan cannot observe half-updated UI settings. When settings are changed, update `AppState`, persist through `domain::settings`, then project the resulting snapshot back to Slint.

## Patterns to Follow

### Pattern 1: Reference-Compatible Domain Services

**What:** Port Python behavior into pure Rust modules before wiring it deeply into Slint. Preserve original names/messages where useful in tests and fixtures.

**When:** Settings defaults, game/mod-manager discovery, scanner problem detection, BA2/archive validation, module parsing, F4SE compatibility checks, downgrader/archive patcher workflows.

**Example:**

```rust
/// Scans a prepared game/mod-manager snapshot and returns reference-compatible problems.
///
/// The function performs no UI updates and does not read mutable application state while
/// running, which allows it to be tested with fixture directories and executed on a worker.
pub fn scan_data_files<F: FileSystem>(
    fs: &F,
    settings: &ScanSettings,
    discovery: &DiscoverySnapshot,
) -> Result<ScanReport, ScanError> {
    // Implementation ports CMT/src/tabs/_scanner.py and CMT/src/scan_settings.py behavior.
    todo!()
}
```

### Pattern 2: Controller as the Only Slint Bridge

**What:** Keep generated Slint types, `Weak<MainWindow>`, `ModelRc`, and `VecModel` inside `app` modules. Domain code returns plain Rust structs.

**When:** Every tab callback and worker completion.

**Example:**

```rust
/// Registers UI callbacks and ensures background results are applied on Slint's event loop.
pub fn wire_scan_callbacks(ui: &MainWindow, controller: ControllerHandle) {
    let ui_weak = ui.as_weak();
    ui.on_start_scan(move || {
        let input = controller.snapshot_for_scan();
        let ui_weak = ui_weak.clone();
        controller.spawn_scan(input, move |event| {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                controller.apply_scan_event(&ui, event);
            });
        });
    });
}
```

### Pattern 3: Filesystem and Process Adapters

**What:** Hide `std::fs`, environment lookup, path walking, URL opening, and subprocess spawning behind small traits.

**When:** Anything that touches the game directory, MO2/Vortex config, external tools, archive/module bytes, or OS shell.

**Why:** This creates test seams that avoid launching a window, avoids accidental CMT submodule writes, and lets phase tests cover domain behavior with temporary fixture directories.

### Pattern 4: Tab Slices with Shared Core First

**What:** Build common state/services before porting tab UI. Each tab should be a vertical slice that adds the Slint surface, controller callbacks, domain logic, and tests for that workflow.

**When:** Roadmap planning and phase ordering.

**Why:** Overview, F4SE, Scanner, Tools, and Settings all depend on shared settings/discovery models. Building UI first without typed domain models would cause callback sprawl and later rewrites.

## Suggested Build Order and Dependencies

1. **Project/Slint shell foundation**
   - Add `slint` and `slint-build`, `build.rs`, `ui/main.slint`, and a minimal generated `MainWindow` with the reference tab order.
   - Establish `src/app`, `src/domain`, `src/platform`, and `src/workers` module skeletons.
   - Dependency reason: every later phase needs stable component boundaries and UI-thread handoff conventions.

2. **Typed models, settings, and platform adapters**
   - Define settings/domain structs, scanner toggle defaults, app paths, filesystem/process traits, and fake adapters for tests.
   - Port settings persistence before Settings UI so other tabs can consume one canonical configuration.
   - Dependency reason: discovery/scanner/tools all need settings and filesystem seams.

3. **Game and mod-manager discovery core**
   - Port `game_info.py` and `mod_manager_info.py` behavior into `domain::discovery` with tests for game path selection, MO2 ini parsing, Vortex/MO2 status, archives, and modules.
   - Dependency reason: Overview, F4SE, Scanner, and tool workflows all depend on knowing game/mod-manager paths and inventories.

4. **Overview tab vertical slice**
   - Implement Overview Slint layout and controller projection for binaries, archive counts/status, module counts/status, update prompts, and helper actions.
   - Dependency reason: this validates discovery models and UI projection without the full scanner complexity.

5. **Settings tab vertical slice**
   - Implement settings UI after the underlying settings store exists; wire changes into `AppState` and persistence.
   - Dependency reason: scanner and downgrader phases need persisted toggles/options.

6. **Scanner engine and Scanner tab**
   - Port scan settings, mod-file list building, problem taxonomy, progress events, result tree rows, details pane data, URL/details/copy actions, and auto-fix feedback.
   - Run scans through `workers`; never traverse directories or parse modules on the Slint thread.
   - Dependency reason: scanner needs discovery, settings, archive/module parsing, and worker handoff patterns.

7. **F4SE tab vertical slice**
   - Port DLL scanning table and compatibility status using discovery/scanner filesystem utilities.
   - Dependency reason: F4SE is narrower than Scanner but benefits from established table models and background scan conventions.

8. **Tools tab and external workflow contracts**
   - Port static external tool buttons/links first, then Downgrade Manager and Archive Patcher workflows.
   - Use `platform::process` for launches and worker jobs for archive patching/downgrade operations.
   - Dependency reason: tool workflows depend on settings, discovery, archive parsing, and process adapter seams.

9. **About tab and final fidelity pass**
   - Port static attribution/link text and Discord invite actions.
   - Run a tab-by-tab comparison against `CMT/src/tabs/*.py` for labels, order, defaults, enabled states, and messages.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Slint-First Business Logic

**What:** Implementing scan/discovery logic inside generated Slint callbacks or storing canonical settings/results only as Slint properties.
**Why bad:** Hard to test without a window, impossible to reuse across tabs, likely to block the UI thread, and prone to reference-behavior drift.
**Instead:** Keep callbacks thin and call pure Rust domain services through the controller.

### Anti-Pattern 2: Passing Live UI Handles to Workers

**What:** Moving strong Slint component handles or `ModelRc` into worker threads.
**Why bad:** Slint UI updates must happen on the event loop; models are not a general cross-thread state store.
**Instead:** Workers emit plain Rust events; controller applies them using `Weak::upgrade_in_event_loop` or `slint::invoke_from_event_loop`.

### Anti-Pattern 3: Untyped String Maps for Reference Data

**What:** Mirroring Python dictionaries with `HashMap<String, String>` everywhere.
**Why bad:** Loses invariants for archive formats, module versions, scan problem categories, mod-manager kind, and settings defaults.
**Instead:** Use Rust enums/structs at the domain boundary and convert to display strings only in view-model mappers.

### Anti-Pattern 4: Rewriting Workflows While Porting

**What:** Modernizing tab flows, labels, grouping, or external tool behavior before the reference app is faithfully reproduced.
**Why bad:** Violates the project goal and makes acceptance ambiguous.
**Instead:** Preserve reference workflows first; document intentional differences later.

## Testing Seams

| Seam | Test Without Window | Notes |
|------|---------------------|-------|
| Settings persistence | Temp directory + fake app path provider | Assert defaults and round-trip values match reference expectations |
| Game/mod-manager discovery | Fixture directories and fake environment variables | Cover MO2 ini parsing, game executable discovery, missing path errors |
| Archive/module parsing | Byte/file fixtures under test data | Assert BA2/module counts, unreadable handling, naming validation |
| Scanner engine | Fake filesystem or temp staged mods | Assert problem categories, relative paths, mod attribution, and messages |
| Worker orchestration | Unit test worker event streams with fake long-running jobs | Assert progress/finished/failed order and cancellation behavior |
| View-model mapping | Pure mapping tests from domain snapshots to Slint row structs | Assert labels, ordering, severity colors/classes before launching UI |
| Process launching | Fake process adapter | Assert intended executable/URL/arguments without running external tools |

Integration smoke tests can instantiate the controller with fake services, but most coverage should live below Slint. Visual/UI fidelity should be checked per tab by comparing against the relevant `CMT/src/tabs/*.py` source during implementation.

## Scalability Considerations

| Concern | Small mod list | Large mod list / many archives | Mitigation |
|---------|----------------|-------------------------------|------------|
| Filesystem traversal | Synchronous domain tests are fine | UI can freeze if run directly | Always execute scans/discovery refreshes in workers |
| Result tables/trees | Replace whole model acceptable | Large result sets may stutter | Batch progress events and replace Slint models on event loop boundaries |
| Binary/module parsing | Fast for few files | Many modules/archives can be CPU and I/O heavy | Parse on workers; cache discovery snapshots when safe |
| External tools | Simple URL/process launch | Long-running patch/downgrade workflows | Process adapter + progress events + cancellation/status state |
| Settings writes | Immediate save acceptable | Repeated toggles can produce redundant writes | Debounce or save on apply/close if reference behavior allows |

## Integration Points

### External Services and OS Interfaces

| Integration | Architecture Boundary | Notes |
|-------------|-----------------------|-------|
| Game install directory | `platform::fs` + `domain::discovery` | Must handle missing/invalid paths with reference-compatible messages |
| MO2/Vortex metadata | `domain::discovery` | MO2 ini parsing and mod staging paths feed Overview/Scanner |
| BA2 archives and ESM/ESL/ESP modules | `domain::archive`, `domain::module` | Keep parsers UI-independent and fixture-tested |
| Browser/Discord/GitHub/Nexus links | `platform::process` or `open` wrapper | Tools/About actions should be injectable in tests |
| Downgrader/archive patcher helpers | `domain::tools` + workers + process/fs adapters | Treat as long-running workflows with progress/failure states |

### Internal Boundaries

- `ui` may depend on generated Slint types only.
- `app` may depend on Slint and all domain/platform modules.
- `domain` may depend on Rust data types and platform traits, but not Slint.
- `platform` may depend on OS crates/APIs, but should expose narrow traits.
- `workers` may depend on domain/platform modules and send plain events back to `app`.

## Sources

- Repository project context: `.planning/PROJECT.md` (HIGH)
- Porting constraints: `AGENTS.md` (HIGH)
- Current Rust crate state: `Cargo.toml`, `src/main.rs` (HIGH)
- Python reference files inspected: `CMT/src/main.py`, `CMT/src/cm_checker.py`, `CMT/src/tabs/*.py`, `CMT/src/game_info.py`, `CMT/src/mod_manager_info.py`, `CMT/src/scan_settings.py`, `CMT/src/patcher/*.py` (HIGH as reference source; detailed behavior still requires per-slice port review)
- Slint Rust documentation via Context7: `slint-build` setup with `build.rs`/`include_modules!`, worker-to-UI updates via `slint::invoke_from_event_loop`, `Weak::upgrade_in_event_loop`, and `ModelRc`/`VecModel` update guidance (HIGH)

# Stack Research

**Domain:** Native Rust/Slint desktop port of a Python/Tkinter Fallout 4 modding utility  
**Project:** `cmt-rs` / Collective Modding Toolkit Rust Port  
**Researched:** 2026-05-16  
**Confidence:** HIGH for Rust/Slint/UI and general Rust crates; MEDIUM for Fallout-specific archive crates because they are niche and need phase validation against the reference app's BA2 behavior.

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| Rust stable, edition 2024 | MSRV 1.85+ for edition 2024; use current stable in CI | Application language/runtime | The crate already uses `edition = "2024"`. Rust gives a single native binary, strong path/error typing, and predictable filesystem/process behavior for replacing Python/Tkinter without carrying a Python runtime. | HIGH |
| Slint | 1.16.1 | Native desktop UI | This is the project direction and the closest fit for a faithful desktop port with declarative `.slint` files. Slint's `TabWidget` maps directly to the reference Tk notebook tabs (`Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`). | HIGH |
| slint-build | 1.16.1 | Compile external `.slint` files at build time | Use external `.slint` files instead of inline UI macros so each tab/dialog can be ported as a readable vertical slice. Slint docs recommend `build.rs` + `slint_build::compile(...)` for larger UIs. | HIGH |
| Cargo build script | Rust std + `slint-build` | UI code generation | Add `build = "build.rs"` and compile `ui/main.slint`; include generated modules with `slint::include_modules!()`. This keeps Rust state/logic separate from Slint markup. | HIGH |
| Tokio runtime | 1.52.3 | Background orchestration, downloads, subprocess monitoring, UI-safe work dispatch | The reference app performs scans, update checks, downloads, and patching. Use a small multi-thread Tokio runtime for long-running orchestration and `spawn_blocking` for synchronous filesystem/patching work. Do not block the Slint event loop. | HIGH |
| `slint::Weak` / `upgrade_in_event_loop` / `invoke_from_event_loop` | Slint 1.16.x APIs | UI-thread handoff | Slint models/images are not generally `Send`; docs show updating models from background work by sending owned data back to the UI thread. This should be the standard handoff pattern for scanner/progress results. | HIGH |

### Supporting Libraries

| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| `serde` | 1.0.228 with `derive` | Typed settings and scan profile serialization | Use for `AppSettings`, scan toggles, remembered paths, and any future import/export. Prefer typed structs/enums over string maps. | HIGH |
| `toml` | 1.1.2 | Human-editable Rust settings format | Use for the new Rust config file. The Python reference uses JSON, but TOML is easier for users to inspect/edit and maps cleanly to typed Rust structs. Preserve defaults and user-facing setting names from the reference. | HIGH |
| `serde_json` | 1.0.149 | Compatibility/migration from Python settings | Use only if importing existing CMT JSON settings or consuming JSON release metadata. Do not make JSON the primary new config format unless compatibility requires it. | HIGH |
| `directories` | 6.0.0 | Platform-specific config/cache/log paths | Use for `ProjectDirs` instead of hard-coded relative files. Keep logs/config outside install directories unless reference behavior explicitly requires a local portable mode. | HIGH |
| `tracing` | 0.1.44 | Structured application logging | Replace Python `logging` while preserving user-facing log levels (`DEBUG`, `INFO`, `ERROR`). Use spans around scans, patching, downloads, and process execution. | HIGH |
| `tracing-subscriber` | 0.3.23 with `env-filter` | Log formatting/filtering and file output | Use a file layer for `cm-toolkit.log` equivalent and optionally `RUST_LOG`/configured filters for development. | HIGH |
| `thiserror` | 2.0.18 | Domain error enums | Use in library/domain modules where callers need to match specific failures: missing Data folder, invalid BA2 header, read-only file, registry lookup failure. | HIGH |
| `anyhow` | 1.0.102 | Top-level application errors | Use at app startup and task boundaries where contextual error chains matter more than matching exact variants. Avoid leaking raw `anyhow` through domain APIs. | HIGH |
| `walkdir` | 2.5.0 | Deterministic recursive filesystem traversal | Default scanner traversal crate. It supports filtering/pruning, depth limits, sorted traversal, symlink loop detection, and detailed per-entry errors. Prefer this over ad hoc recursive `std::fs` code. | HIGH |
| `jwalk` | 0.8.1 | Optional parallel directory walking | Consider only after scanner baselines are correct and performance data shows traversal is the bottleneck. Parallel traversal can reorder results and make UI diffs harder while preserving behavior. | MEDIUM |
| `ignore` | 0.4.25 | Optional path filtering engine | Use only if CMT grows user-defined ignore rules. The reference has explicit scanner whitelists/junk rules, so start with typed rule sets rather than `.gitignore` semantics. | MEDIUM |
| `crc32fast` | 1.5.0 | CRC32 checksums | Use for archive/file validation if matching the Python `zlib.crc32` behavior becomes necessary. Verify exact byte ranges against reference tests. | HIGH |
| `encoding_rs` | 0.8.35 | Non-UTF-8 text decoding | Use for plugin/config/log text that may not be UTF-8. The Python reference has encoded text helpers; Rust should not assume all mod files are UTF-8. | HIGH |
| `binrw` | 0.15.1 | Binary struct parsing | Use for future BA2/DLL/plugin binary readers when the format is stable enough to model. For the current archive patcher, a small explicit byte-level patcher may be safer. | MEDIUM |
| `byteorder` | 1.5.0 | Explicit endian reads/writes | Use for small binary header operations where `binrw` would be overkill, such as patching BA2 version bytes exactly like the reference. | HIGH |
| `ba2` | 3.0.1 | Bethesda BA2 archive library | Treat as a candidate for later scanner/archive-reader phases, not as an immediate core dependency. Validate against Fallout 4 BA2 variants and the reference patcher's exact behavior before adopting. | LOW-MEDIUM |
| `pelite` | 0.10.0 | Windows PE/DLL metadata parsing | Candidate replacement for Python `win32api` DLL version parsing in F4SE scanning. Validate it can read the same version resources and edge cases as the reference. | MEDIUM |
| `windows-registry` | 0.6.1 | Windows registry lookup | Use behind `cfg(windows)` for Fallout 4/Steam install discovery that currently uses Python `winreg`. Prefer the maintained `windows-*` ecosystem over older direct WinAPI bindings. | HIGH |
| `reqwest` | 0.13.3 with `rustls-tls` | HTTP update checks/downloads | Use for Nexus/GitHub update checks and downgrader downloads when porting those workflows. Keep network code out of UI modules. | HIGH |
| `rfd` | 0.17.2 | Native folder/file dialogs | Use for folder pickers if Slint's standard widgets are not sufficient for the exact reference workflow. Wrap it in an adapter so dialogs can be mocked in tests. | MEDIUM |
| `arboard` | 3.6.1 | Clipboard support | Use for About/Tools “Copy Link” behavior if Slint clipboard APIs do not cover the needed desktop behavior. | MEDIUM |
| `open` | 5.3.5 | Open URLs/files with system handlers | Use for reference `webbrowser.open(...)` behavior in About/Tools links. Keep URL constants typed and tested. | HIGH |
| `zip` | 8.6.0 | ZIP/FOMOD inspection | Use only if scanner phases need to inspect ZIP/FOMOD packages. Do not add for the initial UI shell. | MEDIUM |
| `sevenz-rust` | 0.6.1 | 7z archive inspection/extraction | Use only if needed for mod package inspection. The existing reference summary did not show direct 7z extraction in the main tabs, so defer. | LOW-MEDIUM |
| `tempfile` | 3.27.0 | Safe temporary files in tests and patch operations | Use for scanner/patcher tests and for atomic patch workflows that need temp output before replace. | HIGH |
| `assert_fs` | 1.1.3 | Filesystem test fixtures | Use in tests for scanner rules, settings migration, patcher edge cases, and missing/permission-denied paths. | HIGH |
| `insta` | 1.47.2 | Snapshot testing | Use for scanner result trees, settings serialization, and user-facing messages where fidelity to the Python reference matters. | HIGH |
| `clap` | 4.6.1 with `derive` | Optional developer CLI | Use for hidden/dev subcommands such as `cmt-rs scan --data <path>` only if it improves automated verification. Do not expose a CLI-first product unless requested. | HIGH |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo fmt --check` | Formatting gate | Required by project instructions. Run before considering a slice complete. |
| `cargo check` | Fast compile gate | Use after every vertical slice; Slint build errors surface here too. |
| `cargo test` | Domain behavior regression tests | Prioritize tests for settings defaults, path detection, scanner classifications, archive patch bytes, and F4SE version parsing. |
| `cargo clippy --all-targets --all-features` | Lint gate | Required by project instructions. Keep warnings actionable; do not suppress broadly. |
| Slint language tooling | `.slint` editing support | Use editor support where available, but keep generated Rust out of version control. |
| Snapshot fixtures from `CMT/` observations | Fidelity checks | Do not modify `CMT/`; encode observed labels/defaults/messages in Rust tests/fixtures outside the submodule. |

## Installation

Recommended starting point for the first UI shell + settings/scanner foundation:

```bash
# Core UI
cargo add slint@1.16.1
cargo add --build slint-build@1.16.1

# Runtime orchestration and typed app state
cargo add tokio@1.52.3 --features rt-multi-thread,sync,time,process
cargo add serde@1.0.228 --features derive
cargo add toml@1.1.2 serde_json@1.0.149 directories@6.0.0
cargo add tracing@0.1.44
cargo add tracing-subscriber@0.3.23 --features env-filter
cargo add anyhow@1.0.102 thiserror@2.0.18

# Filesystem scanning and binary helpers
cargo add walkdir@2.5.0 crc32fast@1.5.0 encoding_rs@0.8.35 byteorder@1.5.0

# Desktop integration
cargo add open@5.3.5 rfd@0.17.2 arboard@3.6.1

# Windows/Fallout-specific discovery and parsing candidates
cargo add windows-registry@0.6.1 --target 'cfg(windows)'
cargo add pelite@0.10.0 --target 'cfg(windows)'

# Test support
cargo add --dev tempfile@3.27.0 assert_fs@1.1.3 insta@1.47.2
```

Defer these until a phase proves the need:

```bash
# Optional later phases only
cargo add reqwest@0.13.3 --features rustls-tls,json,stream --no-default-features
cargo add binrw@0.15.1
cargo add ba2@3.0.1
cargo add zip@8.6.0 sevenz-rust@0.6.1
cargo add jwalk@0.8.1 ignore@0.4.25
cargo add clap@4.6.1 --features derive
```

Also add:

```toml
[package]
build = "build.rs"
```

and a minimal build script:

```rust
fn main() {
    slint_build::compile("ui/main.slint").expect("failed to compile Slint UI");
}
```

## Recommended Project Structure

Use a layout that keeps Slint UI, typed application state, and filesystem-heavy domain logic separate:

```text
src/
  main.rs                 # startup, tracing, runtime, Slint window wiring
  app.rs                  # app controller, callback registration, UI task bridge
  settings.rs             # AppSettings + ScanSettings typed defaults/load/save
  paths.rs                # Fallout/Data/mod-manager path discovery
  tasks.rs                # background task commands/events, cancellation handles
  scanner/
    mod.rs                # scanner orchestration
    rules.rs              # typed rules from reference scan_settings.py
    results.rs            # problem/result model for UI and tests
  f4se/
    mod.rs                # DLL discovery/version support matrix
  patcher/
    archive.rs            # BA2 version patching, byte-level tests
  desktop/
    dialogs.rs            # rfd wrapper
    links.rs              # open/arboard wrapper
ui/
  main.slint              # window + TabWidget shell
  overview.slint
  f4se.slint
  scanner.slint
  tools.slint
  settings.slint
  about.slint
tests/
  scanner_*.rs
  settings_*.rs
  archive_patcher_*.rs
```

### UI/Event Pattern

- Slint callbacks should enqueue typed commands (`ScanGame`, `RefreshOverview`, `PatchArchives`, `OpenLink`) rather than doing work inline.
- Background tasks should send progress/result events through channels and then update Slint properties/models only via `upgrade_in_event_loop` or `invoke_from_event_loop`.
- Store scanner output in Rust domain models first, then convert to `VecModel`/Slint structs at the UI boundary. This prevents Slint model types from contaminating scanner tests.

## Alternatives Considered

| Recommended | Alternative | Why Not / When to Use Alternative |
|-------------|-------------|-----------------------------------|
| Slint 1.16.x | egui/eframe | egui is excellent for immediate-mode tools, but this project needs a conservative Tkinter notebook-style port with stable layouts and close label/control ordering. Slint markup is a better fidelity target. |
| Slint 1.16.x | iced | Iced is a valid Rust GUI stack, but the repository direction already says Slint and Slint has direct `.slint` designer-friendly UI separation. Switching would add roadmap churn. |
| Slint 1.16.x | Tauri/Electron/web UI | Avoids native Rust GUI complexity but introduces web runtime/deployment complexity and diverges from the desktop-native Slint requirement. |
| `walkdir` | handwritten recursive `std::fs` traversal | Handwritten traversal tends to miss symlink loops, pruning, deterministic ordering, and per-path error reporting. Use `std::fs` only inside small, well-tested operations. |
| `walkdir` first | `jwalk` first | Parallel walking may be faster, but deterministic reference fidelity and simpler error reporting matter more during the port. Use `jwalk` only after profiling. |
| `tracing` | `log` + `env_logger` | `log` is fine for small apps, but `tracing` gives spans for long scans/downloads/patch operations and remains compatible with structured file logging. |
| `toml` new settings | Python-compatible JSON as primary config | JSON compatibility is useful for migration, but TOML is better for a new typed desktop config. Keep JSON importer only if users have existing settings to preserve. |
| `windows-registry` | `winreg` crate | `winreg` is widely used, but `windows-registry` aligns with the maintained `windows-*` ecosystem and current Windows API direction. |
| Custom BA2 patcher + tests | Adopt `ba2` crate immediately | The reference archive patcher changes specific version bytes and emits specific messages. A full archive crate may help later, but first preserve exact byte behavior with focused tests. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Editing or generating files under `CMT/` | `CMT/` is the read-only Python/Tkinter reference submodule and source of truth. Modifying it corrupts comparisons. | Read/inspect `CMT/`, then implement/test outside it. |
| Python/Tkinter runtime dependencies in Rust app | The port goal is a native Rust application. Requiring Python would preserve the old deployment problem. | Rust domain modules + Slint UI. |
| Blocking scans/patches/downloads on Slint callbacks | The reference scanner/downgrader already uses threading; blocking the Slint event loop would freeze the UI. | Tokio tasks, `spawn_blocking`, channels, and Slint event-loop handoff. |
| `unwrap()`/`expect()` in production scanner/patcher paths | Mod directories contain missing, locked, malformed, and non-UTF-8 files. Panics would make the tool unreliable. | `thiserror` domain errors and contextual `anyhow` at task boundaries. |
| Broad “full-feature” dependency additions up front | This port needs fidelity and small vertical slices. Extra dependencies obscure behavior and slow review. | Add optional crates only in the phase that proves the need. |
| SQLite or embedded DB for scanner state | The reference behavior is scan-and-display, not durable indexing. A DB would add migration/schema work without clear value. | In-memory typed models; serialize only settings/user choices. |
| Rayon/global parallelism by default | Parallel mutation/error ordering can make scanner output nondeterministic and harder to compare with Python. | Deterministic single traversal first; profile before parallelizing. |
| Generic ZIP/7z package parsing in the initial milestone | Package parsing is separate from matching the existing tabs/settings/scanner shell. | Defer `zip`/`sevenz-rust` to a specific mod-package inspection phase. |

## Stack Patterns by Variant

**If building the first faithful UI shell:**
- Use `slint`, `slint-build`, typed placeholder models, and no scanner/archive/network crates beyond `tracing` and `directories`.
- Because the first milestone should lock tab names, layout, settings defaults, callback boundaries, and UI-thread handoff before porting heavy behavior.

**If porting scanner filesystem rules:**
- Use `walkdir`, typed `ScanSettings`, `thiserror`, `assert_fs`, and `insta`.
- Because scanner fidelity depends on deterministic traversal, clear classification, and snapshot-visible result trees/messages.

**If porting F4SE/DLL detection:**
- Use `pelite` on Windows plus fallback byte/resource tests from reference examples.
- Because Python currently relies on Windows-specific version/resource APIs; cross-platform PE parsing should be validated before replacing it.

**If porting archive patching:**
- Start with `byteorder`/explicit byte patches and `tempfile` tests that compare before/after bytes and messages.
- Evaluate `ba2`/`binrw` only after exact reference behavior is preserved.

**If porting downgrader/update downloads:**
- Add `reqwest` with `rustls-tls`, stream progress through task events, and keep backup/cleanup options typed.
- Because download progress and cancellation must not block Slint, and TLS/OpenSSL deployment should stay simple.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `slint = 1.16.1` | `slint-build = 1.16.1` | Keep exact minor/patch versions aligned to avoid generated-code/API mismatches. |
| Rust edition 2024 | Rust 1.85+ | The existing crate uses edition 2024. CI should use current stable, but 1.85 is the practical floor for edition support. |
| `tokio = 1.52.3` | Slint event loop | Tokio tasks must not directly mutate Slint state. Send owned data back through Slint event-loop APIs. |
| `tracing-subscriber = 0.3.23` | `EnvFilter` | Enable `env-filter` if using `with_env_filter` or environment-driven logging. |
| `reqwest = 0.13.3` | `tokio = 1.x` | Use `rustls-tls` to avoid native OpenSSL packaging; confirm feature names when adding because reqwest features evolve. |
| `windows-registry = 0.6.1` | Windows-only modules | Guard with `cfg(windows)` and provide non-Windows errors/stubs so the crate can still check on other platforms if desired. |

## Roadmap Implications

1. **Bootstrap Slint shell first**: add `build.rs`, `ui/main.slint`, tab skeletons, tracing setup, and typed callback boundaries.
2. **Port settings/defaults next**: add `serde`/`toml`/`directories`, preserve `update_source`, `log_level`, and scanner toggles before scanner behavior depends on them.
3. **Port scanner as deterministic domain logic**: add `walkdir`, result models, filesystem fixtures, and snapshot tests before optimizing traversal.
4. **Port F4SE and archive tools as separate binary-format phases**: validate `pelite`, `byteorder`, `binrw`, and/or `ba2` against reference behavior and real sample files.
5. **Add network/download stack only when downgrader/update checks are scheduled**: avoid pulling `reqwest` into early UI/scanner phases.

## Sources

- Context7 Slint Rust docs (`/websites/slint_dev_rust_slint`): Slint 1.16 build setup, `slint-build`, external `.slint` compilation, `VecModel`, and `upgrade_in_event_loop` thread handoff. HIGH confidence.
- Context7 Slint widget docs (`/websites/slint_dev_slint`): `TabWidget` and standard widget styles. HIGH confidence.
- crates.io API checked 2026-05-16 for current crate versions: `slint 1.16.1`, `slint-build 1.16.1`, `tokio 1.52.3`, `walkdir 2.5.0`, `serde 1.0.228`, `toml 1.1.2`, `tracing 0.1.44`, `rfd 0.17.2`, `ba2 3.0.1`, and others. HIGH for version existence.
- Context7 Tokio docs (`/websites/rs_tokio_1_49_0`): `spawn_blocking`, channel bridging, and Tokio filesystem caveats. HIGH for architectural pattern; exact latest patch version verified separately via crates.io.
- Context7 Walkdir docs (`/burntsushi/walkdir`): filtering, depth control, symlink handling, and detailed traversal errors. HIGH confidence.
- Context7 Serde docs (`/websites/serde_rs`): `Serialize`/`Deserialize` derive and `serde` dependency setup. HIGH confidence.
- Context7 tracing-subscriber docs (`/websites/rs_tracing-subscriber`): `EnvFilter`, fmt/file logging layers. HIGH confidence.
- Local project files: `.planning/PROJECT.md`, `Cargo.toml`, `AGENTS.md`, and read-only summaries of `CMT/src/main.py`, `CMT/src/cm_checker.py`, `CMT/src/tabs/*.py`, `app_settings.py`, `scan_settings.py`, `game_info.py`, `downgrader.py`, `utils.py`, and `patcher/*.py`. HIGH for project/reference constraints.

# Feature Research

**Domain:** Faithful Rust/Slint desktop port of the Collective Modding Toolkit Fallout 4 utility  
**Researched:** 2026-05-17  
**Confidence:** HIGH for feature inventory and defaults because findings come from `PROJECT.md` plus the Python/Tkinter reference source listed below.

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist because they are present in the reference app. Missing these = the Rust port is not a faithful CMT port.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Fixed desktop shell and tab order: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` | `PROJECT.md` names the original tab order as an active requirement and `cm_checker.py` constructs exactly this notebook order. | MEDIUM | Window identity must remain `Collective Modding Toolkit v...`, with the same non-redesigned workflow shape. Slint can implement the notebook differently visually, but labels/order should not change. |
| Lazy tab load/refresh behavior | The reference loads a tab on selection and refreshes Overview before scans. | MEDIUM | Scanner depends on Overview-derived problem data; keep a central app state that can refresh one tab's domain model without rebuilding unrelated UI. |
| Startup game, PC, and mod-manager discovery | Overview top panel displays Mod Manager, Game Path, Version, and PC Specs. | HIGH | Needed before most visible features are useful. Preserve click-to-open game path, detection detail affordance for MO2, warnings for unsupported/partial Vortex handling, and Windows 11 24H2 + MO2 warning. |
| Update notification banner respecting update source | `cm_checker.py` checks Nexus/GitHub/both unless update source is `none` and shows links. | MEDIUM | Keep Settings values: `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, `Never: Don't Check`. Network failures should not block app launch. |
| Overview: Binaries `(EXE/DLL/BIN)` panel | Reference checks base game/F4SE/Creation Kit binaries, install type, known versions/hashes, and Address Library. | HIGH | Must preserve status labels such as `Installed`, `Not Found`, install-type display, hover-to-version behavior if practical, Address Library missing problem, and `Downgrade Manager...` button. |
| Overview: Archives `(BA2)` panel | Reference counts General/Texture/Total archives, unreadable archives, OG vs NG archive versions, and archive limits. | HIGH | Drives scanner Overview Issues. Must preserve `Archive Patcher...` action, invalid BA2 detection, hardcoded NvFlex/AE texture patch missing checks, and limit-exceeded advice. |
| Overview: Modules `(ESM/ESL/ESP)` panel | Reference counts Full/Light/Total modules, unreadable modules, HEDR v1.00/v0.95/v????, and module limits. | HIGH | Must preserve `Fallout4.ccc` and `plugins.txt` warnings, TES4/HEDR validation, invalid-version detail tree, and limit-exceeded guidance/URLs. |
| Overview problem aggregation | Scanner can include Overview Issues and Overview creates `ProblemInfo` / `SimpleProblemInfo` objects. | HIGH | Use one typed Rust problem model shared by Overview and Scanner. This is a dependency for scanner results and details. |
| F4SE tab DLL scan | Reference scans `Data/F4SE/Plugins`, ignores `msdia*`, parses DLLs, and shows `DLL`, `OG`, `NG`, `AE`, `Your Game` columns. | HIGH | Preserve loading errors: `Data folder not found`, `Data/F4SE/Plugins folder not found`, and `Try launching via your mod manager.` Preserve status semantics: unknown, supported, unsupported, partial/notes. |
| Scanner side-pane scan settings | Reference creates a `Scan Settings` side pane with all checkboxes enabled by default. | MEDIUM | Settings are `Overview Issues`, `Errors`, `Wrong File Formats`, `Loose Previs`, `Junk Files`, `Problem Overrides`, `Race Subgraphs`. Scan button disables if none selected. |
| Scanner execution flow and progress | Reference disables `Scan Game`, clears old results/details, refreshes Overview, shows `Refreshing Overview...`, then `Building mod file index...` / `Scanning... n/N: folder`, with progress bar. | HIGH | In Rust this must run off the Slint UI thread. Preserve result population and re-enable button text `Scan Game`; cancellation is not in the reference and should not be invented for initial parity. |
| Scanner MO2 staging attribution | Reference reads MO2 `modlist.txt`, stage path, selected profile, and overwrite folder to map files/folders/modules/archives to source mods. | HIGH | Core scanner table includes a `mod` column only when staging is available. Vortex remains partial: scan Data only, cannot identify source mod. |
| Scanner problem classes | `PROJECT.md`, `_overview.py`, `_scanner.py`, and `scan_settings.py` define the problem landscape. | HIGH | Include junk files/folders, unexpected formats, misplaced DLLs, loose previs, unpacked `AnimTextData`, invalid archives/modules/archive names, F4SE script overrides, missing files, wrong versions, limits exceeded, and race subgraph record count. |
| Scanner tree results and controls | Reference shows grouped tree results, `Collapse All`, `Expand All`, result count text `N Results ~ Select an item for details`, and selection opens a details pane. | MEDIUM | Slint implementation should preserve grouping by problem type and stage mod attribution where available. |
| Scanner result details pane | Reference details pane shows `Mod`, `Problem`, `Summary`, `Solution`, clickable path, URL open/copy behavior, `Copy Details`, optional `File List`, and optional `Auto-Fix`. | HIGH | Essential to make scan results actionable. Preserve labels and text formatting as closely as practical. |
| Auto-fix actions and feedback | Reference shows `Auto-Fix`, then `Fixed!` or `Fix Failed` for supported solution types. | HIGH | Do not expose auto-fix before implementing the exact same safe operations and result feedback; otherwise scanner parity is misleading. |
| Settings persistence and validation | `app_settings.py` persists `settings.json`, resets invalid values/types, removes unknown settings, and adds new settings. | MEDIUM | Defaults: `log_level = INFO`, update source from `download-source.txt` with Nexus fallback, all scanner toggles true, downgrader backup/delete-delta options true. |
| Settings tab radio groups | Reference Settings tab exposes only `Update Channel` and `Log Level` radio groups. | LOW | Preserve option labels/order and immediate save on change. Scanner toggles are persisted from the Scanner side pane, not shown here. |
| Tools tab external links | Reference groups tool buttons under `Other CM Authors' Tools` and `Other Useful Tools`, each opening the exact URL with tooltips. | LOW | Preserve button labels, multi-line formatting, disabled-state behavior for missing action, and URLs. |
| Toolkit Utilities: `Downgrade Manager` and `Archive Patcher` entry points | Tools tab and Overview panels expose these workflows. | HIGH | Full behavior likely lives in `downgrader.py` and `patcher/`; requirements should split these into later vertical slices after shell/Overview parsing exists. |
| About tab attribution and links | Reference displays title icon, version, wxMichael/Collective Modding Community text, Nexus/GitHub open/copy actions, and Discord invite open/copy actions. | LOW | Preserve user-facing text and `Open Link`, `Copy Link`, `Open Invite`, `Copy Invite` labels. |
| External open/copy affordances | Reference frequently opens files/folders/URLs and copies links/details. | MEDIUM | Needed across Overview, Scanner, Tools, About. Centralize platform-safe open-url/open-folder/copy helpers. |

### Differentiators (Competitive Advantage)

These should improve the Rust port without changing CMT's product direction or visible behavior.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Responsive scans and parsing | Rust/Slint port can avoid Tkinter-style UI stalls while preserving workflows. | HIGH | Required by `PROJECT.md`: long filesystem scans, parsing, and process work off the UI thread. This is a quality differentiator, not a new feature. |
| Typed domain models for game state, settings, scan results, and tool state | Reduces divergence from reference behavior and makes validation/test coverage practical. | MEDIUM | Directly supports faithful porting; avoid unstructured strings/maps except at UI boundaries. |
| Golden/reference tests for classification rules | Prevents regressions in binary/archive/module/scan classification. | MEDIUM | Build tests from observed reference rules and small fixtures; do not need full game installation to test pure parsers. |
| Better error containment around unreadable files and invalid settings | Reference logs and continues for many failure modes. Rust should make this explicit and testable. | MEDIUM | Preserve user-facing messages while improving internal error typing. |
| Conservative Slint visual fidelity | Users get the same layout, grouping, labels, and disabled/enabled states without needing Python/Tkinter. | MEDIUM | This differentiates the port only by portability/maintainability; do not redesign. |
| Shared action helpers for URL/folder open and clipboard | Keeps repeated About/Tools/Scanner actions consistent. | LOW | Should be invisible to users except for reliability. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that may seem useful but should be deliberately deferred or excluded for this milestone.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| New product direction or redesigned workflows | A Rust port invites cleanup and modernization. | `PROJECT.md` explicitly makes UI fidelity and original workflows the priority. Redesign would obscure parity gaps. | Port the existing tabs/workflows first; log possible redesigns for a later milestone. |
| Editing files under `CMT/` | Fixing reference behavior at the source may seem convenient. | `CMT/` is read-only reference material and must not be modified. | Implement Rust behavior outside `CMT/`; document reference discrepancies before diverging. |
| Full Vortex staging support | Vortex users would benefit from source-mod attribution. | Reference explicitly says Vortex is not fully supported and Scanner only looks in Data, so adding staging support changes product behavior and scope. | Preserve partial Vortex warning and Data-only scanner behavior initially. |
| New scanner problem categories beyond the reference | More checks may improve diagnostics. | Scope creep risks false positives and makes parity impossible to validate. | Implement reference problem classes first; propose new checks only after initial parity. |
| Background auto-update/install | Update banner could become an installer. | Reference only checks and opens Nexus/GitHub links; auto-install introduces trust, permissions, and packaging risks. | Preserve link-based update notification. |
| Real-time filesystem watching/rescanning | Users may expect live results while modding. | Reference scan is explicit via `Scan Game`; live scanning adds race/cancellation complexity and UI churn. | Keep explicit scan button and refresh behavior. |
| Archive/module repair beyond existing `Archive Patcher` / auto-fixes | Could make CMT more powerful. | Repair actions are destructive and not table-stakes unless present in reference workflows. | Port existing patcher/autofix actions exactly, with feedback and backup settings. |
| Cross-game support | Architecture could generalize Bethesda tooling. | Product intent and constants are Fallout 4-specific. Generalization would slow the faithful port. | Keep Fallout 4 behavior and labels; consider abstraction only where it helps tests. |
| CLI/headless mode | Rust makes a CLI tempting for scanners. | Project target is a Slint desktop app; CLI is not in the reference. | Keep domain logic testable internally, but do not ship CLI in initial roadmap. |
| Web/mobile UI | Could broaden access. | Explicitly out of scope in `PROJECT.md`. | Ship native Slint desktop only. |
| Python runtime integration | Could reuse reference code quickly. | Project goal is a Rust implementation without Python runtime behavior. | Use Python as reference only; port logic to Rust. |
| Scan cancellation as a launch requirement | Long scans make cancellation attractive. | Reference has no cancellation workflow; adding it changes state handling and testing surface. | Defer until after parity; ensure worker architecture does not preclude adding cancellation later. |

## Feature Dependencies

```text
Desktop shell + tab order
    └──requires──> Settings load/save + assets + shared app state

Game/mod-manager discovery
    ├──requires──> Settings load/save
    ├──enables──> Overview top status
    ├──enables──> F4SE plugin path discovery
    ├──enables──> Scanner Data path scan
    └──enables──> MO2 staging attribution

Overview binary/archive/module parsing
    ├──requires──> Game path + Data path discovery
    ├──produces──> Overview problem aggregation
    ├──enables──> Overview panels
    └──feeds──> Scanner `Overview Issues`

Scanner side-pane settings
    ├──requires──> Settings persistence
    └──configures──> Scanner execution

Scanner execution
    ├──requires──> Game Data path discovery
    ├──requires──> Overview refresh/problem aggregation
    ├──optionally requires──> MO2 stage path + modlist parsing for mod attribution
    ├──produces──> Scanner tree results
    └──produces──> Result details pane actions

Auto-fix actions
    └──requires──> Result details + exact solution/action mapping

Downgrade Manager / Archive Patcher
    ├──requires──> Game install type and file/archive metadata
    └──uses──> Settings defaults for backup and delta cleanup behavior

Tools/About links
    └──requires──> shared URL open + clipboard helpers
```

### Dependency Notes

- **Settings should land early:** defaults affect update checks, scanner toggles, log level, and downgrader options.
- **Game discovery is the central prerequisite:** Overview, F4SE, Scanner, and toolkit utilities all depend on accurate game path, Data path, install type, mod manager, and MO2/Vortex status.
- **Overview should precede full Scanner:** Scanner starts by refreshing Overview and optionally includes Overview problems in its own results.
- **MO2 attribution is a scanner enhancer but still table-stakes:** the reference uses it when available and changes the tree columns/details accordingly.
- **Auto-fix should come after read-only scanner parity:** exposing `Auto-Fix` without exact behavior and feedback would be unsafe.

## MVP Definition

### Launch With (v1)

Minimum faithful port scope for the first usable Rust/Slint milestone.

- [ ] Shell with original window identity and tabs: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
- [ ] Settings load/save/validation with reference defaults.
- [ ] Game path, install type, PC info, and mod-manager discovery sufficient to populate Overview.
- [ ] Overview binary/archive/module panels and shared Overview problem aggregation.
- [ ] F4SE DLL compatibility table.
- [ ] Scanner settings, explicit `Scan Game` flow, progress, grouped results, details pane, copy/open/file-list actions, and all reference problem classes in read-only mode.
- [ ] Tools/About static links and copy/open behavior.

### Add After Validation (v1.x)

Features to add once the read-only diagnostic experience is working and tested.

- [ ] Auto-fix actions — add only after scanner solution mapping is exact and tests cover success/failure feedback.
- [ ] Downgrade Manager workflow — high-value but complex and potentially destructive; port as its own vertical slice.
- [ ] Archive Patcher workflow — high-value but complex binary/archive mutation; port separately with backup/error tests.
- [ ] Update banner network checks — can follow shell/settings if networking/package links need extra validation.

### Future Consideration (v2+)

Features to defer until original behavior is faithfully ported.

- [ ] Better-than-reference Vortex staging attribution — useful, but not original behavior.
- [ ] Scan cancellation, live rescanning, or background file watching — useful UX improvements, but not parity requirements.
- [ ] New diagnostic categories — consider only after reference categories are stable.
- [ ] CLI/headless scanner — useful for tests/automation, but outside current desktop product scope.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Shell/tab order/window identity | HIGH | MEDIUM | P1 |
| Settings persistence/defaults | HIGH | MEDIUM | P1 |
| Game/mod-manager discovery | HIGH | HIGH | P1 |
| Overview panels and problem aggregation | HIGH | HIGH | P1 |
| F4SE DLL scan table | HIGH | HIGH | P1 |
| Scanner settings/execution/results/details | HIGH | HIGH | P1 |
| Tools/About links | MEDIUM | LOW | P1 |
| Auto-fix actions | HIGH | HIGH | P2 |
| Downgrade Manager | HIGH | HIGH | P2 |
| Archive Patcher | HIGH | HIGH | P2 |
| Responsive worker architecture | HIGH | HIGH | P1 |
| Typed domain model/test fixtures | HIGH | MEDIUM | P1 |
| Full Vortex staging support | MEDIUM | HIGH | P3 / defer |
| New scanner checks | MEDIUM | MEDIUM-HIGH | P3 / defer |
| CLI/headless mode | LOW | MEDIUM | P3 / defer |

**Priority key:**
- P1: Must have for initial faithful launch/read-only parity.
- P2: Should have, but port after core diagnostics are stable or because mutation risk requires isolation.
- P3: Future consideration or deliberately excluded from this milestone.

## Original Defaults and Labels to Preserve

- Tabs: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
- Scanner checkboxes, all default `true`: `Overview Issues`, `Errors`, `Wrong File Formats`, `Loose Previs`, `Junk Files`, `Problem Overrides`, `Race Subgraphs`.
- Scanner buttons/status: `Collapse All`, `Expand All`, `Scan Game`, `Scanning...`, `Refreshing Overview...`, `Building mod file index...`, `N Results ~ Select an item for details`.
- Scanner details labels/actions: `Mod:`, `Problem:`, `Summary:`, `Solution:`, `Copy Details`, `File List`, `Auto-Fix`, `Fixed!`, `Fix Failed`.
- Settings groups/options: `Update Channel` with `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, `Never: Don't Check`; `Log Level` with `Debug`, `Info`, `Error`.
- App settings defaults: `log_level = INFO`; `update_source` from `download-source.txt` with `nexus` fallback; all scanner toggles true; `downgrader_keep_backups = true`; `downgrader_delete_deltas = true`.
- Tools groups: `Toolkit Utilities`, `Other CM Authors' Tools`, `Other Useful Tools`.
- Toolkit Utility buttons: `Downgrade Manager`, `Archive Patcher`.
- About link actions: `Open Link`, `Copy Link`, `Open Invite`, `Copy Invite`.

## Sources

- `J:/cmt-rs/.planning/PROJECT.md` — project intent, active/out-of-scope requirements, default settings summary.
- `J:/cmt-rs/AGENTS.md` — UI fidelity, read-only `CMT/`, Rust/Slint implementation constraints.
- `J:/cmt-rs/CMT/src/cm_checker.py` — window identity, tab construction/order, update banner, tab lifecycle.
- `J:/cmt-rs/CMT/src/tabs/_overview.py` — Overview panels, binary/archive/module checks, problem aggregation, utility entry points.
- `J:/cmt-rs/CMT/src/tabs/_f4se.py` — F4SE plugin DLL scanning table and loading errors.
- `J:/cmt-rs/CMT/src/tabs/_scanner.py` — scanner workflow, side pane settings, progress, result grouping, details pane, auto-fix/file-list/copy actions.
- `J:/cmt-rs/CMT/src/tabs/_tools.py` — tool groups, utility buttons, external URLs/tooltips.
- `J:/cmt-rs/CMT/src/tabs/_settings.py` — Settings tab labels/options and immediate persistence.
- `J:/cmt-rs/CMT/src/tabs/_about.py` — About tab attribution, logos, open/copy link actions.
- `J:/cmt-rs/CMT/src/app_settings.py` — settings schema, defaults, validation/reset behavior.
- `J:/cmt-rs/CMT/src/scan_settings.py` — scan setting names, defaults, Data whitelist, junk/proper-format constants, MO2 skip behavior.

---
*Feature research for: Collective Modding Toolkit Rust/Slint port*  
*Researched: 2026-05-17*

# Domain Pitfalls

**Domain:** Rust/Slint port of a Python/Tkinter Fallout 4 modding utility  
**Researched:** 2026-05-16  
**Overall confidence:** HIGH for project-specific risks from the reference source; MEDIUM for Slint concurrency guidance from current Slint docs.

## Critical Pitfalls

Mistakes that cause behavior drift, corrupt user installations, or force major rewrites.

### Pitfall 1: Treating the port as a redesign instead of a behavioral clone

**What goes wrong:** The Rust/Slint app looks cleaner but no longer matches the original tab order, grouping, button text, enabled states, warning dialogs, settings names, or workflow timing.  
**Why it happens:** Slint encourages a fresh declarative UI, while the source of truth is a Tkinter notebook with specific tabs: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, and `About`. The project explicitly prioritizes fidelity over modernization.  
**Consequences:** Roadmap phases appear complete in screenshots but fail user expectations and lose parity with the reference app. Later phases must backtrack through every tab to restore labels and states.  
**Warning signs:**
- A phase plan says "simplify", "modernize", or "improve UX" without an explicit parity note.
- Slint files are implemented from memory instead of a side-by-side pass over `CMT/src/tabs/*.py`.
- User-facing strings are paraphrased, dialogs are merged, or tab ordering changes.
- Completion criteria only mention that a tab exists, not that labels/control ordering/defaults match.
**Prevention:**
- Add a UI parity checklist to every tab phase: tab title, group labels, control order, defaults, disabled/enabled states, tooltip/dialog text, and action wiring.
- Keep `.slint` layout conservative and defer visual redesign until after parity is validated.
- For each phase, record intentional differences in the phase output and ask before changing behavior that appears wrong or incomplete in the Python reference.
**Detection:** Manual side-by-side comparison against the relevant `CMT/src/tabs/_*.py` file before marking a phase done.  
**Phase to address:** Phase 1 shell must lock tab order/window identity; every later tab phase must include a parity acceptance checklist.

### Pitfall 2: Editing or generating files under `CMT/`

**What goes wrong:** The read-only Python reference submodule is formatted, patched, moved, or used as a destination for generated Rust/Slint artifacts.  
**Why it happens:** Agents and tools naturally "fix" inspected code, and some roadmap phases may search broadly for files to update.  
**Consequences:** The project loses its stable source of truth, submodule state becomes dirty, and future parity checks become unreliable.  
**Warning signs:**
- `git status` shows changes under `CMT/`.
- Plans mention refactoring or annotating reference Python.
- Generated snapshots, logs, or fixtures are placed under `CMT/src/`.
**Prevention:**
- Roadmap all implementation under the Rust crate and project planning directories only.
- Add a recurring verification item: `git status --short CMT` must be clean after every phase.
- If reference behavior needs clarification, document it in planning artifacts or Rust tests; never patch the submodule.
**Detection:** Dirty submodule or modified files under `CMT/`.  
**Phase to address:** Phase 0/1 project safety gate, then every phase completion gate.

### Pitfall 3: Scanner correctness drift from lossy path and mod-manager modeling

**What goes wrong:** The Scanner reports the wrong missing files, wrong archive/module ownership, false invalid archive names, or misses staged Mod Organizer files.  
**Why it happens:** The Python scanner combines game `Data` traversal, MO2 stage paths, `overwrite`, skipped directories/suffixes, enabled plugin state, relative path matching, and archive/module name rules. A naive Rust port may scan only the physical `Data` directory or collapse mod origin metadata into plain paths.  
**Consequences:** Users receive bad modding advice, auto-fixes may target the wrong file, and the core value of the toolkit is undermined.  
**Warning signs:**
- Domain models store only `PathBuf` without mod name/origin/source path.
- Scanner tests use only tiny synthetic `Data` folders and no MO2 profile/staging cases.
- Missing-file counts differ depending on whether the app is launched from MO2.
- Warnings like "Missing MO2 settings" are not represented in the Rust UI/state model.
**Prevention:**
- Build typed scanner models before UI polish: `GameFile`, `Module`, `Archive`, `ModOrigin`, `ScanProblem`, `SolutionType`, and `ScanSetting` equivalents.
- Port scan settings and skip rules before implementing scan results rendering.
- Create fixtures for: physical `Data`, MO2 mods directory, MO2 `overwrite`, disabled plugins, missing `plugins.txt`, missing Creation Club list, unexpected file extensions, invalid BA2/module names, and unreadable files.
- Keep problem type and solution type mapping explicit; do not encode scanner results as display strings only.
**Detection:** Golden fixture tests comparing problem categories and user-facing messages to expected reference behavior.  
**Phase to address:** Scanner domain-model phase before Scanner UI phase; MO2/Vortex/game discovery phase before scan execution.

### Pitfall 4: Misreading Fallout 4 install, settings, and enabled-state sources

**What goes wrong:** The app finds the wrong game path, misses GOG/Steam installs, misclassifies OldGen/NextGen/Anniversary states, or counts all modules/archives as enabled when `plugins.txt`/Creation Club data is absent.  
**Why it happens:** `game_info.py` reads registry locations, prompts for `Fallout4.exe`, parses INI files under Documents, tracks module/archive sets, and warns when `plugins.txt` or the CC list is missing. These Windows-specific edge cases are easy to flatten into "pick a folder".  
**Consequences:** Overview, F4SE, Scanner, Downgrader, and Archive Patcher all inherit bad state. Users may patch or downgrade files in the wrong directory.  
**Warning signs:**
- Discovery is implemented only as a manual folder picker.
- There is no representation for unreadable modules/archives or unknown module header versions.
- Missing `plugins.txt` is treated as success instead of a warning state.
- Counts lack separate full/light/v1 module buckets.
**Prevention:**
- Roadmap a dedicated game/mod-manager discovery phase before feature tabs.
- Preserve fallback order and warnings: registry detection, manual `Fallout4.exe` selection, Documents INI parsing, AppData `plugins.txt`, and Creation Club list handling.
- Add tests with fake directory trees and injectable environment/registry abstractions so discovery can be validated without touching a real install.
**Detection:** Overview status snapshots against controlled fixtures for Steam, GOG, missing game, missing `plugins.txt`, missing CC list, and unreadable files.  
**Phase to address:** Foundation/discovery phase; Overview phase should not invent its own discovery logic.

### Pitfall 5: Blocking or mutating Slint UI state from worker threads

**What goes wrong:** Long scans, downloads, archive parsing, or patching freeze the window, or background Rust threads attempt to mutate Slint components/models directly.  
**Why it happens:** The Python app uses `threading.Thread`, queues, and Tk `after()` polling. Slint models and UI handles have thread-affinity constraints; current Slint Rust docs show background work must marshal results back with `invoke_from_event_loop` or `Weak::upgrade_in_event_loop`, and `ModelRc`/UI-owned objects should be updated on the UI thread.  
**Consequences:** UI hangs during large mod lists, data races/panics occur, progress bars do not update reliably, and cancellation/close behavior becomes brittle.  
**Warning signs:**
- Scan/download callbacks call `set_*` on UI handles from a spawned thread.
- `std::fs::read_dir`, archive byte parsing, or HTTP downloads run directly inside Slint callbacks.
- Slint `ModelRc` or UI objects are moved into worker threads.
- The close/minimize path does not consider running background work.
**Prevention:**
- Introduce an app task boundary early: workers send plain Rust data/progress events through channels; only the UI adapter applies them via `upgrade_in_event_loop`/`invoke_from_event_loop`.
- Keep Slint models as view projections of typed Rust state, not the authoritative scanner/download state.
- Add progress/cancel/close behavior to each long-running workflow before adding extra UI detail.
**Detection:** Test large fixture scans and simulated slow downloads; manually verify the window remains responsive and progress updates continue.  
**Phase to address:** Rust/Slint shell architecture phase, then Scanner/Downgrader/Patcher phases.

### Pitfall 6: Unsafe destructive file operations in auto-fix, downgrade, and patch workflows

**What goes wrong:** The Rust port deletes, renames, patches, or overwrites the wrong files; backups are skipped; delta files are removed unexpectedly; Archive Patcher writes incompatible BA2 bytes.  
**Why it happens:** `autofixes.py`, `downgrader.py`, and `patcher/_archives.py` perform high-impact filesystem operations: deleting junk/deltas, archiving loose files, backing up files, downloading replacement files, and writing BA2 version bytes. Python's permissive error handling can hide edge cases that Rust should model explicitly.  
**Consequences:** User game/mod installs can be damaged. This is the highest trust risk in the port.  
**Warning signs:**
- The first implementation writes to real game paths instead of a fixture/sandbox.
- Backups are an option but not tested as default-on behavior.
- Archive patching does not verify `BTDX` magic/current version byte before writing.
- UI offers actions before discovery confirms the target path and desired version.
**Prevention:**
- Build dry-run and operation-plan structs for all destructive workflows; UI should show what will happen before execution.
- Require sandbox fixture tests for every operation: delete, backup, restore/keep backup, delta cleanup, archive byte patch, already-patched skip, unrecognized version skip, permission error, missing file.
- Preserve settings defaults: `downgrader_keep_backups = true` and `downgrader_delete_deltas = true`, but make their consequences visible.
- Fail closed: when target version, magic bytes, or path ownership is unclear, log/report and skip rather than writing.
**Detection:** Golden tests over temporary directories plus manual review of operation logs before enabling real-path writes.  
**Phase to address:** Tools/Downgrader/Patcher domain phase before interactive tool UI.

### Pitfall 7: Update/download behavior diverges from source selection and user expectations

**What goes wrong:** Update checks or file downloads use the wrong source, ignore the packaged `download-source.txt` fallback, do not honor `update_source = nexus/github/both/none`, or surface network failures differently from the reference.  
**Why it happens:** `app_settings.py` derives default update source from `download-source.txt`, falls back to Nexus, and validates settings literals. `cm_checker.py` conditionally checks Nexus/GitHub/both/none and renders source-specific links/tooltips.  
**Consequences:** Users see update prompts they opted out of, miss updates, or cannot reproduce reference behavior. Downloads may also fail without actionable feedback.  
**Warning signs:**
- Settings enum lacks `none` or `both`.
- Update checks occur unconditionally on startup.
- Network errors are swallowed or displayed only in logs.
- Download source is hard-coded in the tool workflow.
**Prevention:**
- Implement settings validation/migration before update/download features.
- Keep source selection as a typed enum with explicit default derivation and invalid-value fallback.
- Add tests for every update-source value and invalid `download-source.txt` content.
- Treat network work as cancellable/background progress events with user-visible failures.
**Detection:** Startup tests with fixture settings and download-source files; manual check that `none` performs no update call.  
**Phase to address:** Settings foundation before Overview update prompts and Downgrader downloads.

### Pitfall 8: Settings migration/defaults drift

**What goes wrong:** Existing `settings.json` files are ignored, invalid values are accepted, missing keys do not get defaults, booleans invert, or JSON formatting/path behavior changes unexpectedly.  
**Why it happens:** The Python settings loader starts from a full default map, loads `settings.json`, validates literal and boolean types, logs invalid values, resaves when needed, and appends a newline after pretty JSON.  
**Consequences:** Scanner categories silently disable/enable differently, downgrader safety options change, and update checks differ from the reference.  
**Warning signs:**
- Rust settings are deserialized directly into a struct without default-per-field fallback.
- Unknown/invalid setting values cause a hard failure instead of reset-and-resave behavior.
- Settings UI and persisted keys use renamed field names.
**Prevention:**
- Preserve original keys (`scanner_OverviewIssues`, `scanner_Errors`, `scanner_WrongFormat`, `scanner_LoosePrevis`, `scanner_JunkFiles`, `scanner_ProblemOverrides`, `scanner_RaceSubgraphs`, `downgrader_keep_backups`, `downgrader_delete_deltas`, etc.).
- Implement layered loading: defaults first, then validated overrides, then resave if migration occurred.
- Add compatibility tests for missing file, missing keys, wrong types, invalid enum literals, and unknown extra keys.
**Detection:** Fixture settings round-trip tests and Settings tab default-state snapshot.  
**Phase to address:** Settings/domain foundation before any feature reads settings.

## Moderate Pitfalls

### Pitfall 1: Windows filesystem edge cases are under-modeled

**What goes wrong:** The app fails on case-insensitive matches, read-only files, Unicode paths, spaces, long paths, permissions, or locked files in mod-manager directories.  
**Prevention:** Normalize comparisons where the reference compares lowercased names; keep display paths original; test paths with spaces/unicode; handle `PermissionDenied` and `NotFound` as reportable states instead of panics.  
**Warning signs:** Use of `unwrap()` on filesystem operations; tests only use ASCII temp paths; path comparisons are string-based without reference behavior.  
**Phase to address:** Discovery/scanner foundation and destructive tools phases.

### Pitfall 2: Collapsing reference problem taxonomy into generic errors

**What goes wrong:** Scanner results become generic warnings, losing `ProblemType`/`SolutionType` semantics that drive details panes, URL/details actions, and auto-fixes.  
**Prevention:** Port enums and result structs before rendering; map display text from typed variants; make auto-fix eligibility a property of the typed problem.  
**Warning signs:** `Vec<String>` scan results; auto-fix code searches message text; details pane lacks per-problem actions.  
**Phase to address:** Scanner model phase before Scanner UI/details phase.

### Pitfall 3: External links/tool launching implemented without failure handling

**What goes wrong:** Tools/About links, Nexus/GitHub links, and external utilities silently fail or block the UI.  
**Prevention:** Wrap URL/open operations in a platform service returning user-visible success/failure; keep links/text from `_tools.py` and `_about.py`; do not treat launch failures as panics.  
**Warning signs:** Direct `Command::new(...).spawn().unwrap()` in UI callbacks.  
**Phase to address:** Tools and About phases.

### Pitfall 4: Logging and status feedback are added too late

**What goes wrong:** Failures occur but users and tests cannot tell whether a path was skipped, unreadable, already patched, or invalid.  
**Prevention:** Define logging/status event types in the foundation; use them consistently in scans, downloads, patching, and update checks.  
**Warning signs:** Only `println!` or UI labels; no structured operation log for tools.  
**Phase to address:** Foundation before Scanner/Tools.

## Minor Pitfalls

### Pitfall 1: Packaging/resource lookup differs from Python assets

**What goes wrong:** Icons, `download-source.txt`, and other assets work in development but not in packaged builds.  
**Prevention:** Centralize asset lookup in Rust; test missing/invalid asset fallback; keep source-selection fallback to Nexus.  
**Phase to address:** Shell/settings foundation.

### Pitfall 2: About/attribution text drifts

**What goes wrong:** GPL/license/credit text or community links are shortened or reworded.  
**Prevention:** Treat `_about.py` strings as parity fixtures and preserve text unless explicitly approved.  
**Phase to address:** About phase.

### Pitfall 3: Verification stops at `cargo check`

**What goes wrong:** The app compiles but important parity and filesystem behavior is untested.  
**Prevention:** Roadmap fixture-based domain tests and side-by-side UI checklists in addition to `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.  
**Phase to address:** Every phase.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|----------------|------------|
| Phase 0/1: Project safety and shell | Dirtying `CMT/`; tab order/window identity drift | Add `git status --short CMT` gate; implement exact title/tab order first |
| Settings foundation | Wrong defaults, invalid migration, update-source drift | Implement default-first validated loader with fixture tests before consumers |
| Game/mod-manager discovery | Wrong install/profile paths; missing warnings for `plugins.txt`/CC list | Abstract env/registry/filesystem and test Steam, GOG, MO2, Vortex, missing-file fixtures |
| Overview | Counts/statuses based on incomplete discovery | Consume shared discovery models only; snapshot status text and counts |
| F4SE | DLL compatibility scan over-simplified | Preserve table/status semantics and unreadable/missing states from reference |
| Scanner model | False positives/negatives from path ownership and stage handling | Port typed problem taxonomy, scan settings, and MO2 stage traversal before UI |
| Scanner UI/details | Looks complete but loses details/actions/autofix eligibility | Bind UI to typed `ScanProblem`; side-by-side check tree, details pane, URL/details/autofix flows |
| Tools/Downgrader | Destructive writes without safety plan | Dry-run operation plan, default backups, sandbox tests, fail-closed writes |
| Archive Patcher | BA2 version byte patched incorrectly | Verify magic/current byte before write; test already-patched, unrecognized, permission, missing cases |
| Update/download | Blocking UI or ignoring `none`/`both` source values | Background tasks with progress events; tests for all update-source enum values |
| About/links | Text/link drift or launch panics | Preserve reference text/URLs; platform service with visible launch failures |

## Looks Done But Isn't Checklist

- [ ] `CMT/` is unmodified after the phase.
- [ ] The phase cites the exact reference files inspected.
- [ ] User-facing labels/messages/defaults were compared side-by-side with the Python source.
- [ ] Long-running work is off the Slint UI thread and marshaled back through event-loop-safe APIs.
- [ ] Filesystem and network errors are displayed or logged as typed states, not panics.
- [ ] Fixture tests cover missing/unreadable files and at least one mod-manager-specific path.
- [ ] Destructive operations have sandbox tests and do not run unless target/version/path checks pass.
- [ ] Settings round-trip preserves original keys and default behavior.

## Sources

- `.planning/PROJECT.md` — project requirements, active roadmap inputs, out-of-scope constraints. Confidence: HIGH.
- `AGENTS.md` — read-only `CMT/`, Rust/Slint direction, UI fidelity, threading, verification rules. Confidence: HIGH.
- `CMT/src/app_settings.py` — settings keys/defaults, `download-source.txt`, validation/resave behavior. Confidence: HIGH.
- `CMT/src/cm_checker.py` — window/tab setup, update checks, lifecycle hooks. Confidence: HIGH.
- `CMT/src/game_info.py` and `CMT/src/mod_manager_info.py` — game path, registry, INI, mod-manager, plugins/Creation Club state. Confidence: HIGH.
- `CMT/src/tabs/*.py` — UI/workflow parity risks for Overview, F4SE, Scanner, Tools, Settings, About. Confidence: HIGH.
- `CMT/src/autofixes.py`, `CMT/src/downgrader.py`, `CMT/src/patcher/*.py` — destructive operation and archive patching risks. Confidence: HIGH.
- Slint Rust docs via Context7 (`/websites/slint_dev_rust_slint`) — `invoke_from_event_loop`, `Weak::upgrade_in_event_loop`, and thread-safe UI/model update guidance. Confidence: HIGH for Slint threading claims.