---
estimated_steps: 17
estimated_files: 3
skills_used: []
---

# T02: Implement empty production Auto Fix service with fake registry

Expected executor skills_used: tdd, observability, verify-before-complete.

Why: The reference AUTO_FIXES registry is empty, so production must not expose or run mutations, but S08 needs a tested registry/service seam for future operations and fake-backed lifecycle coverage.

Do:
1. Add src/services/autofix.rs with a doc-commented AutoFixService over an injectable registry or operation catalog. The production constructor must register zero operations.
2. Implement an Auto-Fix support catalog that the controller can consume without owning operation closures. Eligibility must be typed, using ScannerSolutionKind or the domain operation key from T01, and must not inspect display strings.
3. Implement plan and execute methods that validate scan id, result index, selected-result identity, registered operation, target/path requirements, explicit confirmation, and operation preconditions before the operation closure can run.
4. Return owned Auto-Fix result payloads for success, operation failure, and rejection. Rejections must include a safe message plus a diagnostic/rejection kind for tests and tracing.
5. Provide fake/test operations inside the test module that can succeed, fail, require a target, require confirmation, and fail precondition revalidation. Use a call counter or recorded mutation list to prove invalid requests do not call the operation.
6. Export the service module from src/services/mod.rs.

Failure Modes Q5:
| Dependency | On error | On timeout | On malformed response |
| --- | --- | --- | --- |
| Registered operation | Return Fix Failed with safe details and diagnostic rejection or operation error | Not applicable for current blocking fake operations; future operations must map timeout to Fix Failed | Treat malformed plan/precondition data as rejected before mutation |
| Filesystem precondition facts | Fail closed before execute | Not applicable in S08 because production registry is empty | Fail closed with a safe unsupported or precondition message |

Load Profile Q6: one selected result and one registered operation are evaluated per request; no full rescan or directory traversal should happen in the service. Future real operations may add bounded target revalidation only.

Negative Tests Q7: empty production registry has no supported operations; unknown result index, stale identity, unsupported typed solution, missing target, missing confirmation, and failed precondition all fail closed and do not increment the fake operation call counter.

Done when the service proves empty production parity and fake success/failure/precondition flows through targeted tests.

## Inputs

- `src/domain/autofix.rs`
- `src/domain/scanner.rs`
- `src/services/mod.rs`
- `src/platform/filesystem.rs`

## Expected Output

- `src/services/autofix.rs`
- `src/services/mod.rs`
- `src/domain/autofix.rs`

## Verification

cargo test scanner_autofix_service

## Observability Impact

Service result types and tracing points should expose rejection kind, scan id, result index, operation key, and safe message while keeping raw diagnostics separate from user-facing details.
