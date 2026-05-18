---
id: T04
parent: S04
milestone: M001
key_files:
  - src/services/update.rs
  - src/services/mod.rs
  - Cargo.toml
  - Cargo.lock
  - src/services/overview_collector.rs
key_decisions:
  - Overview update checks are implemented as an async injectable service over `UpdateCheckClient`; failures and no-update states remain silent banner states with capped diagnostics/logs, matching the Python reference.
  - Overview URL/path actions are executed through an injectable `DesktopActions` adapter and converted to `OverviewDesktopActionFeedback` so UI surfaces can show only safe last-action errors.
  - For reqwest 0.13.3, the project uses `default-features = false` with `json` and `rustls` features because `rustls-tls` is not a valid feature name in this version.
duration: 
verification_result: passed
completed_at: 2026-05-17T23:10:05.773Z
blocker_discovered: false
---

# T04: Added a fakeable async Overview update checker and safe desktop link executor with reference-compatible silent failure behavior.

**Added a fakeable async Overview update checker and safe desktop link executor with reference-compatible silent failure behavior.**

## What Happened

Implemented `src/services/update.rs` with an `UpdateCheckClient` trait, reqwest-backed `RealUpdateCheckClient`, source-level update reports, fakeable test surfaces, Nexus/GitHub parsers, and `UpdateCheckService` source selection for `UpdateSource::{None,Nexus,Github,Both}`. The service uses the reference Nexus and GitHub URLs already locked in the Overview domain, applies the reference five-second request timeout, returns banner releases only when a parsed remote numeric version is greater than the current application version, and turns malformed responses, invalid versions, HTTP failures, and client failures into silent diagnostic states. Added `OverviewLinkService` to execute Overview URL/path deferred actions through the injectable `DesktopActions` adapter and return `OverviewDesktopActionFeedback` for safe last-action UI projection. Exported the service module from `src/services/mod.rs`, added reqwest to `Cargo.toml`, and updated `Cargo.lock`. During verification, reqwest 0.13.3 rejected the originally planned `rustls-tls` feature because the current crate feature is named `rustls`; the dependency now uses `default-features = false` with `json` and `rustls`. A rustc diagnostic-rendering ICE occurred while an unused-import warning was present; moving test-only imports into the test module removed the warning and `cargo check` passed. I also resolved pre-existing clippy nested-if warnings in `src/services/overview_collector.rs` so the final clippy gate is clean.

## Verification

Verified the update and link behavior with `cargo test overview_update` (12 tests passed), including disabled source skipping all client calls, Nexus-only, GitHub-only, Both source order/no retries, newer-version banners, equal/older silent no-update, malformed GitHub JSON, Nexus page missing version metadata, invalid version strings, timeout/client/HTTP failures, and desktop URL/path open failure feedback. Ran full project gates: `cargo fmt --check`, `cargo check`, `cargo test` (127 passed), and `cargo clippy --all-targets --all-features`, all with exit code 0.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 387ms |
| 2 | `cargo check` | 0 | ✅ pass | 11615ms |
| 3 | `cargo test overview_update` | 0 | ✅ pass | 29789ms |
| 4 | `cargo test` | 0 | ✅ pass | 8393ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 13164ms |

## Deviations

Used reqwest feature `rustls` instead of the task-plan spelling `rustls-tls` because reqwest 0.13.3 exposes `rustls` as the current rustls TLS feature. Also cleaned small pre-existing clippy `collapsible_if` warnings in `src/services/overview_collector.rs` so the required final clippy gate is warning-free.

## Known Issues

None.

## Files Created/Modified

- `src/services/update.rs`
- `src/services/mod.rs`
- `Cargo.toml`
- `Cargo.lock`
- `src/services/overview_collector.rs`
