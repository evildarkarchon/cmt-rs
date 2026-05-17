# Phase 3: Platform Discovery & Background Adapters - Research

**Researched:** 2026-05-17  
**Domain:** Rust platform discovery, fakeable adapter seams, and Slint-safe worker event handoff  
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
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

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DISC-01 | App can discover or represent the Fallout 4 game path needed by Overview, F4SE, Scanner, Downgrader, and Archive Patcher workflows. | `GameInfo::find_path` reference ordering and `Fallout4.exe` validation are documented below. [VERIFIED: CMT/src/game_info.py] |
| DISC-02 | App can identify mod manager context and display Mod Manager, Game Path, Version, and PC Specs data in the Overview area. | `find_mod_manager` parent-process detection and MO2 INI parsing requirements are documented below. [VERIFIED: CMT/src/utils.py; CMT/src/mod_manager_info.py] |
| DISC-03 | App can read the file and directory sources needed for archive, module, F4SE plugin, scanner, and settings workflows through injectable filesystem adapters. | The existing `SettingsStore` adapter pattern and required discovery filesystem operations are mapped below. [VERIFIED: src/platform/settings_store.rs; 03-SPEC.md] |
| DISC-04 | App can launch URLs, open paths, and run external tools through injectable process adapters with visible failure reporting. | The process/desktop seam recommendation covers process listing, version metadata, URL/path open, external tool launch, and typed action results. [VERIFIED: 03-SPEC.md; CMT/src/cm_checker.py] |
| SAFE-01 | Long-running scans, filesystem traversal, parsing, downloads, patching, and process monitoring run off the Slint UI thread. | Tokio `spawn_blocking` and channel bridging are the standard pattern for blocking filesystem/process work. [CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html] |
| SAFE-02 | Background work returns typed progress, completion, cancellation, and error events to the UI through Slint-safe event-loop handoff. | Slint `invoke_from_event_loop`/`Weak::upgrade_in_event_loop` are documented as thread-safe event-loop handoff APIs. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html] |
| SAFE-03 | Domain logic can be tested without launching a window by using fake filesystem and process adapters. | Existing tests use fake/isolated settings IO; Phase 3 should extend that style to discovery/process/worker seams. [VERIFIED: src/platform/settings_store.rs; src/app/settings_controller.rs] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- Treat `CMT/` as a read-only reference submodule; do not edit, format, move, delete, or generate files under it. [VERIFIED: AGENTS.md]
- Inspect relevant original files in `CMT/src/` before porting behavior; preserve labels, ordering, defaults, validation rules, and user-facing messages unless there is a clear documented reason to diverge. [VERIFIED: AGENTS.md]
- Implement new code outside `CMT/` in Rust with Slint UI files preferred for UI structure/styling and Rust handling application state, filesystem work, parsing, and command execution. [VERIFIED: AGENTS.md]
- Keep UI and domain logic separated enough for non-UI behavior to be tested without launching a window. [VERIFIED: AGENTS.md]
- Avoid blocking the Slint UI thread; run slow filesystem scans, parsing, and process work off-thread and marshal results back through Slint-safe callbacks or event-loop APIs. [VERIFIED: AGENTS.md]
- Use typed Rust models instead of unstructured strings/maps for app state. [VERIFIED: AGENTS.md]
- Prefer small vertical slices and focused dependencies; avoid broad refactors while the port is in progress. [VERIFIED: AGENTS.md]
- Avoid `unwrap()` and `expect()` in production paths unless the invariant is obvious or documented. [VERIFIED: AGENTS.md]
- Add Rust doc comments to public functions/types and methods added or substantially rewritten; add short comments for non-obvious UI-thread handoffs, ownership, cancellation, or compatibility constraints. [VERIFIED: AGENTS.md]
- Required verification gates are `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`. [VERIFIED: AGENTS.md]
- Do not commit unless explicitly asked. [VERIFIED: AGENTS.md]

## Summary

Phase 3 should be planned as a non-visual foundation slice: add typed domain models for Fallout 4 installation state and mod-manager context, platform adapters for filesystem/registry/process/desktop operations, and a reusable worker event contract plus Slint event-loop handoff seam. [VERIFIED: 03-SPEC.md; 03-CONTEXT.md] The Rust implementation should preserve the Python reference discovery order: running manager game path first, then current working directory if it contains `Fallout4.exe`, then Bethesda/GOG registry paths, with `Fallout4.exe` path inputs normalized to the parent game directory. [VERIFIED: CMT/src/game_info.py]

The key planning risk is mixing platform side effects into domain logic or Slint callbacks. [VERIFIED: AGENTS.md; src/platform/settings_store.rs] Use traits and fake implementations for filesystem, registry, process listing, version metadata, and desktop open/launch behavior; keep real Windows operations in `platform` modules and keep `domain` types pure and testable. [VERIFIED: 03-SPEC.md; src/domain/mod.rs; src/platform/settings_store.rs]

Worker work should be event-driven now even though real scanner/F4SE/download/patch workflows are later phases. [VERIFIED: 03-SPEC.md] Define a shared event envelope with task id/kind/status and typed payloads for discovery, scan, patch, download, external process, cancellation, and errors; deliver owned events to UI/controller code through `slint::invoke_from_event_loop` or `Weak::upgrade_in_event_loop`. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]

