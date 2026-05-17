---
estimated_steps: 22
estimated_files: 8
skills_used: []
---

# T05: Wire Overview controller and workers

---
estimated_steps: 10
estimated_files: 7
skills_used:
  - rust-async-patterns
  - tdd
  - verify-before-complete
---
Why: The Overview tab must refresh automatically and on demand without blocking Slint, while settings and desktop actions flow through existing app and worker seams.

Do:
1. Add `src/app/overview_controller.rs` with a testable controller/state reducer for initial loading, refresh requested, refresh completed, refresh failed, update-check completed, desktop action completed, and last safe action error.
2. Expose a read-only current settings snapshot or current update source from `SettingsController` so Overview uses the persisted S02 setting without duplicating settings parsing.
3. Extend worker events with an Overview-specific payload or typed wrapper that can carry owned `OverviewSnapshot` and safe failures across `WorkerEventSink` handoff; preserve existing S03 worker tests.
4. Compose `DiscoveryService`, `Overview` collector/diagnostics, update service, and `DesktopActions` behind injected traits or small adapters so controller tests use fakes and production uses real adapters.
5. In `src/main.rs`, create or enter a Tokio runtime suitable for `WorkerRuntime::spawn_blocking_task`, bind Overview callbacks, schedule an initial refresh after window creation, and keep Slint state updates on the event loop.
6. Add callbacks for Refresh, open game path, open Nexus update link, and open GitHub update link; failed actions update safe visible state and emit logs.
7. Add controller/worker tests for initial loading, refresh success, refresh failure, update_source none skipping update work, stale result handling if a second refresh completes first, desktop action success/failure, and worker panic/failure mapping.

Done when: the app can produce and apply Overview snapshots through the worker handoff path without blocking the UI thread, and tests cover state transitions with fakes.

Requirement Impact Q4: rerun S02 settings tests and S03 worker tests because this task adds consumers and possibly payload variants.
Failure Modes Q5: discovery failure, collector failure, update failure, desktop failure, worker spawn failure, handoff failure, and panic all map to safe states or worker errors.
Load Profile Q6: only one active refresh should update the UI at a time; stale or cancelled refreshes must not overwrite newer state.
Negative Tests Q7: fake worker failure, fake handoff failure, update_source none, desktop open failure, missing game path, and refresh requested twice.

## Inputs

- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/services/overview_collector.rs`
- `src/services/update.rs`
- `src/app/settings_controller.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
- `src/platform/desktop.rs`
- `src/main.rs`

## Expected Output

- `src/app/overview_controller.rs`
- `src/app/mod.rs`
- `src/app/settings_controller.rs`
- `src/main.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/services/overview.rs`
- `src/services/update.rs`

## Verification

cargo test overview_controller

## Observability Impact

Adds inspectable Overview refresh/action phases, worker event payloads, and safe last-error state for future debugging.
