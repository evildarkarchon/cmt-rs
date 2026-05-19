# S11 Research: Validation Traceability Remediation

## Summary

S11 is a documentation/validation remediation slice, not a product-behavior slice. The implementation slices S01-S10 are complete and current milestone state reports S11 as the only pending slice; the blocker is that the GSD requirement and validation artifacts do not yet reflect the completed evidence.

The main discovery is that `.planning/REQUIREMENTS.md` is the reliable source for meaningful requirement names. It contains the original 54 v1 requirements (`FOUND-*`, `SET-*`, `DISC-*`, `OVR-*`, `F4SE-*`, `SCAN-*`, `TOOL-*`, `ABOUT-*`, `SAFE-*`). The current `.gsd/REQUIREMENTS.md` has the same count but almost all records are rendered as `Untitled`, many are still `active`, and the traceability table says `none` / `unmapped` for most rows. The ID mapping is by list order: R001-R005 are `FOUND-01..05`, R006-R011 are `SET-01..06`, R012-R016 are `DISC-01..05`, R017-R024 are `OVR-01..08`, R025-R029 are `F4SE-01..05`, R030-R039 are `SCAN-01..10`, R040-R045 are `TOOL-01..06`, R046-R049 are `ABOUT-01..04`, and R050-R054 are `SAFE-01..05`.

Artifact inventory found the expected validation blockers:

- `.gsd/REQUIREMENTS.md`: 54 unique IDs, 53 `Untitled`, 42 active, trace table mostly `unmapped`.
- `S01`: missing `S01-UAT.md` and `S01-ASSESSMENT.md`; S01 summary plus `.planning/phases/01-*/01-VALIDATION.md` and `01-VERIFICATION.md` have enough evidence to backfill them honestly.
- `S02` through `S10`: UAT and assessment files are present.
- `S10`: `S10-ASSESSMENT.md` exists but is a short remediation-round note; `S10-UAT.md` exists and is a UAT procedure. Because S10 summary explicitly says no manual real-install UAT was performed, any S11 repair should avoid implying that the S10 UAT was manually executed.
- `S07-SUMMARY.md` has the known attribution error: its frontmatter says S02 provides `Main shell, settings persistence baseline, and tab wiring patterns`. It should credit S01 for the main shell/tab wiring and S02 for settings persistence/scanner settings.

No product code changes are needed. `git status --short CMT` returned no output during research.

## Skills Discovered

Installed/relevant skills from the prompt:

- `write-docs` is relevant for producing stranger-readable traceability and UAT/assessment text.
- `verify-before-complete` is relevant for the final S11 closeout because fresh evidence must be produced before claiming the milestone is ready.
- `review` is useful before final validation because the slice changes audit artifacts that validators will consume.

Skill marketplace checks performed:

- `npx skills find "Slint"` returned unrelated lint/accessibility/web skills; none installed.
- `npx skills find "Rust Cargo"` returned `cargo-fuzz` and low-install generic Rust skills; none were directly relevant to this docs/traceability remediation, so none installed.

No external library docs were needed; this slice works against local GSD/planning artifacts and existing Cargo gates.

## Active Requirements This Slice Supports

S11 does not own new product capabilities. It supports all currently active R012-R053 by repairing their status/traceability to point at the completed implementation slices. Based on the completed slice summaries and the original `.planning/REQUIREMENTS.md` mapping, all R001-R054 can be represented as `validated` if proof text is honest about the evidence class. Do not claim manual desktop/game-install UAT where only automated, source-contract, fake-backed, or procedure-level evidence exists.

## Implementation Landscape

### Files and purpose