**Primary recommendation:** Plan Phase 3 in four slices: domain discovery models, fakeable platform adapters, reference-compatible discovery/MO2 parsing, then worker event + Slint handoff tests. [VERIFIED: 03-SPEC.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Fallout 4 installation representation | API / Backend (Rust domain) | Database / Storage (filesystem inputs only) | Typed game state belongs in pure Rust domain models; filesystem/registry only supply inputs. [VERIFIED: AGENTS.md; 03-SPEC.md] |
| Fallout 4 discovery side effects | API / Backend (platform adapter) | OS / Registry | Registry, current directory, file existence, and INI reads are OS/platform operations that should be hidden behind fakeable adapters. [VERIFIED: CMT/src/game_info.py; src/platform/settings_store.rs] |
| Mod manager process detection | API / Backend (platform process adapter) | OS process table | The Python reference walks parent processes and reads executable version metadata; Rust should isolate that behind `ProcessAdapter`. [VERIFIED: CMT/src/utils.py] |
| MO2 INI parsing | API / Backend (Rust domain/service) | Filesystem adapter | Parsing `%BASE_DIR%`, profiles, skips, and game validation is deterministic domain logic once file text is supplied. [VERIFIED: CMT/src/mod_manager_info.py] |
| URL/path/tool launch | API / Backend (desktop/process adapter) | OS shell | Launch/open operations must return typed action results and must not show dialogs directly. [VERIFIED: 03-CONTEXT.md] |
| Background task execution | API / Backend (workers) | Browser / Client (Slint event loop handoff) | Blocking work runs in workers; UI state changes happen only after owned events are delivered to the Slint event loop. [CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html; docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `slint` | 1.16.1 | Slint UI runtime and event-loop handoff APIs | Already in the project and provides `invoke_from_event_loop` / `Weak::upgrade_in_event_loop` for cross-thread UI updates. [VERIFIED: Cargo.toml; CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html] |
| `tokio` | 1.52.3 | Worker runtime, channels, and blocking-work orchestration | Already in the project; Tokio documents `spawn_blocking` plus `mpsc` bridging for sync filesystem/process work. [VERIFIED: Cargo.toml; VERIFIED: cargo search tokio; CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html] |
| `thiserror` | 2.0.18 | Typed discovery/platform/worker error enums | Already in the project and fits known typed error kinds with safe user-facing messages. [VERIFIED: Cargo.toml] |
| `tracing` | 0.1.44 | Diagnostics for platform/discovery failures without leaking raw errors to UI | Already in the project; use diagnostics/logging for raw OS details while returning safe messages. [VERIFIED: Cargo.toml; 03-CONTEXT.md] |
| `serde` | 1.0.228 | Serializable worker/discovery payloads if later persistence/logging needs it | Already in the project for typed settings; keep domain models typed. [VERIFIED: Cargo.toml; src/domain/settings.rs] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `windows-registry` | 0.6.1 | Read Bethesda/GOG and MO2 CurrentInstance registry values on Windows | Add behind `cfg(windows)` for real registry discovery; fake adapter covers tests and non-Windows. [VERIFIED: cargo search windows-registry; VERIFIED: CMT/src/game_info.py] |
| `sysinfo` | 0.39.2 | Cross-platform process list, process name/path/parent metadata | Use for real process enumeration if it can reproduce parent-chain detection; fake adapter remains the acceptance-test path. [VERIFIED: cargo search sysinfo; CITED: docs.rs/sysinfo/latest/sysinfo/struct.Process.html] |
| `pelite` | 0.10.0 | Windows PE version resource reading | Candidate for manager executable version metadata and later F4SE DLL metadata; docs expose version-info support. [VERIFIED: cargo search pelite; CITED: docs.rs/pelite/latest/pelite/resources/version_info/index.html] |
| `open` | 5.3.5 | Open URLs and paths with OS handlers | Use inside the desktop adapter only; convert failures into typed action results. [VERIFIED: cargo search open] |
| `walkdir` | 2.5.0 | Deterministic recursive directory enumeration for future scanner/archive/module sources | Add only if Phase 3 implements reusable enumeration needed for archive/module set fixtures; otherwise defer to Scanner phase. [VERIFIED: cargo search walkdir; VERIFIED: 03-SPEC.md] |
| `tokio-util` | 0.7.18 | `CancellationToken` for worker cancellation coordination | Add if planner wants standard cancellable worker handles; otherwise model cancellation events first and add runtime cancellation in the first real long-running workflow. [VERIFIED: cargo search tokio-util; ASSUMED] |
| `tempfile` | 3.27.0 | Isolated filesystem fixtures for integration-style tests | Add as dev-dependency if fake adapters are not enough for adapter implementation tests. [VERIFIED: cargo search tempfile] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `sysinfo` process enumeration | Handwritten Windows Toolhelp/WMI via `windows` crate | Handwritten Windows process APIs may match parent-chain details but increase unsafe/API surface; start with a trait so implementation can change. [ASSUMED] |
| `pelite` for version resources | Direct Win32 `GetFileVersionInfoW` via `windows` crate | Direct API mirrors Python `win32api.GetFileVersionInfo`, but `pelite` is testable against fixture bytes and can work off Windows for PE files. [VERIFIED: CMT/src/utils.py; CITED: docs.rs/pelite/latest/pelite/resources/version_info/index.html] |
| `open` for URL/path launch | `std::process::Command` with `cmd /C start` / `xdg-open` / `open` | Handwritten shell launching is platform-specific and injection-prone; keep it behind adapter if needed. [ASSUMED] |
| `tokio-util::CancellationToken` | Custom `AtomicBool` / channel message | Custom cancellation is enough for Phase 3 event tests, but `CancellationToken` gives a common reusable primitive for later long tasks. [ASSUMED] |

**Installation:**
```bash
cargo add windows-registry sysinfo pelite open
cargo add --dev tempfile
# Optional only if runtime cancellation is implemented in Phase 3:
cargo add tokio-util
```

**Version verification:** `cargo search` on 2026-05-17 returned `tokio 1.52.3`, `windows-registry 0.6.1`, `open 5.3.5`, `sysinfo 0.39.2`, `walkdir 2.5.0`, `tokio-util 0.7.18`, `pelite 0.10.0`, and `tempfile 3.27.0`. [VERIFIED: cargo search]

## Architecture Patterns

### System Architecture Diagram

```text
Slint callbacks / startup controller
        |
        v
Rust app controller enqueues WorkerCommand or calls discovery service
        |
        v
WorkerRuntime / DiscoveryService
        |
        +--> ProcessAdapter.list_processes + version metadata
        |       |
        |       +--> ManagerContext (Mod Organizer / Vortex / none)
        |
        +--> RegistryAdapter.current values (Windows real or fake)
        |
        +--> FileSystemAdapter.exists/read_dir/read_text
                |
                +--> GameInstallation { game_path, data_path?, f4se_path?, INI paths, archives, modules }
        |
        v
WorkerEvent { envelope, payload }
        |
        v
EventLoopHandoff.invoke(event)
        |
        v
Slint event loop applies owned event to controller/UI state
```

### Recommended Project Structure

```text
src/
├── domain/
│   ├── discovery.rs       # GameInstallation, GamePaths, ArchiveSet, ModuleSet, DiscoveryError
│   ├── mod_manager.rs     # ManagerKind, ManagerContext, Mo2Context, Mo2Ini parser results
│   └── settings.rs        # Existing settings domain remains unchanged
├── platform/
│   ├── filesystem.rs      # FileSystem trait, RealFileSystem, FakeFileSystem for tests
│   ├── process.rs         # ProcessAdapter trait, ProcessInfo, DesktopActionResult
│   ├── registry.rs        # RegistryAdapter trait, WindowsRegistryAdapter, FakeRegistry
│   └── settings_store.rs  # Existing settings adapter pattern
├── services/
│   └── discovery.rs       # Orchestrates adapters into typed discovery result
└── workers/
    ├── events.rs          # WorkerEvent envelope, TaskKind, TaskStatus, payload variants
    ├── handoff.rs         # EventSink trait, SlintEventLoopSink, RecordingEventSink
    └── mod.rs             # WorkerRuntime facade and command routing
```

### Pattern 1: Adapter Traits With Fake-Backed Tests

**What:** Define small, synchronous traits for platform operations (`exists`, `is_file`, `is_dir`, `read_text`, `read_dir`, registry string lookup, process list, version metadata, open URL/path, launch tool). [VERIFIED: 03-SPEC.md]  
**When to use:** Discovery and adapter tests must run without real Fallout 4 installs, process lists, registry state, or desktop handlers. [VERIFIED: 03-SPEC.md]  
**Example:**
```rust
/// Filesystem operations discovery needs without binding tests to the real OS.
pub trait FileSystem {
    fn is_file(&self, path: &Path) -> Result<bool, FileSystemError>;
    fn is_dir(&self, path: &Path) -> Result<bool, FileSystemError>;
    fn read_text(&self, path: &Path) -> Result<String, FileSystemError>;
    fn read_dir_sorted(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError>;
}
```
Source: existing `SettingsStore`/`AssetResolver` trait-based IO seam. [VERIFIED: src/platform/settings_store.rs]

### Pattern 2: Reference-Compatible Discovery Pipeline

**What:** Run discovery in the locked order: manager game path, current working directory, registry values; validate candidate paths with `is_dir(path) && is_file(path / "Fallout4.exe")`; normalize direct `Fallout4.exe` inputs to parent directory. [VERIFIED: CMT/src/game_info.py; CMT/src/utils.py]  
**When to use:** Any startup or Overview refresh discovery path. [VERIFIED: 03-SPEC.md]  
**Example:**
```rust
fn normalize_candidate(path: &Path, fs: &dyn FileSystem) -> Result<PathBuf, DiscoveryError> {
    let candidate = if fs.is_file(path)? { path.parent().unwrap_or(path) } else { path };
    if fs.is_dir(candidate)? && fs.is_file(&candidate.join("Fallout4.exe"))? {
        return Ok(candidate.to_path_buf());
    }
    Err(DiscoveryError::not_found())
}
```
Source: `GameInfo.find_path` lines 217-275 and `utils.is_fo4_dir` lines 171-172. [VERIFIED: CMT/src/game_info.py; CMT/src/utils.py]

### Pattern 3: Worker Event Envelope + Typed Payload

**What:** Use one envelope with task identity/kind/status plus payload variants. [VERIFIED: 03-CONTEXT.md]  
**When to use:** All long-running or externally-triggered work, including future scanner, patch, download, and external-process tasks. [VERIFIED: 03-SPEC.md]  
**Example:**
```rust
pub struct WorkerEvent {
    pub task_id: TaskId,
    pub task_kind: TaskKind,
    pub status: TaskStatus,
    pub payload: WorkerPayload,
}

pub enum TaskStatus {
    Started,
    Progress { text: Option<String>, current: Option<u64>, total: Option<u64> },
    CancellationRequested,
    CancellationAcknowledged,
    Completed,
    Cancelled,
    Failed,
}
```
Source: locked event-shape decisions D-09 through D-12. [VERIFIED: 03-CONTEXT.md]

### Pattern 4: Slint Event-Loop Sink

**What:** Worker threads send owned domain events to a sink that schedules event-loop application; no Slint models or UI handles are mutated from workers. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]  
**When to use:** Any background completion/progress/cancel/error event that affects UI state. [VERIFIED: AGENTS.md; 03-SPEC.md]  
**Example:**
```rust
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: WorkerEvent) -> Result<(), HandoffError>;
}

pub struct SlintEventLoopSink<F> {
    apply: F,
}

impl<F> EventSink for SlintEventLoopSink<F>
where
    F: Fn(WorkerEvent) + Send + Sync + Clone + 'static,
{
    fn emit(&self, event: WorkerEvent) -> Result<(), HandoffError> {
        let apply = self.apply.clone();
        slint::invoke_from_event_loop(move || apply(event)).map_err(HandoffError::from)
    }
}
```
Source: Slint docs state `invoke_from_event_loop` is thread-safe and closures execute on the event-loop thread. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]

