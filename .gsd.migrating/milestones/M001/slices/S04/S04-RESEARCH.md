# S04 Research: Overview Diagnostics & Updates

## Summary

Depth: deep targeted research. The Rust/S03 foundation is solid, but S04 is the first visible consumer of discovery, platform, worker, settings, and reference parsing behavior, and the Python Overview tab has a large amount of implicit domain logic.

Key finding: the current Rust app only has an inert `OverviewTab`, settings callbacks, pure discovery contracts, fakeable platform seams, `DiscoveryService`, and generic worker events. It does **not** yet have binary/archive/module diagnostics, update checking, scanner-ready Overview problems, Slint overview models, or controller/worker wiring. S04 should therefore start with a pure typed `OverviewSnapshot`/problem-feed service and only then project that state into Slint.

The highest-risk reference fidelity areas are binary version/hash classification, enabled BA2/plugin detection from INIs/`Fallout4.ccc`/`plugins.txt`, silent update-check behavior, and partial discovery handling. Keep all OS/file/network/desktop access behind injected adapters and worker tasks; tests must use fakes.

## Active Requirements Owned or Supported

- **Typed Overview state without direct OS queries in tests**: S04 directly owns this. Existing `DiscoveryReport`, `Fallout4Installation`, `DiscoveredModManager`, and `SystemMetadata` are typed and fakeable, but S04 must add typed Overview view/domain state rather than pushing raw strings through Slint.
- **Valid game path vs missing optional Data/F4SE folders**: S03 intentionally allows a valid game path with missing `Data` or `Data/F4SE/Plugins`. S04 must render missing `Data`, missing `Fallout4.ccc`, missing `plugins.txt`, and optional F4SE path gaps as inline panel/problem states, not as fatal game-discovery failures.
- **Typed platform/worker boundaries**: open-folder/open-link and refresh/update work must use `DesktopActions`, `DiscoveryService`, future update traits, and `WorkerRuntime`/`WorkerEventSink` handoff. Slint callbacks should enqueue typed commands only.

## Skills Discovered

- Existing installed skills relevant from the prompt: `rust-async-patterns` can inform Tokio/worker handoff; `observability` can inform logging/diagnostics. I did not invoke them because this unit is research and the local S03 patterns are already explicit.
- Ran `npx skills find "Slint"`: results were unrelated lint/accessibility skills.
- Ran `npx skills find "Rust Slint"`: relevant-looking results included `bobmatnyc/claude-mpm-skills@rust-desktop-applications` and `bahayonghang/my-claude-code-settings@lib-slint-expert`.
- Attempted to install `bahayonghang/my-claude-code-settings@lib-slint-expert -g -y` and `bobmatnyc/claude-mpm-skills@rust-desktop-applications -g -y`; both failed with “No matching skills found”. No new skills were installed.

## Implementation Landscape

### Existing Rust files and purpose

