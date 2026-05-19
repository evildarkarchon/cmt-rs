---
estimated_steps: 24
estimated_files: 3
skills_used: []
---

# T02: Implement safe header patch and restore execution

---
estimated_steps: 9
estimated_files: 3
skills_used:
  - tdd
  - rust-async-patterns
  - security-review
  - verify-before-complete
---
Why: Archive Patcher is mutation-heavy and must fail closed while avoiding full multi-GB archive copies. This task turns the read-only plan into a byte-level executor and restore path behind fakeable filesystem seams.

Do:
1. Extend `WritableFilesystem` in `src/platform/filesystem.rs` with a bounded byte-range write operation suitable for writing the BA2 version field at offset 4, and implement it for `RealFilesystem` using random-access file IO without full-file replacement.
2. Preserve existing read-only `Filesystem` boundaries; only confirmed executor code should require `WritableFilesystem`.
3. In `src/services/archive_patcher.rs`, implement confirmed patch execution that re-previews or revalidates the plan digest before mutation, writes the D033 latest restore manifest before touching archive bytes, and then processes files sequentially.
4. For each file, revalidate canonical containment, regular file metadata, BTDX magic, current version, desired transition, and expected post-patch value immediately before the byte write.
5. Map reference-compatible per-file outcomes to log rows: missing file, permission/in-use, unknown OS error, unrecognized format, unrecognized version, skipping already-patched archive, patched to target, and final `Patching complete. N Successful, M Failed.`.
6. Implement restore-last-run by reading the app-owned manifest, validating each file's current magic/version against the expected post-patch state, and writing back the saved original header/version bytes or logging a safe skip/failure.
7. Continue processing after per-file failures and return aggregate success/failure counts.
8. Add fake/sandbox tests for byte-level transitions v7 to v1, v8 to v1, v1 to v8, already patched skip, unknown version skip, bad magic skip, missing file, permission failure, manifest write failure, partial success, restore success, and restore stale-file skip.
9. Avoid `unwrap()` and `expect()` in production paths; use typed errors or safe result rows.

Done when: executor tests prove mutation touches only validated header bytes, manifest creation precedes writes, partial failures are visible but non-fatal, and restore refuses stale or ambiguous files.

Failure Modes Q5: Manifest write failure aborts mutation before any header changes; read/write permission errors become per-file failures; digest mismatch aborts the confirmed run before mutation; restore mismatch skips rather than writes.
Load Profile Q6: Runtime cost is one manifest write plus bounded header probes and one byte-range write per valid file; no full BA2 archive copy or full-file read is allowed.
Negative Tests Q7: Plan digest mismatch, missing file, denied file, short header, bad magic, unknown version, manifest serialization failure, byte-write failure, and stale restore manifest.

## Inputs

- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/services/downgrader.rs`
- `src/platform/filesystem.rs`
- `src/domain/discovery.rs`

## Expected Output

- `src/platform/filesystem.rs`
- `src/services/archive_patcher.rs`
- `src/domain/archive_patcher.rs`

## Verification

cargo test archive_patcher_executor --quiet
cargo test platform::filesystem --quiet

## Observability Impact

Adds per-file execution diagnostics and final summary rows that distinguish validation, manifest, permission, unknown OS, and restore-staleness failures.