- `.planning/REQUIREMENTS.md` — canonical source of old requirement titles/descriptions and phase mapping. Use this instead of inventing requirement names.
- `.gsd/REQUIREMENTS.md` — primary remediation target. Prefer the GSD requirement update/render path if available; direct file edits risk being overwritten if the DB is later rendered again.
- `.gsd/STATE.md` — currently reports `42 active · 12 validated`; should reflect the repaired state after requirement updates/rendering.
- `.gsd/milestones/M001/slices/S01/S01-SUMMARY.md` — contains S01 completed evidence for Slint build pipeline, tab order, module boundaries, gates, and untouched `CMT/`.
- `.planning/phases/01-slint-shell-port-architecture/01-VALIDATION.md` — strongest source for S01 verification map, manual-only caveats, and green checks.
- `.planning/phases/01-slint-shell-port-architecture/01-VERIFICATION.md` — strongest source for S01 assessment/UAT proof, including observable truths and requirement coverage.
- `.gsd/milestones/M001/slices/S01/S01-ASSESSMENT.md` — missing target; backfill from S01 summary + old phase validation/verification.
- `.gsd/milestones/M001/slices/S01/S01-UAT.md` — missing target; write as source-contract/developer UAT. Clearly state manual GUI UAT was not performed in S11 if it was not.
- `.gsd/milestones/M001/slices/S07/S07-SUMMARY.md` — fix frontmatter `requires` attribution.
- `.gsd/milestones/M001/slices/S10/S10-ASSESSMENT.md` and `S10-UAT.md` — present; inspect/possibly annotate for clarity that S10 UAT is procedure/automated evidence unless actually run.
- `.gsd/milestones/M001/slices/S11/*` — final S11 summary/UAT/assessment will need to record remediation and final gates.

### Natural seams for execution planning

1. **Requirement traceability repair**: use `.planning/REQUIREMENTS.md` to fill titles/descriptions/status/owners/supporting/proof for R001-R054 in `.gsd/REQUIREMENTS.md` / GSD requirement records.
2. **Missing S01 artifacts**: create `S01-ASSESSMENT.md` and `S01-UAT.md` from S01 summary and old Phase 1 validation/verification evidence.
3. **Completed-slice provenance repair**: patch `S07-SUMMARY.md` frontmatter so S01 owns main shell/tab wiring while S02 owns settings persistence. Review S10 artifacts for any misleading manual-UAT implication.
4. **Audit verifier and final gates**: run a small artifact/traceability script first, then the required Cargo gates and `git status --short CMT`.
5. **Validation closeout**: rerun/prepare M001 validation round 1 after artifacts and gates are green.

## Suggested Requirement Traceability Matrix

Use the proof strings as starting points; expand them where the requirements tool/renderer has room. Suggested status for all rows: `validated`, with evidence class explicit.

