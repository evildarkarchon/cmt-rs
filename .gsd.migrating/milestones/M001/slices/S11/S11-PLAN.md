# S11: Validation Traceability Remediation

**Goal:** Repair M001 validation traceability and missing validation artifacts without changing Rust/Slint product behavior, then rerun final documentation and Cargo gates so validation round 1 can trust the evidence.
**Demo:** Validation round 1 shows requirement traceability repaired, missing assessment and UAT artifacts present, S07 integration attribution corrected, and quality gates still passing.

## Must-Haves

- R001-R054 in `.gsd/REQUIREMENTS.md` have meaningful non-placeholder titles/descriptions, evidence-based statuses, primary owning slices, supporting evidence, and non-`unmapped` proof strings.
- S01 has honest backfilled `S01-ASSESSMENT.md` and `S01-UAT.md` artifacts based on completed S01 evidence, with no claim of manual GUI/game-install UAT unless actually performed.
- S10 assessment/UAT artifact presence is checked and any limitations are represented honestly in S11 closeout rather than overstated.
- S07 dependency/provenance documentation correctly credits S01 for the main shell/tab wiring and S02 for settings persistence/scanner settings.
- Final verification runs include the slice-local artifact verifier, `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`, and `git status --short CMT` with no CMT modifications.
- M001 validation round 1 is recorded as passing, or any remaining blockers are precisely documented as validation blockers rather than hidden.

## Proof Level

- This slice proves: Final-assembly documentation and validation proof. The slice proves auditability of the completed M001 port through requirement traceability, missing-artifact remediation, corrected provenance, and fresh quality-gate evidence. Real desktop/game-install runtime and human UAT are not required for S11 unless the executor actually performs and records them; simulated manual evidence is forbidden.

## Integration Closure

S11 consumes completed S01-S10 summaries/task summaries, the v1 requirements source in `.planning/REQUIREMENTS.md`, and existing GSD milestone state. It introduces no product runtime wiring. Integration is closed when repaired traceability, artifacts, dependency attribution, Cargo gates, clean CMT status, and milestone validation evidence all agree that M001 can proceed to completion, or when any validation blocker is explicitly carried forward.

## Verification

- Improves agent-facing observability of milestone closure: `.gsd/REQUIREMENTS.md` becomes a readable traceability matrix, S01/S11 UAT and assessment artifacts expose evidence class and caveats, the slice-local verifier gives mechanical failure localization, and final validation/gate outputs provide a current audit trail.

## Tasks

- [x] **T01: Rebuild requirement traceability from completed evidence** `est:2h`
  Expected executor skills: write-docs, verify-before-complete.
  - Files: `.gsd/REQUIREMENTS.md`, `.gsd/milestones/M001/slices/S11/verify_s11_artifacts.py`
  - Verify: python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --requirements

- [x] **T02: Backfill missing S01 UAT and assessment artifacts** `est:1h`
  Expected executor skills: write-docs, verify-before-complete.
  - Files: `.gsd/milestones/M001/slices/S01/S01-ASSESSMENT.md`, `.gsd/milestones/M001/slices/S01/S01-UAT.md`
  - Verify: python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --artifacts

- [x] **T03: Correct completed-slice provenance attribution** `est:30m`
  Expected executor skills: write-docs, verify-before-complete.
  - Files: `.gsd/milestones/M001/slices/S07/S07-SUMMARY.md`
  - Verify: python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --provenance

- [x] **T04: Run final gates and record validation round 1** `est:1.5h`
  Expected executor skills: verify-before-complete, review, write-docs.
  - Files: `.gsd/milestones/M001/slices/S11/S11-UAT.md`, `.gsd/milestones/M001/slices/S11/S11-ASSESSMENT.md`, `.gsd/milestones/M001/slices/S11/S11-SUMMARY.md`, `.gsd/milestones/M001/M001-VALIDATION.md`
  - Verify: python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --all
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Files Likely Touched

- .gsd/REQUIREMENTS.md
- .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py
- .gsd/milestones/M001/slices/S01/S01-ASSESSMENT.md
- .gsd/milestones/M001/slices/S01/S01-UAT.md
- .gsd/milestones/M001/slices/S07/S07-SUMMARY.md
- .gsd/milestones/M001/slices/S11/S11-UAT.md
- .gsd/milestones/M001/slices/S11/S11-ASSESSMENT.md
- .gsd/milestones/M001/slices/S11/S11-SUMMARY.md
- .gsd/milestones/M001/M001-VALIDATION.md
