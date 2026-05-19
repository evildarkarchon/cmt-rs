---
id: T05
parent: S09
milestone: M001
key_files:
  - ui/downgrader_window.slint
  - ui/main.slint
  - ui/overview_tab.slint
  - ui/tools_tab.slint
  - src/main.rs
key_decisions:
  - Recorded MEM068: Downgrader Slint source is treated as a source contract validated by Rust tests, and runtime close/Escape handling must honor the close-blocked contract because Slint markup alone does not reproduce Tk WM_DELETE/Escape veto behavior.
duration: 
verification_result: passed
completed_at: 2026-05-18T22:22:59.076Z
blocker_discovered: false
---

# T05: Verified the Downgrader Slint modal source contract, generated-component import, Overview/Tools entrypoint callbacks, and source-contract tests for reference labels and deferred Archive Patcher behavior.

**Verified the Downgrader Slint modal source contract, generated-component import, Overview/Tools entrypoint callbacks, and source-contract tests for reference labels and deferred Archive Patcher behavior.**

## What Happened

Inspected the existing `ui/downgrader_window.slint`, `ui/main.slint`, `ui/overview_tab.slint`, `ui/tools_tab.slint`, `src/main.rs`, and the read-only Python reference files (`CMT/src/downgrader.py`, `CMT/src/globals.py`, `CMT/src/modal_window.py`). The planned source contract was already present: `DowngraderWindow` is a conservative fixed-size 600x334 Slint window titled `Downgrader`, preserves the Current Game, Current Creation Kit, Desired Version, Options, Patch\n All, About, log, progress, and inline plan/confirmation surfaces, uses basename display for Archive2 rows, and exposes controller-facing properties/callbacks for target/options, patch/confirm/about/close, status rows, plan rows, log rows, progress text/percent, patch/about enablement, and close-blocked state. `ui/main.slint` imports the generated Downgrader component and forwards Overview/Tools downgrade-manager callbacks, while Archive Patcher remains disabled/deferred. Existing source-contract tests in `src/main.rs` cover required labels, title/size, callback/property names, missing Anniversary target option, inline confirmation copy, and deferred Archive Patcher text. No source edits were necessary because the workspace already contained the planned T05 implementation.

## Verification

Ran the task-required contract test filter `cargo test s09_downgrader_slint_contract`, which passed both Downgrader source-contract tests. Ran `cargo check`, which compiled Slint and Rust successfully. Also ran `cargo fmt --check` as a formatting guard; it passed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test s09_downgrader_slint_contract` | 0 | ✅ pass | 43851ms |
| 2 | `cargo check` | 0 | ✅ pass | 19777ms |
| 3 | `cargo fmt --check` | 0 | ✅ pass | 705ms |

## Deviations

No source edits were needed; the planned files and tests already existed in the workspace and passed verification. The Skill activation tool requested by the execution rules was not exposed in this tool environment, so skill calls could not be made.

## Known Issues

No new issues discovered. Runtime modal opening and real Downgrader worker wiring remain for the remaining S09 task(s).

## Files Created/Modified

- `ui/downgrader_window.slint`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
- `src/main.rs`
