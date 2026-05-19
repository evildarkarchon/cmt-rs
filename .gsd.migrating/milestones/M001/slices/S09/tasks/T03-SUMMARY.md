---
id: T03
parent: S09
milestone: M001
key_files:
  - Cargo.toml
  - Cargo.lock
  - src/platform/filesystem.rs
  - src/services/downgrader.rs
  - src/services/autofix.rs
  - src/services/discovery.rs
  - src/services/f4se.rs
  - src/services/overview_collector.rs
  - src/services/scanner.rs
key_decisions:
  - D032: Pin SHA-256 hashes, exact patch byte sizes, and expected VCDIFF output sizes for the twelve reference delta-patches release assets in the Rust downgrader service.
duration: 
verification_result: passed
completed_at: 2026-05-19T00:01:40.024Z
blocker_discovered: false
---

# T03: Hardened Downgrader confirmed execution with canonical containment checks, pinned xdelta integrity, bounded VCDIFF output, and active-file-preserving replacement.

**Hardened Downgrader confirmed execution with canonical containment checks, pinned xdelta integrity, bounded VCDIFF output, and active-file-preserving replacement.**

## What Happened

Reworked the Downgrader mutation path so confirmed execution revalidates the game root through canonicalization, resolves each managed target/backup/patch path under that canonical root, rejects symlink/reparse-point managed paths where the filesystem seam can expose them, and validates containment immediately before destructive operations. The executor now copies current files into backups instead of renaming the active file away, restores/patches via same-directory replacement after bytes are ready, and leaves the active file in place on download/apply/write failures. Added a pinned integrity manifest for the twelve reference GitHub delta-patch assets with exact sizes, SHA-256 digests, and expected VCDIFF output byte counts. Local xdelta files now go through the same size/hash checks as downloaded deltas, and the VCDIFF applier writes into a bounded writer capped by the expected output size. Extended the filesystem seam with no-follow metadata, canonicalization, and atomic/same-directory replacement helpers, then updated fake metadata construction sites after adding the reparse-point flag.

## Verification

Verified the targeted executor and read-only plan filters, formatting, compile, full unit suite, and clippy gate. Commands run: cargo test downgrader_executor --message-format short; cargo test downgrader_service_plan --message-format short; cargo fmt --check; cargo check --message-format short; cargo test --message-format short; cargo clippy --all-targets --all-features --message-format short. All commands exited 0. Clippy still reports warnings but does not fail under the current project lint configuration.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test downgrader_executor --message-format short 2>&1 | tail -120` | 0 | ✅ pass | 39457ms |
| 2 | `cargo test downgrader_service_plan --message-format short 2>&1 | tail -120` | 0 | ✅ pass | 8278ms |
| 3 | `cargo fmt --check` | 0 | ✅ pass | 584ms |
| 4 | `cargo check --message-format short 2>&1 | tail -120` | 0 | ✅ pass | 25464ms |
| 5 | `cargo test --message-format short 2>&1 | tail -160` | 0 | ✅ pass | 8287ms |
| 6 | `cargo clippy --all-targets --all-features --message-format short 2>&1 | tail -160` | 0 | ✅ pass | 40072ms |

## Deviations

Added `sha2` as a focused dependency for T03 integrity checks and updated nearby fake filesystem metadata constructors after extending the metadata shape.

## Known Issues

Clippy exits successfully but reports existing warning-level findings, including functions with too many arguments. No failing tests remain from T03 verification.

## Files Created/Modified

- `Cargo.toml`
- `Cargo.lock`
- `src/platform/filesystem.rs`
- `src/services/downgrader.rs`
- `src/services/autofix.rs`
- `src/services/discovery.rs`
- `src/services/f4se.rs`
- `src/services/overview_collector.rs`
- `src/services/scanner.rs`
