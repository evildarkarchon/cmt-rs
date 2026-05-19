# S01 UAT: Slint Shell Port Architecture

**Milestone:** M001  
**Slice:** S01  
**Backfilled by:** S11 validation traceability remediation  
**UAT type:** Backfilled developer/source-contract UAT record with automated gate evidence.  
**Execution status:** S11 did not manually run GUI UAT, did not use a real Fallout 4 install, and did not perform fresh desktop interaction testing for this backfill.

## Purpose

This artifact records the user-acceptance contract for the completed S01 shell foundation. It is not evidence of a newly executed manual desktop session. It backfills the missing S01 UAT file from `S01-SUMMARY.md`, `.planning/phases/01-slint-shell-port-architecture/01-VALIDATION.md`, and `.planning/phases/01-slint-shell-port-architecture/01-VERIFICATION.md`.

## Acceptance Records

| Area | Acceptance Contract | Evidence Status |
|---|---|---|
| Shell launch | Running the Rust crate should construct and run the generated Slint `MainWindow`. | Source-contract evidence: `src/main.rs` includes generated modules, calls `MainWindow::new()`, and runs the window; historical `cargo check`/`cargo test`/`cargo clippy` gates passed. |
| Application identity | The native window title should be `Collective Modding Toolkit`. | Source-contract evidence: `ui/main.slint` set the title, and Phase 1 verification recorded this as verified. |
| Tab order | The shell should expose tabs in this order: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`. | Automated/source-contract evidence: Rust shell-label tests and Slint source checks matched `CMT/src/enums.py` and `CMT/src/cm_checker.py`. |
| Tab behavior at S01 | Each tab should be selectable in the shell but intentionally inert, showing only reserved-for-later placeholder content. | Source-contract evidence: one Slint component per tab contained inert scope-note placeholder text, and S01 verification checked for absence of behavior keywords. |
| Module boundaries | Later behavior should have safe seams for app/controller, domain, platform, and worker code outside Slint markup. | Source-contract evidence: `src/app/mod.rs`, `src/domain/mod.rs`, `src/platform/mod.rs`, and `src/workers/mod.rs` were documented no-op boundaries; tests constructed boundary markers. |
| Reference safety | Implementing S01 should not mutate the read-only Python/Tkinter reference in `CMT/`. | Historical command evidence: S01 and Phase 1 records report `git status --short CMT` with no output. |
| Verification commands | The slice should pass the core Rust gates before closure. | Historical command evidence: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` passed in S01/Phase 1 records. |

## Developer UAT Procedure for Future Re-Run

If a future validator wants live GUI confirmation, use this procedure on a normal desktop environment:

1. Build and run the application from the Rust crate.
2. Confirm the visible window title is `Collective Modding Toolkit`.
3. Confirm the tabs appear in this exact order: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
4. Select each tab and confirm it shows only the S01 inert placeholder/scope-note content for that phase, with no scan, settings write, network, subprocess, archive, or game-file behavior.
5. Confirm no files under `CMT/` are modified by the run.

These are procedure steps, not claims that S11 executed them.

## Evidence Used for This Backfill

- `.gsd/milestones/M001/slices/S01/S01-SUMMARY.md` — completed S01 task summaries, files changed, decisions, stubs, gates, and read-only `CMT/` check.
- `.planning/phases/01-slint-shell-port-architecture/01-VALIDATION.md` — validation map, manual-only GUI caveats, shell-contract test map, and threat references.
- `.planning/phases/01-slint-shell-port-architecture/01-VERIFICATION.md` — verified observable truths for build/run shell, title/tab order, module boundaries, Cargo gates, and `CMT/` safety.

## Not Proven By This UAT Record

- Fresh S11 manual desktop launch or visual inspection.
- Pixel-perfect fidelity to the original Tkinter application.
- Real Fallout 4 installation, mod-manager, archive, scanner, downgrader, or network workflows.
- Later tab behavior implemented in S02-S10.
- Performance or responsiveness of long-running workflows beyond the S01 fact that no such workflows existed yet.
