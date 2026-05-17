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
