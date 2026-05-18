# S05 Research: Tools Shell, Links & About

## Research Depth

Targeted research. The main technology and architecture are already established by S01-S04, but S05 touches new static-link surfaces, clipboard behavior, Slint image resources, and worker/action feedback identity.

## Active Requirements

No `REQUIREMENTS.md` content was preloaded for this unit. S05 primarily supports the milestone success criteria around faithful tab structure, safe external actions, and buildable/testable Rust/Slint slices.

## Skills Discovered

- Installed skills already relevant by category: `rust-async-patterns` for worker/off-thread patterns, `test` / `verify-before-complete` for gates, and `observability` for visible failure feedback and tracing discipline.
- `npx skills find "Slint"` returned only unrelated lint/accessibility skills.
- `npx skills find "Rust Slint GUI"` returned `rust-desktop-applications` (259 installs, broad), `rust-cli-tui-developer` (39 installs, tangential), and `lib-slint-expert` (31 installs, directly named but low signal).
- Attempted `npx skills add bahayonghang/my-claude-code-settings@lib-slint-expert -g -y`; the installer cloned the repo but failed with “No matching skills found for: lib-slint-expert”. No new skill was installed.
- `npx skills find "Rust clipboard"` returned generic clipboard skills with low/directly-mismatched relevance; none installed.

## Summary

S05 should replace the inert Tools and About placeholders with static, reference-shaped Slint tabs backed by Rust-owned action definitions and fakeable platform adapters. The Python reference is simple but fidelity-sensitive: Tools is three labelframe columns of buttons, most with static URLs plus help tooltips; About is title/logo/credit text plus Nexus, Discord, and GitHub open/copy rows. The current Rust project has the right shell, `DesktopActions`, worker handoff, and Overview safe-error pattern, but it has no Tools/About domain module, no clipboard adapter, no Rust-owned image assets, no Tools/About callbacks in `ui/main.slint`, and tests currently assert Tools/About are inert placeholders.

Recommended direction: add a small `src/domain/tools.rs` contract for labels/URLs/action IDs, a `src/platform/clipboard.rs` fakeable clipboard boundary, Slint-free controller/reducer state for Tools/About feedback, and wire URL/copy requests through `WorkerRuntime`/`WorkerEvent` instead of executing actions directly in callbacks. Copy the four required reference images into a Rust-owned asset path and use Slint image resources only after a one-image `cargo check` proof.

## Reference Contract

### Tools tab source: `CMT/src/tabs/_tools.py`

Group order and button order are dict insertion order in the reference. Each group is a `ttk.Labelframe`; each button is a full-width row. Utility actions instantiate modal workflows in Python but must remain disabled/deferred in S05.

| Group | Entries in order | Action / URL | Help text semantics |
| --- | --- | --- | --- |
| `Toolkit Utilities` | `Downgrade Manager`; `Archive Patcher` | Internal modal actions | No tooltip in reference for these; S05 should show disabled/deferred state. |
| `Other CM Authors' Tools` | `Bethini Pie` | `https://www.nexusmods.com/site/mods/631` | `Bethini Pie (Performance INI Editor) makes editing INI config files simple.\nDiscord channel: #bethini-doubleyou-etc` |
|  | `CLASSIC Crash Log Scanner` | `https://www.nexusmods.com/fallout4/mods/56255` | `Scans Buffout crash logs for key indicators of crashes.\nYou can also post crash logs to the CM Discord for assistance.\nDiscord channel: #fo4-crash-logs` |
|  | `  Vault-Tec Enhanced\nFaceGen System (VEFS)` | `https://www.nexusmods.com/fallout4/mods/86374` | `Automates the process of generating FaceGen models and textures with xEdit/CK.\nDiscord channel: #bethini-doubleyou-etc` |
|  | `PJM's Precombine/Previs\n    Patching Scripts` | `https://www.nexusmods.com/fallout4/mods/69978` | `Scripts to find precombine/previs (flickering/occlusion) errors in your mod list, and optionally generate a patch to fix those problems.` |
|  | `DDS Texture Scanner` | `https://www.nexusmods.com/fallout4/mods/71588` | `Sniff out textures that might CTD your game. With BA2 support.\nDiscord channel: #nistonmakemod` |
| `Other Useful Tools` | `xEdit / FO4Edit` | `https://github.com/TES5Edit/TES5Edit#xedit` | `Module editor and conflict detector for Bethesda games.\nFO4Edit/SSEEdit are xEdit, renamed to auto-set a game mode.` |
|  | `Creation Kit Platform\n   Extended (CKPE)` | `https://www.nexusmods.com/fallout4/mods/51165` | `Various patches and bug fixes for the Creation Kit to make life easier.` |
|  | `Cathedral Assets\nOptimizer (CAO)` | `https://www.nexusmods.com/skyrimspecialedition/mods/23316` | `An automation tool used to optimize BSAs, meshes, textures and animations.` |
|  | `BA2 Merging Automation\n     Tool (BMAT)` | `https://www.nexusmods.com/fallout4/mods/89306` | `Automated BA2 files repackaging and merging.` |
|  | `IceStorm's Texture Tools` | `https://storage.icestormng-mods.de/s/QG43aExydefeGXy` | `Converts textures from various formats into a Fallout 4 compatible format.` |
|  | `CapFrameX` | `https://www.capframex.com/` | `Benchmarking tool - Record FPS, frametime, and sensors; analyse and plot the results.` |