- `ui/overview_tab.slint`: placeholder only. It exports `OverviewTab` with heading `Overview` and text `Overview behavior is reserved for a later port phase.`
- `ui/main.slint`: root `MainWindow` only exposes settings properties/callbacks (`update-source`, `log-level`). The Overview tab is instantiated as `OverviewTab {}` with no properties/callbacks. The update banner surface does not exist yet.
- `src/main.rs`: creates `MainWindow`, loads `SettingsController`, binds settings callbacks, and runs Slint. Tests currently assert Overview/F4SE/Scanner/Tools/About are inert placeholders. S04 must remove Overview from `INERT_TAB_COMPONENTS` and add new source-contract tests.
- `src/app/settings_controller.rs`: owns last persisted `AppSettings` and immediate-save/revert behavior. It exposes visible UI values but does not expose a full current settings snapshot. S04 likely needs a read method for `AppSettings` or at least current `UpdateSource`.
- `src/domain/settings.rs`: `UpdateSource::{Both,Github,Nexus,None}` wire values are already present and defaults are reference-compatible. Scanner `overview_issues` toggle already exists for later Scanner consumption.
- `src/domain/discovery.rs`: pure types exist for `Fallout4Installation`, `Fallout4InstallType`, `ArchiveRecord`, `ArchiveFormat`, `ArchiveVersion`, `ModuleRecord`, `ModuleKind`, `ModuleHeaderVersion`, `Fallout4IniFiles`, and discovery errors. These are not yet populated with real archive/module/INI data.
- `src/domain/mod_manager.rs`: typed MO2/Vortex context exists. MO2 carries selected profile, directories, portable/source paths, skip rules, and executables. Vortex is intentionally identity-only.
- `src/services/discovery.rs`: `DiscoveryService::discover` returns `DiscoveryReport { game, mod_manager, system_metadata, attempts, manager_steps }`. It discovers game path, optional `Data`, optional `Data/F4SE/Plugins`, manager, and system metadata. It does not scan base binaries, archives, modules, INIs, `Fallout4.ccc`, or `plugins.txt`.
- `src/platform/filesystem.rs`: `Filesystem` trait has metadata/read/read_dir/walk_dir. Real adapter exists. Test fake is private inside tests; S04 can either create local service fakes or promote reusable fake helpers if useful.
- `src/platform/process.rs`: `ProcessInspector` exposes process list, file version, and `SystemMetadata`. Version metadata stores a 3-part semantic version plus optional raw string; binary classification must use the raw four-part version when available. `SystemMetadata` has OS, version, architecture, CPU brand, RAM, logical CPUs, but no GPU/VRAM.
- `src/platform/desktop.rs`: `DesktopActions` and `DesktopActionResult` exist for open URL/path/tool actions with safe messages. Fake desktop adapter is private to tests.
- `src/workers/*`: generic worker facade exists. `WorkerPayload` has text-oriented variants (`Discovery(WorkerMessage)`, etc.) and `ExternalAction`, but no typed Overview result payload yet. `WorkerRuntime::spawn_blocking_task` requires an active Tokio runtime handle.
- `Cargo.toml`: dependencies include Slint/Tokio/walkdir/tracing/sysinfo/windows, but not `reqwest`, `crc32fast`, or `byteorder`.

### Reference Overview UI facts

Primary source: `CMT/src/tabs/_overview.py`.

Top block labels/order:

1. `Mod Manager:`
2. `Game Path:`
3. `Version:`
4. `PC Specs:`
5. Refresh button with tooltip `Refresh`

Manager row behavior:

- Displays `{manager.name} v{manager.version} [Profile: {manager.selected_profile or 'Unknown'}]` or `Not Found`.
- No manager is red/bad and tooltip text is `Your mod manager must launch the app to be detected.`
- MO2 gets an info icon opening detected settings. MO2 <= 2.5.2 on Windows 11 24H2 gets a warning tooltip about VFS issues.
- Vortex gets warning tooltip: it is not fully supported; Overview should be accurate but Scanner only looks in Data/staging unsupported.

Panel labels/buttons:

- `Binaries (EXE/DLL/BIN)` with base binary rows plus `Address Library:` and button `Downgrade Manager...`.
- `Archives (BA2)` with `General:`, `Texture:`, `Total:`, `Unreadable:`, separator, `v1 (OG):`, `v7/8 (NG):`, button `Archive Patcher...`.
- `Modules (ESM/ESL/ESP)` with `Full:`, `Light:`, `Total:`, `Unreadable:`, separator, `HEDR v1.00:`, `HEDR v0.95:`, `HEDR v????:`.
- Reference packs the three panels side-by-side. Keep this layout conservative in Slint.

Reference limits/colors:

- `MAX_ARCHIVES_GNRL = 256`, `MAX_ARCHIVES_DX10 = 255`.
- `MAX_MODULES_FULL = 254`, `MAX_MODULES_LIGHT = 4096`.
- Count colors: good below 95% of limit, warning at 95% through limit, bad over limit. Over-limit non-total counts add `Limit Exceeded` Overview problems.

### Reference binary behavior

Base file map from `CMT/src/globals.py::BASE_FILES`:

- `Fallout4.exe`
- `Fallout4Launcher.exe`
- `steam_api64.dll`
- `f4se_loader.exe`
- `f4se_steam_loader.dll`
- `CreationKit.exe`
- `Tools\\Archive2\\Archive2.exe` (display key becomes `Archive2.exe` in the Python `file_info` map)

Classification in `_overview.py`:

