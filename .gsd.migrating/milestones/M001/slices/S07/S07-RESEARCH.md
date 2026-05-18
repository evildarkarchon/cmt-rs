# S07 Research: Scanner Read Only Results

## Depth

Deep targeted research. The Rust project already has established Slint, settings, discovery, Overview, F4SE, worker, desktop, and clipboard patterns, but this slice is the first full scanner workflow and has non-trivial reference traversal, MO2 attribution, progress, grouping, detail, and action behavior.

## Active Requirements

No root `REQUIREMENTS.md` entries were preloaded for this run. S07 supports the milestone constraints around faithful UI/behavior parity, Slint-free domain logic, fakeable adapters, non-blocking workers, and CMT read-only reference usage.

## Skill Activation and Discovery

- Skill cues applied from the prompt: `decompose-into-slices` means planner should split S07 into thin vertical tasks with explicit dependency order; `write-docs` means this research is written for a fresh planner/executor rather than relying on session context. The `Skill` tool itself was not exposed in the available tool namespace during this run, so no external skill file was loaded through that tool.
- Existing installed relevant skills in prompt: `rust-async-patterns`, `tdd`, `observability`, `review`, and `verify-before-complete` are relevant for later execution/review.
- Skill discovery for core missing tech (`Slint Rust GUI`) found:
  - `bobmatnyc/claude-mpm-skills@rust-desktop-applications` — 259 installs, broad Rust desktop relevance.
  - `bahayonghang/my-claude-code-settings@lib-slint-expert` — 31 installs, directly Slint-specific.
  - `terraphim/terraphim-skills@gpui-components` — not relevant.
- Installed the directly relevant Slint skill with `npx skills add bahayonghang/my-claude-code-settings@lib-slint-expert -g -y` (`gsd_exec` id `3a044b4d-c41b-489a-ba57-67e59f10ed7b`). It may only appear in future sessions/prompts.

## Summary

S07 should add a new Scanner domain/service/controller/worker/UI vertical slice rather than extending placeholder UI directly. The right shape is the S04/S06 pattern: pure scanner data in `src/domain/scanner.rs`, adapter-backed scan engine in `src/services/scanner.rs`, Slint-free reducer in `src/app/scanner_controller.rs`, owned scanner worker payloads in `src/workers/events.rs`, and `ui/scanner_tab.slint` as a render/callback surface only.

The highest-risk behavior is not Slint layout; it is faithfully reproducing reference scanner classification while improving partial-failure visibility without blocking UI or mutating files. The first proof should be fake-backed scanner service tests that build a small Data/MO2 fixture and produce grouped read-only rows/details in deterministic order.

## Recommendation

1. **Build scanner in three layers before runtime wiring:**
   - Domain contract: settings snapshot, problem type labels, solution text, result records, details/action descriptors, grouping/ordering, copy-details rendering.
   - Service: fakeable scanner engine over `Filesystem`, discovery/Overview facts, MO2 context, desktop/clipboard action adapters.
   - Controller: toggles, save-on-scan-start state transition, progress, grouped flat UI rows, selected details, action feedback, stale scan id handling.
2. **Run the full scan as a single background worker that refreshes Overview first.** The worker should emit progress `Refreshing Overview...`, build discovery + overview collected facts + `OverviewSnapshot`, then emit `Building mod file index...` if a Data scan is needed, then emit `Scanning... n/N: folder` progress during Data traversal. Return an owned scanner completion payload that includes the scanner snapshot and, if practical, the fresh `OverviewSnapshot` so the app can also update the Overview controller after a scanner-triggered refresh.
3. **Do not move Slint handles/models or settings controllers into workers.** Capture `AppSettings` and scanner toggles on the UI thread, persist scanner toggles before worker scheduling, then pass owned settings/discovery inputs into the worker.
4. **Use explicit read-only action buttons in the embedded details pane.** Tk binds path and URL labels to clicks/right-clicks; Slint should expose safe buttons/callbacks such as `Copy Details`, `File List`, `Open Path`, `Open URL`, and `Copy URL`, hidden/disabled when unavailable. Keep `Auto-Fix`, `Fixed!`, and `Fix Failed` absent in S07.
5. **Implement deterministic grouping rather than Python set-order instability.** Use a reference enum/order for groups, then sort rows by `(group_order, mod_name, display_path)` or documented equivalent. This supports tests/screenshots and is explicitly allowed by S07 context.
6. **Use recursive `read_dir` with pruning instead of `Filesystem::walk_dir` for Scanner.** The current `RealFilesystem::walk_dir` returns a single error for the whole traversal. Scanner needs top-down pruning, root-folder progress, and partial-failure continuation; implement a scanner-specific traversal over `Filesystem::read_dir` so unreadable child folders can produce visible warning/error rows while siblings continue.