### Anti-Patterns to Avoid

- **Mutating Slint state from Tokio tasks:** Slint docs require UI work to run on the event-loop thread; send owned events and apply them in the event loop. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]
- **Returning `None` for known failures:** Known discovery/adapter failures need typed error kinds plus safe messages. [VERIFIED: 03-CONTEXT.md]
- **Letting MO2 errors silently fall through to registry discovery:** Locked decision D-07 requires manager-specific typed errors for missing/incomplete/non-Fallout MO2 state. [VERIFIED: 03-CONTEXT.md]
- **Adding manual picker UI in Phase 3:** The phase returns recoverable discovery results; later UI phases decide how to prompt. [VERIFIED: 03-CONTEXT.md]
- **Using real filesystem/process/registry in acceptance tests:** Fake-backed tests are explicit acceptance criteria. [VERIFIED: 03-SPEC.md]

## Reference Behavior to Preserve

| Behavior | Reference Source | Planning Impact |
|----------|------------------|-----------------|
| `GameInfo` constructs manager context before path discovery and then loads game INIs. | `GameInfo.__init__` calls `find_mod_manager()`, `find_path()`, `load_game_inis()`. [VERIFIED: CMT/src/game_info.py] | Discovery service should return manager and game state together or make ordering explicit. |
| `Data` path and `Data/F4SE/Plugins` are derived only if directories exist. | `game_path` setter sets missing paths to `None`. [VERIFIED: CMT/src/game_info.py] | Do not fail valid game discovery when these directories are absent. |
| Documents INI path is `Documents\My Games\Fallout4` with `Fallout4.ini`, `Fallout4Prefs.ini`, and `Fallout4Custom.ini`. | `load_game_inis` reads those names. [VERIFIED: CMT/src/game_info.py] | Model INI paths/read results for later language/archive decisions. |
| English BA2 suffixes are `main`, `textures`, `voices_en`; non-English adds `voices_{language}`. | `load_game_inis` sets `ba2_suffixes`. [VERIFIED: CMT/src/game_info.py] | Archive-set representation should not hard-code English only. |
| Parent-process walk checks up to 8 ancestors for `ModOrganizer.exe` or `Vortex.exe`. | `find_mod_manager` loop. [VERIFIED: CMT/src/utils.py] | Process adapter should expose parent PID or already-resolved ancestor list. |
| Manager version falls back to `0.0.0` when file version metadata is unavailable. | `find_mod_manager` uses `Version("0.0.0")` fallback. [VERIFIED: CMT/src/utils.py] | Version parser should return typed `Version { major, minor, patch }`, defaulting on metadata miss. |
| MO2 portable mode checks `portable.txt` and `ModOrganizer.ini` beside the executable. | `find_path` portable branch. [VERIFIED: CMT/src/game_info.py] | Adapter fixtures need executable-dir MO2 INI cases. |
| MO2 instance mode reads `HKCU\Software\Mod Organizer Team\Mod Organizer` `CurrentInstance` and then `LOCALAPPDATA\ModOrganizer\{instance}\ModOrganizer.ini`. | `find_path` instance branch. [VERIFIED: CMT/src/game_info.py] | Registry and environment access should be adapter inputs. |
| MO2 defaults include `%BASE_DIR%/mods`, `%BASE_DIR%/overwrite`, `%BASE_DIR%/profiles`, `.mohidden`, and empty skip directories. | `read_mo2_ini` default map. [VERIFIED: CMT/src/mod_manager_info.py] | MO2 parser tests should cover defaults and `%BASE_DIR%` substitution. |
| Non-Fallout MO2 INI raises `Only Fallout 4 is supported.\ngameName is '...' in INI: ...`. | `read_mo2_ini`. [VERIFIED: CMT/src/mod_manager_info.py] | Preserve as safe user-facing manager-specific message. |
| Missing MO2 selected profile raises `Profile is not set in ModOrganizer.ini.` | `read_mo2_ini`. [VERIFIED: CMT/src/mod_manager_info.py] | Preserve as manager-specific typed error. |
| Invalid registry game path message includes `A Fallout 4 installation could not be found.` and registry path guidance. | `find_path` invalid registry branch. [VERIFIED: CMT/src/game_info.py] | Acceptance tests must assert message text. |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Slint UI thread handoff | Custom cross-thread UI mutation or raw component sharing | `slint::invoke_from_event_loop` / `Weak::upgrade_in_event_loop` | Slint documents these as thread-safe and event-loop-bound. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html] |
| Blocking filesystem/process work in async tasks | Direct blocking calls on the UI/event-loop path | Tokio runtime + `spawn_blocking` + channels | Tokio documents `spawn_blocking` and `mpsc` bridging for sync/blocking work. [CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html] |
| Windows registry access parser | Manual `reg.exe` subprocess parsing | `windows-registry` behind `RegistryAdapter` | Registry values are part of OS API; subprocess text parsing is brittle. [VERIFIED: cargo search windows-registry; ASSUMED] |
| Windows PE version metadata | Ad hoc byte parsing | `pelite` or a focused Win32 version API wrapper behind `ProcessAdapter` | Python uses Windows version metadata; `pelite` has version-info resource support. [VERIFIED: CMT/src/utils.py; CITED: docs.rs/pelite/latest/pelite/resources/version_info/index.html] |
| URL/path opening | Inline shell commands in UI callbacks | `DesktopAdapter` wrapping `open` or platform API | Failures must be typed and visible, and adapters must not show dialogs. [VERIFIED: 03-CONTEXT.md] |

