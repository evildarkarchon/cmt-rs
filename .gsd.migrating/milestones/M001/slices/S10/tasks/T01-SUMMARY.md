---
id: T01
parent: S10
milestone: M001
key_files:
  - src/domain/archive_patcher.rs
  - src/services/archive_patcher.rs
  - src/domain/mod.rs
  - src/services/mod.rs
  - src/services/downgrader.rs
key_decisions:
  - Archive Patcher preview planning remains Slint-free and read-only, consuming Overview archive records and probing only 12-byte BA2 prefixes through `Filesystem::read_prefix`.
  - Archive Patcher candidate sorting uses a case-insensitive path key with original-path tie-breaker to match Python Windows `Path` ordering rather than Linux/WSL `PathBuf` ordering.
  - Latest restore manifest models are JSON-serializable domain payloads only; mutation, manifest writing, restore execution, and UI/controller streaming remain deferred to later S10 tasks.
duration: 
verification_result: passed
completed_at: 2026-05-19T01:35:51.911Z
blocker_discovered: false
---

# T01: Added a Slint-free Archive Patcher domain contract and read-only preview planner with fail-closed BA2 header validation.

**Added a Slint-free Archive Patcher domain contract and read-only preview planner with fail-closed BA2 header validation.**

## What Happened

Created `src/domain/archive_patcher.rs` with reference Archive Patcher modal strings, desired-version targets, filter/about copy, candidate rows, preview plan rows, log/progress/summary models, restore manifest entries, JSON-serializable latest manifest payloads, and stable preview-plan digest support. Added `src/services/archive_patcher.rs` as the read-only planner over the existing `Filesystem` trait and Overview `ArchiveRecord` data: it selects enabled candidates by the reference target inversion (`v1 (OG)` selects v7/v8, `v8 (NG)` selects v1), applies basename filtering case-insensitively, sorts candidates using Windows/Python reference path semantics, reads only 12-byte BA2 prefixes, validates BTDX magic/version/known format/path containment, and produces fail-closed row failures instead of writable rows for malformed or unreadable data. Exported the new domain/service modules and added public-import coverage in `src/domain/mod.rs`. During verification, plain `cargo test ...` initially hit a rustc 1.95.0 ICE in the pre-existing `services::downgrader::tests` dead-code lint path, so I added `#[allow(dead_code)]` to that test module only; production behavior is unchanged and the required plain commands now run normally.

## Verification

Verified reference strings, target inversion, filter behavior, deterministic candidate sorting, no-candidate messaging, bounded prefix reads, bad magic/version/format/short-header failures, unreadable and uncontained path failures, already-target and stale-target rejection, digest stability across request ids, digest changes for path/version/target changes, formatting, compile, full test suite, and clippy. Final gates passed: `cargo fmt --check`, `cargo test archive_patcher_domain --quiet`, `cargo test archive_patcher_service_plan --quiet`, `cargo check --quiet`, `cargo test --quiet`, and `cargo clippy --all-targets --all-features --quiet`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 634ms |
| 2 | `cargo test archive_patcher_domain --quiet` | 0 | ✅ pass | 42977ms |
| 3 | `cargo test archive_patcher_service_plan --quiet` | 0 | ✅ pass | 8590ms |
| 4 | `cargo check --quiet` | 0 | ✅ pass | 25078ms |
| 5 | `cargo test --quiet` | 0 | ✅ pass | 8572ms |
| 6 | `cargo clippy --all-targets --all-features --quiet` | 0 | ✅ pass (warnings emitted, non-fatal) | 36167ms |

## Deviations

Added `#[allow(dead_code)]` to the existing `src/services/downgrader.rs` test module to work around an unrelated rustc 1.95.0 ICE in the dead-code lint so the requested plain test commands could complete. No production code behavior changed.

## Known Issues

`cargo clippy --all-targets --all-features --quiet` exits 0 but still emits existing warnings in `src/main.rs` about `field_reassign_with_default`; those warnings are outside this task's Archive Patcher domain/service scope.

## Files Created/Modified

- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/domain/mod.rs`
- `src/services/mod.rs`
- `src/services/downgrader.rs`