Reference also attaches generic URL tooltips based on URL host: Nexus links get `View on Nexus Mods`, GitHub links get `View on GitHub`, all others get `Open website`. Exact tooltip widgets may not be worth blocking on in Slint; preserve the help semantics as adjacent muted helper text or the closest practical tooltip affordance.

### About tab source: `CMT/src/tabs/_about.py`, `CMT/src/globals.py`, `CMT/src/utils.py`

- Title display: `APP_TITLE.rsplit(maxsplit=1)` => `Collective Modding\nToolkit`.
- Version/credit text is reference-visible original version, not Cargo package version: `v0.6.1\n\nCreated by wxMichael for the\nCollective Modding Community\n#cm-toolkit on Discord`.
- Link constants:
  - Nexus: `https://www.nexusmods.com/fallout4/mods/87907`
  - Discord: `https://discord.gg/tktyEyYHZH`
  - GitHub: `https://github.com/wxMichael/Collective-Modding-Toolkit`
- Link rows and button labels:
  - Nexus logo row: `Open Link`, `Copy Link`
  - Discord logo row: `Open Invite`, `Copy Invite`
  - GitHub logo row: `Open Link`, `Copy Link`
- Copy behavior in `copy_text_button`: clear clipboard, append text, save original button label, set button text to `Copied!`, disable the button, then after 3000ms restore original label and `NORMAL` state.
- Image files required from read-only reference assets, copied into Rust-owned resources before use:
  - `icon-256.png` — 256x256 RGBA
  - `logo-nexusmods.png` — 256x61 RGBA
  - `logo-discord.png` — 256x49 RGBA
  - `logo-github.png` — 194x60 RGBA

## Current Implementation Landscape

### Existing files and purpose

- `ui/tools_tab.slint` and `ui/about_tab.slint` are 20-line inert placeholders with only a heading and “behavior is reserved” text.
- `ui/main.slint` imports the two tab components but instantiates them as `ToolsTab {}` and `AboutTab {}` with no properties or callbacks. Main already forwards a large Overview API and Settings callbacks; S05 should follow that style.
- `ui/settings_tab.slint` shows the local Slint style pattern: dark `Rectangle`, `GroupBox`, static labels, callback emission, and simple source-contract tests.
- `ui/overview_tab.slint` shows the visible error banner, disabled deferred action controls, and exported model pattern used in S04.
- `src/platform/desktop.rs` already provides a fakeable `DesktopActions` trait and `RealDesktopActions` for URL/path/tool launch. Real desktop opens are Windows-only via `ShellExecuteW`; off-Windows failures are explicit `UnsupportedPlatform` safe messages.
- `src/platform/mod.rs` owns `PlatformOperation`, `PlatformErrorKind`, and `PlatformError`; it lacks a clipboard/write operation.
- `src/workers/events.rs` already has `WorkerTaskKind::DesktopAction` and generic `ExternalActionPayload`, but that payload has no UI action ID. It also has Overview-specific worker payloads for typed UI reducers.
- `src/workers/mod.rs` and `src/workers/handoff.rs` provide the established off-thread execution and Slint event-loop sink. Reuse rather than launching URLs/copying directly from callbacks.
- `src/app/overview_controller.rs` is the best pattern for a Slint-free reducer and safe action error mapping. It is Overview-specific; do not put Tools/About logic into it.
- `src/services/update.rs` contains `OverviewLinkService`, which executes `OverviewDeferredActionTarget::Url/Path` through `DesktopActions`. This can inspire a generic link service, but it is typed around Overview domain concepts.
- `src/main.rs` currently binds Settings and Overview only. Its tests include `INERT_TAB_COMPONENTS` containing F4SE, Scanner, Tools, and About; S05 must update this to leave only F4SE/Scanner inert and add Tools/About contract tests.
- `Cargo.toml` has no clipboard dependency. The project stack research already recommended `arboard = 3.6.1` for desktop clipboard if Slint's own APIs are not sufficient.
- No Rust-owned app image assets exist outside `CMT/` right now.