**Key insight:** The stable product contract is typed discovery/worker events, not the first production implementation of Windows process/registry APIs; traits let later phases improve real adapters without rewriting domain tests. [VERIFIED: 03-SPEC.md]

## Common Pitfalls

### Pitfall 1: Treating Missing `Data` as Discovery Failure
**What goes wrong:** A valid `Fallout4.exe` directory is rejected because `Data` or `Data/F4SE/Plugins` is missing. [VERIFIED: CMT/src/game_info.py]  
**Why it happens:** Later F4SE/scanner workflows need those folders, but the reference stores them as optional derived state. [VERIFIED: CMT/src/game_info.py]  
**How to avoid:** Validate only the game dir and `Fallout4.exe`; model missing derived paths as `None`. [VERIFIED: 03-CONTEXT.md]  
**Warning signs:** Tests require `Data` to exist before `GameInstallation` is returned. [VERIFIED: 03-SPEC.md]

### Pitfall 2: Scope Creep Into Scanner/F4SE/Overview UI
**What goes wrong:** Phase 3 starts rendering diagnostics panels or running scanner classifications. [VERIFIED: 03-SPEC.md]  
**Why it happens:** Discovery state is consumed by those later phases. [VERIFIED: ROADMAP.md]  
**How to avoid:** Keep Phase 3 contract/model/test focused; later phases consume adapters/events. [VERIFIED: 03-CONTEXT.md]  
**Warning signs:** New Slint visible controls beyond minimal binding/handoff proof. [VERIFIED: 03-SPEC.md]

