---
estimated_steps: 14
estimated_files: 10
skills_used: []
---

# T06: Wire runtime entrypoints and closeout checks

---
estimated_steps: 10
estimated_files: 10
skills_used:
  - rust-async-patterns
  - tdd
  - verify-before-complete
---
Why: The slice is only complete when the Overview and Tools entrypoints open the real Downgrader modal, persist workflow options safely, schedule background status/plan/run workers, refresh Overview after completion, and leave Archive Patcher deferred.
Do: Wire `src/main.rs` to own a `DowngraderController`, create/show the generated `DowngraderWindow`, bind modal callbacks, project controller state into Slint properties/models, and route worker events back through the existing event-loop sink pattern. Add `SettingsController::save_downgrader_settings_for_workflow` so `Keep Backups` and `Delete Patches` are persisted at workflow start; on save failure, revert visible options and do not plan or run with unpersisted preferences. Update Tools domain/service/controller behavior so `tools.downgrade_manager` opens the modal instead of returning a deferred rejection, while `tools.archive_patcher` remains disabled/deferred until S10. Update Overview downgrade projection so the button is enabled when the workflow can be opened and still fails closed in the modal if discovery cannot establish a safe game root. Schedule status, plan, and confirmed run work on worker threads using owned request payloads, real discovery and filesystem adapters, the downloader/applier seams from T03, and no Slint handles in closures. On completion, refresh/redraw Downgrader status and request an Overview refresh; keep `Patch\n All` disabled and close/Escape blocked while running. Add runtime wiring tests that use fakes or source-level assertions to prove callback routing, settings-save failure, worker spawn failure, stale completion, Overview refresh request after completion, and Archive Patcher still deferred.
Failure Modes Q5: Discovery unsupported/missing root shows safe modal log/status and disables mutation. Settings save failure reverts options and suppresses workers. Worker spawn failure shows a safe error. Download/apply failures remain per-file log rows. Off-Windows real discovery/platform failures stay safe and fake-backed tests remain cross-platform.
Load Profile Q6: Background workers own fixed-size plan/status payloads and emit bounded log/progress events; no UI-thread blocking is allowed for CRC, filesystem, network, or xdelta work.
Negative Tests Q7: Cover Overview open, Tools open, unknown Tools action, Archive Patcher deferred, missing game path, settings-save failure, close blocked while running, run completion refreshing status, and no Slint handle/model captured by worker closures.
Done when: Targeted runtime tests pass, full closeout checks pass, and the modal is live from both entrypoints without regressing settings, overview, tools, worker, scanner, or auto-fix behavior.

## Inputs

- `src/domain/downgrader.rs`
- `src/services/downgrader.rs`
- `src/app/downgrader_controller.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/app/settings_controller.rs`
- `src/domain/tools.rs`
- `src/services/tools.rs`
- `src/app/tools_controller.rs`
- `src/main.rs`
- `ui/downgrader_window.slint`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`

## Expected Output

- `src/main.rs`
- `src/app/settings_controller.rs`
- `src/domain/tools.rs`
- `src/services/tools.rs`
- `src/app/tools_controller.rs`
- `src/services/downgrader.rs`
- `src/workers/events.rs`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`

## Verification

cargo test s09_downgrader_runtime_wiring
cargo test settings
cargo test overview
cargo test tools
cargo test worker
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Observability Impact

Completes end-to-end diagnostics: tracing spans identify open/status/plan/run/settings/worker phases; modal log exposes reference-style per-file messages; Overview refresh after completion gives a second visible state surface.
