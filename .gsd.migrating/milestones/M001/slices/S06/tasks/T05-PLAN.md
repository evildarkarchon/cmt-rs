---
estimated_steps: 11
estimated_files: 13
skills_used: []
---

# T05: Run S06 quality gates

Expected executor skills for task-plan frontmatter: verify-before-complete.

Why: S06 touches dependencies, PE parsing, background workers, MainWindow wiring, and Slint UI. Completion must be supported by fresh full-gate evidence and CMT must remain read-only.

Do:
1. Run the full required project gates after T01 through T04 are complete.
2. Run focused S06 filters if any full-gate failure needs localization.
3. Verify that CMT remains unmodified.
4. If failures reveal a real implementation defect, fix the owning task files rather than weakening tests or scope.
5. Capture the exact pass or fail command evidence in the task summary.

Failure Modes Q5: formatting, compilation, unit/source-contract regressions, clippy warnings, and accidental CMT modifications all block task completion.

Negative Tests Q7: the full cargo test suite must include the S06 malformed DLL, missing folder, empty folder, ignored msdia, unknown game, stale worker, and source-contract cases from prior tasks.

Done when: all required commands pass and git status --short CMT is empty.

## Inputs

- `Cargo.toml`
- `Cargo.lock`
- `src/domain/f4se.rs`
- `src/services/f4se.rs`
- `src/app/f4se_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/main.rs`
- `ui/f4se_tab.slint`
- `ui/main.slint`
- `CMT/src/tabs/_f4se.py`
- `CMT/src/utils.py`
- `CMT/src/globals.py`

## Expected Output

- Update the implementation and proof artifacts needed for this task.

## Verification

cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Observability Impact

Produces final evidence that the observable UI/controller/worker failure states and all regression gates remain healthy.
