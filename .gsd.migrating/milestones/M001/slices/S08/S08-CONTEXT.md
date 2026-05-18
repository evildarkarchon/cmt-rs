---
id: S08
milestone: M001
status: ready
---

# S08: Scanner Auto Fix Actions — Context

<!-- Slice-scoped context. Milestone-only sections (acceptance criteria, completion class,
     milestone sequence) do not belong here — those live in the milestone context. -->

## Goal

Add the Scanner Auto-Fix eligibility, execution-state, result-feedback, and fail-closed extension path while preserving that the current checked-in reference registers no real Auto-Fix operations.

## Why this Slice

S08 follows S07 because Scanner results, typed solutions, selected-result details, read-only action feedback, and worker handoff must exist before Auto-Fix can be safely attached; it also keeps destructive scanner actions isolated before the later Downgrade Manager and Archive Patcher workflows.

## Scope

### In Scope

- Preserve the checked-in reference behavior that `AUTO_FIXES` is currently empty: production S08 should not invent or enable destructive fixes that are not registered in the reference source.
- Add typed Auto-Fix plumbing/registry semantics so Scanner details can show `Auto-Fix` only when a result's typed solution has a registered, tested operation.
- Treat the production registry as empty for current parity, so normal users see no Auto-Fix buttons until a real supported operation is explicitly added later.
- Keep Auto-Fix hidden for unsupported result types, matching `_scanner.py` behavior (`if problem_info.solution in AUTO_FIXES`) rather than showing disabled placeholders for every result.
- Preserve reference labels and state names for the Auto-Fix lifecycle when operations exist or in fake-backed tests: `Auto-Fix`, `Fixing...`, `Fixed!`, `Fix Failed`, and `Auto-Fix Results`.
- Add fail-closed behavior for tampered/stale callbacks: an Auto-Fix request for an unsupported result, unknown result id, stale scan result, missing target, or failed precondition must not mutate files and must surface safe failure feedback.
- Define the safety contract for future real fixes: present a planned operation preview and require explicit user confirmation before any file mutation runs.
- Define the freshness contract for future real fixes: revalidate target path/problem preconditions immediately before mutation and fail closed if the scan result is stale or files changed after scanning.
- Present Auto-Fix result details inline in the S07 embedded Scanner details area instead of adding a separate modal/window, while retaining the `Auto-Fix Results` heading/copy.
- Use injected fake Auto-Fix operations in tests to prove success, failure, `Fixing...`, `Fixed!`, `Fix Failed`, result details, row/checkmark state, and worker handoff without enabling production mutations.
- Keep Auto-Fix work off the Slint UI thread and marshal owned request/result events through the existing worker/event-loop handoff pattern.
- Add tests proving the current production registry has no supported operations, unsupported results show no button, tampered requests fail closed, and fake registered operations follow the expected state transitions.

### Out of Scope

- Implementing real delete, rename, archive, move, patch, backup, restore, or repair operations in production.
- Adding new scanner problem categories or new solution types beyond the reference scanner/S07 typed results.
- Showing disabled Auto-Fix placeholders or eligibility badges for unsupported results.
- Adding scan cancellation, auto-rescan, or live filesystem monitoring as part of Auto-Fix.
- Implementing Downgrade Manager or Archive Patcher behavior; those remain S09/S10.
- Adding better-than-reference Vortex staging/source-mod attribution.
- Mutating user files based only on the scan-time result without a fresh precondition revalidation step.

## Constraints

- `CMT/` remains read-only; inspect it for parity but implement all Rust/Slint behavior outside the submodule.
- Current source-of-truth behavior is that `CMT/src/autofixes.py::AUTO_FIXES` is empty; S08 must document this discrepancy with roadmap wording rather than guess missing intended fixes.
- The production Auto-Fix registry must fail closed by default and contain no real mutating operations unless a later explicit scoped decision adds them.
- Do not expose an Auto-Fix button unless the selected Scanner result maps to a registered operation.
- Any future real operation must be plan-based, confirmation-gated, revalidated just before mutation, and testable against sandbox/fake filesystem fixtures.
- Worker tasks must carry owned Auto-Fix requests/results; Slint handles/models must not cross worker threads.
- User-facing failure text must be safe and actionable; raw OS/file errors belong in diagnostics/logs/tests.
- Auto-Fix state must remain tied to the selected scan result and should not imply that the underlying scanner result was rescanned or globally removed.
- Preserve the S07 read-only Scanner behavior for all unsupported results.

## Integration Points

### Consumes

- `CMT/src/autofixes.py` — Source of truth for `AutoFixResult`, empty `AUTO_FIXES`, `do_autofix` lifecycle labels, and `Auto-Fix Results` feedback behavior.
- `CMT/src/tabs/_scanner.py` — Source of truth for showing Auto-Fix only when the selected result's solution is registered, and for button labels `Auto-Fix`, `Fixed!`, and `Fix Failed`.
- `CMT/src/enums.py` — Source of truth for `SolutionType` labels used to decide Auto-Fix eligibility.
- `CMT/src/helpers.py` — Source of truth for per-result `autofix_result` storage on `ProblemInfo` / `SimpleProblemInfo`.
- `.gsd/milestones/M001/slices/S07/S07-CONTEXT.md` — Defines that S07 hides Auto-Fix, provides embedded details, typed results, read-only actions, and Scanner controller/worker foundations consumed by S08.
- `src/domain/scanner.rs` — Expected S07 typed scanner result and solution metadata used to attach eligibility and result state.
- `src/app/scanner_controller.rs` — Expected S07 selected-result/details reducer that S08 extends with Auto-Fix button/result state.
- `src/platform/filesystem.rs` — Future/fakeable precondition and sandbox filesystem boundary for Auto-Fix operation tests.
- `src/workers/events.rs`, `src/workers/handoff.rs`, and `src/workers/mod.rs` — Existing owned worker event and Slint-safe handoff pattern for any Auto-Fix execution.
- `ui/scanner_tab.slint` and `ui/main.slint` — Expected S07 Scanner UI/callback surface to extend with gated Auto-Fix presentation and feedback.

### Produces

- `src/domain/autofix.rs` or scanner-domain Auto-Fix additions — Typed Auto-Fix registry, operation ids, eligibility checks, operation plans, preconditions, result status, and safe user-facing labels.
- `src/services/autofix.rs` or scanner-service Auto-Fix additions — Fail-closed execution service with an empty production registry and fake/test registries for success/failure lifecycle coverage.
- `src/app/scanner_controller.rs` — Extended selected-result state for Auto-Fix visibility, `Fixing...`, `Fixed!`, `Fix Failed`, inline `Auto-Fix Results`, and stale/unsupported request handling.
- `src/workers/events.rs` — Auto-Fix-specific worker payloads or Scanner worker payload extensions carrying owned request ids, result ids, safe details, and failure state.
- `ui/scanner_tab.slint` — Gated Auto-Fix button and inline result-details surface that remain hidden when the production registry has no supported operation.
- `ui/main.slint` — Auto-Fix callback/property forwarding through `MainWindow` if needed by the S07 Scanner component.
- Rust tests and Slint source-contract tests — Coverage for empty production registry, hidden unsupported buttons, fake registered success/failure paths, label fidelity, stale callback failure, confirmation/precondition contracts, worker handoff, and non-mutation guarantees.

## Open Questions

- None at discussion closeout. If real Auto-Fix operations are requested later, first define the exact supported solution types, operation plans, confirmation copy, backups/undo expectations, and sandbox verification fixtures in a new scoped decision or slice update.
