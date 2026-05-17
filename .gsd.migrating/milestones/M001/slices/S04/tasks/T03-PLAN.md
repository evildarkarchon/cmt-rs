---
estimated_steps: 21
estimated_files: 6
skills_used: []
---

# T03: Collect Overview filesystem facts

---
estimated_steps: 10
estimated_files: 5
skills_used:
  - tdd
  - verify-before-complete
---
Why: The user-visible Overview must be populated from real discovered installations, but the filesystem and process work must remain fakeable and off the UI thread.

Do:
1. Add `src/services/overview_collector.rs` or an equivalent submodule that collects the injected facts consumed by T02 from a `Fallout4Installation` plus `Filesystem`, `ProcessInspector`, and configured environment paths.
2. Classify reference base files from `CMT/src/globals.py` using process `file_version` raw four-part versions when available and CRC32 fallback when needed; add `crc32fast` to `Cargo.toml` and update `Cargo.lock`.
3. Read only bounded file metadata and headers: BA2 magic/version enough to produce `ArchiveRecord`, plugin/module header and flags enough to produce `ModuleRecord`, and optional Address Library bin existence for the detected game version.
4. Parse `Fallout4.ccc`, `plugins.txt`, and INI archive lists defensively to mark enabled archives/modules and to create missing-file facts without modal warnings.
5. Preserve deterministic traversal and sorted output so snapshot tests are stable; do not mutate user files and do not follow risky write paths.
6. Keep the collector adapter-backed and testable with local fake implementations of `Filesystem` and `ProcessInspector`; do not query real OS state in unit tests.
7. Add tests for direct binary version classification, CRC fallback classification, unknown binary, missing base file, missing Data, missing Address Library, BA2 v1/v7/v8/unknown/unreadable, module full/light/HEDR variants/unreadable, missing ccc, missing plugins.txt, and enabled-state fallback.

Done when: given a fake Data tree and fake version metadata, the collector returns stable typed facts that T02 can turn into a full Overview snapshot.

Threat Surface Q3: local mod file bytes and file names are untrusted; limit reads, validate lengths before indexing, and never execute discovered files.
Failure Modes Q5: permission errors and malformed headers become unreadable or invalid facts with diagnostic details, not panics.
Load Profile Q6: traversal is O(number of Data files); read only small headers except CRC fallback for known base files.
Negative Tests Q7: short BA2 headers, bad magic, invalid HEDR bytes, non-UTF-8 text fallbacks where practical, missing files, and permission/read failures.

## Inputs

- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/domain/discovery.rs`
- `src/platform/filesystem.rs`
- `src/platform/process.rs`
- `CMT/src/tabs/_overview.py`
- `CMT/src/globals.py`
- `CMT/src/utils.py`
- `Cargo.toml`
- `Cargo.lock`

## Expected Output

- `src/services/overview_collector.rs`
- `src/services/mod.rs`
- `src/services/overview.rs`
- `src/domain/overview.rs`
- `Cargo.toml`
- `Cargo.lock`

## Verification

cargo test overview_collector

## Observability Impact

Collector results should include phase/count diagnostics and safe error details so refresh logs can identify whether failures came from binary, archive, module, or enablement parsing.
