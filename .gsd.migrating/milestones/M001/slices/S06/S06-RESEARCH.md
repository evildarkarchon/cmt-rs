# S06 Research: F4SE Diagnostics

## Requirements Scope

No active `REQUIREMENTS.md` entries were preloaded for this slice. S06 owns the F4SE diagnostics tab behavior described in the slice context: reference-shaped table, lazy non-blocking scan, reference DLL export/version classification, missing-folder messages, and safe handling for unreadable/malformed DLLs.

## Skills Discovered

- Installed skills already relevant from the prompt: `rust-async-patterns` is adjacent for worker/runtime handoff, but this slice mostly follows existing local worker patterns rather than new async API design.
- `npx skills find "Slint"` returned accessibility/lint skills, not a Slint/Rust GUI skill relevant enough to install.
- `npx skills find "PE DLL parsing"` returned .NET decompile/reverse-engineering skills that are tangential and not suitable for a Rust PE export parser implementation.
- `npx skills find "Rust GUI"` returned `bobmatnyc/claude-mpm-skills@rust-desktop-applications` as plausibly relevant, but `npx skills add ... -g -y` failed with `No matching skills found for: rust-desktop-applications`; no new skill was installed.

## Summary

S06 is a targeted-to-deep slice: the app already has strong Slint/controller/worker patterns from S04/S05, but the risky part is faithfully replacing Python `WinDLL(... DONT_RESOLVE_DLL_REFERENCES)` with a safe, cross-platform-testable DLL export/version parser. The best implementation path is to add pure F4SE domain models, an adapter-backed scan service over `Filesystem`, a PE export inspector (recommended: `pelite = "0.10.0"`), a Slint-free controller with monotonic scan IDs, and MainWindow wiring that triggers the scan once when the F4SE tab becomes active.

Do not infer support from DLL names or file version metadata. The reference only proves support from exported F4SE symbols and `F4SEPlugin_Version.compatibleVersions` values.

## Reference Behavior to Preserve

Source of truth files:

- `CMT/src/tabs/_f4se.py`
  - Tab title: `F4SE`.
  - Loading text: `Scanning DLLs...`.
  - Missing data message: `Data folder not found`.
  - Missing plugin folder message: `Data/F4SE/Plugins folder not found`, with `\nTry launching via your mod manager.` appended when no manager is detected.
  - Scans direct children of `Data/F4SE/Plugins`, only `*.dll`, and skips names starting with `msdia`.
  - Table headings: `DLL`, `OG`, `NG`, `AE`, `Your Game`.
  - Layout: left tree/table plus scrollbar, right heading `F4SE DLLs`, right legend text.
  - Reference cell/icon semantics:
    - unknown/non-F4SE: `❓` (`BLACK QUESTION MARK ORNAMENT`) for OG/NG/AE; reference leaves `Your Game` blank because it inserts only three values.
    - supported: `✔` (`HEAVY CHECK MARK`).
    - unsupported for OG/NG/AE columns: empty string via `EMOJI_DLL_BAD = ""`.
    - unsupported for `Your Game`: `❌` (`CROSS MARK`).
    - ambiguous NG/AE support from `F4SEPlugin_Version` without a known compatible version: `⚠` (`WARNING SIGN`).
    - row tag is `note` when `Your Game` is `⚠`, `good` when `✔`, otherwise `bad`; non-F4SE rows are neutral.
- `CMT/src/utils.py::parse_dll`
  - `IsF4SE = has F4SEPlugin_Load OR F4SEPlugin_Preload`.
  - `SupportsOG = has F4SEPlugin_Query`.
  - `SupportsNGAE = has F4SEPlugin_Version`.
  - If `SupportsNGAE`, read `F4SEPlugin_Version.compatibleVersions`.
  - `SupportsNG = true` only if compatible versions contain `0x010A3D40` or `0x010A3D80`.
  - `SupportsAE = true` only if any compatible version is `> 0x010B0890`.
  - Otherwise `SupportsNG`/`SupportsAE` stay `None`, which renders as warning for NG/AE when `SupportsNGAE` is true.
