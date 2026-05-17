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