### Prior architecture notes from memory

- Platform OS access is isolated behind fakeable traits in `src/platform`; real adapters return typed platform errors.
- Future tabs should preserve the S04 data flow: pure domain snapshots/controllers, adapter-backed services, owned worker payloads, and UI mutation only through the Slint event-loop sink.
- Background work should emit owned `WorkerEvent` envelopes; use `RecordingEventSink` in tests and `SlintEventLoopSink` for UI handoff.

## Recommendation

### Domain/action contracts

Add `src/domain/tools.rs` and export it from `src/domain/mod.rs`. Keep this Slint-free and OS-free.

Suggested contents:

- Reference constants: `APP_TITLE`, `REFERENCE_APP_VERSION`, `ABOUT_TITLE_DISPLAY`, `ABOUT_CREDIT_TEXT`, `NEXUS_LINK`, `DISCORD_INVITE`, `GITHUB_LINK`.
- `ToolGroup`, `ToolEntry`, `ToolEntryKind::{DeferredUtility, ExternalLink}`, stable `ToolActionId`, and static/group-returning functions that preserve reference group/button order.
- `AboutLink`, `AboutLinkId::{Nexus, Discord, Github}`, open/copy button labels, and image resource names.
- Lookup functions from Slint callback IDs (`"bethini-pie"`, `"classic-crash-log-scanner"`, `"vefs"`, `"pjm-precombine-previs"`, `"dds-texture-scanner"`, `"xedit-fo4edit"`, `"ckpe"`, `"cao"`, `"bmat"`, `"icestorm-texture-tools"`, `"capframex"`, `"nexus"`, `"discord"`, `"github"`) to typed domain entries.
- Tests that assert exact group titles, labels including embedded newlines/leading spaces, URLs, help text, deferred utilities, About text, and copy/open labels.

Avoid putting URLs in `.slint` callbacks. Slint should emit stable action IDs; Rust should resolve IDs to static URL constants through the domain module.

### Clipboard/platform boundary

Add `src/platform/clipboard.rs` and export it from `src/platform/mod.rs`.

Suggested shape:

- Add `PlatformOperation::WriteClipboard` or `PlatformOperation::CopyToClipboard` with safe labels/messages, e.g. success `Copied link.` and failure `Clipboard write failed.`.
- `ClipboardActionResult` similar to `DesktopActionResult`, or reuse `PlatformResult<()>` with a small wrapper for safe text.
- `ClipboardActions` trait with `fn copy_text(&self, text: &str) -> ClipboardActionResult`.
- `RealClipboardActions` backed by `arboard::Clipboard` (add `arboard = "3.6.1"` to `Cargo.toml`) and fake adapter tests for success/failure. Keep raw adapter errors in diagnostics/tracing, not UI text.

### Controllers/services

Add Slint-free reducers, either combined or separate:

- `src/app/tools_controller.rs`: tracks last safe Tools action error/status and handles deferred utility tampering by returning `Downgrade Manager is reserved for a later port phase.` / `Archive Patcher is reserved for a later port phase.`.
- `src/app/about_controller.rs`: tracks About last safe action error plus copy button state per link (`Copy Link`, `Copy Invite`, `Copied!`, disabled until reset). Include pure methods for `copy_succeeded(link_id)`, `copy_failed(link_id, safe_message)`, and `copy_reset(link_id)`.

Add a small service module if useful, e.g. `src/services/tools.rs`, for executing domain link/copy actions through injected `DesktopActions` and `ClipboardActions`. Do not couple it to Slint.

### Worker/event identity

URL opens and clipboard writes can stall/fail and should not run inline on the Slint event thread. Reuse `WorkerRuntime` and `SlintEventLoopSink`.

Important design choice: current `WorkerPayload::ExternalAction` has no stable UI action ID, so S05 needs one of these before wiring:

1. Extend `ExternalActionPayload` with an optional/required `action_id: String` and maybe `surface: ExternalActionSurface` (`Tools`, `About`). Add `ExternalActionKind::Clipboard` for copy actions. This is reusable for Scanner copy/open actions in S07/S08.
2. Add an S05-specific worker payload, e.g. `WorkerPayload::Tools(ToolsWorkerPayload)` with typed `ToolActionId` / `AboutLinkId` fields, mirroring the Overview-specific approach.
3. Parse action IDs out of stable task IDs. This is least invasive but weaker; only use if avoiding worker enum churn is more important than type safety.