## Reference Findings

### UI and flow source of truth

Primary reference files:

- `CMT/src/tabs/_scanner.py` — Scanner tab layout, progress flow, tree grouping, details pane, copy/open/file-list behavior, scan traversal.
- `CMT/src/scan_settings.py` — setting labels/order/tooltips/default keys, whitelist/junk/proper-format constants, save-on-scan-start timing.
- `CMT/src/enums.py` — `ProblemType` and `SolutionType` labels/text.
- `CMT/src/globals.py` — scanner tooltips, race-subgraph info/threshold, archive name whitelist, F4SE script names.
- `CMT/src/helpers.py` — `ProblemInfo`/`SimpleProblemInfo` field behavior, including `<Unmanaged>` default for path problems except `File Not Found`.

Reference labels/strings that should be source-contract tested:

- Scanner controls: `Collapse All`, `Expand All`, `Scan Settings`, `Scan Game`, `Scanning...`.
- Scanner setting labels in order: `Overview Issues`, `Errors`, `Wrong File Formats`, `Loose Previs`, `Junk Files`, `Problem Overrides`, `Race Subgraphs`.
- Progress/status strings: `Refreshing Overview...`, `Building mod file index...`, `Scanning... n/N: folder`, `N Results ~ Select an item for details`, including `0 Results ~ Select an item for details`.
- Details labels/buttons: `Mod:`, `Problem:`, `Summary:`, `Solution:`, `Copy Details`, optional `File List`.
- Race file-list title/text: `Race Animation Subgraph Records`; informational text comes from `INFO_SCAN_RACE_SUBGRAPHS`.

### Scan settings contract

`ScanSettings` reference behavior:

- All scanner settings default true in `settings.json` and in the Python side pane.
- The Python `SidePane` checkboxes initialize `BooleanVar(value=True)` and only save to settings when `ScanSettings(self.side_pane)` is constructed at `Scan Game` time. S07 intentionally wants persisted/default state from S02 visible on load but still save changes only when `Scan Game` starts.
- `skip_data_scan` stays true unless any enabled setting outside `{OverviewIssues, RaceSubgraphs}` is true.
- MO2 skip suffixes/dirs are combined with reference scanner skips:
  - Always ignored folders: `bodyslide`, `fo4edit`, `robco_patcher`, `source`.
  - Always skipped file suffix: `.vortex_backup`.
  - MO2 adds parsed `skip_file_suffixes` and `skip_directories`.
- **Gotcha:** `ScanSetting.Errors` is exposed and persisted but `_scanner.py` never branches on it. It only makes `skip_data_scan` false when enabled. S07 scope requires visible rows for partial failures; if those are gated by `Errors`, document that as filling an incomplete reference intent rather than exact Python behavior.

### Scanner classification rules from `_scanner.py`

Data traversal rules:

- Top-level Data folders not in `DATA_WHITELIST` are pruned and not scanned, except `fomod` can produce a junk folder row before pruning when `Junk Files` is enabled.
- Top-level `vis` produces `Loose Previs` with solution `ArchiveFolder` and is pruned when `Loose Previs` is enabled.
- Under top-level `meshes`:
  - folder `precombined` produces `Loose Previs` and is pruned.
  - folder `animtextdata` produces `Loose AnimTextData` and is pruned when `Problem Overrides` is enabled.
