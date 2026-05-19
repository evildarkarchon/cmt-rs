---
estimated_steps: 10
estimated_files: 4
skills_used: []
---

# T04: Run final gates and record validation round 1

Expected executor skills: verify-before-complete, review, write-docs.

Why: S11 is complete only when the repaired traceability and artifacts survive both mechanical documentation checks and the standard Rust quality gates. The final result must be recorded in S11 closeout artifacts and M001 validation evidence, with honest caveats for any UAT that was not actually performed.

Do: Run the slice-local verifier in `--all` mode after T01-T03. Run fresh project gates from `J:/cmt-rs`: `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`, and `git status --short CMT`. The CMT status command must produce no output; if it does, stop and report the read-only submodule violation rather than proceeding. Do not change Rust/Slint product behavior as part of this task; if a Cargo gate fails due to product code, investigate as a blocker and do not hide it in documentation.

Create final S11 closeout artifacts: `S11-UAT.md` should state exactly which verification was run and which evidence class applies; `S11-ASSESSMENT.md` should assess remediation completeness, requirement coverage, S10 artifact acceptability, S07 attribution repair, and remaining limitations; `S11-SUMMARY.md` should summarize work performed and gate results. Then run milestone validation round 1 through the GSD validation tool if available, writing `.gsd/milestones/M001/M001-VALIDATION.md`; if the tool renders a different validation path, record the actual path in the S11 summary. If validation returns `needs-remediation`, do not mark the milestone complete; preserve the precise blockers and proposed remediation.

Q3 Threat surface: final validation can be abused by overstating proof. Evidence must include fresh command outputs and must not claim manual real-install UAT unless performed.
Q4 Requirement impact: re-verifies all R001-R054 traceability and milestone success criteria; no product requirements are changed.
Q5 Failure modes: Cargo gate failure blocks completion; validation-tool absence requires a manually written validation evidence artifact plus explicit note; dirty CMT status blocks completion; missing final artifacts fail the verifier.
Q6 Load profile: Cargo test/clippy are the only heavy operations and run locally; no network, secrets, or external services are required.
Q7 Negative checks: `--all` verifier must catch missing artifacts/placeholders/provenance regression, and CMT status must catch accidental submodule modifications.

Done when: all final commands pass, CMT remains clean, S11 UAT/assessment/summary and M001 validation evidence are written with honest caveats, and either validation round 1 passes or remaining blockers are explicit.

## Inputs

- `.gsd/REQUIREMENTS.md`
- `.gsd/milestones/M001/M001-ROADMAP.md`
- `.gsd/milestones/M001/slices/S11/S11-CONTEXT.md`
- `.gsd/milestones/M001/slices/S11/S11-RESEARCH.md`
- `.gsd/milestones/M001/slices/S11/verify_s11_artifacts.py`
- `.gsd/milestones/M001/slices/S01/S01-ASSESSMENT.md`
- `.gsd/milestones/M001/slices/S01/S01-UAT.md`
- `.gsd/milestones/M001/slices/S07/S07-SUMMARY.md`
- `.gsd/milestones/M001/slices/S10/S10-ASSESSMENT.md`
- `.gsd/milestones/M001/slices/S10/S10-UAT.md`
- `Cargo.toml`
- `Cargo.lock`
- `src`
- `ui`
- `CMT`

## Expected Output

- `.gsd/milestones/M001/slices/S11/S11-UAT.md`
- `.gsd/milestones/M001/slices/S11/S11-ASSESSMENT.md`
- `.gsd/milestones/M001/slices/S11/S11-SUMMARY.md`
- `.gsd/milestones/M001/M001-VALIDATION.md`

## Verification

python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --all
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Observability Impact

Produces fresh closeout and validation artifacts plus command-gate evidence, making any remaining milestone blocker inspectable from a single validation trail.
