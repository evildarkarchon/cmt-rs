# S08 Research: Scanner Auto Fix Actions

## Summary

S08 should add Auto-Fix plumbing, state, tests, and UI gates without enabling any production file mutations. The checked-in Python reference has an Auto-Fix lifecycle implementation, but `CMT/src/autofixes.py::AUTO_FIXES` is an empty dictionary. In the reference Scanner details pane, the Auto-Fix button is created only when `problem_info.solution in AUTO_FIXES`, so the current production behavior is that normal users see no Auto-Fix button.

The Rust S07 Scanner is a read-only implementation with solid seams: pure scanner domain data, `ScannerController` reducer, owned worker events, Slint projection, and runtime action workers. The main S08 gap is that Rust scanner results currently store `solution: Option<String>` rather than the typed `ScannerSolutionKind` identity that the reference uses as the Auto-Fix registry key. Auto-Fix eligibility should be keyed by typed solution metadata, not string matching in the UI.

The existing S07 code also contains deferred Auto-Fix domain markers (`ScannerActionKind::AutoFixDeferred`, `with_deferred_auto_fix`, and `AUTO_FIX_DEFERRED_STATUS`) while the Slint contract test explicitly prohibits `Auto-Fix`, `Fixed!`, `Fix Failed`, and `auto-fix` strings in the UI. S08 should replace this deferred/read-only-action placeholder with a real hidden-by-default Auto-Fix state model: production catalog empty, fake/test catalog non-empty.

## Recommendation

Implement S08 as a non-mutating production slice with test-only fake operations:

1. Add a pure domain Auto-Fix contract (`src/domain/autofix.rs`) with reference labels (`Auto-Fix`, `Fixing...`, `Fixed!`, `Fix Failed`, `Auto-Fix Results`), button/status state, result-details payloads, operation metadata, plan/confirmation fields, and fail-closed rejection kinds.
2. Extend `ScannerResult` to retain typed solution identity (`Option<ScannerSolutionKind>`) alongside the display string. Add typed constructors/helpers and update scanner service call sites that currently call `ScannerSolutionKind::* .into_solution_text()` before storing results.
3. Add `src/services/autofix.rs` with an empty production registry and injectable fake/test operations. The service should validate scan/result identity, operation support, target/precondition requirements, confirmation, and operation outcome before returning an owned result payload.
4. Extend `ScannerController` with result-indexed Auto-Fix state and a request method that starts with `Fixing...` only for eligible selected results. Unsupported/tampered callbacks should fail closed with safe feedback and must not call the operation.
5. Extend `workers::ScannerWorkerPayload` with Auto-Fix completion payloads and use the existing owned worker handoff pattern. Use a distinct task prefix such as `s08-scanner-autofix:` and a write-oriented task kind (`WorkerTaskKind::Patch`) rather than routing through the S07 desktop/clipboard action executor.
6. Add Slint properties/callbacks for a conditional Auto-Fix button and inline `Auto-Fix Results` details section. Keep the production empty registry path hidden by default.

Do not add real delete, rename, archive, move, patch, backup, restore, or repair operations in this slice. If future slices add real operations, add a separate file-mutation adapter/trait instead of expanding the current read-only `Filesystem` trait prematurely.

## Requirements and Scope Notes

No root `REQUIREMENTS.md` entries were preloaded for this slice.

S08 owns/supports these slice-scoped requirements:

- Current production registry remains empty and production users see no Auto-Fix buttons.
- Auto-Fix visibility is gated by a typed registered operation, not disabled placeholders.
- Lifecycle labels match the reference: `Auto-Fix`, `Fixing...`, `Fixed!`, `Fix Failed`, `Auto-Fix Results`.
- Tampered/stale/unsupported requests fail closed and do not mutate files.
- Future real-operation safety contract is explicit: plan preview, confirmation required, and pre-mutation revalidation.
- Worker execution is off the Slint UI thread and returns owned payloads.
- Inline details replace the reference modal while retaining the `Auto-Fix Results` heading/copy.

## Skills Discovered

