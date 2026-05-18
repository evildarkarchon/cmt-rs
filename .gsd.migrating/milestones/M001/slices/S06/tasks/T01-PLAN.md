---
estimated_steps: 13
estimated_files: 2
skills_used: []
---

# T01: Define F4SE domain contract

Expected executor skills for task-plan frontmatter: tdd, verify-before-complete.

Why: S06 needs a Slint-free contract that locks the reference labels, legend text, icon semantics, loading messages, and row compatibility rules before worker or UI code depends on them.

Do:
1. Inspect the listed CMT reference inputs as read-only and preserve their exact user-facing strings.
2. Create src/domain/f4se.rs and export it from src/domain/mod.rs.
3. Add constants for the tab title, loading text Scanning DLLs..., table columns DLL, OG, NG, AE, Your Game, heading F4SE DLLs, missing-folder messages, mod-manager hint, legend text, and reference icon strings.
4. Add typed Slint-free models such as F4seGameTarget, F4seDllFacts, F4seCompatibilityCell, F4seDllRow, F4seScanSnapshot, F4seScanStatus, and row severity or tag names.
5. Implement pure render/classification helpers matching CMT/src/tabs/_f4se.py: non-F4SE rows show question marks for OG, NG, and AE and a blank Your Game; OG support comes from F4SEPlugin_Query; NG and AE support come from F4SEPlugin_Version plus compatibleVersions; unsupported OG, NG, and AE columns use the reference blank cell while unsupported Your Game uses the cross mark; ambiguous NGAE support uses the warning icon.
6. Model unknown current-game classification separately from confirmed incompatibility so Your Game can show warning and explanatory detail without hiding DLL facts.
7. Add unit tests named with f4se_domain that lock column order, legend copy, loading and missing-folder strings, icon mapping, non-F4SE rendering, ambiguous NGAE rendering, current-game mapping, and unknown-game warning behavior.

Failure Modes Q5: malformed scan facts must produce unknown or warning cells instead of panics; unknown game target must not be treated as confirmed incompatible.

Negative Tests Q7: include non-F4SE facts, F4SE facts with no F4SEPlugin_Version, NGAE facts with no recognized compatible version, and unknown game target.

Done when: the domain module has no Slint, filesystem, process, or worker dependencies; the reference strings are test-locked; and later tasks can build rows from typed facts without re-reading CMT.

## Inputs

- `CMT/src/tabs/_f4se.py`
- `CMT/src/globals.py`
- `src/domain/discovery.rs`
- `src/domain/mod.rs`

## Expected Output

- `src/domain/f4se.rs`
- `src/domain/mod.rs`

## Verification

cargo test f4se_domain

## Observability Impact

Establishes visible status, severity, detail, and safe-message fields that later controller/UI code can expose when a scan fails or compatibility is unknown.