- `CMT/src/globals.py::ABOUT_F4SE_DLLS`
  - Exact legend text:

```text
This checks all DLLs in
Data/F4SE/Plugins/ for
version-specific code to
determine OG/NG support.

✔ Version is supported

❌ Version not supported

❓ Not an F4SE DLL.
May still be loaded by
other DLLs.

⚠ Consult mod page to
verify version support if
you see this icon.
Some DLLs' version support
cannot be reliably
determined.
```

Important intentional Rust safety improvement required by slice scope: the Python reference can abort `_load()` if `parse_dll` raises. S06 must keep unreadable, malformed, unsupported-host, or unclassifiable DLLs visible as rows and continue scanning.

## Existing Rust Landscape

### Established patterns to reuse

- `src/workers/mod.rs` provides `WorkerRuntime::spawn_blocking_task`, lifecycle events, panic-to-failure mapping, and cancellation tokens. Use this for filesystem enumeration and DLL parsing.
- `src/workers/handoff.rs` provides `WorkerEventSink`, `RecordingEventSink`, and `SlintEventLoopSink`. Follow memory MEM013/MEM025: workers emit owned events only; Slint handles/models must only be mutated on the event loop.
- `src/workers/events.rs` already has typed payload variants for Overview and Tools/About. Add an F4SE-specific payload rather than carrying unstructured scan strings.
- `src/app/overview_controller.rs` is the best reducer template: monotonic request IDs, stale-result rejection, spawn-failure mapping, and pure state transitions.
- `src/app/tools_controller.rs` / `src/app/about_controller.rs` are simpler examples of Slint-free reducers that apply owned worker payloads and ignore unrelated payloads.
- `src/main.rs` has the wiring patterns to copy:
  - Create controller as `Arc<Mutex<_>>`.
  - Bind a `SlintEventLoopSink` that upgrades the weak `MainWindow`, applies reducer event, and projects state into Slint properties.
  - Schedule work through `WorkerRuntime` and handle `WorkerSpawnError` synchronously by updating the controller.
  - Project domain rows to exported Slint row structs through `ModelRc<VecModel<_>>`.
- `ui/overview_tab.slint` and `ui/tools_tab.slint` show the current conservative dark UI style: `#202020` background, fixed-width left labels/buttons, safe error banners, and `ScrollView` for longer content.

### Current gaps for S06

- `ui/f4se_tab.slint` is still a centered placeholder.
- `ui/main.slint` imports `F4seTab` but does not expose F4SE properties, row model, activation callback, or tab-current binding.
- `src/app/mod.rs` exports no F4SE controller module.
- `src/domain/mod.rs` exports no F4SE domain module.
- `src/services/mod.rs` exports no F4SE scan service.
- `src/workers/events.rs` has no F4SE payload variant.
- `src/main.rs` initializes Overview/Tools/About controllers but not F4SE.

### Discovery and game-version caveat

- `src/services/discovery.rs::installation_for_game_path` supplies optional `data_path` and `f4se_plugins_path` from the discovered game path. This covers the missing-folder checks.
- `DiscoveryService` alone does **not** classify the current game version; `Fallout4Installation.install_type` defaults to `Unknown` until Overview binary collection runs.
- To fill the `Your Game` column correctly, production F4SE scanning should either:
  1. run `OverviewCollector` and derive the current game type from its `Fallout4.exe` binary fact, or
  2. factor the base-file classification helper out of `overview_collector` into a shared service.
- Lowest-risk path: reuse `OverviewCollector` in the F4SE worker initially, then derive `OldGen/DownGrade -> OG`, `NextGen -> NG`, `Anniversary -> AE`; treat `Unknown`, `Obsolete`, `NextGenAnniversary`, and `NotFound` as unclassifiable for `Your Game`.

## Recommended Design

### New domain module: `src/domain/f4se.rs`

Purpose: pure reference contract and display semantics, no filesystem/Slint/platform calls.