- Junk files: exact `thumbs.db`, `desktop.ini`, `.ds_store`; suffix `.tmp`, `.bak`.
- F4SE script override: only direct children of `Data/Scripts`; only when a source mod name is known from MO2 index; filename is in `F4SE_CRC`; the reference does **not** compare CRC in scanner, it only checks the filename.
- Wrong format:
  - If top folder has a whitelist and file extension is not in it, emit `Unexpected Format`.
  - Any `.dll` outside `F4SE/Plugins` is `Unexpected Format` even under top folders with no whitelist.
  - If extension is in `PROPER_FORMATS`, check sibling files with expected extension(s):
    - found: summary says expected format was found and solution `DeleteOrIgnoreFile`.
    - not found: summary says expected format was NOT found and solution `ConvertDeleteOrIgnoreFile`.
  - Unknown extension solution: `If this file type is expected here, please report it.`
- Invalid BA2 archive name:
  - Applies to `.ba2` files not in `ARCHIVE_NAME_WHITELIST` and not in enabled archives.
  - Name must be `<plugin stem> - <suffix>.ba2`; suffix must be in game BA2 suffix list (`main`, `textures`, `voices_en`, and `voices_<language>` when language is not English).
  - Extra data lines: `\nValid Suffixes: ...` and `Example: <stem> - Main.ba2`.
- Race subgraphs:
  - If `Race Subgraphs` enabled, count bytes `b"\x00SADD"` in enabled module files.
  - If total count > `RACE_SUBGRAPH_THRESHOLD` (`100`), emit simple result `Race Subgraph Record Count` with path/display `"{total} SADD Records from {module_count} modules"`, `INFO_SCAN_RACE_SUBGRAPHS` summary, and file list of `(count, module_path)`.

Important constants:

- `DATA_WHITELIST`: `f4se` = unrestricted; `materials` = `bgem,bgsm,txt`; `meshes` = `bto,btr,hko,hkx,hkx_back,hkx_backup,lst,max,nif,obj,sclp,ssf,tri,txt,xml`; `music` = `wav,xwm`; `textures` = `dds`; `scripts` = `pex,psc,txt,zip`; `sound` = `cdf,fuz,lip,wav,xwm`; `vis` = `uvd`.
- `PROPER_FORMATS`: texture image extensions `bmp,jpeg,jpg,png,psd,tga -> dds`; sound `mp3 -> wav,xwm`.
- `ARCHIVE_NAME_WHITELIST` has 39 lowercase names in `CMT/src/globals.py`; copy exactly rather than reconstructing.
- `F4SE_CRC` has 29 script names; for S07 only the keys/names are needed.

### MO2 and Vortex behavior

- Reference MO2 stage paths come from reversed `modlist.txt` enabled lines (`+ModName`) under `profiles/<selected_profile>/modlist.txt`, then appends overwrite when present.
- `ModFiles` maps relative folder/file paths to `(mod_name, source_path)`, plus root-level modules/archives.
- Scanner display includes a mod column only when `using_stage` is true (`manager && stage_path` in Python; use `DiscoveredModManager::ModOrganizer` with valid stage context in Rust).
- Overview problems with mod `OVERVIEW` are remapped to source mod by `relative_path` when MO2 index exists; otherwise they become blank/unknown in the reference.
- Vortex is identity-only in the Rust project and reference support is partial. S07 should scan Data only, show no staged mod column, and not fabricate Vortex source-mod attribution.

### Details/action behavior

- `Copy Details` text in reference is exactly:
  - with mod: `Mod: {mod}\nProblem: {relative_path}\nSummary: {summary}\nSolution: {solution}\n`
  - without mod: same minus the `Mod:` line.
- Path open target is the file itself if a directory, otherwise parent folder when file exists or parent exists.
- URL actions are driven from the first `extra_data` string when it starts with `http`; reference left-click opens URL and right-click copies URL. Slint can use explicit buttons.
- `File List` appears for simple results with `file_list`; race list uses columns `Records` and ` Module` in the reference modal.
- S07 should hide Auto-Fix entirely, even if a solution text later maps to reference `AUTO_FIXES`.