| ID | Old req | Requirement title | Primary owner | Supporting slices | Proof basis |
|---|---|---|---|---|---|
| R001 | FOUND-01 | Developer can build and run a Slint desktop application from the Rust crate. | S01 | none | S01 Slint dependency/build pipeline, `build.rs`, `ui/main.slint`, `src/main.rs`, and final Cargo gates passed. |
| R002 | FOUND-02 | User sees `Collective Modding Toolkit` identity and tab order `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`. | S01 | S02-S10 preserve shell | S01 tab-order/source-contract tests and CMT references in `CMT/src/enums.py` / `cm_checker.py`; later slices preserved the shell. |
| R003 | FOUND-03 | Developer can add behavior through separated UI, app/controller, domain, platform, and worker modules. | S01 | S03, S04-S10 | S01 module boundaries; later slices use app/domain/services/platform/workers without placing domain logic in Slint. |
| R004 | FOUND-04 | Developer can run core verification commands for the current slice. | S01 | S02-S10 | S01 ran the four core gates; every completed slice summary records passing relevant Cargo gates. |
| R005 | FOUND-05 | Developer can verify implementation changes do not modify files under `CMT/`. | S01 | S02-S10 | S01 and later summaries treat CMT as read-only; research `git status --short CMT` returned no output. |
| R006 | SET-01 | Settings load with reference-compatible defaults when no settings file exists. | S02 | none | S02 settings domain/store tests and full gates. |
| R007 | SET-02 | Settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`. | S02 | S07, S09 | S02 JSON wire tests; S07 consumes scanner settings and S09 consumes downgrader preferences. |
| R008 | SET-03 | Update channel choices match reference labels. | S02 | S04 | S02 Slint source contract tests; S04 update checks consume `update_source`. |
| R009 | SET-04 | Log level choices match validated settings contract. | S02 | none | S02 re-scoped to include `Warning`; source/domain tests and gates passed. |
| R010 | SET-05 | Scanner settings default to enabled for reference categories. | S02 | S07 | S02 domain tests; S07 UI/controller consumes scanner settings. |
| R011 | SET-06 | Invalid/incomplete settings fail safely, preserving valid values and repairing defaults. | S02 | none | S02 domain/store/controller repair and rollback tests. |
| R012 | DISC-01 | App can discover or represent the Fallout 4 game path for downstream workflows. | S03 | S04, S06, S07, S09, S10 | S03 DiscoveryService tests; downstream slices consume discovery/game path. |
| R013 | DISC-02 | App can identify mod-manager context and display Mod Manager, Game Path, Version, PC Specs. | S04 | S03 | S03 discovery/mod-manager/system metadata; S04 Overview displays the summary rows. |
| R014 | DISC-03 | App reads file/directory sources through injectable filesystem adapters. | S03 | S04, S06, S07, S09, S10 | S03 platform filesystem seam; all later read/mutation slices use fakeable adapters. |
| R015 | DISC-04 | App launches URLs/paths/tools through injectable adapters with visible failure reporting. | S05 | S03, S04, S07 | S03 desktop/tool action seam; S05 Tools/About visible link/copy failures; S04/S07 safe open/copy actions. |
| R016 | DISC-05 | App performs update checks according to `update_source` without blocking UI/startup. | S04 | S02, S03 | S02 setting, S03 worker handoff, S04 update-check service and Overview banner tests. |
| R017 | OVR-01 | Overview shows `Mod Manager:`, `Game Path:`, `Version:`, `PC Specs:`. | S04 | S03 | S04 source contract/projection tests and Overview summary rows. |
| R018 | OVR-02 | Overview Binaries panel shows game/F4SE/Creation Kit and Address Library status. | S04 | S03 | S04 collector/diagnostics tests and summary. |
| R019 | OVR-03 | User can open Downgrade Manager from Overview binaries panel. | S09 | S04 | S04 deferred control; S09 live modal from Overview and Tools with runtime tests. |
| R020 | OVR-04 | Overview Archives panel shows General, Texture, Total, Unreadable, OG, NG counts. | S04 | S10 | S04 archive diagnostics; S10 consumes archive records. |
| R021 | OVR-05 | User can open Archive Patcher from Overview archives panel. | S10 | S04, S05 | S04 deferred control; S10 live Archive Patcher from Overview and Tools with runtime tests. |
| R022 | OVR-06 | Overview Modules panel shows Full, Light, Total, HEDR v1.00, v0.95, unknown counts. | S04 | none | S04 module collector/diagnostics and UI source contract tests. |
| R023 | OVR-07 | Overview diagnostics produce typed problem records for Scanner Overview Issues. | S04 | S07 | S04 `OverviewProblem` feed; S07 consumes Overview-derived issues. |
| R024 | OVR-08 | Overview refresh/update-banner preserves update source semantics and links. | S04 | S02, S05 | S04 update service/banner/link behavior; S05 desktop link failure pattern. |
| R025 | F4SE-01 | User can open F4SE tab and scan `Data/F4SE/Plugins` DLLs. | S06 | S03, S04 | S06 lazy/read-only F4SE tab and scan service tests. |
| R026 | F4SE-02 | F4SE table columns are `DLL`, `OG`, `NG`, `AE`, `Your Game`. | S06 | none | S06 Slint/source contract tests. |
| R027 | F4SE-03 | F4SE compatibility status is reference-compatible across game generations. | S06 | S03 | S06 domain/service/DLL inspector tests. |
| R028 | F4SE-04 | Missing Data or plugin folder guidance is reference-compatible. | S06 | S03 | S06 safe missing-folder status/error tests. |
| R029 | F4SE-05 | F4SE scanning runs without blocking the Slint UI thread. | S06 | S03 | S06 worker payload/lazy activation/runtime wiring tests. |
| R030 | SCAN-01 | Scanner tab shows `Scan Game`, `Scan Settings`, `Collapse All`, `Expand All`. | S07 | S01 | S07 Slint source contract and UI tests. |
| R031 | SCAN-02 | User can enable/disable scanner categories matching defaults/settings keys. | S07 | S02 | S02 settings contract; S07 scanner settings UI/controller persistence-on-scan. |
| R032 | SCAN-03 | User can start scan and see progress/status without blocking UI. | S07 | S03 | S07 controller/worker/progress tests and runtime wiring. |
| R033 | SCAN-04 | Scanner builds mod file list from game/mod-manager context with supported attribution. | S07 | S03 | S07 scan service MO2 attribution and Vortex Data-only handling tests. |
| R034 | SCAN-05 | Scanner classifies reference problem types. | S07 | S04, S06 | S07 scanner domain/service tests for taxonomy and overview/F4SE-related inputs. |
| R035 | SCAN-06 | Scan results are grouped/expandable with problem/files detail information. | S07 | none | S07 grouped row model/details tests and Slint contract. |
| R036 | SCAN-07 | Selecting a result shows `Mod:`, `Problem:`, `Summary:`, `Solution:` details. | S07 | none | S07 controller/detail projection and UI tests. |
| R037 | SCAN-08 | Detail actions support URL open/copy and `Copy Details`. | S07 | S05 | S07 read-only actions through desktop/clipboard adapters. |
| R038 | SCAN-09 | Auto-fix actions appear only where supported and show `Fixed!` / `Fix Failed`. | S08 | S07 | S08 fail-closed Auto-Fix domain/service/controller/worker/Slint/runtime tests. |
| R039 | SCAN-10 | Scanner results can include Overview-derived issues when enabled. | S07 | S04, S02 | S04 problem feed; S07 overview handoff and settings-driven scan tests. |
| R040 | TOOL-01 | Tools tab groupings match reference. | S05 | none | S05 static contract tests and summary. |
| R041 | TOOL-02 | User can launch Toolkit Utilities for Downgrade Manager and Archive Patcher. | S10 | S05, S09 | S05 utility entries; S09 live Downgrade Manager; S10 live Archive Patcher completes both. |
| R042 | TOOL-03 | External tool links open with reference labels and visible failure reporting. | S05 | S03 | S05 Tools service/controller tests. |
| R043 | TOOL-04 | Downgrade Manager honors backup and delta cleanup settings before file changes. | S09 | S02 | S02 persisted preferences; S09 modal/executor tests. |
| R044 | TOOL-05 | Archive Patcher uses fail-closed plans validating inputs before writing. | S10 | S03, S04 | S10 planner/executor tests: digest confirmation, BA2 validation, manifest-before-write. |
| R045 | TOOL-06 | File-changing tool operations run off UI thread with responsive status/errors. | S10 | S03, S09 | S09 and S10 modal/controller/worker event tests. |
| R046 | ABOUT-01 | About attribution matches reference including created-by text. | S05 | none | S05 About source contract tests. |
| R047 | ABOUT-02 | User can open/copy project/community links from About tab. | S05 | S03 | S05 About controller/service tests. |
| R048 | ABOUT-03 | User can open/copy Discord invite action. | S05 | S03 | S05 About link/copy tests. |
| R049 | ABOUT-04 | Link actions report failures visibly. | S05 | S03 | S05 visible failure feedback tests; S04/S07 reuse safe action pattern. |
| R050 | SAFE-01 | Long-running scans/traversal/parsing/downloads/patching/process monitoring run off Slint UI thread. | S03 | S04, S06, S07, S09, S10 | S03 worker foundation plus later worker-backed workflow tests. |
| R051 | SAFE-02 | Background work returns typed progress/completion/cancellation/error events via Slint-safe handoff. | S03 | S04, S06, S07, S09, S10 | S03 WorkerEvent/Handoff tests; later slices add owned payloads. |
| R052 | SAFE-03 | Domain logic is testable without launching a window using fake adapters. | S03 | S02-S10 | S03 fake platform adapters; later services/controllers are Slint-free and test-backed. |
| R053 | SAFE-04 | File-changing workflows use backups/dry-run plans/validation/fail-closed behavior. | S10 | S08, S09 | S08 fail-closed Auto-Fix registry, S09 digest-bound downgrader, S10 manifest/digest/byte-range patcher. |
| R054 | SAFE-05 | Labels, tab ordering, defaults, and messages are compared against `CMT/src/` before completing slices. | S01 | S02-S10 | S01 CMT tab references; each behavior slice cites source-contract/fidelity tests and known deviations. |

## Artifact Backfill Details

### S01 assessment

Recommended content shape:

- Verdict: `roadmap-confirmed`.
- Basis: S01 completed Slint shell, title/tab order, external Slint build pipeline, and app/domain/platform/worker module seams.
- Roadmap check: S01 provided the shell all later slices consumed; no ordering changes needed after S01.
- Evidence: S01 summary, old Phase 1 validation/verification, final gates, `git status --short CMT` no output.
- Caveat: S01 was foundation-only; real tab behavior intentionally deferred to S02-S10.

### S01 UAT

Recommended content shape:

- UAT type: backfilled developer/source-contract UAT plus automated shell-contract evidence.
- Execution status: S11 did not manually run GUI UAT unless the executor actually does; do not say it did.
- Steps/expected outcomes: cargo run opens `Collective Modding Toolkit`; tabs are in reference order; each tab was inert at S01; no file/network/process behavior; CMT remains untouched.
- Evidence basis: old Phase 1 validation says manual-only GUI checks were not automated; Phase 1 verification proves source/build/contract truths.
- Not proven: pixel-perfect GUI, later tab behavior, live settings/scanner/tools workflows.

### S07 dependency attribution

Current frontmatter snippet in `S07-SUMMARY.md`:

```yaml
requires:
  - slice: S02
    provides: Main shell, settings persistence baseline, and tab wiring patterns
