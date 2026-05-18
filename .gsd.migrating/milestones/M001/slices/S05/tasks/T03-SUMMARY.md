---
id: T03
parent: S05
milestone: M001
key_files:
  - ui/tools_tab.slint
  - ui/about_tab.slint
  - ui/main.slint
  - src/main.rs
key_decisions:
  - Keep static Tools/About URLs out of Slint and emit only stable Rust-owned action ids from the UI.
  - Keep Downgrade Manager and Archive Patcher visible but disabled in Slint with explicit S09/S10 deferral text.
  - Use per-row About copy label/enabled properties plus 3000ms Slint Timers to request copy-label resets after success.
duration: 
verification_result: passed
completed_at: 2026-05-18T01:54:59.017Z
blocker_discovered: false
---

# T03: Replaced the Tools and About Slint placeholders with reference-shaped tabs, safe status surfaces, stable action-id callbacks, image assets, copy timers, and source-contract tests.

**Replaced the Tools and About Slint placeholders with reference-shaped tabs, safe status surfaces, stable action-id callbacks, image assets, copy timers, and source-contract tests.**

## What Happened

Implemented declarative Slint surfaces for the S05 Tools and About tabs. `ui/tools_tab.slint` now has the three reference labelframe-style groups, preserved button labels/order, helper text, dark palette continuity, a visible safe-error banner, stable callback ids, and disabled Downgrade Manager/Archive Patcher entries with explicit S09/S10 deferral text. `ui/about_tab.slint` now renders the reference title split, Rust-owned icon/logo resources, version/credit text, Nexus/Discord/GitHub rows, open/copy buttons, safe-error banner, per-row copy label/enabled properties, and 3000ms reset timers that emit stable copy action ids. `ui/main.slint` forwards the new Tools/About properties and callbacks without embedding static URLs. `src/main.rs` source-contract tests now remove Tools/About from the inert placeholder set and lock the S05 visual/action surface, disabled utility markers, image references, callback forwarding, copy timers, and no-direct-URL Slint boundary. An initial contract test failure exposed an over-broad `CMT/` marker assertion against an existing reference-source comment in `main.slint`; I narrowed that guard to raw URLs/webbrowser calls/image URLs into `CMT/`, which matches the task threat model.

## Verification

Ran the focused task verification `cargo test s05_slint_contract` after implementation and after rustfmt; the final focused run passed with 3 S05 Slint contract tests. Also ran the project quality gates relevant to the shared shell source: `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features`; all passed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo fmt --check` | 0 | ✅ pass | 445ms |
| 2 | `cargo check` | 0 | ✅ pass | 17707ms |
| 3 | `cargo test s05_slint_contract` | 0 | ✅ pass | 33551ms |
| 4 | `cargo test` | 0 | ✅ pass | 8425ms |
| 5 | `cargo clippy --all-targets --all-features` | 0 | ✅ pass | 19027ms |

## Deviations

None.

## Known Issues

Runtime wiring for the new callbacks/properties is intentionally deferred to T04; T03 only exposes and forwards the Slint surface.

## Files Created/Modified

- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `src/main.rs`