- `get_file_info` reads file version via `get_file_version`; if present it looks up the four-part version string first, then CRC32. If version is absent it uses CRC32 as the displayed version string.
- `InstallType.NGAE` is narrowed to the current game install type when the game is NG or AE.
- `Fallout4.exe` determines `game.install_type`.
- Unknown `Fallout4.exe` adds `Unknown Game Version` with summary containing possible causes and solution `Either update the game/verify files in Steam, or report this issue.`
- Missing `CreationKit.exe` and `Archive2.exe` are neutral rather than bad. Missing `f4se_steam_loader.dll` is neutral on NG/AE installs.
- Other missing binaries add `ProblemType.FileNotFound`, summary `This file is missing from your game installation.`
- Mismatched binary versions add `ProblemType.WrongVersion`, summary `The version of this binary does not match your installed game version.`
- Address Library check derives `Data/F4SE/Plugins/version-{Fallout4.exe version with dots replaced by dashes}.bin`; missing adds File Not Found with Nexus link `https://www.nexusmods.com/fallout4/mods/47327` and summary beginning `Address Library is a requirement...`.
- For OG installs, `Fallout4 - Startup.ba2` CRC32 skipping the 12-byte BA2 header differentiates Down-Grade when CRC is `A5808F5F`; missing Startup BA2 adds File Not Found with Verify Files solution.

### Reference archive behavior

- If `Data` is missing, archive scan returns early; the Data problem is added by module scan.
- Enabled archives come from INI `archive` section lists: `sresourceindexfilelist`, `sresourcestartuparchivelist`, `sresourcearchivelist`, `sresourcearchivelist2`.
- It adds plugin-associated archives for each enabled module and suffixes from language (`main`, `textures`, `voices_en`, plus `voices_{language}` for non-English).
- If `bNVFlexEnable=1`, `Fallout4 - Nvflex.ba2` is hardcoded as enabled; missing creates a File Not Found problem.
- If install type is AE, `Fallout4 - TexturesPatch.ba2` is hardcoded as enabled; missing creates a File Not Found problem.
- For each enabled BA2: read 12 bytes; invalid read -> `Invalid Archive`, summary `Failed to read archive due to permissions or the file is missing.`
- Invalid magic or short read -> `Archive is either corrupt or not in Bethesda Archive 2 format.`
- Version byte accepted values: 1 = OG, 7/8 = NG; anything else -> `Archive version ({n}) is not valid for Fallout 4.`
- Format bytes accepted values: `GNRL` counts General, `DX10` counts Texture; anything else -> `Archive format ({text}) is not valid for Fallout 4.`
- Tracks `archives_unreadable`, `archives_og`, `archives_ng`, `ba2_count_gnrl`, `ba2_count_dx10`.

Note: the Python code checks `head[4]` rather than decoding a full little-endian u32 version. A Rust parser may use a u32 for typed clarity, but tests should explicitly decide whether non-zero bytes 5..7 are tolerated or rejected.

### Reference module behavior

- If `Data` is missing, adds SimpleProblemInfo path `Data`, problem `File Not Found`, summary `The Data folder was not found in your game install path.`, solution `VerifyFiles`.
- Starts enabled modules from base `GAME_MASTERS` that exist.
- Reads `Fallout4.ccc` from game path and adds listed CC modules that exist. Missing adds File Not Found problem with summary `The CC list file was not found in your game install path.\nThis is used to detect which CC modules/archives may be enabled.` In S04 scope, do **not** show the reference modal warning; keep inline.
- Reads `%LOCALAPPDATA%/Fallout4/plugins.txt`. Missing/unreadable adds File Not Found problem with summary `plugins.txt was not found.\nThis is used to detect which modules/archives are enabled.` Solution is `N/A` if a mod manager is detected, otherwise `Launch this app with your mod manager.` Missing also falls back to all `.esp/.esl/.esm` files in Data. In S04 scope, do not show modal warning.
- For each enabled module: read 34 bytes.
  - Read failure -> `Invalid Module`, summary `Failed to read module due to permissions or the file is missing.`
  - Short/non-`TES4` -> `Module is either corrupt or not in TES4 format.`
  - Missing `HEDR` at bytes 24..28 marks unreadable but does not add a problem in the reference.
  - HEDR bytes 30..34 equal `0.95` or `1.00` count known versions; other float values add `Invalid Module` with summary `Module version ({hedr}) is not valid for Fallout 4.{valid_games_str}` and a long Creation Kit/xEdit solution.
  - Flags at bytes 8..12 with bit `0x0200`, or `.esl` extension, count as Light; otherwise Full.

