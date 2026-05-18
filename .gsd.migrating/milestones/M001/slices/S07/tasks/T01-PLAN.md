---
estimated_steps: 5
estimated_files: 2
skills_used: []
---

# T01: Define Scanner domain contract

Expected executor skills: tdd, decompose-into-slices, verify-before-complete.

Why: Scanner needs a Slint-free contract before service, controller, and UI work can share labels, ordering, details, and action descriptors without reimplementing reference strings in multiple layers.

Do: Inspect the reference inputs listed below, then create `src/domain/scanner.rs` and export it from `src/domain/mod.rs`. Define scanner category labels/order, reference progress/result-count constants, problem type labels and deterministic group order, solution text, read-only action descriptors, result/detail records, optional mod attribution, optional file-list metadata, overview-problem mapping helpers, and copy-details rendering. Keep this layer pure: no filesystem, Slint, platform, or worker imports. Preserve the S07 intentional UI difference only as data, not behavior. Add unit tests whose names include `scanner_domain` for label order, default category projection from `ScannerSettings`, result-count text including zero, deterministic grouping/sorting, overview problem mapping with URL/detail preservation, and copy-details text with and without `Mod:`.

Done when: later tasks can use domain types for scan results/details/actions without knowing Slint structs or reference Python classes, and `cargo test scanner_domain` passes.

Negative tests Q7: empty result sets, overview problems with no path, pathless limit problems, missing solution text, URL and non-URL extra data, and unknown/custom problem labels.

## Inputs

- `CMT/src/tabs/_scanner.py`
- `CMT/src/scan_settings.py`
- `CMT/src/enums.py`
- `CMT/src/globals.py`
- `src/domain/settings.rs`
- `src/domain/overview.rs`
- `src/domain/mod.rs`

## Expected Output

- `src/domain/scanner.rs`
- `src/domain/mod.rs`

## Verification

cargo test scanner_domain

## Observability Impact

Adds stable problem/status labels and safe detail rendering that logs and UI can reference consistently; no runtime observability yet.
