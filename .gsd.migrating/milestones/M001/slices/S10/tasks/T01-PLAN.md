---
estimated_steps: 22
estimated_files: 4
skills_used: []
---

# T01: Define Archive Patcher domain and preview plans

---
estimated_steps: 8
estimated_files: 4
skills_used:
  - write-docs
  - tdd
  - verify-before-complete
---
Why: S10 needs a Slint-free contract before UI or mutation code so the reference labels, candidate semantics, log vocabulary, and confirmation model are testable without launching a window or touching real game files.

Do:
1. Create `src/domain/archive_patcher.rs` with public constants and typed models for modal title, desired-version labels, default target, filter explainer text, About title/body, candidate rows, plan rows, log rows, progress, summary counts, restore manifest entries, and reference-visible messages.
2. Export the module from `src/domain/mod.rs` and add public-import or reference-string tests consistent with existing domain module style.
3. Create `src/services/archive_patcher.rs` with a read-only planning service that consumes current Overview/discovery `ArchiveRecord` values plus desired target and name filter.
4. Implement candidate selection exactly from the slice contract: target `v1 (OG)` selects enabled `NextGen7` and `NextGen8`; target `v8 (NG)` selects enabled `OldGen`; disabled archives are ignored; basename filtering is case-insensitive; output is deterministic and log-ready with `Showing N files to be patched.` or `Nothing to do!`.
5. Add read-only validation and preview-plan models that inspect only BA2 header prefixes through `Filesystem::read_prefix`, verify BTDX magic, version, known format, path containment inputs, target transition, and restore-manifest feasibility without writing.
6. Add a stable digest for the preview plan so later confirmation can reject changed candidates.
7. Add JSON-serializable latest-manifest models but do not implement mutation in this task.
8. Keep all domain/service behavior free of Slint handles, UI models, and real OS assumptions.

Done when: domain and read-only service tests prove reference strings, target inversion, filter behavior, sorted candidates, no-candidate messaging, bad magic/version/format planning failures, and stable digest changes when path/version/target changes.

Failure Modes Q5: Missing Overview archive data becomes a safe empty plan; unreadable headers become per-file plan failures; malformed path/header data never becomes a writable row.
Load Profile Q6: Planning is O(candidate count) and reads bounded header prefixes only, so large BA2 files are not loaded into memory.
Negative Tests Q7: Empty filter, mixed-case filter, disabled archives, short headers, `XXXX` magic, unknown version, unknown format, no candidates, and already-target archives.

## Inputs

- `CMT/src/patcher/_archives.py`
- `CMT/src/patcher/_base.py`
- `CMT/src/globals.py`
- `src/domain/discovery.rs`
- `src/domain/overview.rs`
- `src/services/overview_collector.rs`
- `src/services/downgrader.rs`
- `src/platform/filesystem.rs`

## Expected Output

- `src/domain/archive_patcher.rs`
- `src/domain/mod.rs`
- `src/services/archive_patcher.rs`
- `src/services/mod.rs`

## Verification

cargo test archive_patcher_domain --quiet
cargo test archive_patcher_service_plan --quiet

## Observability Impact

Introduces the user-visible log row, progress, plan summary, and failure vocabulary that later worker/controller layers will stream and expose.