### Reference update behavior

Primary sources: `CMT/src/cm_checker.py::check_for_updates` and `CMT/src/utils.py::{check_for_update_nexus,check_for_update_github}`.

- `update_source == "none"`: return immediately; no network calls and no UI state.
- `"nexus"`: check only Nexus.
- `"github"`: check only GitHub.
- `"both"`: check both.
- No update or failed network/parse checks: no visible status/banner. Only logs are emitted.
- If either source returns a newer version than `APP_VERSION = 0.6.1`, create a pale-green top banner above tabs with text `An update is available:` and link labels `v{version} (NexusMods)` and/or `v{version} (GitHub)`. Link clicks open `NEXUS_LINK` or `GITHUB_LINK`.
- Nexus check scrapes page meta label; GitHub check reads latest release JSON. For Rust, add a fakeable update client so tests do not hit the network.

## Recommendation

1. **Add pure Overview domain/view state first** in `src/domain/overview.rs`.
   - Suggested types: `OverviewSnapshot`, `OverviewTopStatus`, `BinaryStatusRow`, `ArchiveSummary`, `ModuleSummary`, `UpdateBanner`, `OverviewProblem`, `ProblemKind`, `SolutionKind`, `StatusSeverity`.
   - Keep reference labels/constants here or in an adjacent constants module: app version `0.6.1`, Nexus/GitHub URLs, binary map, limits, module/header constants, language archive suffixes, problem/solution labels.
   - Problem model should preserve fields needed by Scanner: problem type label, path, relative path, mod/source marker (`OVERVIEW` where reference uses it), summary, solution text/enum, `extra_data`, and optional detail/file-list metadata.

2. **Add a pure diagnostics service** in `src/services/overview.rs` (or split parsers into `src/domain/overview_{binary,archive,module}.rs` if it grows large).
   - Inputs should be injected facts/adapters: `DiscoveryReport` or a `DiscoveryService`, `Filesystem`, `ProcessInspector`, current `AppSettings`/`UpdateSource`, appdata/documents paths for `plugins.txt` and INIs, and an update-check trait.
   - Output is an owned `OverviewSnapshot`. Do not use Slint types in the service.
   - Binary/version/hash parsing should use `ProcessInspector::file_version` raw string plus CRC32; add `crc32fast` rather than hand-rolling CRC.
   - Archive and module parsing should use byte slices from `Filesystem::read_bytes`; add small explicit parsers and tests rather than adopting a full BA2/plugin parser now.

3. **Add a fakeable update checker** (`src/services/update.rs` or inside overview service).
   - Trait shape: selected-source orchestration should be testable independently from HTTP.
   - Real implementation can use `reqwest` with 5s timeouts inside a worker/background task. Keep failures as diagnostics/logs and return no banner.
   - Do not let UI or tests directly call the network.

4. **Wire Slint as a projection layer only**.
   - Expand `ui/main.slint` with update-banner properties/callbacks (`open-nexus-update`, `open-github-update`) above the `TabWidget`.
   - Replace `ui/overview_tab.slint` placeholder with fixed reference panel labels and Slint properties/model(s) for status values. For binary rows, either use a generated row model or fixed rows based on the seven base file entries; a model is more extensible, fixed rows are simpler and closer to reference ordering.
   - Add callbacks: `refresh-requested()`, `open-game-path-requested()`, and possibly deferred utility callbacks for disabled `Downgrade Manager...` / `Archive Patcher...` controls. S04 should make destructive utility entry points disabled or explanatory.

5. **Add an Overview controller** in `src/app/overview_controller.rs` and refactor `main.rs` only enough to bind it.
   - The controller should own/receive `MainWindow::as_weak()`, settings snapshot access, platform/service adapters, `WorkerRuntime`, and a `SlintEventLoopSink` or equivalent closure.
   - Initial refresh should run after startup through the same path as the Refresh button.
   - Open actions should call `DesktopActions` and update a small visible status/error string on failure. Avoid `os.startfile`/`webbrowser` equivalents in UI code.

