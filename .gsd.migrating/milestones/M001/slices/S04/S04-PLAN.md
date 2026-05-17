# S04: Overview Diagnostics & Updates

**Goal:** Deliver a faithful, responsive Overview tab that shows game, manager, PC specs, binary, archive, module, update, and problem status from typed Rust discovery and diagnostics.
**Demo:** User can see Overview game, binary, archive, module, and update status panels populated from typed discovery and diagnostics.

## Must-Haves

- Owned active requirements:
- Later Overview work can consume typed installation, manager, and system metadata state without direct OS queries in tests.
- Later F4SE and Scanner work can distinguish valid game paths from missing optional Data or Data/F4SE/Plugins folders.
- Later tools and background workflows can use typed platform and worker boundaries instead of blocking Slint callbacks.
- Must-haves:
- Overview is no longer an inert placeholder and keeps the reference top status rows: Mod Manager, Game Path, Version, PC Specs.
- The Binaries (EXE/DLL/BIN), Archives (BA2), and Modules (ESM/ESL/ESP) panels expose reference-shaped labels, counts, status classes, and detail text.
- The Overview problem feed is typed and scanner-ready, including problem type, path or relative path when applicable, summary, solution, link/detail metadata, and source marker.
- Initial refresh and Refresh run off the Slint UI thread; missing or partial discovery is shown inline rather than as a modal interruption.
- Update checks respect AppSettings.update_source: none skips work, selected Nexus/GitHub sources run, newer versions show the green update banner, and no-update or failed checks stay silent except logs and diagnostics.
- Game path and update link actions use DesktopActions and show safe visible failure feedback when opening fails.
- Downgrade Manager and Archive Patcher controls remain visually aligned with the reference but disabled or clearly deferred until S09/S10.
- Threat Surface Q3:
- Abuse: user-controlled install paths, mod file names, BA2/plugin bytes, and update response bodies are untrusted. Do not execute discovered files; only read bounded metadata and open explicit path or URL targets through DesktopActions.
- Data exposure: local absolute paths may be displayed because the reference shows them; diagnostics must not expose secrets, tokens, or raw network response bodies.
- Input trust: filesystem, INI, plugins.txt, Fallout4.ccc, archive/module headers, and update responses must be parsed defensively and fail into inline problems or logged diagnostics.
- Requirement Impact Q4:
- Re-verify S02 settings load/save behavior because update_source now drives Overview work.
- Re-verify S03 discovery, platform, desktop, and worker contracts because S04 consumes them through the real app path.
- Decision revisited: D018 establishes the S04 typed snapshot and adapter-backed architecture.
- Failure Modes Q5:
- Missing game, missing Data, missing Fallout4.ccc, missing plugins.txt, unreadable archive/module, unsupported file-version API, malformed update response, network timeout, worker panic, and desktop-open failure are all represented as safe states, worker failures, logs, or problem-feed entries.
- Load Profile Q6:
- Data folders may contain thousands of files. Header reads should be bounded, traversal deterministic, UI updates batched as owned snapshots, and Slint models updated only on the event loop.
- Negative Tests Q7:
- Cover no game path, valid game path without Data, missing optional F4SE path, unreadable BA2/plugin records, count limits exceeded, unknown binary CRC/version, malformed GitHub/Nexus responses, update_source none, and desktop action failure.
- Slice verification:
- cargo fmt --check
- cargo check
- cargo test
- cargo clippy --all-targets --all-features
- git status --short CMT

## Proof Level

- This slice proves: Integration proof. Fake-backed domain, service, controller, update, desktop, and worker tests prove behavior without a host Fallout 4 install or live network. cargo check proves Slint composition. Human UAT is not required for closeout, but a later visual pass may compare the Overview tab against the Python reference.

## Integration Closure

Consumes S03 DiscoveryService, Fallout4Installation, DiscoveredModManager, SystemMetadata, Filesystem, ProcessInspector, DesktopActions, and worker handoff seams; consumes S02 AppSettings update_source. Introduces Overview domain snapshots, problem feed, collector, update service, controller, worker payloads, and Slint bindings. Leaves mutation workflows to S09/S10 and scanner rendering to S07/S08 while providing their problem-feed input.