### Pitfall 3: Non-Deterministic Enumeration Breaking Future Snapshot Tests
**What goes wrong:** Archive/module sets have unstable order, causing flaky tests and UI diffs. [ASSUMED]  
**Why it happens:** Filesystem APIs often return directory entries in OS-dependent order. [ASSUMED]  
**How to avoid:** Adapter should offer sorted enumeration or domain should sort `PathBuf`s before returning. [ASSUMED]  
**Warning signs:** Tests compare vectors from fake/real filesystem without sorting. [ASSUMED]

### Pitfall 4: Losing Reference MO2 Error Text
**What goes wrong:** Rust returns generic `InvalidConfig` instead of messages like `Profile is not set in ModOrganizer.ini.` [VERIFIED: CMT/src/mod_manager_info.py]  
**Why it happens:** Error enums separate kinds from messages but forget reference text. [VERIFIED: 03-CONTEXT.md]  
**How to avoid:** Store `kind`, `user_message`, and optional diagnostic fields on known errors. [VERIFIED: 03-CONTEXT.md]  
**Warning signs:** Tests assert only error variants, not user-facing strings. [VERIFIED: 03-SPEC.md]

### Pitfall 5: Slint Handles Crossing Worker Boundaries
**What goes wrong:** Background tasks capture `MainWindow` or Slint model objects directly. [CITED: docs.slint.dev/latest/docs/rust/slint/struct.ModelRc.html]  
**Why it happens:** It is convenient to update UI from completion closures. [ASSUMED]  
**How to avoid:** Worker emits owned `WorkerEvent`; controller applies via `invoke_from_event_loop`/`upgrade_in_event_loop`. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]  
**Warning signs:** Worker modules import generated Slint component types. [ASSUMED]