- Applied the required `observability` skill guidance from the installed skill list: S08 should have explicit failure modes, structured tracing around unattended worker actions, and separate safe user messages from diagnostics.
- Installed skill already available and relevant: `rust-async-patterns` for Tokio/worker handoff patterns. No extra activation was needed for local Rust async patterns because S07 already established the worker model.
- Skill search for `Slint` returned unrelated accessibility/linting results first and no high-confidence installed Slint skill.
- Skill search for `Rust Slint` found candidates including `bobmatnyc/claude-mpm-skills@rust-desktop-applications` (259 installs) and `bahayonghang/my-claude-code-settings@lib-slint-expert` (31 installs). Install attempts failed because the published repositories did not expose matching skill names at install time, so no new skills were installed.

## Reference Behavior

### `CMT/src/autofixes.py`

- Defines `AutoFixResult(success: bool, details: str)`.
- Defines `AUTO_FIXES: dict[SolutionType, Callable[..., AutoFixResult]] = {}`. This is the critical parity fact: production reference has no registered fixes.
- `do_autofix` only runs a fix when `problem_info.autofix_result is None`; repeated clicks show the existing result.
- Before work: button text becomes `Fixing...` and is disabled.
- Success: button text becomes `Fixed!`, style resets, button re-enabled, and the selected tree row gets a check image.
- Failure: button text becomes `Fix Failed`, style resets, button re-enabled.
- Results are shown in an `AboutWindow` titled `Auto-Fix Results`; S08 should present the same heading inline in the embedded details area.

### `CMT/src/tabs/_scanner.py`

- `ResultDetailsPane.set_info` destroys/recreates buttons on selection change.
- Auto-Fix button is only created under `if self.problem_info.solution in AUTO_FIXES`.
- Button label is derived from stored per-problem `autofix_result`: `Auto-Fix` before execution, `Fixed!` after success, `Fix Failed` after failure.
- Command passes the selected tree id into `do_autofix`, which updates that selected row on success.

### `CMT/src/enums.py` and `CMT/src/helpers.py`

- `SolutionType` is a `StrEnum`; reference scanner often stores the enum member on `ProblemInfo.solution`, not just a string.
- `ProblemInfo` and `SimpleProblemInfo` both carry `autofix_result: AutoFixResult | None`.

## Current Rust Implementation Landscape

### Domain: `src/domain/scanner.rs`

- Provides Scanner constants, categories, problem types, `ScannerSolutionKind`, `ScannerResult`, `ScannerActionDescriptor`, `ScannerActionFeedback`, and grouping/snapshot helpers.
- `ScannerResult` currently stores `solution: Option<String>` and `auto_fix_deferred: bool`. It does not retain the typed solution key needed to match the reference `AUTO_FIXES` behavior.
- `ScannerActionKind` currently includes `AutoFixDeferred`, but the S07 action model is otherwise read-only (`CopyDetails`, `OpenLocation`, URL copy/open, `ShowFileList`). Auto-Fix should not be implemented as another read-only action because it has different lifecycle and future mutation semantics.
- `ScannerResult::read_only_actions()` can append a disabled deferred Auto-Fix descriptor when `auto_fix_deferred` is set. S08 should replace this with a separate Auto-Fix eligibility/state model and keep read-only actions read-only.

### Service: `src/services/scanner.rs`

- Produces `ScannerResult` values from filesystem/discovery facts. It currently converts typed solution kinds to strings at call sites via `ScannerSolutionKind::* .into_solution_text()`.
- Key helper: `path_result(..., solution: Option<String>) -> ScannerResult`. This is the best seam to update first so scanner-created results keep typed solution identity.
- Overview-problem mapping in `domain/scanner.rs` may only have string solutions; those should remain unsupported for Auto-Fix unless a future explicit mapper can prove a typed solution key.

### Controller: `src/app/scanner_controller.rs`

- Owns scanner settings, scan lifecycle, results, selected details, file-list visibility, and read-only action feedback.
- Uses monotonic scan ids and rejects stale scan/progress/action events.
- `request_selected_action` validates action ids and selected action availability, then writes safe failure feedback for invalid/tampered action ids.
- Good seam: add `request_auto_fix(...)` / `auto_fix_started(...)` / `auto_fix_completed(...)` methods that mirror scan/action stale handling but update per-result Auto-Fix state.

### Workers: `src/workers/events.rs` and `src/workers/mod.rs`

