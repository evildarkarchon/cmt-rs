---
estimated_steps: 22
estimated_files: 5
skills_used: []
---

# T04: Add update and link services

---
estimated_steps: 8
estimated_files: 5
skills_used:
  - tdd
  - rust-async-patterns
  - verify-before-complete
---
Why: S04 must match the reference update banner behavior and safe open-only actions while keeping network and desktop behavior injectable.

Do:
1. Add `src/services/update.rs` with an `UpdateCheckClient` trait, real reqwest-backed implementation, fake test client, source-specific results, and an `UpdateCheckService` that respects `UpdateSource::{None,Nexus,Github,Both}`.
2. Add `reqwest` with rustls/json support to `Cargo.toml` and update `Cargo.lock`; keep network timeouts reference-compatible at about five seconds and avoid blocking Slint callbacks directly.
3. Implement Nexus and GitHub parsing to return `Some(newer_version)` only when the remote version is greater than `env!("CARGO_PKG_VERSION")` or a test-injected current version; malformed, no-update, timeout, and HTTP failures return silent no-banner state plus diagnostics/logs.
4. Add update banner link metadata using the reference Nexus and GitHub URLs from `CMT/src/globals.py`.
5. Add or extend Overview action facts for `DesktopActions` results so game-path open and update-link open failures can be projected as safe visible messages.
6. Export the update service from `src/services/mod.rs` and feed its typed state into the Overview snapshot contracts from T01/T02.
7. Add tests for update_source none skipping all clients, nexus only, github only, both sources, newer version banner, equal/older versions silent, malformed response silent, network failure silent with diagnostics, and desktop action failure feedback.

Done when: update and open-only behavior is fully fake-tested and exposes no banner unless a selected source reports a newer version.

Threat Surface Q3: remote HTML/JSON is untrusted; parse minimally, cap diagnostics, and never render raw response bodies in UI.
Failure Modes Q5: network timeout, 4xx/5xx, invalid JSON, missing version metadata, invalid semver, and desktop-open failure are silent or safe-message states.
Load Profile Q6: at most two update requests per refresh when update_source is Both; no retry loops in S04.
Negative Tests Q7: disabled source, malformed GitHub JSON, Nexus page without version meta, invalid version string, timeout/client error, and failed URL/path open.

## Inputs

- `src/domain/overview.rs`
- `src/domain/settings.rs`
- `src/platform/desktop.rs`
- `CMT/src/cm_checker.py`
- `CMT/src/globals.py`
- `CMT/src/utils.py`
- `Cargo.toml`
- `Cargo.lock`

## Expected Output

- `src/services/update.rs`
- `src/services/mod.rs`
- `src/domain/overview.rs`
- `Cargo.toml`
- `Cargo.lock`

## Verification

cargo test overview_update

## Observability Impact

Adds source-specific update-check diagnostics and safe desktop-action feedback without surfacing no-update or network-failure banners to users.
