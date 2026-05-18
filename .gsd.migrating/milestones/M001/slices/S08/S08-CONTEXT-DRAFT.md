---
id: S08
milestone: M001
status: draft
---

# S08: Scanner Auto Fix Actions — Context Draft

## Goal

Add the Scanner Auto-Fix extension path and result-state plumbing in a fail-closed way, while preserving that the current checked-in reference has no registered Auto-Fix operations.

## Confirmed Human Decisions So Far

- Auto-Fix scope: use framework/plumbing only for the current reference because `CMT/src/autofixes.py` defines `AUTO_FIXES = {}`. Do not invent destructive fixes in S08.
- User-visible outcome: S08 can be considered complete when it proves the Auto-Fix registry/result path is safe and fail-closed, even though current users see no Auto-Fix buttons because there are no supported registered operations.
- Unsupported visibility: keep Auto-Fix hidden for results without a registered, tested operation, matching `_scanner.py` behavior.
- Safety contract for future real fixes: if any real mutation is added later, show the planned operation(s) and require explicit confirmation before running.
- Future stale-target behavior: future real fixes must revalidate target path/problem preconditions immediately before mutation and fail closed if scan results are stale or files changed.
- Feedback pattern for future/fail-closed attempts: preserve button state labels `Auto-Fix`, `Fixing...`, `Fixed!`, and `Fix Failed`, and present `Auto-Fix Results` details inline in the S07 embedded Scanner details area rather than requiring a separate modal.

## Reference Findings

- `CMT/src/autofixes.py` defines `AutoFixResult`, `AUTO_FIXES`, and `do_autofix`, but `AUTO_FIXES` is currently empty.
- `CMT/src/tabs/_scanner.py` only shows the Auto-Fix button when `problem_info.solution in AUTO_FIXES`.
- Existing labels/messages to preserve when operations exist: `Auto-Fix`, `Fixing...`, `Fixed!`, `Fix Failed`, and `Auto-Fix Results`.

## Open Items To Resolve

- None currently; ask wrap-up before writing final context.