- `WorkerPayload::Scanner(ScannerWorkerPayload)` already carries scanner-owned payloads across the UI handoff boundary.
- Current `ScannerWorkerPayload` variants are `ScanCompleted` and read-only `ActionCompleted`.
- Add a dedicated Auto-Fix completion/result variant rather than overloading `ScannerActionFeedback`, because Auto-Fix needs result index, button state, details heading/text, and row fixed state.

### Runtime/UI bridge: `src/main.rs`

- `bind_scanner_callbacks` wires S07 callbacks for toggles, scan, result selection, copy details, file list, open path, open URL, and copy URL.
- `request_scanner_action` schedules read-only desktop/clipboard work with task prefix `s07-scanner-action:` and `WorkerTaskKind::DesktopAction`.
- `project_scanner_state` and `apply_scanner_projection` are the central seams for adding UI properties.
- S07 source-contract tests currently assert that `Auto-Fix`, `Fixed!`, `Fix Failed`, and `auto-fix` are absent from `ui/scanner_tab.slint` and `ui/main.slint`. S08 must update these tests to require hidden-by-default gated properties/callbacks instead.

### Slint: `ui/scanner_tab.slint` and `ui/main.slint`

- `ScannerTab` has explicit properties for each action button (`scanner-open-path-enabled`, `scanner-open-url-enabled`, `scanner-copy-url-enabled`, `scanner-file-list-enabled`) and callback forwarding through `MainWindow`.
- Add explicit Auto-Fix properties rather than generic dynamic action lists, to match the existing style and keep contract tests simple:
  - `scanner-auto-fix-visible: bool`
  - `scanner-auto-fix-enabled: bool`
  - `scanner-auto-fix-label: string`
  - `scanner-auto-fix-results-visible: bool`
  - `scanner-auto-fix-results-heading: string`
  - `scanner-auto-fix-results-details: string`
  - callback `auto-fix-requested()` / `scanner-auto-fix-requested()`
- Add a result-row field such as `auto_fix_state: string` or `auto_fix_fixed: bool` so fake tests can prove the reference checkmark-equivalent row state without depending on image assets.

## Natural Seams and Suggested Work Units

### T01: Domain Auto-Fix contract and typed solution keys

Files:
- `src/domain/autofix.rs` (new)
- `src/domain/scanner.rs`
- `src/domain/mod.rs`

Purpose:
- Freeze reference labels and Auto-Fix state names.
- Add typed solution identity to `ScannerResult`.
- Remove or supersede `AutoFixDeferred` as a read-only action concept.
- Define result fingerprint/identity used to reject stale or tampered worker completions.

Suggested types:
- `AutoFixButtonState::{Hidden, Ready, Fixing, Fixed, Failed}` with label helper.
- `AutoFixResultDetails { heading, details }` where heading defaults to `Auto-Fix Results`.
- `AutoFixOperationMetadata { id, solution_kind, requires_target, requires_confirmation }`.
- `AutoFixPlan { operation_id, summary, steps, requires_confirmation }`.
- `AutoFixResultStatus` / `AutoFixOutcome` with success/failure and safe diagnostics split.
- `ScannerResultFingerprint` or equivalent stable identity derived from scan id + result index + problem type + detail path + absolute path + solution key.

First tests:
- Reference labels exactly match Python labels.
- Empty/default catalog makes every result unsupported/hidden.
- Fake catalog makes only matching typed solution results eligible.
- Custom/string-only solutions remain unsupported.

### T02: Auto-Fix service registry and fail-closed executor

Files:
- `src/services/autofix.rs` (new)
- `src/services/mod.rs`

Purpose:
- Production registry is explicitly empty.
- Fake operations are injectable in tests.
- Service validates support, selected result, target/preconditions, confirmation, and fingerprint before invoking an operation.

Recommended contract:
- `AutoFixRegistry::production()` / `production_auto_fix_registry()` returns empty.
- Registry exposes a pure catalog for the controller/UI, but operations remain in the service layer.
- `AutoFixOperation` fake/test trait should separate `plan`, `revalidate`, and `execute` so future real operations cannot mutate based only on scan-time facts.
- For now, fake operations can return success/failure details; production has no operations.

Fail-closed test cases:
- Production registry supports no `ScannerSolutionKind`.
- Unknown result index does not call fake operation.
- Unsupported solution does not call fake operation.
- Missing target does not call fake operation when operation requires a path.
- Missing confirmation returns a confirmation-required/precondition failure and does not execute.
- Revalidation failure returns `Fix Failed`-class result and does not mutate.
- Operation failure returns safe `Fix Failed` feedback and diagnostic detail only in test/log surfaces.