Recommended contents:

- Reference constants:
  - `F4SE_LOADING_TEXT = "Scanning DLLs..."`
  - `F4SE_DLLS_HEADING = "F4SE DLLs"`
  - `F4SE_TABLE_COLUMNS = ["DLL", "OG", "NG", "AE", "Your Game"]`
  - `ABOUT_F4SE_DLLS` exact legend text.
  - icon constants for `✔`, `❌`, `❓`, `⚠`, plus the reference blank unsupported cell for OG/NG/AE if preserving exact cell text.
- Parse facts mirroring `DLLInfo`:
  - `F4seDllFacts { is_f4se, supports_og, supports_ngae, supports_ng: Option<bool>, supports_ae: Option<bool> }`.
- Scan/display structs:
  - `F4seGameVersion { OldGen, NextGen, Anniversary, Unknown { label: String } }`.
  - `F4seDllRow { dll_name, og, ng, ae, your_game, severity, note }` or typed cells that project to this.
  - `F4seScanResult { rows: Vec<F4seDllRow>, game_version: F4seGameVersion, warning: Option<String> }`.
- Pure functions:
  - `game_version_from_install_type(Fallout4InstallType) -> F4seGameVersion`.
  - `row_from_facts(dll_name, facts/result, game_version) -> F4seDllRow`.
  - `missing_plugins_message(manager_detected: bool) -> String`.

Tests here should lock the reference constants and all compatibility mapping branches before UI/workers are touched.

### New service module: `src/services/f4se.rs`

Purpose: enumerate direct plugin DLLs over `Filesystem`, call an injected inspector/parser, and return pure domain rows/failures.

Recommended shape:

```rust
pub trait F4seDllInspector {
    fn inspect(&self, path: &Path, bytes: &[u8]) -> Result<F4seDllFacts, F4seDllInspectionError>;
}

pub struct F4seScanService<'a, F: Filesystem + ?Sized, I: F4seDllInspector + ?Sized> {
    filesystem: &'a F,
    inspector: &'a I,
}
```

Service responsibilities:

- If `installation.data_path` is `None`, return/load state error `Data folder not found`.
- If `installation.f4se_plugins_path` is `None`, return/load state error `Data/F4SE/Plugins folder not found`, appending the mod-manager hint only when discovery reports no manager.
- `read_dir` only the plugin directory; do not recurse.
- Include only entries where `file_type == File`, extension case-insensitively equals `.dll`, and file name does not start with `msdia` (reference is case-sensitive on `startswith("msdia")`; decide explicitly in implementation and test it).
- Sort order can follow existing `RealFilesystem::read_dir` deterministic path order. This is safer than Python `Path.iterdir()` OS order; document as a deterministic-port choice if visible.
- For each selected DLL:
  - read bytes with `Filesystem::read_bytes` (or `read_prefix` only if parser can work with partial bytes; PE exports need full mapped sections, so whole bytes are simpler).
  - inspector success -> reference row mapping.
  - filesystem read error -> visible row with unknown/warning cells and safe note; continue.
  - inspector parse error -> visible row with unknown/warning cells and safe note; continue.
- Empty plugin folder with no DLLs is success with `rows.is_empty()` and no error.

### PE/DLL inspector recommendation

Use `pelite = "0.10.0"` instead of loading arbitrary mod DLLs.

Rationale:

- The Python reference uses `WinDLL(..., DONT_RESOLVE_DLL_REFERENCES)` to query exports. Loading user DLLs into the toolkit process is riskier and Windows-only; parsing PE bytes is safer, fakeable, and works in CI on non-Windows hosts.
- `cargo info pelite` reports `0.10.0`, MIT, docs at docs.rs, and it is a memory-safe PE reader.
- Cached `pelite` source shows the needed APIs:
  - `pelite::pe64::PeFile::from_bytes(&bytes)`.
  - `PeFile::exports()?.by()?`.
  - `by.name_linear(b"F4SEPlugin_Load")` / `by.name_linear(b"F4SEPlugin_Preload")` / `by.name_linear(b"F4SEPlugin_Query")` / `by.name_linear(b"F4SEPlugin_Version")`.
  - `Export::symbol()` gives the RVA for `F4SEPlugin_Version`.
  - `Pe::slice_bytes` / `Pe::derva_slice::<u32>` can read bytes at that RVA.