Recommendation: choose option 1 if the planner wants a generic external-action foundation for Scanner later; choose option 2 if the implementation should minimize broader worker API changes. Avoid silently mapping by target URL alone because multiple UI actions could share targets later.

### Slint UI

`ui/tools_tab.slint`:

- Import `GroupBox`/`Button` from `std-widgets.slint` as needed.
- Replace placeholder with dark background, `ScrollView`, horizontal three-column group layout matching reference groups.
- Utility buttons visible but disabled with status text like `Deferred until the Downgrade Manager workflow is ported.` and `Deferred until the Archive Patcher workflow is ported.`. If the Rust callback is invoked by tampered UI/test, fail closed with the reserved message.
- External buttons call `root.tool-action-requested("...")` and show help text in an adjacent/below muted `Text` block. This preserves tooltip semantics without requiring exact Tk tooltip behavior.
- Add `in-out property <string> tools-last-action-error` for a visible safe error banner. Success can clear the error; reference open-link success is silent.

`ui/about_tab.slint`:

- Use reference title and credit text exactly.
- Three link rows with logos where practical, vertical separators if easy, and exact button labels.
- Add callback IDs: `about-open-link-requested(string)` and `about-copy-link-requested(string)`.
- Add copy-label/enabled properties per link, e.g. `about-nexus-copy-label`, `about-nexus-copy-enabled`, etc., or a small exported struct/model if preferred.
- Add `about-last-action-error` banner for failed open/copy.
- Copy success must set the clicked copy button label to `Copied!`, disable it, and restore after 3000ms.

`ui/main.slint`:

- Add Tools/About properties and callbacks at `MainWindow` level and forward them to tab components, mirroring Overview/Settings forwarding.
- Add source-contract tests for callback forwarding and property binding.

### Assets

Copy, do not reference mutably from `CMT/` at runtime. A straightforward location is `ui/assets/images/` so `about_tab.slint` can use relative Slint image URLs such as `@image-url("assets/images/icon-256.png")` from within `ui/about_tab.slint` (verify with `cargo check` after copying one image before building the full tab). Alternative: root `assets/images/` with `@image-url("../assets/images/...")` from `ui/`.

If strict missing-image degradation is required, use Slint `image` properties populated by Rust with a runtime load attempt and leave the property empty on failure. If compile-time `@image-url` is used, missing resources fail the build rather than degrading at runtime; source/asset existence tests should make that explicit. The slice context only requires current Rust/Slint layout resource resolution, so compile-time resources are likely acceptable after a first proof.

## Natural Seams / Suggested Task Cuts

1. **Reference contract/domain + assets inventory**
   - Add `src/domain/tools.rs`, exports, exact label/URL/help/About tests.
   - Copy four image files into Rust-owned assets and add asset existence/source tests.
   - No Slint callbacks or platform actions yet.

2. **Clipboard and external-action feedback core**
   - Add `src/platform/clipboard.rs`, `PlatformOperation` clipboard variant, `arboard` dependency, fake tests.
   - Add Tools/About controller reducers with success/failure/copy-reset tests.
   - Decide and implement worker payload identity (generic external payload extension or S05-specific payload).

3. **Tools tab UI and wiring**
   - Replace `ui/tools_tab.slint`, add callbacks/properties in `ui/main.slint`, bind Rust callbacks in `src/main.rs`.
   - Use fake/domain tests first; production callback schedules `RealDesktopActions` through worker runtime.
   - Update `INERT_TAB_COMPONENTS` tests so Tools is no longer considered inert.

4. **About tab UI, copy feedback, and assets**
   - Replace `ui/about_tab.slint`, add logo resources, open/copy callbacks, copy label properties.
   - Wire open via `DesktopActions`, copy via `ClipboardActions`, and copy reset via Slint timer or a typed delayed worker event.
   - Keep Discord button labels distinct: `Open Invite` / `Copy Invite`.

5. **Closeout contracts and gates**
   - Add/adjust source-contract tests for exact group order, labels, About text, image resource references, callback forwarding, disabled utility state, and safe failure banners.
   - Run full gates.

## First Proof

Highest-risk first proof: implement one domain action, one fake clipboard/open failure path, and one Slint asset/callback compile path before filling all labels.

Recommended proof sequence:

1. Add `src/domain/tools.rs` with only Nexus/About and one Tools external link plus tests.
2. Add a minimal `ui/about_tab.slint` image reference to a copied `icon-256.png` and run `cargo check` to validate Slint resource syntax/path.
3. Add fake-backed controller tests proving:
   - open failure maps to a safe visible error,
   - copy success maps `Copy Link` -> `Copied!` disabled,
   - copy reset restores `Copy Link` enabled,
   - copy failure does not show `Copied!`.
