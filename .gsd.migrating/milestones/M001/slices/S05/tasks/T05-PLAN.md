---
estimated_steps: 18
estimated_files: 17
skills_used: []
---

# T05: Run full S05 verification gates and CMT cleanliness check

---
estimated_steps: 5
estimated_files: 0
skills_used:
  - verify-before-complete
  - test
---
Why: S05 touches Slint resources, Rust action boundaries, workers, Cargo dependencies, and main entrypoint wiring. Closeout must prove the whole crate still builds/tests/lints and the read-only reference submodule stayed untouched.

Do:
1. Run formatting, compile, full test, and clippy gates after the last code/resource change.
2. Run a `CMT/` cleanliness check and report if the reference submodule has any modifications.
3. If any gate fails, fix the responsible S05 files only, then rerun the entire failed gate set after the fix. Do not mark the task complete with stale verification.

Done when: all listed commands exit 0 and the final response can quote fresh evidence for fmt, check, test, clippy, and CMT cleanliness.

Threat Surface (Q3): verifies no destructive utility workflow was accidentally enabled through source-contract tests and no reference files changed.
Requirement Impact (Q4): this is the final re-verification for shell, Settings, Overview, worker, action-boundary, and Slint resource integration touched by S05.
Failure Modes (Q5): build/test/clippy failure blocks completion; dirty `CMT/` status blocks completion until investigated and resolved without overwriting unrelated user changes.
Load Profile (Q6): full cargo gates exercise compile/test load only.
Negative Tests (Q7): full tests include focused negative cases from T01-T04 plus existing regression suites.

## Inputs

- `Cargo.toml`
- `Cargo.lock`
- `src/domain/tools.rs`
- `src/platform/clipboard.rs`
- `src/services/tools.rs`
- `src/app/tools_controller.rs`
- `src/app/about_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `src/main.rs`
- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `resources/images/icon-256.png`
- `resources/images/logo-nexusmods.png`
- `resources/images/logo-discord.png`
- `resources/images/logo-github.png`

## Expected Output

- Update the implementation and proof artifacts needed for this task.

## Verification

cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Observability Impact

No new runtime surface; this task validates that the observability and failure-state surfaces added earlier are covered by executable tests and compile in the real app.
