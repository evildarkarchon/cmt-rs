---
estimated_steps: 13
estimated_files: 4
skills_used: []
---

# T01: Add typed Auto Fix domain contract and solution identities

Expected executor skills_used: tdd, design-an-interface, verify-before-complete.

Why: S08 eligibility must follow the reference registry key behavior, but S07 Scanner results currently carry only solution display text plus a deferred read-only Auto-Fix placeholder. This task establishes typed Auto-Fix state and typed solution identity before any service, controller, or UI wiring is allowed.

Do:
1. Add src/domain/autofix.rs with public doc-commented constants for Auto-Fix, Fixing..., Fixed!, Fix Failed, and Auto-Fix Results; typed status/button/result-detail structs; operation id or key types; plan preview, confirmation, request, completion, and rejection types; and future-safety fields for confirmation and pre-mutation revalidation.
2. Export the new module from src/domain/mod.rs and add the public import assertion beside the existing scanner domain assertions.
3. Extend ScannerResult in src/domain/scanner.rs to retain typed solution identity, such as Option<ScannerSolutionKind>, alongside the display solution text. Add a stable selected-result identity or fingerprint used later to reject stale/tampered requests.
4. Remove the S07 deferred Auto-Fix placeholder from read-only Scanner actions: no ScannerActionKind::AutoFixDeferred, no ScannerActionTarget::Deferred, no auto_fix_deferred flag, and no with_deferred_auto_fix helper. Auto-Fix must become a separate write-capable contract, not a disabled read-only action.
5. Update src/services/scanner.rs result construction so scanner-owned results created from known ScannerSolutionKind variants retain that typed key. Overview-imported or custom string-only solutions must remain display-only and not become Auto-Fix eligible by string matching.
6. Add focused tests named with the scanner_autofix_domain filter covering label fidelity, typed solution identity, string-only non-eligibility, stable identity changes when result facts change, and absence of the old deferred read-only action.

Failure Modes Q5: malformed/custom solution strings must produce no typed operation key; missing paths must be represented without panics; stale identity data must be deterministic and owned.

Load Profile Q6: identity generation is per displayed result and should be cheap string/path hashing or structural comparison, not filesystem IO.

Negative Tests Q7: custom display-only solution, Overview string solution, missing solution, and former with_deferred_auto_fix behavior all remain ineligible or removed.

Done when the domain compiles, scanner scan-service constructors preserve typed keys, and the targeted domain tests pass without reading or writing user files.

## Inputs

- `CMT/src/autofixes.py`
- `CMT/src/enums.py`
- `CMT/src/helpers.py`
- `CMT/src/tabs/_scanner.py`
- `src/domain/scanner.rs`
- `src/domain/mod.rs`
- `src/services/scanner.rs`

## Expected Output

- `src/domain/autofix.rs`
- `src/domain/scanner.rs`
- `src/domain/mod.rs`
- `src/services/scanner.rs`

## Verification

cargo test scanner_autofix_domain

## Observability Impact

No runtime signals yet. This task creates the typed status, rejection, result-detail, and diagnostic fields that later tasks surface through logs and UI state.