```

Recommended replacement:

```yaml
requires:
  - slice: S01
    provides: Main shell, reference tab order, and MainWindow/tab wiring patterns.
  - slice: S02
    provides: Settings persistence baseline and scanner settings contract.
```

Only this one occurrence was found by `rg -n "Main shell|settings persistence baseline|tab wiring" .gsd/milestones/M001/slices/S07`.

### S10 artifacts

`S10-ASSESSMENT.md` and `S10-UAT.md` are present. `S10-ASSESSMENT.md` is short because it records the validation round 0 remediation decision; keep or expand it rather than deleting it. `S10-UAT.md` describes a manual destructive-safety smoke test procedure, while `S10-SUMMARY.md` states no manual real-install UAT was performed. If touched, add an explicit evidence/status note such as: “This is a UAT procedure/backfilled acceptance artifact; S10 closeout evidence is automated/fake-backed unless a sandbox run is separately recorded.”

## First Proof / Fast Validator

Before running expensive Cargo gates, add/run a small artifact validator after remediation. It should assert at least:

- `.gsd/REQUIREMENTS.md` has no `Untitled` and no `unmapped` proof rows.
- R001-R054 all appear with meaningful titles and validated/evidence-backed status, or any active rows have an explicit gap note.
- `S01-ASSESSMENT.md`, `S01-UAT.md`, `S10-ASSESSMENT.md`, and `S10-UAT.md` exist.
- `S07-SUMMARY.md` contains a `slice: S01` requirement for shell/tab wiring and does not attribute `Main shell` to S02.
- Backfilled UAT text includes “not manually run/executed” or equivalent where applicable.

Sketch:

```bash
python3 - <<'PY'
from pathlib import Path
import re
req = Path('.gsd/REQUIREMENTS.md').read_text(encoding='utf-8')
assert 'Untitled' not in req
assert 'unmapped' not in req
for i in range(1, 55):
    assert f'R{i:03d}' in req
