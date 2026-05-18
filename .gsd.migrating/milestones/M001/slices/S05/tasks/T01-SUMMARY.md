---
id: T01
parent: S05
milestone: M001
key_files:
  - src/domain/tools.rs
  - src/domain/mod.rs
  - resources/images/icon-256.png
  - resources/images/logo-nexusmods.png
  - resources/images/logo-discord.png
  - resources/images/logo-github.png
key_decisions:
  - Use `src/domain/tools.rs` as the pure Rust single source of truth for S05 Tools/About labels, ids, URLs, copy feedback, deferred utility metadata, and resource paths.
  - Represent Tools utility entries as disabled/deferred metadata with safe status text until the Downgrade Manager and Archive Patcher workflows are ported.
  - Copy required reference images into `resources/images/` and expose those Rust-owned paths instead of depending on `CMT/` at runtime.
duration: 
verification_result: passed
completed_at: 2026-05-18T01:16:58.278Z
blocker_discovered: false
---

# T01: Added a pure Tools/About reference contract module, exported it, and copied the required About image assets into Rust-owned resources.

**Added a pure Tools/About reference contract module, exported it, and copied the required About image assets into Rust-owned resources.**

## What Happened

Re-checked the Tools/About Python reference source and globals for labels, group order, URLs, attribution text, image names, and host hint behavior. Added `src/domain/tools.rs` as the S05 single source of truth with typed Tools action ids, About action/link ids, ordered tool groups, exact multi-line labels and help text, reference URL constants, URL hint mapping including fallback, About title/credit/open/copy labels, `Copied!` copy feedback/reset timing, disabled/deferred utility metadata, and Rust-owned resource path constants. Exported the module from `src/domain/mod.rs` and extended the public import smoke test. Copied `icon-256.png`, `logo-nexusmods.png`, `logo-discord.png`, and `logo-github.png` from the read-only reference tree into `resources/images/`. Added focused `s05_reference_contract` unit tests for exact group/button order, URLs, help text, hint fallback, About labels/actions/resources, disabled utility metadata, unique ids, and resource presence outside `CMT/`. Initial formatting check reported rustfmt diffs; `cargo fmt` was applied and the final gates passed.

## Verification

Verified formatting, focused S05 contract tests, compile surface, and clippy. `cargo test s05_reference_contract` passed with 5 S05 tests, including the copied image resource presence test; `cargo check` and `cargo clippy --all-targets --all-features` both passed after exporting the new domain module.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 398ms |
| 2 | `cargo test s05_reference_contract` | 0 | ✅ pass | 37616ms |
| 3 | `cargo check` | 0 | ✅ pass | 13784ms |
| 4 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 16249ms |

## Deviations

Additionally inspected `CMT/src/utils.py` to verify the copy-button success label (`Copied!`) and reset timing used by About; no files under `CMT/` were edited.

## Known Issues

None.

## Files Created/Modified

- `src/domain/tools.rs`
- `src/domain/mod.rs`
- `resources/images/icon-256.png`
- `resources/images/logo-nexusmods.png`
- `resources/images/logo-discord.png`
- `resources/images/logo-github.png`
