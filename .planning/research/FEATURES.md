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
