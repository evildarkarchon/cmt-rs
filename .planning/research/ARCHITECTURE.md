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
