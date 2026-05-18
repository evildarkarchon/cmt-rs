---
id: T01
parent: S08
milestone: M001
key_files:
  - src/domain/autofix.rs
  - src/domain/mod.rs
  - src/domain/scanner.rs
  - src/services/scanner.rs
key_decisions:
  - Auto-Fix operation eligibility is derived only from retained typed `ScannerSolutionKind` values mapped to `AutoFixOperationKey`; display strings are never matched into eligibility.
  - Scanner result selection identities are computed on demand from displayed/owned result facts instead of stored as a mutable field, avoiding stale identity fields while still giving future Auto-Fix requests an owned fingerprint for stale/tamper rejection.
duration: 
verification_result: passed
completed_at: 2026-05-18T08:13:25.232Z
blocker_discovered: false
---

# T01: Added a fail-closed Scanner Auto-Fix domain contract with typed solution keys, deterministic selection identities, and scanner-owned typed solution preservation.

**Added a fail-closed Scanner Auto-Fix domain contract with typed solution keys, deterministic selection identities, and scanner-owned typed solution preservation.**

## What Happened

Inspected the reference Auto-Fix registry and Scanner details pane: `CMT/src/autofixes.py` keys handlers by `SolutionType`, currently has an empty `AUTO_FIXES` registry, and uses the exact labels `Auto-Fix`, `Fixing...`, `Fixed!`, `Fix Failed`, and `Auto-Fix Results`. Added `src/domain/autofix.rs` as a pure domain module with doc-commented lifecycle constants, typed operation keys, button/status/result-detail structs, plan preview, confirmation, request, completion, rejection, and pre-mutation revalidation payloads. Exported the module from `src/domain/mod.rs` and added public import assertions.

Extended `ScannerResult` with `solution_kind: Option<ScannerSolutionKind>`, `with_solution_kind`, `auto_fix_operation_key`, and deterministic `selection_identity()` generation from already-owned/displayed result facts without filesystem I/O. Removed the S07 deferred read-only Auto-Fix placeholder entirely: no `ScannerActionKind::AutoFixDeferred`, no `ScannerActionTarget::Deferred`, no deferred status constant, no `auto_fix_deferred` field, and no `with_deferred_auto_fix` helper. Updated read-only Scanner actions to remain copy/open/url/file-list only.

Updated `src/services/scanner.rs` so scanner-owned results constructed from known `ScannerSolutionKind` variants retain typed keys via a dedicated helper, while Overview handoff and custom string-only solution text remain display-only and ineligible by string matching. Added focused `scanner_autofix_domain` tests for label fidelity, typed solution identity, string-only non-eligibility, deterministic identity changes, removal of the old deferred action, and scan-service preservation/non-preservation behavior.

## Verification

Ran `cargo fmt --check`, formatted once after the initial check found drift, then reran `cargo fmt --check` successfully. Ran `cargo test scanner_autofix_domain`; all 8 targeted tests passed and the crate compiled for the filtered test run. Also confirmed the old deferred Auto-Fix symbols no longer appear under `src/` with an `rg` scan.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 535ms |
| 2 | `cargo test scanner_autofix_domain` | 0 | ✅ pass (8 passed; 0 failed; 261 filtered out) | 38249ms |

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `src/domain/autofix.rs`
- `src/domain/mod.rs`
- `src/domain/scanner.rs`
- `src/services/scanner.rs`
