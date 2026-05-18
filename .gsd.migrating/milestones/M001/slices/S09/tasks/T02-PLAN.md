---
estimated_steps: 13
estimated_files: 3
skills_used: []
---

# T02: Build status snapshot and preview plan service

---
estimated_steps: 8
estimated_files: 3
skills_used:
  - write-docs
  - tdd
  - security-review
---
Why: The first `Patch All` action must classify the six reference files and build an inline plan while proving that no mutation occurs before confirmation.
Do: Add `src/services/downgrader.rs` and export it from `src/services/mod.rs`. Implement a `DowngraderService` over the existing read-only `Filesystem` trait that validates the discovered Fallout 4 game root, resolves only the six constant relative paths under that root, rejects absolute or escaping paths, computes CRC32 status snapshots, applies the `steam_api64.dll` Next-Gen and Anniversary display rule, chooses the reference default target from `Fallout4.exe`, reads backup CRCs for plan accuracy, and builds plan rows for skip, invalid-backup cleanup, restore-from-backup, current-backup creation, delta download, patch apply, and optional cleanup. The service must return safe user-facing failures if the root is missing or unsafe and must not call any mutation, downloader, or applier during status or plan building.
Failure Modes Q5: Missing or unsupported discovery returns a safe failure and disabled mutation. Permission/read errors classify the affected file as unknown or plan failure according to reference-safe behavior. Malformed relative-path definitions are rejected before plan output.
Negative Tests Q7: Cover missing root, path escape attempts, missing files, already-target files, Anniversary/Obsolete/Unknown/unsupported CRCs, valid and invalid desired/current backups, and no mutation during plan generation.
Done when: Fake-backed tests prove status classification, reference row order, target defaulting, plan contents, path safety, and zero mutation before confirmation.

## Inputs

- `src/domain/downgrader.rs`
- `src/domain/discovery.rs`
- `src/platform/filesystem.rs`
- `src/services/mod.rs`
- `CMT/src/downgrader.py`

## Expected Output

- `src/services/downgrader.rs`
- `src/services/mod.rs`
- `src/domain/downgrader.rs`

## Verification

cargo test downgrader_service_plan

## Observability Impact

Adds service-level tracing for status and plan requests, safe-path rejection, unsupported file classification, and generated plan counts so future agents can distinguish discovery, read, and planning failures.