for p in [
    '.gsd/milestones/M001/slices/S01/S01-ASSESSMENT.md',
    '.gsd/milestones/M001/slices/S01/S01-UAT.md',
    '.gsd/milestones/M001/slices/S10/S10-ASSESSMENT.md',
    '.gsd/milestones/M001/slices/S10/S10-UAT.md',
]:
    assert Path(p).exists(), p
s07 = Path('.gsd/milestones/M001/slices/S07/S07-SUMMARY.md').read_text(encoding='utf-8')
assert 'slice: S01' in s07
assert 'provides: Main shell, settings persistence baseline, and tab wiring patterns' not in s07
PY
```

## Verification

Final S11 verification should run fresh after the documentation/traceability edits:

1. Artifact validator above or equivalent.
2. `cargo fmt --check`
3. `cargo check`
4. `cargo test`
5. `cargo clippy --all-targets --all-features`
6. `git status --short CMT`
7. Rerun or prepare M001 validation round 1 and record the result/blockers.

Since S11 should not touch product code, Cargo failures are likely environmental or pre-existing drift; still record them honestly rather than claiming green gates from earlier slices.

## Risks and Constraints

- **GSD DB vs rendered file**: `.gsd/REQUIREMENTS.md` appears rendered from stored requirements. Prefer `gsd_requirement_update` or equivalent if available; manual file-only edits may be overwritten by later renders and may not satisfy validators that read DB state.
- **Honest UAT language**: Backfilled UAT is acceptable only if labeled as backfilled/source-contract/automated evidence. Do not imply manual GUI, real Fallout 4 install, live network, or destructive archive tests occurred unless they are actually run in S11.
- **No product behavior changes**: Do not edit Rust/Slint behavior for S11. If validation round 1 asks for product changes, stop and ask before expanding scope.
- **CMT read-only**: Do not edit, format, move, delete, or generate files under `CMT/`.
- **S07 summary is completed history**: Change only the inaccurate provenance frontmatter, not implementation facts.

## Sources

- `.planning/REQUIREMENTS.md` — original v1 requirement list and phase traceability.
- `.gsd/REQUIREMENTS.md` — current broken rendered requirement matrix.
- `.gsd/STATE.md` — active/validated counts before remediation.
- `.gsd/milestones/M001/slices/S01/S01-SUMMARY.md` — S01 completion evidence.
- `.planning/phases/01-slint-shell-port-architecture/01-VALIDATION.md` and `01-VERIFICATION.md` — S01 validation/UAT/assessment evidence.
- `.gsd/milestones/M001/slices/S02..S10/*-SUMMARY.md` — completed capability and gate evidence.
- `.gsd/milestones/M001/slices/S10/S10-ASSESSMENT.md` — validation round 0 remediation rationale.
- `.gsd/milestones/M001/slices/S10/S10-UAT.md` — present S10 UAT procedure artifact.
- `gsd_milestone_status(M001)` — S01-S10 complete, S11 pending.
- Research runs: `gsd_exec` inventory `d8c2d261-f52a-4482-a1f9-16b4b5ae3e68`, frontmatter parse `dbce2ab0-11d1-4dba-b6f9-98698447010d`, requirement refs `7d141d86-c8c9-4f99-bee7-04f5610790be`, completed-slice evidence `62651b4b-cd11-4ac7-b53a-5c3d62ad16c7`.