## Existing Rust Implementation Landscape

### Reusable patterns and files

- `src/domain/settings.rs` already contains `ScannerSettings` with all seven booleans, default true, and reference-compatible mixed-case keys.
- `src/platform/settings_store.rs` saves/loads full `AppSettings` through `settings.json` and can be reused by `SettingsController`.
- `src/app/settings_controller.rs` owns the last persisted `AppSettings` and save/revert logic for immediate Settings-tab radios. S07 should extend it or add a small method to persist a full scanner settings snapshot at scan-start time; avoid bypassing it with a second unsynchronized store.
- `src/domain/overview.rs` already defines scanner-ready `OverviewProblem` records with path, relative path, mod name, summary, solution, links, details, and severity.
- `src/services/overview.rs` is the pure Overview diagnostics builder. `src/services/overview_collector.rs` performs filesystem collection and contains private BA2 suffix logic that Scanner also needs; consider moving shared suffix derivation to a reusable helper rather than duplicating silently.
- `src/services/discovery.rs` returns `DiscoveryReport` with `game`, `mod_manager`, system metadata, attempts, and manager steps.
- `src/domain/mod_manager.rs` has `ModOrganizerContext`, directories, skip rules, selected profile, and Vortex identity-only context.
- `src/platform/filesystem.rs` has fakeable `Filesystem`, `read_dir`, `read_bytes`, `read_to_string`, `is_file`, `is_dir`. Its test fake is private; S07 service tests can add a local fake or make a reusable test helper only if justified.
- `src/platform/desktop.rs` and `src/platform/clipboard.rs` provide fakeable read-only action boundaries with safe messages.
- `src/services/tools.rs` shows the action-service feedback shape and failure separation used by S05.
- `src/workers/events.rs`, `src/workers/mod.rs`, and `src/workers/handoff.rs` already support progress events, typed worker payload variants, `RecordingEventSink`, and `SlintEventLoopSink`.
- `src/app/f4se_controller.rs` is the best stale-safe scan controller template: monotonic scan ids, `WorkerTaskKind::Scan`, active id, safe spawn-failure mapping, and owned payload handling.
- `ui/f4se_tab.slint` is the closest table/status UI template. `ui/scanner_tab.slint` is currently an inert placeholder and must be replaced.
- `ui/main.slint` currently forwards Overview/F4SE/Tools/About/Settings properties and callbacks. Scanner has no properties/callbacks yet.
- `src/main.rs` currently wires controllers/sinks/workers. It is large; S07 wiring should follow existing helper naming and projection patterns, but consider keeping scanner-specific projection helpers compact and tested.

### New files expected

- `src/domain/scanner.rs`
  - Reference constants/labels/order.
  - `ScannerSettingKey`/`ScannerSettingsSnapshot` adapter from `domain::settings::ScannerSettings`.
  - `ScannerProblemType`, `ScannerSolution`, `ScannerResult`, `ScannerResultDetails`, `ScannerFileListEntry`, action descriptors.
  - Group order and render helpers.
  - Overview problem mapping helpers.
- `src/services/scanner.rs`
  - `ScannerScanService<F: Filesystem>` and request/report/diagnostics types.
  - MO2 mod-file index builder over `DiscoveredModManager::ModOrganizer`.
  - Data traversal/classification engine.
  - Race subgraph counting.
  - Scanner action service over `DesktopActions` + `ClipboardActions`.
- `src/app/scanner_controller.rs`
  - Slint-free reducer for toggles, scan ids, progress, grouped rows, selection, details, file list visibility, and action feedback.
  - Save-on-scan-start candidate setting transition may call into/coordinate with `SettingsController` from main, but reducer should remain testable without filesystem.
- `src/workers/events.rs`
  - Add `ScannerWorkerPayload`, likely variants `ScanCompleted { scan_id, snapshot, overview_snapshot? }` and `ActionCompleted { request_id/action, feedback }`.
  - Re-export in `src/workers/mod.rs`.
