---
estimated_steps: 1
estimated_files: 5
skills_used: []
---

# T03: Harden confirmed executor safety and delta integrity

Redo the confirmed execution safety layer before S09 closeout. Preserve the existing read-only status/plan behavior, but harden real mutation semantics: canonicalize/revalidate the game root and every managed parent/target immediately before mutation, reject symlink/junction/reparse-point target escape where the platform can detect it, and ensure resolved targets remain under the canonical root. Change replacement order so active files remain intact until replacement bytes are produced, cryptographically/integrity-checked, CRC-checked against the desired target, and ready for atomic/same-directory replace; on download/apply/write failure, leave the active file in place and preserve usable backups. Add SHA-256 or stronger pinned integrity checks for downloaded patch assets and/or expected output files, enforce the same size cap on existing local delta files as downloaded deltas, and cap VCDIFF output to the expected target size. Update `DeltaDownloader`/`DeltaApplier`/filesystem seams and fake-backed tests accordingly, including rollback/no-active-file-loss, symlink/junction escape, pinned-hash failure, oversized local patch, and expansion-bomb failures.

## Inputs

- `S09 closeout reviewer and security findings`
- `Existing T03 executor implementation and tests`
- `CMT/src/downgrader.py reference semantics`

## Expected Output

- `Hardened `DowngraderService::execute_confirmed` and platform filesystem write helpers`
- `Fake-backed executor/security tests proving no active-file loss, canonical path containment, pinned integrity checks, local patch caps, and bounded VCDIFF output`
- `Updated dependency/decision notes if integrity metadata changes`

## Verification

cargo test downgrader_executor
cargo test downgrader_service_plan
cargo test
cargo clippy --all-targets --all-features

## Observability Impact

Adds execution tracing and file-level result diagnostics for backup, restore, download, apply, cleanup, permission, and xdelta failures while keeping the modal log user-safe.
