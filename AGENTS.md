# Agent Instructions

## Project Goal

Port the application in `CMT/` to Rust using the Slint GUI framework. The port should preserve the original application's behavior and make the UI look as close to the original as practical.

## Reference Source

- Treat `CMT/` as a read-only reference submodule.
- Do not edit, format, move, delete, or generate files under `CMT/`.
- When implementing a feature, inspect the relevant original files in `CMT/src/` first and preserve labels, tab structure, control ordering, defaults, validation rules, and user-facing messages unless there is a clear reason to diverge.
- If the reference app appears wrong or incomplete, document the discrepancy and ask before changing the intended Rust behavior.

## Rust And Slint Direction

- Implement new code in the Rust project outside `CMT/`.
- Prefer Slint `.slint` files for UI structure and styling, with Rust handling application state, filesystem work, parsing, and command execution.
- Keep UI and domain logic separated enough that non-UI behavior can be tested without launching a window.
- Avoid blocking the Slint UI thread. Run slow filesystem scans, parsing, or process work off the UI thread and marshal results back through Slint-safe callbacks or event-loop APIs.
- Use typed Rust models for app state instead of passing unstructured strings or maps through the code.

## UI Fidelity Requirements

- Match the original CMT layout, tab names, labels, button text, grouping, spacing, and enabled/disabled states as closely as Slint allows.
- Preserve original workflows before redesigning them. Do not modernize or simplify UI flows unless explicitly requested.
- When adding a new screen or porting a tab, compare it against the source implementation in `CMT/src/tabs/` and note any intentional differences in the final response.
- Keep visual changes conservative. The goal is a faithful Rust/Slint port, not a new design language.

## Implementation Practices

- Port in vertical slices: one tab, dialog, or workflow at a time, keeping the app buildable after each slice.
- Prefer small, direct changes over broad refactors while the port is in progress.
- Use idiomatic Rust error handling. Avoid `unwrap()` and `expect()` in production paths unless the invariant is obvious or documented.
- Add Rust doc comments (`///`) to public functions, public types, and methods that are added or substantially rewritten. Add short comments for non-obvious constraints, UI-thread handoffs, ownership, cancellation, or compatibility behavior.
- Do not remove existing comments unless the code they describe is removed or the comment has become wrong.
- Keep dependencies focused. Add crates only when they directly support the port, and prefer widely used crates with clear maintenance.

## Verification

Before considering a change complete, run the most relevant checks available for the current project state:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`

If a check cannot run because the project is still being bootstrapped or a dependency is unavailable, report that explicitly with the reason.

## Git And Workspace Safety

- The `CMT/` submodule is reference-only; never commit modifications inside it for this port.
- Do not overwrite or revert unrelated user changes.
- Keep generated build artifacts out of version control.
- Do not make commits unless the user explicitly asks.

<!-- GSD:project-start source:PROJECT.md -->
## Project

**Collective Modding Toolkit Rust Port**

This project ports the existing `CMT/` Collective Modding Toolkit desktop application to Rust using the Slint GUI framework. The Rust application should preserve the original Tkinter application's workflows, tab structure, labels, defaults, validation behavior, and user-facing messages as closely as practical while keeping the implementation idiomatic, testable, and responsive.

The reference app is the Python source under `CMT/src/`; the new implementation lives outside `CMT/` in the Rust crate. The initial milestone is an "Initial Port" that establishes a faithful, buildable Rust/Slint foundation and then ports the original app in narrow vertical slices.

**Core Value:** Fallout 4 mod users can run a faithful Rust/Slint Collective Modding Toolkit that performs the same practical checks and utility workflows as the original CMT app without relying on the Python/Tkinter implementation.

### Constraints

- **Reference source**: `CMT/` is read-only and must be inspected before porting any behavior - it is the source of truth for labels, ordering, defaults, validation, and messages.
- **Tech stack**: Rust with Slint for UI; Rust handles application state, filesystem work, parsing, and command execution.
- **UI fidelity**: Match the original layout, tab names, grouping, button text, enabled/disabled states, and conservative visual language as closely as Slint allows.
- **Responsiveness**: Slow scans, archive parsing, filesystem traversal, and process work must run off the Slint UI thread and marshal results back safely.
- **Quality gates**: Relevant checks are `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` before considering implementation slices complete.
- **Scope control**: Port in vertical slices and avoid broad refactors or new dependencies unless they directly support the port.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

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
# Core UI
# Runtime orchestration and typed app state
# Filesystem scanning and binary helpers
# Desktop integration
# Windows/Fallout-specific discovery and parsing candidates
# Test support
# Optional later phases only
## Recommended Project Structure
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
- Use `slint`, `slint-build`, typed placeholder models, and no scanner/archive/network crates beyond `tracing` and `directories`.
- Because the first milestone should lock tab names, layout, settings defaults, callback boundaries, and UI-thread handoff before porting heavy behavior.
- Use `walkdir`, typed `ScanSettings`, `thiserror`, `assert_fs`, and `insta`.
- Because scanner fidelity depends on deterministic traversal, clear classification, and snapshot-visible result trees/messages.
- Use `pelite` on Windows plus fallback byte/resource tests from reference examples.
- Because Python currently relies on Windows-specific version/resource APIs; cross-platform PE parsing should be validated before replacing it.
- Start with `byteorder`/explicit byte patches and `tempfile` tests that compare before/after bytes and messages.
- Evaluate `ba2`/`binrw` only after exact reference behavior is preserved.
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
## Sources
- Context7 Slint Rust docs (`/websites/slint_dev_rust_slint`): Slint 1.16 build setup, `slint-build`, external `.slint` compilation, `VecModel`, and `upgrade_in_event_loop` thread handoff. HIGH confidence.
- Context7 Slint widget docs (`/websites/slint_dev_slint`): `TabWidget` and standard widget styles. HIGH confidence.
- crates.io API checked 2026-05-16 for current crate versions: `slint 1.16.1`, `slint-build 1.16.1`, `tokio 1.52.3`, `walkdir 2.5.0`, `serde 1.0.228`, `toml 1.1.2`, `tracing 0.1.44`, `rfd 0.17.2`, `ba2 3.0.1`, and others. HIGH for version existence.
- Context7 Tokio docs (`/websites/rs_tokio_1_49_0`): `spawn_blocking`, channel bridging, and Tokio filesystem caveats. HIGH for architectural pattern; exact latest patch version verified separately via crates.io.
- Context7 Walkdir docs (`/burntsushi/walkdir`): filtering, depth control, symlink handling, and detailed traversal errors. HIGH confidence.
- Context7 Serde docs (`/websites/serde_rs`): `Serialize`/`Deserialize` derive and `serde` dependency setup. HIGH confidence.
- Context7 tracing-subscriber docs (`/websites/rs_tracing-subscriber`): `EnvFilter`, fmt/file logging layers. HIGH confidence.
- Local project files: `.planning/PROJECT.md`, `Cargo.toml`, `AGENTS.md`, and read-only summaries of `CMT/src/main.py`, `CMT/src/cm_checker.py`, `CMT/src/tabs/*.py`, `app_settings.py`, `scan_settings.py`, `game_info.py`, `downgrader.py`, `utils.py`, and `patcher/*.py`. HIGH for project/reference constraints.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
