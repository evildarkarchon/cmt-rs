---
estimated_steps: 19
estimated_files: 10
skills_used: []
---

# T06: Close with safety regression and quality gates

---
estimated_steps: 5
estimated_files: 0
skills_used:
  - verify-before-complete
  - review
  - security-review
---
Why: This mutation workflow should not be marked complete until all focused tests, adjacent regression tests, and required Rust quality gates have fresh evidence in the current execution context.

Do:
1. Run all focused Archive Patcher tests from the earlier tasks and fix any remaining failures in the owning task files rather than weakening assertions.
2. Run adjacent Overview, Tools, Settings, and Worker tests to catch regressions in shared entrypoints and handoff state.
3. Run the project quality gates required by AGENTS.md.
4. Perform a safety review pass over filesystem mutation code: no full-archive reads for patching, no writes before manifest creation, no mutation from UI callbacks, no `unwrap()` or `expect()` in production mutation paths, and no writes under `CMT/`.
5. Document any intentional reference deviations in the slice summary when execution completes.

Done when: all verification commands exit 0 with fresh output, or any unavailable check is explicitly reported with the concrete reason.

Failure Modes Q5: If any quality gate fails, stop and fix the implementation or record a blocker rather than claiming completion.
Load Profile Q6: Full `cargo test` and clippy are expected to be the longest checks; no new network or external game install should be required.
Negative Tests Q7: The aggregate test suite must still include and pass the malformed header, missing file, permission, stale manifest, digest mismatch, stale worker event, empty candidate, and no-discovery cases.

## Inputs

- `src/domain/archive_patcher.rs`
- `src/services/archive_patcher.rs`
- `src/app/archive_patcher_controller.rs`
- `src/workers/events.rs`
- `src/platform/filesystem.rs`
- `src/main.rs`
- `ui/archive_patcher_window.slint`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`

## Expected Output

- Update the implementation and proof artifacts needed for this task.

## Verification

cargo test archive_patcher_domain --quiet
cargo test archive_patcher_service_plan --quiet
cargo test archive_patcher_executor --quiet
cargo test archive_patcher_controller --quiet
cargo test archive_patcher_worker_payload --quiet
cargo test s10_archive_patcher_slint_contract --quiet
cargo test s10_archive_patcher_runtime_wiring --quiet
cargo test overview --quiet
cargo test tools --quiet
cargo test worker --quiet
cargo fmt --check
cargo check --quiet
cargo test --quiet
cargo clippy --all-targets --all-features --quiet

## Observability Impact

Confirms that the user-visible and tracing surfaces added in earlier tasks survive integration and adjacent regression tests.