### T03: ScannerController Auto-Fix state reducer

Files:
- `src/app/scanner_controller.rs`

Purpose:
- Tie Auto-Fix state to result index, like the reference stores `autofix_result` on each problem.
- Keep controller Slint-free and platform-free.
- Produce owned worker requests and reduce owned worker outcomes.

Suggested behavior:
- Constructor accepts an `AutoFixCatalog` or uses default empty catalog.
- Selected detail projection includes `auto_fix_visible`, `auto_fix_enabled`, `auto_fix_label`, and current result-details text.
- `request_auto_fix()`:
  - If no selection/result/catalog support: set safe feedback and return no worker.
  - If result already has completed Auto-Fix details: surface inline `Auto-Fix Results` again and return no worker.
  - If eligible: set that result to `Fixing...`, disable button, clear/prepare feedback, and return `ScannerAutoFixWorkerRequest`.
- `auto_fix_completed()`:
  - Ignore stale scan ids/fingerprints.
  - Update the matching result's state to `Fixed!` or `Fix Failed`.
  - Show inline `Auto-Fix Results` details when the selected result matches.
  - Preserve that this does not imply a rescan or removal from scanner results.

Tests:
- Unsupported selected result hides button and tampered request fails closed.
- Fake eligible result transitions `Auto-Fix` -> `Fixing...` -> `Fixed!`.
- Fake failure transitions `Auto-Fix` -> `Fixing...` -> `Fix Failed`.
- Completed result click re-shows stored details without scheduling another operation.
- Stale worker result is ignored.
- Row/checkmark projection state updates only for the completed result.

### T04: Worker payloads and runtime wiring

Files:
- `src/workers/events.rs`
- `src/workers/mod.rs` if helper tests need updates
- `src/main.rs`

Purpose:
- Schedule Auto-Fix work off the UI thread and marshal owned results back through `SlintEventLoopSink`.
- Keep real production registry empty, so callbacks normally do not schedule workers.

Suggested runtime functions:
- `request_scanner_auto_fix(...)`
- `prepare_scanner_auto_fix_execution(...)`
- `scanner_auto_fix_task(scan_id, result_index, operation_id)` with prefix `s08-scanner-autofix:`
- `execute_scanner_auto_fix_payload(...)` using production empty registry in normal runtime and fake registry in tests.
- `scanner_auto_fix_failure_feedback_from_event(...)` or an equivalent mapping for worker spawn/failure events.

Use `WorkerTaskKind::Patch` for Auto-Fix because future implementations may mutate files. Do not route through the S07 `DesktopAction` executor.

### T05: Slint UI and projection