- `ui/scanner_tab.slint`
  - Export `ScannerUiRow` and maybe `ScannerFileListUiRow` structs.
  - Embedded scan settings pane, grouped rows, status/progress, details pane, action buttons, inline feedback.
- `ui/main.slint`
  - Scanner properties/models/callbacks forwarded through `MainWindow`.
- `src/main.rs`
  - Instantiate `ScannerController`, bind sink/callbacks, project scanner state to Slint, schedule scan/action workers, apply completion to scanner and optionally overview.

## Natural Seams / Suggested Task Decomposition

1. **Domain contract + source-contract tests**
   - Add `domain::scanner` constants and pure models.
   - Tests lock setting labels/order, problem/solution labels, progress/result strings, group order, details copy text, and no Auto-Fix labels.
   - Update `domain/mod.rs` exports/importability tests.

2. **Scanner service first proof**
   - Implement fake-backed `ScannerScanService` for a small Data fixture.
   - Cover categories: overview mapping, junk file/fomod, wrong format/proper format, loose previs, AnimTextData, F4SE script override, invalid archive name, race subgraph.
   - Cover MO2 modlist attribution and Vortex/Data-only no-attribution behavior.
   - Cover partial failure rows/diagnostics without aborting siblings.

3. **Controller reducer + worker payload**
   - Add `ScannerController` with monotonic scan ids and stale-event rejection like F4SE.
   - Handle progress events from `WorkerPayload::Progress` and final `ScannerWorkerPayload`.
   - Add selection/details state, expand/collapse state if implemented, zero-result completion, action-feedback state.
   - Add worker payload variant and tests.

4. **UI Slint replacement**
   - Replace placeholder `ui/scanner_tab.slint` with embedded panes.
   - Use exported row structs and callback ids; do not put scanner logic in Slint.
   - Add/adjust source-contract tests in `src/main.rs`: placeholder removed, labels in order, properties/callbacks exist, Auto-Fix absent.

5. **Runtime wiring**
   - Extend `SettingsController` for scanner toggle persistence at scan start.
   - Bind scanner callbacks in `main.rs`.
   - Worker orchestration performs Overview refresh first, then scanner; emits progress; applies final state on Slint event loop.
   - Add action workers for open/copy/read-only feedback.

6. **Closeout verification**
   - Focused scanner tests first, then full cargo gates.

## First Proof to Build

The first executor should start with `src/domain/scanner.rs` + `src/services/scanner.rs` tests before any Slint work. A good red/green fixture:

- Fake Data tree:
  - `Data/fomod/` => `Junk File` folder.
  - `Data/Vis/` => `Loose Previs` folder.
  - `Data/Meshes/Precombined/` => `Loose Previs`.
  - `Data/Meshes/AnimTextData/` => `Loose AnimTextData`.
  - `Data/Textures/icon.png` with no `icon.dds` => `Unexpected Format` + convert/delete/ignore solution.
  - `Data/Sound/theme.mp3` with `theme.xwm` => `Unexpected Format` + expected format found + delete/ignore solution.
  - `Data/Scripts/Actor.pex` attributed to an MO2 mod => `F4SE Script Override`.
  - `Data/Tools/helper.dll` or `Data/Meshes/helper.dll` => misplaced DLL / unexpected format.
  - `Data/BadArchive.ba2` not enabled and no valid suffix => `Invalid Archive Name`.
  - Enabled module bytes containing >100 `\x00SADD` sequences => race subgraph simple result + file list.
- Fake MO2:
  - `profiles/Default/modlist.txt` with `+Problem Mod` and overwrite path.
  - Source file index maps relative paths to `Problem Mod` and source paths.
- Expected assertions:
  - Stable group order, row display names, mod attribution, details labels/copy text, no mutation calls.
  - With all data toggles off except `OverviewIssues` or `RaceSubgraphs`, Data traversal is skipped.
  - With Vortex context, rows still appear but no staged mod column/source attribution is invented.

This proof de-risks the real scanner behavior before UI/property churn.

## Risks and Watch-outs

