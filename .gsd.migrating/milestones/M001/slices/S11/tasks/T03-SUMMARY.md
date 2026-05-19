---
id: T03
parent: S11
milestone: M001
key_files:
  - .gsd/milestones/M001/slices/S07/S07-SUMMARY.md
  - .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py
key_decisions:
  - Enforced the S07 provenance correction mechanically in the existing S11 --provenance verifier rather than relying only on prose review.
duration: 
verification_result: passed
completed_at: 2026-05-19T04:58:24.679Z
blocker_discovered: false
---

# T03: Corrected S07 dependency provenance so shell/tab wiring is credited to S01 and settings/scanner settings to S02, with a verifier guard against regression.

**Corrected S07 dependency provenance so shell/tab wiring is credited to S01 and settings/scanner settings to S02, with a verifier guard against regression.**

## What Happened

Inspected S07, S01, and S02 summaries plus S11 research and the existing S11 verifier. S07 frontmatter contained the documented incorrect dependency attribution: S02 was credited with the main shell and tab wiring. I first extended the provenance verifier's existing --provenance mode with the narrow Q7 guard from the task plan, then ran it against the unmodified S07 summary to confirm it failed for the intended reason. After that negative check, I updated S07-SUMMARY.md's requires block to add S01 as the Main shell/reference tab order/MainWindow tab-wiring dependency and narrow S02 to the settings persistence/scanner settings contract. I also audited completed slice summaries for the same S02-only shell provenance attribution class and found no remaining instances.

## Verification

Ran the S11 provenance verifier before and after the S07 correction. The guarded verifier failed before the fix with the expected S01-missing/S02-shell-attribution errors, then passed after the S07 frontmatter correction. A separate completed-slice summary audit also passed, confirming no completed slice summary still attributes Main shell/MainWindow/tab wiring provenance to S02.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --provenance` | 1 | ✅ expected fail before S07 fix; verifier caught missing S01 and incorrect S02 shell attribution | 260ms |
| 2 | `python3 - <<'PY'
from pathlib import Path
import re
root = Path('.gsd/milestones/M001/slices')
problems = []
for summary in sorted(root.glob('S[0-9][0-9]/S[0-9][0-9]-SUMMARY.md')):
    text = summary.read_text(encoding='utf-8')
    for match in re.finditer(r'(?m)^  - slice: S02\n    provides: (?P<provides>.+)$', text):
        provides = match.group('provides')
        if re.search(r'Main shell|MainWindow|tab wiring', provides, re.IGNORECASE):
            problems.append(f'{summary}: {provides}')
if problems:
    print('bad S02 shell provenance remains:')
    for problem in problems:
        print(problem)
    raise SystemExit(1)
print('audit ok: no completed slice summary attributes Main shell/MainWindow/tab wiring to S02')
PY` | 0 | ✅ pass; no completed slice summary attributes shell/tab wiring to S02 | 167ms |
| 3 | `python3 .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py --provenance` | 0 | ✅ pass; provenance verifier confirms traceability IDs and S07 dependency attribution | 256ms |

## Deviations

Updated .gsd/milestones/M001/slices/S11/verify_s11_artifacts.py in addition to S07-SUMMARY.md because the task's Q7 negative-check requirement explicitly said the verifier should fail if the incorrect S07 attribution remained.

## Known Issues

None.

## Files Created/Modified

- `.gsd/milestones/M001/slices/S07/S07-SUMMARY.md`
- `.gsd/milestones/M001/slices/S11/verify_s11_artifacts.py`