## Verification

- Add tracing spans or structured logs for overview refresh start/completion/failure, filesystem collection counts, binary/archive/module classification summaries, selected update sources, silent update failures, desktop-open failures, and worker lifecycle. Expose last refresh state and safe last action error in the Overview UI so future agents can localize failures without reproducing the whole scan.

## Tasks

- [x] **T01: Define Overview snapshot contracts** `est:3h`
  ---
  estimated_steps: 7
  estimated_files: 2
  skills_used:
    - design-an-interface
    - tdd
    - verify-before-complete
  ---
  Why: S04 needs a typed state boundary before any Slint or OS wiring so tests can assert Overview behavior without host filesystem, registry, process, or network access.
  - Files: `src/domain/overview.rs`, `src/domain/mod.rs`
  - Verify: cargo test overview_domain

- [x] **T02: Compute pure Overview diagnostics** `est:5h`
  ---
  estimated_steps: 9
  estimated_files: 3
  skills_used:
    - tdd
    - verify-before-complete
  ---
  Why: The reference Overview logic mixes diagnostics with Tk widgets. This task extracts the scanner-ready decisions into a pure service before adding filesystem or Slint wiring.
  - Files: `src/services/overview.rs`, `src/services/mod.rs`, `src/domain/overview.rs`
  - Verify: cargo test overview_diagnostics

- [x] **T03: Collect Overview filesystem facts** `est:6h`
  ---
  estimated_steps: 10
  estimated_files: 5
  skills_used:
    - tdd
    - verify-before-complete
  ---
  Why: The user-visible Overview must be populated from real discovered installations, but the filesystem and process work must remain fakeable and off the UI thread.
  - Files: `src/services/overview_collector.rs`, `src/services/mod.rs`, `src/services/overview.rs`, `src/domain/overview.rs`, `Cargo.toml`, `Cargo.lock`
  - Verify: cargo test overview_collector

- [ ] **T04: Add update and link services** `est:4h`
  ---
  estimated_steps: 8
  estimated_files: 5
  skills_used:
    - tdd
    - rust-async-patterns
    - verify-before-complete
  ---
  Why: S04 must match the reference update banner behavior and safe open-only actions while keeping network and desktop behavior injectable.
  - Files: `src/services/update.rs`, `src/services/mod.rs`, `src/domain/overview.rs`, `Cargo.toml`, `Cargo.lock`
  - Verify: cargo test overview_update

- [ ] **T05: Wire Overview controller and workers** `est:6h`
  ---
  estimated_steps: 10
  estimated_files: 7
  skills_used:
    - rust-async-patterns
    - tdd
    - verify-before-complete
  ---
  Why: The Overview tab must refresh automatically and on demand without blocking Slint, while settings and desktop actions flow through existing app and worker seams.
  - Files: `src/app/overview_controller.rs`, `src/app/mod.rs`, `src/app/settings_controller.rs`, `src/main.rs`, `src/workers/events.rs`, `src/workers/mod.rs`, `src/services/overview.rs`, `src/services/update.rs`
  - Verify: cargo test overview_controller

- [ ] **T06: Replace Overview Slint placeholder** `est:5h`
  ---
  estimated_steps: 9
  estimated_files: 4
  skills_used:
    - tdd
    - verify-before-complete
  ---
  Why: The slice is only user-visible when the Slint tab presents the typed snapshot with reference-shaped layout and callbacks.
  - Files: `ui/overview_tab.slint`, `ui/main.slint`, `src/main.rs`, `src/app/overview_controller.rs`
  - Verify: cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Files Likely Touched

- src/domain/overview.rs
- src/domain/mod.rs
- src/services/overview.rs
- src/services/mod.rs
- src/services/overview_collector.rs
- Cargo.toml
- Cargo.lock
- src/services/update.rs
- src/app/overview_controller.rs
- src/app/mod.rs
- src/app/settings_controller.rs
- src/main.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/overview_tab.slint
- ui/main.slint