- **Reference `Errors` no-op:** Do not invent broad new checks accidentally. If S07 uses `Errors` to show unreadable/missing path rows, make that a narrow, documented intentional completion of S07 partial-failure scope.
- **Overview refresh duplication:** `build_overview_snapshot` currently lives in `main.rs` and discards collected facts after building `OverviewSnapshot`. Scanner needs Overview problems plus enabled module/archive facts. Factor shared worker orchestration carefully or duplicate temporarily with tests; do not let Scanner consume stale Overview controller state when the reference flow refreshes first.
- **Private BA2 suffix helper:** `overview_collector::ba2_suffixes` is private. Scanner invalid archive name checks need the same logic. Prefer extracting to a shared pure helper to avoid drift.
- **Partial traversal:** Avoid `Filesystem::walk_dir` for scanner classification because it is all-or-nothing and cannot easily prune top-level folders like the reference. Manual `read_dir` traversal also makes progress messages easier.
- **Path normalization:** Reference uses Windows paths and `startswith("f4se\\plugins")`. Rust tests may run cross-platform. Normalize components case-insensitively instead of string-comparing separators.
- **Settings thread safety:** `SettingsController` is `Rc<RefCell<...>>` and not `Send`; scanner workers must receive cloned settings only after UI-thread persistence succeeds/fails.
- **Main size:** `src/main.rs` is already large. Keep scanner wiring idiomatic with existing patterns, but avoid putting scanner classification logic there.
- **No destructive actions:** Scanner action service must only call open/copy adapters. No delete/rename/archive/patch/backup APIs belong in S07.
- **Mod attribution:** Use MO2 staged index only. Vortex/Data-only attribution should remain unknown/blank/`<Unmanaged>` according to the row type; never guess mod names from path fragments.
- **Human UAT likely unavailable in auto-mode:** Source-contract and controller/projection tests are important because GUI visual verification may not run.

## Verification Plan

Focused tests to add/run as execution proceeds:

- `cargo test scanner_domain`
- `cargo test scanner_service`
- `cargo test scanner_controller`
- `cargo test scanner_worker_payload`
- `cargo test s07_scanner_slint_contract`
- `cargo test s07_scanner_runtime_wiring`

Required closeout gates:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- Verify `CMT/` remains unmodified before closeout (e.g. `git status --short CMT` when allowed by that unit's tool policy).

## Sources / Evidence

- `CMT/src/tabs/_scanner.py` read directly for flow, UI labels, details, MO2 index, traversal, and action behavior.
- `CMT/src/scan_settings.py` read directly for setting labels/order/defaults, whitelist/junk/proper formats, skip timing.
- `CMT/src/enums.py` read directly for problem/solution labels.
- `CMT/src/helpers.py` inspected via `gsd_exec` for `ProblemInfo` and `SimpleProblemInfo` fields/defaults (`799c7446-fced-4856-ba84-3adc7c5e3e0e`).
- `CMT/src/globals.py` extracted for scanner tooltips/race threshold/archive whitelist/F4SE script names (`ea0a5f78-f2b9-4b7e-939c-375ed46401a7`, `b3515e37-1239-438e-9a3a-ef83256f3c12`, `fa053275-4623-4326-b08c-249795863e72`).
- Current Rust files read directly: `src/domain/settings.rs`, `src/app/settings_controller.rs`, `src/domain/overview.rs`, `src/domain/discovery.rs`, `src/domain/mod_manager.rs`, `src/platform/filesystem.rs`, `src/platform/desktop.rs`, `src/platform/clipboard.rs`, `src/workers/events.rs`, `src/workers/mod.rs`, `src/workers/handoff.rs`, `src/app/f4se_controller.rs`, `src/services/f4se.rs`, `src/services/overview_collector.rs`, `ui/scanner_tab.slint`, `ui/main.slint`, `ui/f4se_tab.slint`, `Cargo.toml`.
- Relevant memories: MEM003 settings persistence, MEM013 worker events, MEM017/MEM025 Overview pattern, MEM044/MEM046 F4SE scan pattern, MEM047 scanner Errors gotcha.