Files:
- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`

Purpose:
- Present Auto-Fix only when the controller projection says it is visible.
- Preserve reference labels and inline results heading.

UI shape:
- In the existing Details button row, add:
  - `if root.scanner-auto-fix-visible: Button { text: root.scanner-auto-fix-label; enabled: root.scanner-auto-fix-enabled; clicked => root.auto-fix-requested(); }`
- In the Details group below feedback/file-list text, add an inline Auto-Fix results section:
  - heading text `Auto-Fix Results`
  - details text from controller/service
- In `ScannerResultRow`, show a small `✓` or equivalent when the row's Auto-Fix state is fixed. This is the Slint analogue of the reference tree image.

Source-contract updates:
- Replace S07 “must not contain Auto-Fix markers” assertions with:
  - Auto-Fix properties default to hidden/false.
  - Button is conditional on `scanner-auto-fix-visible`.
  - Callback forwards through `MainWindow`.
  - Production empty registry projection hides the button.

## First Proof

The biggest unblocker is typed eligibility. Start by proving this before any UI work:

1. Add typed solution identity to `ScannerResult` and update `path_result` / scanner service call sites to preserve `ScannerSolutionKind`.
2. Add an empty production Auto-Fix catalog/registry.
3. Add tests showing:
   - All current production scanner results are ineligible with the production catalog.
   - A fake catalog registered for one `ScannerSolutionKind` makes only matching results eligible.
   - String/custom Overview-derived solutions remain ineligible.

Once this passes, the controller/UI work can be built without guessing how eligibility is computed.

## Observability and Failure-State Requirements

Apply agent-first observability to every worker path. Suggested structured events:

- `s08-scanner-autofix-requested` with `scan_id`, `result_index`, `operation_id`, `solution_kind`, `has_target`.
- `s08-scanner-autofix-unavailable` for unsupported/no-selection/unknown-result cases.
- `s08-scanner-autofix-plan-created` and `s08-scanner-autofix-confirmation-required` for future real operations.
- `s08-scanner-autofix-scheduled` / `started` with task id.
- `s08-scanner-autofix-precondition-failed` before any mutation.
- `s08-scanner-autofix-completed` with success/failure and safe message.
- `s08-scanner-autofix-stale-ignored` for stale scan/result events.
- `s08-scanner-autofix-handoff-failed` if event-loop handoff fails.

Keep UI messages safe and concise. Raw OS or fake-operation diagnostics should be retained in diagnostic fields/tracing/tests, not primary Slint text.

## Risks and Constraints

- **Eligibility drift:** Do not match Auto-Fix by display string; reference uses `SolutionType` enum identity. Preserve typed `ScannerSolutionKind` in Rust results.
- **Production mutation risk:** The registry must be empty by default. Tests should prove production registry has no supported operations.
- **Read-only action confusion:** Existing S07 `ScannerActionKind` is mostly read-only. Auto-Fix should have separate state and worker payloads because it is future write-capable.
- **Stale scan/result races:** Existing scan-id stale rejection is a good base, but Auto-Fix also needs result-index/fingerprint validation because it mutates per-result state.
- **Confirmation contract:** Even though production has no real operations, the contract should force future real operations through plan preview + explicit confirmation + immediate precondition revalidation.
- **Filesystem mutation seam:** `src/platform/filesystem.rs` is currently read-only. Do not add real write methods until a future slice defines exact operations and sandbox fixtures.
- **UI parity:** Reference uses a modal `AboutWindow`; S08 intentionally deviates by showing `Auto-Fix Results` inline per slice scope. Preserve heading/copy and document this in completion notes.

## Verification Plan

Targeted tests to add/update:

- `cargo test autofix_domain`
- `cargo test autofix_service`
- `cargo test scanner_controller_auto_fix`
- `cargo test scanner_worker_payload_auto_fix`
- `cargo test s08_scanner_slint_contract`
- `cargo test s08_scanner_runtime_wiring`

Existing tests likely affected:

- `cargo test scanner_domain` — because `ScannerResult` fields/constructors and deferred Auto-Fix tests need updates.
- `cargo test scanner_scan_service` — because solution call sites should preserve typed keys.
- `cargo test scanner_controller` — because selection/detail/action state expands.
- `cargo test s07_scanner_slint_contract` or renamed S08 contract tests — because S07 prohibited Auto-Fix source strings.
- `cargo test s07_scanner_runtime_wiring` — because projection structs gain Auto-Fix fields.

Full closeout gates remain:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`

## Sources

Local source inspected:

- `CMT/src/autofixes.py` — reference Auto-Fix result type, empty `AUTO_FIXES`, lifecycle labels, and `Auto-Fix Results` modal behavior.
- `CMT/src/tabs/_scanner.py` — reference details pane button gating and labels.
- `CMT/src/enums.py` — `SolutionType` labels and enum identity.
- `CMT/src/helpers.py` — per-problem `autofix_result` storage.
- `src/domain/scanner.rs` — current scanner domain, solution/action/result model, deferred Auto-Fix placeholder.
- `src/services/scanner.rs` — scanner service result construction and solution-kind-to-string conversion points.
- `src/app/scanner_controller.rs` — reducer, stale-event handling, selected detail/action state.
- `src/workers/events.rs` and `src/workers/mod.rs` — owned worker payload/handoff patterns.
- `src/main.rs` — scanner callback binding, action scheduling, projection, source-contract/runtime tests.
- `ui/scanner_tab.slint` and `ui/main.slint` — Scanner UI properties/callback forwarding and details action surface.
- `src/platform/filesystem.rs` and `src/platform/mod.rs` — read-only filesystem contract and platform operation taxonomy.

No external library docs were needed; this slice is primarily local architecture and reference-parity work.
