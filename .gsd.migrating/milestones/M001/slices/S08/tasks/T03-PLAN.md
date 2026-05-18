---
estimated_steps: 17
estimated_files: 3
skills_used: []
---

# T03: Wire Auto Fix lifecycle through Scanner controller and workers

Expected executor skills_used: rust-async-patterns, tdd, observability, verify-before-complete.

Why: Auto-Fix is stateful and potentially write-capable, so the Scanner controller must own render state and stale-event rejection while workers carry only owned payloads. Slint handles, filesystem adapters, and operation closures must not enter the controller.

Do:
1. Extend ScannerController with an empty-default Auto-Fix support catalog and a fake-catalog constructor for tests. Existing ScannerController::new must preserve production behavior with no visible Auto-Fix controls.
2. Track per-result Auto-Fix state keyed by scan id, result index, and result identity. Selecting a result should expose visible/button/result-detail state only when the typed solution is supported by the injected catalog.
3. Add a request_selected_auto_fix transition that rejects no selection, unsupported result, stale result, or missing support with safe feedback; otherwise it sets the selected result to Fixing..., disables the button, and returns an owned worker request containing scan id, result index, result identity, operation key, and a WorkerTask using an s08-scanner-autofix prefix and WorkerTaskKind::Patch.
4. Add Auto-Fix completion handling that applies Fixed! or Fix Failed, stores inline Auto-Fix Results details, sets row fixed/check state on success, re-enables the button after completion, and ignores stale completions.
5. Extend ScannerWorkerPayload in src/workers/events.rs for Auto-Fix completed payloads, plus constructors/accessors/tests. Export any new worker request/payload helpers through src/workers/mod.rs if needed.
6. Add failure-event handling helpers or controller reducer coverage so worker failures for Auto-Fix tasks become Fix Failed feedback without leaking raw diagnostics in UI text.

Failure Modes Q5:
| Dependency | On error | On timeout | On malformed response |
| --- | --- | --- | --- |
| Worker payload | Apply Fix Failed for matching current result; ignore stale events | Treat timeout as future worker failure input; controller remains in Fixing until a failure/completion is delivered | Ignore payloads whose task prefix, scan id, result index, or identity do not match |
| Auto-Fix support catalog | Hide button and reject callback safely | Not applicable | Unsupported or duplicate operation keys are rejected or deduplicated deterministically |

Load Profile Q6: state is a small map per current scan result list; selecting rows and completing one worker must not clone Slint models or perform filesystem IO.

Negative Tests Q7: unsupported selected result has no button/request; tampered callback while no button is visible fails closed; selection changes and newer scans make old completions stale; fake success shows Fixed! and details; fake failure shows Fix Failed and details; worker failure safe message excludes diagnostics.

Done when controller and worker-payload tests prove lifecycle labels, stale rejection, row fixed state, and owned worker payload handling without a GUI.

## Inputs

- `src/domain/autofix.rs`
- `src/domain/scanner.rs`
- `src/services/autofix.rs`
- `src/app/scanner_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Expected Output

- `src/app/scanner_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Verification

cargo test scanner_controller_autofix
cargo test scanner_worker_payload_autofix

## Observability Impact

Controller transitions should log requested, rejected, fixing, completed, failed, and stale-ignored Auto-Fix events with scan id/result index/operation key and safe messages.