6. **Ensure an active Tokio runtime exists for workers**.
   - Current `main` does not create/enter a Tokio runtime. `WorkerRuntime::spawn_blocking_task` will return `NoActiveRuntime` from Slint callbacks unless `main` creates a runtime and enters it around `app.run()` or otherwise runs inside `#[tokio::main]`.
   - Prefer explicit runtime creation/enter guard around Slint run for minimal churn: create `tokio::runtime::Runtime`, enter it, construct/bind app, then `app.run()` while guard/runtime live.

7. **Extend worker payloads deliberately**.
   - Current `WorkerPayload` cannot carry an `OverviewSnapshot`. Add `WorkerPayload::Overview(OverviewSnapshot)` if using `WorkerEventSink` for refresh completion, or use a separate typed channel that is still handed back with `slint::invoke_from_event_loop`. Reusing worker events is more consistent with S03.

## Natural Seams for Planner

1. **Reference constants + Overview domain model**
   - Files: `src/domain/overview.rs`, `src/domain/mod.rs`.
   - Proof: unit tests lock problem/solution labels, limits, status severity threshold rules, and base-file display order.
   - Independent of Slint and network.

2. **Binary diagnostics**
   - Files: `src/services/overview.rs` or `src/domain/overview_binary.rs`; `Cargo.toml` for `crc32fast`.
   - Proof: fake filesystem/process tests for missing optional binaries, wrong version, unknown Fallout4.exe, Address Library path, Startup BA2 downgrade CRC, and use of raw four-part version metadata.
   - Depends on domain model.

3. **Module/archive diagnostics**
   - Files: same service or parser modules.
   - Proof: byte-fixture tests for BA2 GNRL/DX10/v1/v7/v8, invalid archive magic/version/format, TES4/HEDR v0.95/v1.00/unknown, light flag/`.esl`, missing Data, missing `Fallout4.ccc`, missing `plugins.txt`, and over-limit problems.
   - Depends on domain model and filesystem trait; independent of Slint.

4. **Update-check service**
   - Files: `src/services/update.rs`, `src/services/mod.rs`, `Cargo.toml` if adding `reqwest`.
   - Proof: fake client tests for `none` skips, `nexus`/`github` source selection, `both` calls both, failed/no-newer checks return no banner, newer version returns banner link data.
   - Independent from archive/module scanning except shared `UpdateSource` and `OverviewSnapshot`.

5. **Slint contract and mapping**
   - Files: `ui/overview_tab.slint`, `ui/main.slint`, `src/app/overview_view_model.rs` or `src/app/overview_controller.rs`, `src/main.rs` tests.
   - Proof: source tests for labels/order/callbacks/properties; mapping tests from `OverviewSnapshot` to Slint strings/severity colors; update `INERT_TAB_COMPONENTS` to exclude Overview.
   - Depends on domain model but not real OS/network.

6. **Controller + worker + desktop actions**
   - Files: `src/app/overview_controller.rs`, `src/main.rs`, `src/workers/events.rs` if adding `Overview` payload.
   - Proof: fake/recording sink tests for refresh lifecycle, UI-safe handoff, refresh failure visible status, open game path success/failure, update link open success/failure, and no UI-thread OS calls.
   - Depends on service outputs and Slint root API.

## First Proof / Suggested Tracer Bullet

Build the pure `OverviewDiagnosticsService` around fakes before touching Slint heavily:

- Fake a valid `DiscoveryReport` with game path `C:/Games/Fallout 4`, optional missing `Data`, fake manager `None`, and fake system metadata.
- Run `overview_service.refresh(...)` with `UpdateSource::None`.
- Assert the snapshot still displays the game path and PC specs, marks archives/modules unavailable/zero, contains a scanner-ready `File Not Found` problem for `Data` with the exact reference summary/Verify Files solution, and records that no update check was attempted.

This proves the key S03-to-S04 contract: valid installation discovery and missing optional paths stay distinct, results are typed, and no network/OS/Slint access is needed in tests. Then add binary classification and parser fixtures incrementally.

## Verification Plan

Required final gates:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `git status --short CMT` must remain empty because `CMT/` is read-only.

Targeted tests to add before closeout:

