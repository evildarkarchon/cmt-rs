---
estimated_steps: 1
estimated_files: 9
skills_used: []
---

# T06: Complete runtime wiring, live feedback, About action, and tests

Redo the runtime integration closeout work after executor hardening. Implement a real Downgrader modal About action that shows the preserved `ABOUT_DOWNGRADING_TITLE` and `ABOUT_DOWNGRADING` copy instead of logging a deferred no-op. Bind confirmed runs to the exact reviewed plan or a stable digest; if files/backups materially change after preview, abort with a safe message and require a new preview. Emit progress/log events live from the worker while downloads/apply work are running rather than buffering everything until `execute_confirmed` returns. Refresh Overview after completion with the current persisted settings snapshot or a Send-safe settings access pattern, not `AppSettings::default()`. Add substantive runtime wiring tests under the required `s09_downgrader_runtime_wiring` filter for Overview and Tools open, settings-save failure/revert, worker spawn failure, stale completion, close blocked while running, live progress/log projection, About action, post-run Overview refresh settings, Archive Patcher still deferred, and confirmed-plan mismatch fail-closed behavior.

## Inputs

- `S09 closeout reviewer and security findings`
- `Existing T04 controller state machine`
- `Existing T06 runtime wiring implementation and tests`
- `CMT/src/globals.py about copy and CMT/src/modal_window.py close behavior`

## Expected Output

- `Runtime Downgrader About action displays preserved reference about copy`
- `Run requests are bound to reviewed plan/digest and fail closed on mismatch`
- `Live worker progress/log delivery reaches the modal while work is active`
- `Overview completion refresh uses current settings`
- `At least one substantive test matching `s09_downgrader_runtime_wiring` plus full regression gates`

## Verification

cargo test s09_downgrader_runtime_wiring
cargo test downgrader_controller
cargo test downgrader_worker_payload
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