4. Expand to the full reference contract after these pass.

This proof unblocks the risky parts: resource packaging, generated callback names, clipboard abstraction, and temporary copy state.

## Risks and Constraints

- **Do not edit `CMT/`**. It is only the read-only reference and asset source.
- **Existing tests will fail until updated** because `src/main.rs` currently asserts Tools/About are inert placeholders with no callbacks/URLs/process markers.
- **Multi-line Button text may need verification**. Tk buttons include embedded newlines and leading spaces. Slint standard `Button` may need explicit height/min-height or a custom action-button component to preserve display.
- **Tooltip fidelity is approximate** unless Slint tooltip support is confirmed. Preserve all tooltip/help strings in domain tests and render them as adjacent helper text if exact hover tooltips are unavailable.
- **Copy reset should not block the UI thread**. Prefer `slint::Timer` on the event loop for the 3000ms label reset, with pure controller reset tests. If using `tokio::time::sleep`, add Tokio's `time` feature; current `tokio` features are only `rt-multi-thread`, `macros`, and `sync`.
- **Clipboard dependency can fail at runtime** if the OS clipboard is unavailable. This must surface as a safe banner/status, not as `Copied!`.
- **Real URL opens are Windows-only today**. Off-Windows developer clicks will show `URL open is not supported on this platform.`, which is consistent with current platform adapter behavior.
- **Static URLs only**. Do not accept arbitrary URLs from Slint strings; callback IDs resolve to domain constants.
- **About version should remain `v0.6.1`** for reference parity, not Cargo `0.1.0`, unless a later product decision changes Rust port branding.
- **Deferred utilities must stay disabled/fail-closed**. Do not instantiate or stub live Downgrade Manager/Archive Patcher workflows in S05.

## Verification Plan

Focused tests during implementation:

- `cargo test tools` / `cargo test about` after adding domain/controller tests.
- `cargo test clipboard` after adding platform clipboard.
- `cargo test main_window_forwards` or equivalent after Slint callback forwarding changes.
- `cargo test tools_tab` / `cargo test about_tab` source-contract filters after UI replacement.

Closeout gates for S05 implementation:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `git status --short CMT` when the active unit permits git commands; otherwise perform a non-mutating CMT cleanliness/source-marker inspection and report why git was skipped.

Specific contract assertions to add/update:

- Tools group labels are exactly `Toolkit Utilities`, `Other CM Authors' Tools`, `Other Useful Tools` in order.
- Tools button labels, including embedded newlines/leading spaces, match reference order.
- Tools URL constants and help strings match reference.
- Toolkit utility buttons are disabled/deferred and never schedule destructive work in S05.
- About title, version/credit text, link URLs, button labels, image resource names, and copy feedback labels match reference.
- Failed desktop open and failed clipboard copy map to safe visible UI text.
- Successful copy maps to `Copied!` disabled and reset restores the original label.
- `ui/main.slint` forwards all Tools/About callbacks/properties to `MainWindow` callbacks.
- Placeholder/inert tests now cover only F4SE and Scanner, not Tools/About.

## Sources

Reference files read:

- `CMT/src/tabs/_tools.py`
- `CMT/src/tabs/_about.py`
- `CMT/src/globals.py`
- `CMT/src/utils.py`
- `CMT/src/assets/images/*` image inventory/dimensions

Local implementation files inspected:

- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `ui/settings_tab.slint`
- `ui/overview_tab.slint`
- `src/main.rs`
- `src/domain/overview.rs`
- `src/platform/mod.rs`
- `src/platform/desktop.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
- `src/app/overview_controller.rs`
- `src/app/settings_controller.rs`
- `src/services/update.rs`
- `src/services/overview.rs`
- `Cargo.toml`
- `build.rs`

Useful persisted research outputs:

- `.gsd/exec/d6a3593d-c94f-4501-91b3-679c1882f1b3.stdout` — extracted exact Tools/About reference contract.
- `.gsd/exec/f875d5a9-794a-40bf-b890-1fdcecd0be09.stdout` — asset inventory and existing Tools/About markers.
- `.gsd/exec/66ae84e9-3722-49c6-89af-27a9be59ac56.stdout` — current domain constants/deferred action markers and absence of Rust-owned assets.
- `.gsd/exec/0a3bde12-08b4-4d05-b6e0-abd89273286e.stdout` — broad current-code surface summary.
- `.gsd/exec/9865a745-876c-4cb3-a4e5-1f1e91af6b8e.stdout` — reference image dimensions.