## Code Examples

Verified patterns from official or local sources:

### Slint Event-Loop Handoff
```rust
let handle_weak = handle.as_weak();
std::thread::spawn(move || {
    let foo = 42;
    handle_weak.upgrade_in_event_loop(move |handle| handle.set_foo(foo));
});
```
Source: Slint `Weak::upgrade_in_event_loop` example. [CITED: docs.slint.dev/latest/docs/rust/slint/struct.Weak.html]

### Tokio Blocking Work Progress Channel
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(2);
let worker = tokio::task::spawn_blocking(move || {
    for x in 0..10 {
        tx.blocking_send(x).unwrap();
    }
});
while let Some(value) = rx.recv().await {
    // Convert progress values into WorkerEvent payloads here.
}
worker.await.unwrap();
```
Source: Tokio `spawn_blocking` docs show `mpsc` with `blocking_send` from blocking code. [CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html]

### Reference Fallout 4 Directory Test
```python
def is_fo4_dir(path: Path) -> bool:
    return is_dir(path) and is_file(path / "Fallout4.exe")
```
Source: Python reference. [VERIFIED: CMT/src/utils.py]

### Reference Invalid Registry Message
```text
A Fallout 4 installation could not be found.

The path set in your registry is:
{registry_path}

If this is not correct, please run the Fallout 4 Launcher to correct it.
```
Source: Python reference. [VERIFIED: CMT/src/game_info.py]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Python Tkinter directly asks message boxes/file dialogs during discovery. | Rust Phase 3 returns recoverable typed discovery results with reference-compatible messages and no manual picker UI. | Locked in Phase 3 context. [VERIFIED: 03-CONTEXT.md] | Planner should not add Slint file picker UI in Phase 3. |
| Python `GameInfo` mutates `StringVar` and can `sys.exit()` on discovery failure. | Rust should separate pure domain results from UI decisions and never exit from domain/platform discovery. | Port architecture requirement. [VERIFIED: AGENTS.md; CMT/src/game_info.py] | Planner should test errors as values. |
| Inert `WorkerRuntime` marker. | Shared worker events and Slint-safe event-loop sink. | Phase 3 target. [VERIFIED: src/workers/mod.rs; 03-SPEC.md] | Planner should replace/extend marker with real contracts. |
| Settings-only platform adapter. | Broader filesystem/registry/process/desktop adapters. | Phase 3 target. [VERIFIED: src/platform/settings_store.rs; 03-SPEC.md] | Planner can mirror `AssetResolver` style. |

**Deprecated/outdated:**
- Direct UI-thread blocking for update checks/discovery is not acceptable in the Rust port because AGENTS.md requires slow filesystem/process work off the Slint UI thread. [VERIFIED: AGENTS.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tokio-util::CancellationToken` is preferable if Phase 3 implements runtime cancellation handles rather than only cancellation event shapes. | Standard Stack | Planner may add an unnecessary dependency; can defer until first cancellable workflow. |
| A2 | Handwritten shell launching is more brittle and injection-prone than wrapping `open` or platform APIs. | Alternatives / Don't Hand-Roll | If `open` cannot provide required behavior, adapter implementation may need platform-specific code. |
| A3 | Directory enumeration order is OS-dependent enough to require explicit sorting. | Common Pitfalls | If not sorted, later tests may be flaky; if over-sorted, could diverge from reference ordering where order matters. |
| A4 | Workers importing generated Slint component types is a warning sign of boundary leakage. | Common Pitfalls | Some adapter implementations might safely hold `Weak<MainWindow>` in app layer; keep generated types out of domain/worker core. |

## Open Questions (RESOLVED)

1. **Should Phase 3 add `walkdir` now or defer it to Scanner Phase 7?**
   - What we know: Phase 3 requires deterministic directory enumeration inputs for archive/module collections, but full scanner traversal is out of scope. [VERIFIED: 03-SPEC.md]
   - RESOLVED: Phase 3 will add only a minimal `read_dir_sorted` adapter for deterministic shallow enumeration. `walkdir` is deferred to Scanner Phase 7 unless implementation proves recursive archive/module fixture enumeration is required by the locked Phase 3 acceptance tests. [VERIFIED: 03-SPEC.md; 03-PATTERNS.md]
