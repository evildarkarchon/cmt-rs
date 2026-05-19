---
estimated_steps: 25
estimated_files: 9
skills_used: []
---

# T05: Wire entrypoints workers and Overview refresh

---
estimated_steps: 10
estimated_files: 9
skills_used:
  - rust-async-patterns
  - tdd
  - security-review
  - verify-before-complete
---
Why: The slice is only complete when the modal is reachable from both existing UI entrypoints, consumes current Overview archive records, runs patch/restore workers off-thread, and refreshes Overview after completion.

Do:
1. Extend the Overview snapshot/controller path to retain the current discovery-collected archive records needed by the Archive Patcher while keeping existing archive summary rows unchanged.
2. Update Overview projection and tests so archive records are carried from `Fallout4Installation.archives` or collected facts into the current controller snapshot without adding UI-side scans.
3. Enable Overview `Archive Patcher...` and Tools `Archive Patcher` controls; remove the deferred S10 status text and route `ToolActionId::ArchivePatcher` as a live modal entrypoint while preserving other Tools actions.
4. In `src/main.rs`, instantiate `ArchivePatcherController` and `ArchivePatcherWindow`, bind worker sink and callbacks, project controller state to Slint row models/properties, and open from Overview or Tools.
5. On modal open, build candidates from the current Overview archive records. If discovery/archive data is missing, show a safe empty/error modal with write controls disabled and a refresh/fix discovery message.
6. Schedule planning, confirmed patch, and restore work through `WorkerRuntime::spawn_blocking_task`; pass owned request payloads and never mutate Slint handles from worker closures.
7. Save and read the latest manifest from an app-owned path outside the game `Data` directory, using the manifest model from T01/T02.
8. After patch or restore completion, trigger the existing Overview refresh path using the current shared settings snapshot so archive counts and candidates update.
9. Add runtime/source tests proving Overview and Tools open the modal, candidate source uses Overview archive records, write controls disable while running, stale events are ignored, Overview refresh is scheduled after completion, and deferred status text is gone.
10. Keep Archive Patcher open during patch/restore and block close/Escape while controller state is running.

Done when: runtime wiring tests prove both entrypoints open the same workflow, workers and controller exchange typed payloads, Overview records are the only candidate source, and Overview refresh occurs after completion.

Failure Modes Q5: Missing Overview state opens a disabled modal; worker spawn failure becomes modal status/log text; manifest path/write failures abort before mutation; Overview refresh failure remains visible through existing Overview safe-action/error surfaces.
Load Profile Q6: Runtime keeps one modal/controller instance and uses background workers for filesystem mutation; UI updates are row-model projections on event-loop handoff.
Negative Tests Q7: Tools action unknown id, Overview open before first refresh, no archive records, stale run-completed payload, worker spawn failure, plan mismatch after filter change, and completed patch triggering refresh exactly once.

## Inputs

- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/app/archive_patcher_controller.rs`
- `ui/archive_patcher_window.slint`
- `src/main.rs`
- `src/domain/tools.rs`
- `src/services/tools.rs`
- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/services/overview_collector.rs`
- `src/app/overview_controller.rs`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
- `ui/main.slint`

## Expected Output

- `src/main.rs`
- `src/domain/tools.rs`
- `src/services/tools.rs`
- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/app/overview_controller.rs`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
- `ui/main.slint`

## Verification

cargo test s10_archive_patcher_runtime_wiring --quiet
cargo test overview --quiet
cargo test tools --quiet
cargo test worker --quiet

## Observability Impact

Completes runtime observability by tracing open source, request id, worker stage, spawn failures, stale events, and post-run Overview refresh scheduling.
