---
estimated_steps: 15
estimated_files: 5
skills_used: []
---

# T03: Implement confirmed executor and write seams

---
estimated_steps: 9
estimated_files: 6
skills_used:
  - rust-async-patterns
  - tdd
  - security-review
  - verify-before-complete
---
Why: After inline confirmation, S09 must safely execute the reference backup, restore, download, xdelta apply, and cleanup semantics against sandbox fixtures before real game paths are reachable.
Do: Add a separate writable filesystem trait to `src/platform/filesystem.rs` instead of expanding the existing read-only `Filesystem` contract. Extend `PlatformOperation` with safe write/copy/rename/remove labels. Implement real filesystem mutation methods with typed `PlatformError` mapping and local fake/recording implementations for tests. In `src/services/downgrader.rs`, add `DeltaDownloader` and `DeltaApplier` traits, a reqwest-backed downloader with bounded progress callbacks, and a production xdelta applier only after adding/proving a compatible dependency with a tiny deterministic fixture. Add confirmed execution that revalidates current file and backup CRCs immediately before each mutation, processes the six files independently, creates/reuses/removes `_downgradeBackup` and `_upgradeBackup` files according to direction and `Keep Backups`, downloads only the needed `NG-to-OG-{file}.xdelta` or `OG-to-NG-{file}.xdelta`, applies deltas with the current backup as input, honors `Delete Patches`, continues after per-file failures where safe, and never deletes the only valid source backup after a failed apply.
Failure Modes Q5: File write/copy/rename/remove errors log `Failed patching {file}` and continue where safe. Download timeout/request failure and malformed or failed xdelta apply log failure and preserve backups. If no proven production xdelta applier exists, fail the task and replan rather than shipping a silently non-functional production path.
Load Profile Q6: Fixed six-file execution, bounded patch downloads, sequential mutation, and progress events per file/download prevent unbounded concurrency or memory growth.
Negative Tests Q7: Cover valid desired-backup restore with and without `Keep Backups`, invalid backup deletion, current-backup creation/reuse, as-needed delta download, delete-deltas cleanup, failed download, failed apply, read-only/permission failures, unsupported source generation, and no downloader/applier calls for skipped files.
Done when: Executor tests prove reference log rows and mutation/download/apply calls for success and failure paths, and the production applier has a fixture proof if a dependency is added.

## Inputs

- `src/services/downgrader.rs`
- `src/platform/mod.rs`
- `src/platform/filesystem.rs`
- `src/domain/downgrader.rs`
- `Cargo.toml`
- `Cargo.lock`
- `CMT/src/downgrader.py`

## Expected Output

- `src/platform/mod.rs`
- `src/platform/filesystem.rs`
- `src/services/downgrader.rs`
- `Cargo.toml`
- `Cargo.lock`

## Verification

cargo test downgrader_executor

## Observability Impact

Adds execution tracing and file-level result diagnostics for backup, restore, download, apply, cleanup, permission, and xdelta failures while keeping the modal log user-safe.
