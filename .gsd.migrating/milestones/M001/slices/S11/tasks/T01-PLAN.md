---
estimated_steps: 9
estimated_files: 2
skills_used: []
---

# T01: Rebuild requirement traceability from completed evidence

Expected executor skills: write-docs, verify-before-complete.

Why: M001 validation is blocked because `.gsd/REQUIREMENTS.md` still contains placeholder `Untitled` records and `unmapped` trace rows for most R001-R054 requirements, even though implementation slices S01-S10 are complete. The executor must make the requirements file audit-grade, not merely checker-satisfying.

Do: Use `.planning/REQUIREMENTS.md` as the canonical source for v1 requirement titles and descriptions, preserving the known ID mapping by list order: R001-R005 FOUND, R006-R011 SET, R012-R016 DISC, R017-R024 OVR, R025-R029 F4SE, R030-R039 SCAN, R040-R045 TOOL, R046-R049 ABOUT, and R050-R054 SAFE. For every R001-R054 record, update through the GSD requirement-update/render path if the harness exposes it; if no DB-backed requirement update tool is available, make a surgical Markdown remediation and document that fallback in the task summary. Assign primary owning slices from the original phase/slice mapping, add supporting slices when later workflows satisfy the requirement (for example Overview Downgrade Manager via S09 and Archive Patcher via S10), and write proof strings tied to completed slice summaries, task summaries, source-contract/runtime tests, and recorded Cargo gates. Preserve the S02 discrepancy note for R009: the Rust port intentionally validates `Warning` in addition to the Python tab's visible Debug/Info/Error labels because the reference settings schema accepts persisted WARNING.

Also create a slice-local verifier script at `.gsd/milestones/M001/slices/S11/verify_s11_artifacts.py`. It must be a small Python check with modes such as `--requirements`, `--artifacts`, `--provenance`, and `--all`; `--requirements` should fail on missing R001-R054 IDs, `Untitled`, `unmapped`, missing primary owners, missing proof text, or inconsistent active/validated counts. Keep the verifier under `.gsd/` so product tests do not read GSD/planning paths.

Q3 Threat surface: the main abuse case is false or overstated validation evidence. Do not mark a requirement validated unless completed slice evidence exists; if evidence is genuinely missing, leave the status active and record the gap. No secrets or user data should be introduced.
Q4 Requirement impact: touches all R001-R054 traceability records; does not change product requirements or decisions.
Q5 Failure modes: if GSD requirement-update tooling is unavailable, fall back to direct Markdown remediation and make the fallback explicit; if a source summary is missing, use the artifact inventory and stop rather than invent evidence.
Q7 Negative checks: the verifier must fail on placeholder titles, `unmapped` proof, missing owners, and duplicate/missing IDs.

Done when: `.gsd/REQUIREMENTS.md` is readable by a fresh reviewer, R001-R054 are present with meaningful titles/proof/owners, no placeholder trace rows remain, and `python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --requirements` passes.

## Inputs

- `.planning/REQUIREMENTS.md`
- `.gsd/REQUIREMENTS.md`
- `.gsd/STATE.md`
- `.gsd/state-manifest.json`
- `.gsd/milestones/M001/M001-ROADMAP.md`
- `.gsd/milestones/M001/slices/S11/S11-CONTEXT.md`
- `.gsd/milestones/M001/slices/S11/S11-RESEARCH.md`
- `.gsd/milestones/M001/slices/S01/S01-SUMMARY.md`
- `.gsd/milestones/M001/slices/S02/S02-SUMMARY.md`
- `.gsd/milestones/M001/slices/S03/S03-SUMMARY.md`
- `.gsd/milestones/M001/slices/S04/S04-SUMMARY.md`
- `.gsd/milestones/M001/slices/S05/S05-SUMMARY.md`
- `.gsd/milestones/M001/slices/S06/S06-SUMMARY.md`
- `.gsd/milestones/M001/slices/S07/S07-SUMMARY.md`
- `.gsd/milestones/M001/slices/S08/S08-SUMMARY.md`
- `.gsd/milestones/M001/slices/S09/S09-SUMMARY.md`
- `.gsd/milestones/M001/slices/S10/S10-SUMMARY.md`

## Expected Output

- `.gsd/REQUIREMENTS.md`
- `.gsd/milestones/M001/slices/S11/verify_s11_artifacts.py`

## Verification

python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --requirements

## Observability Impact

Turns requirement state into an inspectable traceability matrix and adds a mechanical verifier that localizes missing owners/proofs/placeholders before milestone validation is rerun.
