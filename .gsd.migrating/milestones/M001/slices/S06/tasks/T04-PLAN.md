---
estimated_steps: 16
estimated_files: 3
skills_used: []
---

# T04: Wire F4SE Slint tab and lazy scan

Expected executor skills for task-plan frontmatter: rust-async-patterns, tdd, verify-before-complete.

Why: The slice is only user-visible when MainWindow forwards F4SE properties and activates a background scan on first tab selection while rendering the reference-shaped table and legend.

Do:
1. Replace ui/f4se_tab.slint placeholder with a conservative reference-shaped layout: left table with headers DLL, OG, NG, AE, Your Game; row severity coloring; optional safe row detail; vertical scrolling; right heading F4SE DLLs; and exact ABOUT_F4SE_DLLS legend text.
2. Export a F4seUiRow Slint struct with dll, og, ng, ae, your_game, severity, and detail fields.
3. Add F4SE properties to ui/main.slint for status text, busy flag, loading or error text, unknown-game detail, and row model, plus a callback for F4SE tab activation. Bind TabWidget current-index or the Slint-supported equivalent so the callback fires when tab index 1 becomes active. Do not add a manual refresh button.
4. Update src/main.rs to create F4seController, apply initial projection, bind a SlintEventLoopSink for F4SE worker events, and schedule F4seScanService in WorkerRuntime::spawn_blocking_task only from the lazy activation path.
5. Build the scan payload by reusing RealFilesystem, RealRegistry, RealProcessInspector, DiscoveryService, OverviewCollector, and PeliteF4seDllInspector inside the worker. Use OverviewCollector-derived Fallout4.exe facts to select OG, NG, AE, or unknown for Your Game per D025.
6. Project domain rows into F4seUiRow using ModelRc<VecModel<_>> only on the Slint event loop. Worker closures must return owned Rust snapshots only.
7. Add tracing around f4se scan requested, started, discovery failure, missing folder, dll count, per-DLL failures, stale worker ignored, and completed states.
8. Add source-contract tests named s06_f4se_slint_contract in src/main.rs proving the placeholder text is gone, the reference columns and legend are present, no manual refresh button is present, MainWindow exposes and forwards F4SE properties/callbacks, and the shell tab order remains unchanged.
9. Add runtime wiring tests named s06_f4se_runtime_wiring proving projection from controller state to Slint row structs, first-activation scheduling, spawn-failure mapping, worker completion application, and unrelated worker event ignore behavior.

Failure Modes Q5: no Tokio runtime, worker spawn failure, discovery failure, unknown current game, malformed DLL rows, and Slint event-loop handoff failure must all surface as safe status/error text or tracing rather than panics.

Load Profile Q6: slow discovery, OverviewCollector reuse, directory enumeration, and DLL parsing all run off the UI thread; large row models should be rebuilt once per scan result, not per file on the UI thread.

Negative Tests Q7: source-contract absence of manual refresh, activation callback only for F4SE tab, empty rows, error status, unknown-game warning row, and unrelated worker events.

Done when: cargo check compiles the real Slint entrypoint and focused tests prove lazy tab activation, worker handoff, projection, and source contracts.

## Inputs

- `ui/f4se_tab.slint`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `src/main.rs`
- `src/app/f4se_controller.rs`
- `src/services/f4se.rs`
- `src/services/discovery.rs`
- `src/services/overview_collector.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`

## Expected Output

- `ui/f4se_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Verification

cargo test s06_f4se_slint_contract
cargo test s06_f4se_runtime_wiring
cargo check

## Observability Impact

Adds UI-visible F4SE scan status and error surfaces plus tracing and projection tests covering the full controller to worker to Slint handoff.