- `F4SEPlugin_Version.compatibleVersions` offset is 528 bytes from the exported `F4SEPluginVersionData` base:
  - `dataVersion` u32: 4
  - `pluginVersion` u32: 4
  - `name` char[256]: 256
  - `author` char[256]: 256
  - `addressIndependence` u32: 4
  - `structureIndependence` u32: 4
  - total before `compatibleVersions`: 528
  - then 16 little-endian u32 values.
- Treat absent export directory (`pelite::Error::Null`) as `F4seDllFacts::default()` / not F4SE, not as a malformed DLL. Treat invalid PE headers, corrupt export tables, or unreadable version-data RVAs as row-level warnings.
- Fallout 4/F4SE DLLs are 64-bit. A `pe64` parser is enough for the main path; if a 32-bit DLL appears, the required behavior is a visible unknown/warning row, not a crash. A pe32 fallback can be added later if needed for nicer non-F4SE classification.

### New app controller: `src/app/f4se_controller.rs`

Purpose: Slint-free reducer for lazy state and worker event application.

Recommended state:

```rust
pub enum F4seScanPhase { NotStarted, Loading, Ready, Error }
pub struct F4seState {
    pub phase: F4seScanPhase,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub rows: Vec<F4seDllRow>,
}
```

Recommended behavior:

- `request_scan()`:
  - if `NotStarted` or previous `Error` should be retryable? Scope says first-open lazy scan only and no manual refresh. Prefer only `NotStarted` auto-schedules; failed scans remain visible until a future refresh feature unless planner explicitly wants retry-on-reopen.
  - assign monotonic `scan_id` and set `Loading` with `Scanning DLLs...`.
  - return `F4seScanRequest { scan_id, task() }`.
- `scan_completed(scan_id, result)` applies only if latest.
- `scan_failed(scan_id, WorkerFailure)` applies safe error only if latest.
- `scan_spawn_failed(scan_id, WorkerSpawnError)` maps to a safe `F4SE scan could not be started.` error.
- `handle_worker_event(event)` applies only `WorkerPayload::F4seDiagnostics(...)` and scan failures for task ids prefixed by `f4se-scan-`; unrelated payloads ignored.

### Worker event extension

Recommended additions in `src/workers/events.rs`:

```rust
pub enum WorkerPayload {
    ...
    F4seDiagnostics(F4seWorkerPayload),
}

pub enum F4seWorkerPayload {
    ScanCompleted { scan_id: u64, result: Box<F4seScanResult> },
}
```

Use existing `WorkerTaskKind::Scan` for the task kind unless there is a strong reason to add a new kind. A stable id prefix like `f4se-scan-{scan_id}` is enough for reducer routing and logs.

### Slint UI and lazy activation

`ui/f4se_tab.slint` should replace the placeholder with a reference-shaped layout:

- Export `F4seUiRow` with `dll`, `og`, `ng`, `ae`, `your-game`, `severity`, and optional `note`/`detail`.
- Left side: fixed-width table header and rows matching reference column order and approximate widths (`DLL` ~240px, `OG`/`NG`/`AE` ~60px, `Your Game` ~80px). `ScrollView` around rows is a practical Slint replacement for Tk `Treeview` + scrollbar.
- Right side: heading `F4SE DLLs` and legend text.
- Show `Scanning DLLs...` while loading.
- Show the missing-folder error message as the tab loading error state.
- If rows are empty after a successful scan, show the normal empty table and legend, not an error banner. Avoid inventing a prominent `No DLLs found` error.
- No manual refresh button in this slice.

`ui/main.slint` needs F4SE properties and callback forwarding:

- Import `F4seUiRow`.
- Add properties: `f4se-status-message`, `f4se-loading`, `f4se-error-message`, `f4se-rows`, possibly `f4se-legend-text` if projecting the domain constant instead of hardcoding.
- Add callback `f4se-scan-requested()`.
- Bind the `F4seTab` properties and callback.
- Add a tab-current property and activation trigger.

Cached Slint 1.16.1 widget source shows `TabWidget`/`TabBarBase` has an `in-out property <int> current`; the likely pattern is:

```slint
in-out property <int> active-tab-index: 0;

TabWidget {
    current <=> root.active-tab-index;
    ...
}
```

Then trigger `root.f4se-scan-requested()` when `active-tab-index == 1`. Verify exact Slint property-change syntax with `cargo check`; if `changed active-tab-index => { ... }` is not accepted, the fallback is to bind an `activated` bool into `F4seTab` and use a property-change handler there, or use whatever Slint 1.16 generated property-change API exposes. Do not schedule the scan at startup just to avoid this; scope requires lazy first-open behavior.

### `src/main.rs` wiring

Recommended additions:

- Instantiate `let f4se_controller = Arc::new(Mutex::new(F4seController::new()));`.
- Initial projection after app creation: `apply_current_f4se_state(&app, &f4se_controller);`.
- Add `bind_f4se_worker_sink`, `bind_f4se_callbacks`, `request_f4se_scan`, `build_f4se_scan_payload`, `apply_current_f4se_state`, and `format_f4se_rows` functions following the Overview/Tools patterns.
- Production worker should build a discovery report using the same `DiscoveryRequest` setup as `build_overview_snapshot` (`current_dir`, `LOCALAPPDATA`, current process id), derive manager presence from `report.mod_manager`, then run the F4SE scan service if a game installation exists.
- For current game type, either run `OverviewCollector` and read the `Fallout4.exe` fact, or factor out binary classification. Do not trust the bare `installation.install_type` from `DiscoveryService`.
- Add tracing events for scan requested, spawn failed, discovery failed, missing folder, DLL count, parse/read warning count, worker event applied/ignored/dropped.

## Natural Work Seams for Planner

1. **Pure F4SE domain contract**
   - Files: `src/domain/f4se.rs`, `src/domain/mod.rs`.
   - Tests: constants, legend, icon/cell mapping, game-version mapping, malformed/unknown row mapping.
   - Independent from Slint and PE parser.

2. **F4SE scan service with fake inspector**
   - Files: `src/services/f4se.rs`, `src/services/mod.rs`.
   - Tests: missing data, missing plugins with/without manager hint, empty folder, direct DLL filtering, `msdia` ignore, parser failure row, unreadable row, unknown game version.
   - Can use an in-test fake `Filesystem` and fake `F4seDllInspector` first; no PE dependency required for these tests.

3. **PE export inspector**
   - Files: `Cargo.toml`, `Cargo.lock`, `src/services/f4se.rs` or `src/services/f4se_pe.rs`.
   - Tests: at minimum, existing non-F4SE DLL bytes classify as non-F4SE; synthetic/fixture bytes for F4SE exports if available. Keep parser behind the inspector trait so service/controller tests are not blocked on complex PE fixture generation.
   - Highest-risk technical proof.

4. **Controller and worker payloads**
   - Files: `src/app/f4se_controller.rs`, `src/app/mod.rs`, `src/workers/events.rs`, `src/workers/mod.rs`.
   - Tests: lazy request only once, loading state, completion, stale ignored, failure/spawn failure, unrelated payload ignored.
   - Independent from Slint markup.

5. **Slint F4SE table UI and MainWindow contract**
   - Files: `ui/f4se_tab.slint`, `ui/main.slint`, `src/main.rs` tests.
   - Tests: source-contract tests for placeholder removal, headings/columns order, legend, no refresh button, property/callback forwarding, tab order preserved.