2. **Should process version metadata use `pelite` or direct Win32 APIs first?**
   - What we know: Python uses `win32api.GetFileVersionInfo`; `pelite` has PE version-info docs. [VERIFIED: CMT/src/utils.py; CITED: docs.rs/pelite/latest/pelite/resources/version_info/index.html]
   - RESOLVED: Version metadata remains behind `ProcessAdapter`. Phase 3 acceptance relies on fake-backed tests for parsed version and `0.0.0` fallback behavior. The first real implementation should use `pelite` behind the adapter for portable PE fixture parsing; direct Win32 APIs remain an implementation swap if `pelite` diverges from reference behavior. [VERIFIED: 03-SPEC.md; CMT/src/utils.py; 03-PATTERNS.md]
3. **How much Slint integration should Phase 3 test?**
   - What we know: Acceptance requires a testable handoff seam and code review that workers do not mutate Slint objects directly. [VERIFIED: 03-SPEC.md]
   - RESOLVED: Phase 3 will unit-test `RecordingEventSink` without launching a Slint window and add a narrow compile-level `SlintEventLoopSink` wrapper using `slint::invoke_from_event_loop`. GUI event-loop behavior is deferred to later UI phases if headless execution is unreliable; Phase 3 still verifies that core worker/event code does not import generated Slint component/model types. [VERIFIED: 03-SPEC.md; 03-PATTERNS.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust compiler | Build/test Phase 3 | ✓ | `rustc 1.95.0` | — [VERIFIED: rustc --version] |
| Cargo | Dependency/build/test commands | ✓ | `cargo 1.95.0` | — [VERIFIED: cargo --version] |
| Git | Verify `CMT/` unchanged | ✓ | `2.54.0.windows.1` | — [VERIFIED: git --version] |
| PowerShell | Local command execution on Windows | ✓ | `7.6.1` | — [VERIFIED: pwsh --version] |
| Fallout 4 install | Production discovery manual validation | Unknown | — | Fake filesystem/registry fixtures satisfy automated tests. [VERIFIED: 03-SPEC.md] |
| Running MO2/Vortex | Production process detection manual validation | Unknown | — | Fake process fixtures satisfy automated tests. [VERIFIED: 03-SPEC.md] |
| Windows registry state | Production registry discovery manual validation | Unknown | — | Fake registry fixtures and non-Windows `UnsupportedPlatform` errors. [VERIFIED: 03-CONTEXT.md] |

**Missing dependencies with no fallback:** None for automated Phase 3 planning/implementation; fake adapters cover required tests. [VERIFIED: 03-SPEC.md]

**Missing dependencies with fallback:** Real Fallout 4/MO2/Vortex/registry state may be absent; use fake fixtures and typed unsupported/not-found results. [VERIFIED: 03-SPEC.md]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness through Cargo. [VERIFIED: existing inline `#[cfg(test)]` modules] |
| Config file | `Cargo.toml`; no separate test config. [VERIFIED: Cargo.toml] |
| Quick run command | Use task-specific split commands after module names exist, e.g. `cargo test discovery -- --nocapture && cargo test filesystem_adapter -- --nocapture && cargo test workers:: -- --nocapture`; before then use targeted module tests. [ASSUMED] |
| Full suite command | `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features` [VERIFIED: AGENTS.md] |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| DISC-01 | Valid FO4 dir, executable-path normalization, missing install, invalid registry message, optional derived `Data`/F4SE paths, INI/archive/module representation | unit | `cargo test discovery:: -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| DISC-02 | MO2/Vortex process detection, version parse and `0.0.0` fallback, MO2 portable/instance INI parse, non-Fallout/profile errors | unit | `cargo test mod_manager:: -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| DISC-03 | FileSystem fake supports file/dir existence, text read, deterministic enumeration, no real filesystem in discovery tests | unit | `cargo test filesystem_adapter -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| DISC-04 | Process/desktop fake covers process list, version metadata success/failure, URL/path/tool launch success/failure typed results | unit | `cargo test process_adapter -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| SAFE-01 | Worker runtime exposes off-UI-thread command path for blocking work | unit/code review | `cargo test workers:: -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| SAFE-02 | Events cover start/progress/counts/completion/cancellation/error and handoff sink routes owned events | unit | `cargo test worker_events -- --nocapture && cargo test handoff -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |
| SAFE-03 | Domain discovery tests use fake filesystem/process/registry and no Slint window | unit | `cargo test discovery -- --nocapture` | ❌ Wave 0 [VERIFIED: 03-SPEC.md] |

### Sampling Rate
- **Per task commit:** `cargo fmt --check && cargo test <touched_module>` [VERIFIED: AGENTS.md]
- **Per wave merge:** `cargo check && cargo test` [VERIFIED: AGENTS.md]
- **Phase gate:** `cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features`, plus `git status --short CMT` shows no modifications. [VERIFIED: AGENTS.md]

### Wave 0 Gaps
- [ ] `src/domain/discovery.rs` tests — covers DISC-01. [VERIFIED: 03-SPEC.md]
- [ ] `src/domain/mod_manager.rs` tests — covers DISC-02 and MO2 parser parity. [VERIFIED: CMT/src/mod_manager_info.py]
- [ ] `src/platform/filesystem.rs` fake and tests — covers DISC-03. [VERIFIED: 03-SPEC.md]
- [ ] `src/platform/process.rs` fake and tests — covers DISC-04. [VERIFIED: 03-SPEC.md]
- [ ] `src/platform/registry.rs` fake and Windows real stub tests — supports DISC-01/DISC-02. [VERIFIED: CMT/src/game_info.py]
- [ ] `src/services/discovery.rs` orchestration tests — covers discovery ordering and error text. [VERIFIED: CMT/src/game_info.py]
- [ ] `src/workers/events.rs` and `src/workers/handoff.rs` tests — covers SAFE-01/SAFE-02. [VERIFIED: 03-SPEC.md]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No authentication/session capability in Phase 3. [VERIFIED: 03-SPEC.md] |
| V3 Session Management | no | No sessions in a local desktop adapter phase. [VERIFIED: 03-SPEC.md] |
| V4 Access Control | no | No multi-user authorization model; local filesystem permissions are respected by OS errors. [ASSUMED] |
| V5 Input Validation | yes | Validate paths as files/dirs through adapters; parse INI/config values into typed enums/structs; never trust registry/process strings blindly. [VERIFIED: CMT/src/game_info.py; 03-SPEC.md] |
| V6 Cryptography | no | No cryptography in Phase 3. [VERIFIED: 03-SPEC.md] |

### Known Threat Patterns for Rust Desktop Platform Adapters

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Launching untrusted URL/path/tool targets | Tampering / Elevation of Privilege | Keep launch/open in a typed adapter, validate operation kind and target type, return typed failure instead of shell-string construction. [VERIFIED: 03-CONTEXT.md; ASSUMED] |
| Leaking raw OS paths/errors into UI | Information Disclosure | Return safe user messages and log raw diagnostics with `tracing` only when needed. [VERIFIED: 03-CONTEXT.md] |
| Malformed MO2 INI causing panic | Denial of Service | Parse as recoverable typed errors; avoid `unwrap()`/`expect()` on user-controlled files. [VERIFIED: AGENTS.md; CMT/src/mod_manager_info.py] |
| Blocking UI via process/filesystem traversal | Denial of Service | Use worker runtime and event-loop handoff for slow work. [VERIFIED: AGENTS.md; CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html] |

## Sources

### Primary (HIGH confidence)
- `03-CONTEXT.md` — locked implementation decisions D-01 through D-16, scope boundaries, canonical references. [VERIFIED: .planning/phases/03-platform-discovery-background-adapters/03-CONTEXT.md]
- `03-SPEC.md` — locked Phase 3 requirements and acceptance criteria. [VERIFIED: .planning/phases/03-platform-discovery-background-adapters/03-SPEC.md]
- `AGENTS.md` — read-only `CMT/`, Rust/Slint architecture, UI-thread and verification constraints. [VERIFIED: AGENTS.md]
- `CMT/src/game_info.py` — Fallout 4 discovery order, path normalization, derived paths, INI loading, and not-found messages. [VERIFIED: CMT/src/game_info.py]
- `CMT/src/utils.py` — `is_fo4_dir`, process manager detection, version fallback, registry helper, encoded text helpers. [VERIFIED: CMT/src/utils.py]
- `CMT/src/mod_manager_info.py` — MO2 INI parsing, defaults, `%BASE_DIR%`, skip rules, and error messages. [VERIFIED: CMT/src/mod_manager_info.py]
- `src/platform/settings_store.rs` and `src/app/settings_controller.rs` — existing injectable IO/controller patterns and fake-backed tests. [VERIFIED: local source]
- Slint Rust docs — `invoke_from_event_loop`, `Weak::upgrade_in_event_loop`, and model update handoff examples. [CITED: docs.slint.dev/latest/docs/rust/slint/fn.invoke_from_event_loop.html]
- Tokio docs — `spawn_blocking` and `mpsc` bridging with `blocking_send`. [CITED: docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html]

### Secondary (MEDIUM confidence)
- `sysinfo` docs — process refresh and executable path access. [CITED: docs.rs/sysinfo/latest/sysinfo/struct.Process.html]
- `pelite` docs — PE version-info resource support. [CITED: docs.rs/pelite/latest/pelite/resources/version_info/index.html]
- crates.io/cargo search — current crate versions for recommended additions. [VERIFIED: cargo search]

### Tertiary (LOW confidence)
- Assumptions around launch-command injection risk, enumeration ordering, and cancellation primitive choice. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — existing dependencies verified in `Cargo.toml`; candidate dependency versions verified with `cargo search`; API patterns checked in official docs where available. [VERIFIED: Cargo.toml; cargo search; Context7]
- Architecture: HIGH — constrained by locked SPEC/CONTEXT and existing module seams. [VERIFIED: 03-SPEC.md; src/domain/mod.rs; src/platform/mod.rs; src/workers/mod.rs]
- Reference behavior: HIGH — directly inspected `CMT/src/game_info.py`, `utils.py`, `mod_manager_info.py`, and `cm_checker.py`. [VERIFIED: CMT/src]
- Pitfalls: MEDIUM — core pitfalls are verified by spec/reference; ordering and cancellation details include assumptions. [VERIFIED: 03-SPEC.md; ASSUMED]

**Research date:** 2026-05-17  
**Valid until:** 2026-06-16 for architecture/reference findings; re-check crate versions before implementation dependency edits. [ASSUMED]