- Overview UI source contract includes top labels, panel titles, row labels, Refresh callback, open-game-path callback, update banner/link callbacks, and deferred `Downgrade Manager...` / `Archive Patcher...` presentation.
- Existing shell inert-placeholder test is updated so only F4SE/Scanner/Tools/About remain inert.
- Domain tests lock `ProblemKind`/`SolutionKind` display strings from `CMT/src/enums.py` and `helpers.py` problem-field semantics.
- Diagnostics tests cover: no manager problem, manager display/profile/Vortex warning/MO2 Windows 11 24H2 warning, missing Data, missing `Fallout4.ccc`, missing `plugins.txt`, base binary missing/wrong/unknown, Address Library missing, Startup BA2 downgrade detection, BA2 invalid/unreadable/counts/limits, module invalid/unreadable/counts/limits.
- Update tests cover all four `UpdateSource` values, newer vs same/older versions, HTTP/parse failure silence, and link target selection.
- Worker/controller tests cover initial refresh, manual refresh, stale result avoidance if possible, failure status, and open action failure via fake `DesktopActions`.

## Risks and Watch-outs

- **Worker runtime is not active today**: `main.rs` must create/enter Tokio before callbacks can call `WorkerRuntime::spawn_blocking_task` successfully.
- **`WorkerPayload` is currently string-oriented**: add a typed Overview payload or use a clearly documented typed handoff channel. Do not serialize snapshots to strings just to fit the current enum.
- **SystemMetadata lacks GPU/VRAM**: reference PC Specs shows OS/RAM and CPU/GPU+VRAM. Either extend the adapter with GPU fields behind fakes or display an intentional `Unknown GPU`/reduced spec string and document the temporary divergence. Do not query GPU directly from UI/tests.
- **INI/plugins paths need injection**: `DiscoveryRequest` has `local_appdata` for MO2, but no Documents path for `Fallout4.ini` files. Add `OverviewDiagnosticsRequest` fields for documents/appdata paths or a new environment/known-folder adapter.
- **Binary version lookup needs four-part raw strings**: `SemanticVersion` alone cannot match reference `BASE_FILES` keys like `1.10.163.0`. Use `VersionMetadata.raw` when present; fall back to CRC32.
- **Reference uses modal warnings for missing `Fallout4.ccc`/`plugins.txt` on first load**: S04 scope explicitly re-scopes these to inline problem states. Tests should assert no modal/dialog callback is required.
- **Reference BA2 version check uses `head[4]`**: choose and test Rust behavior intentionally. A full u32 parser is cleaner but may be stricter than Python.
- **Slint models are not worker-thread state**: keep `OverviewSnapshot` plain Rust and construct/set Slint models only on the event loop.
- **Private test fakes**: existing filesystem/process/desktop fakes live inside test modules. S04 service tests can define local fakes or promote reusable test utilities, but avoid production exposure unless needed.
- **CMT source remains read-only**: all reference maps/constants must be copied into Rust source/tests outside `CMT/`; do not generate files inside the submodule.

## Sources Consulted

- Memory store: S03 architecture/pattern notes for pure discovery contracts, platform seams, and worker handoff.
- Current Rust files: `ui/overview_tab.slint`, `ui/main.slint`, `ui/settings_tab.slint`, `src/main.rs`, `src/app/mod.rs`, `src/app/settings_controller.rs`, `src/domain/settings.rs`, `src/domain/discovery.rs`, `src/domain/mod_manager.rs`, `src/services/discovery.rs`, `src/platform/{filesystem,process,desktop,mod}.rs`, `src/workers/{events,handoff,mod}.rs`, `Cargo.toml`.
- Reference files: `CMT/src/tabs/_overview.py`, `CMT/src/cm_checker.py`, `CMT/src/globals.py`, `CMT/src/enums.py`, `CMT/src/helpers.py`, `CMT/src/utils.py`, `CMT/src/game_info.py`, `CMT/src/mod_manager_info.py`.
- Research exec artifacts: `.gsd/exec/d5b7f407-9cf5-4f58-ab47-75af6327de4c.stdout`, `.gsd/exec/00386ef9-a516-469a-ae54-616876f3def6.stdout`, `.gsd/exec/6801c9b8-c216-4a60-b88d-7e9e5ec81334.stdout`, `.gsd/exec/41b72ed6-aec5-4504-9039-e6130582c954.stdout`, `.gsd/exec/7da985c2-ab7a-403f-9ef5-e04ab0c58bbc.stdout`.