6. **Runtime wiring**
   - Files: `src/main.rs`.
   - Tests: projection helper maps pure rows to `F4seUiRow`; worker failure maps to safe error; activation callback schedules scan and projects loading; source-contract test locks lazy activation plumbing.

## First Proof

Build the PE export/version inspector and pure row mapping first, before UI work. This is the slice's biggest uncertainty and the only part that could force an architecture change.

Minimum first-proof acceptance:

- Given facts equivalent to `parse_dll`, row mapping exactly matches reference semantics for OG, NG, AE, non-F4SE, and ambiguous `F4SEPlugin_Version` cases.
- Given bytes for a normal non-F4SE DLL, the inspector returns `is_f4se = false` rather than a hard failure.
- Given an injected parser failure, the scan service still returns a visible warning/unknown row and continues other DLLs.
- The service never guesses from filename/version metadata.

If generating a full synthetic PE fixture is too large for T01, keep the parser behind `F4seDllInspector`, prove service/controller/UI with fake inspector, and make a separate parser task with focused fixtures. Do not block all S06 work on PE fixture complexity.

## Verification Plan

Focused tests to add/run during implementation:

- `cargo test f4se_domain` or named tests covering constants and row mapping.
- `cargo test f4se_scan` for service enumeration/errors/filtering.
- `cargo test f4se_controller` for reducer and worker payload transitions.
- `cargo test f4se_slint_contract` / source-contract tests in `src/main.rs` for UI labels/callbacks/property forwarding.
- Existing broad checks before completion:
  - `cargo fmt --check`
  - `cargo check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features`
- Read-only safety check: `git status --short CMT` should stay empty (or use the GSD-approved recorded/allowed check pattern during closeout).

## Risks and Watch-outs

- **Do not load arbitrary DLLs** in the Rust process. Python did this with `WinDLL`, but Rust should parse PE exports from bytes for safety and CI portability.
- **Bare discovery does not know the game version.** Use Overview binary classification or factor it out; otherwise `Your Game` will stay misleadingly unknown even on valid installs.
- **`F4SEPlugin_Version` is a data export.** Its `compatibleVersions` array must be read from the export RVA + 528 bytes as 16 little-endian u32 values.
- **Ambiguous NG/AE is warning, not false.** If `F4SEPlugin_Version` exists but the exact NG/AE support cannot be proven, reference UI uses `⚠` for NG/AE and `Your Game` when relevant.
- **Malformed DLLs are visible rows.** This is stricter than the Python reference and explicitly required by the slice.
- **No manual refresh.** Lazy first-open scan only; do not add a button to make wiring easier.
- **Slint TabWidget activation needs compile verification.** Cached Slint source confirms a `current` property, but the exact property-change syntax should be proven with `cargo check` early in the UI task.
- **Keep CMT read-only.** All copied constants/tests live in Rust files outside `CMT/`.

## Sources Consulted

- `CMT/src/tabs/_f4se.py` — F4SE loading, table layout, missing-folder messages, icon/tag row mapping.
- `CMT/src/utils.py` — `parse_dll` symbol/version-data rules and `F4SEPluginVersionData` field layout.
- `CMT/src/globals.py` — `ABOUT_F4SE_DLLS`, icon meanings, colors.
- `CMT/src/game_info.py` and `CMT/src/enums.py` — current game version helpers and install-type labels.
- `src/workers/mod.rs`, `src/workers/events.rs`, `src/workers/handoff.rs` — owned worker event and Slint event-loop handoff pattern.
- `src/app/overview_controller.rs`, `src/app/tools_controller.rs`, `src/app/about_controller.rs` — reducer/state transition patterns.
- `src/services/discovery.rs`, `src/services/overview_collector.rs` — optional data/F4SE paths and current game binary classification.
- `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`, `ui/f4se_tab.slint` — current Slint shell and UI style.
- `cargo info pelite` plus cached `pelite-0.10.0` source — PE export APIs and error variants.
- Cached Slint 1.16.1 widget source — `TabWidget`/`TabBarBase` expose an in-out `current` property usable for active-tab tracking.
