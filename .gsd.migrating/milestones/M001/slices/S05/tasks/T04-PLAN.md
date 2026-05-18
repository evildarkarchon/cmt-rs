---
estimated_steps: 22
estimated_files: 5
skills_used: []
---

# T04: Wire Tools and About callbacks through workers in the real app entrypoint

---
estimated_steps: 10
estimated_files: 5
skills_used:
  - verify-before-complete
  - rust-async-patterns
  - tdd
---
Why: The slice is only useful if the real `MainWindow` composes the new reducers, adapters, worker handoff, and Slint properties. This task closes the runtime loop while keeping potentially slow or failing external actions off the Slint event thread.

Do:
1. In `src/main.rs`, instantiate shared Tools and About controllers beside the existing settings/overview controllers and apply their initial state to the new Slint properties before `run()`.
2. Bind Tools callbacks: parse the stable action id, update immediate fail-closed errors for unknown/deferred/internal ids, and schedule enabled external URL opens through `WorkerRuntime::spawn_blocking_task` using `RealDesktopActions` and a Tools-specific `SlintEventLoopSink` handler.
3. Bind About callbacks: schedule open-link actions through `RealDesktopActions`, schedule copy actions through the production clipboard adapter, and route Timer reset callbacks back to the About controller.
4. Add Tools/About worker sink handlers that ignore unrelated payloads, update the correct controller on completion/failure, apply render state back to Slint properties, and log handoff failures safely.
5. Ensure worker spawn failure paths update visible safe error text immediately and emit structured tracing events. Do not block the UI thread and do not perform desktop/clipboard calls directly inside Slint callback closures.
6. Add focused tests whose names include `s05_runtime_wiring` for projection helpers, spawn-failure/error mapping helpers, callback id mapping, and that Tools/About worker payloads are accepted while unrelated payloads are ignored. These tests should not require a real Slint window, browser, or clipboard.

Done when: clicking any enabled Tools/About open/copy control in the real app goes through the worker/action boundary, failures are visible, copy-label reset is wired, and deferred utility controls remain non-executable.

Threat Surface (Q3): callback ids are untrusted at the Rust boundary; only known static ids can produce actions. Public URLs only; no secrets/PII. Clipboard copies only public reference URLs/invites from domain constants.
Requirement Impact (Q4): re-verify S04 Overview refresh/action wiring remains intact because `src/main.rs` composition changes; settings callbacks must still compile and pass source tests.
Failure Modes (Q5): worker spawn failure -> safe visible error; action adapter failure -> safe visible error; handoff failure -> structured log; poisoned controller lock -> log and no panic; stale/foreign worker payload -> ignored.
Load Profile (Q6): one blocking worker per click. At 10x rapid clicks, Tokio blocking pool and OS shell/clipboard become the limiting shared resources; copy buttons should reduce repeated copy pressure by disabling during `Copied!`.
Negative Tests (Q7): unknown ids, disabled utilities, spawn failure helper, adapter failure payloads, unrelated worker payloads, copy reset for each About row, and non-Windows unsupported desktop path.

## Inputs

- `src/app/tools_controller.rs`
- `src/app/about_controller.rs`
- `src/services/tools.rs`
- `src/platform/clipboard.rs`
- `src/platform/desktop.rs`
- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Expected Output

- `src/main.rs`
- `src/app/tools_controller.rs`
- `src/app/about_controller.rs`
- `src/services/tools.rs`
- `src/workers/events.rs`

## Verification

cargo test s05_runtime_wiring

## Observability Impact

Adds runtime tracing around Tools/About action scheduling, spawn failure, adapter success/failure, worker-event delivery, ignored payloads, copy success, copy reset, and lock poisoning. Users get inline safe error banners instead of silent failures.
