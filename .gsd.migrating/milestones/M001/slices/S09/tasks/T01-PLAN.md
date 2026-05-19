---
estimated_steps: 10
estimated_files: 2
skills_used: []
---

# T01: Added a pure downgrader domain contract with reference labels, CRC maps, backup/patch helpers, and typed row payloads.

---
estimated_steps: 6
estimated_files: 2
skills_used:
  - write-docs
  - tdd
---
Why: The destructive workflow needs a Slint-free, IO-free source of truth for reference labels, file order, CRC maps, target names, backup names, patch names, status vocabulary, about copy, log messages, and plan/log row types before any service or UI code can rely on strings.
Do: Create `src/domain/downgrader.rs` from the read-only Python references. Preserve the exact modal title `Downgrader`, group labels, desired-version labels `Old-Gen` and `Next-Gen`, button labels including `Patch\n All`, initial log line, about title/body, tooltip copy, six file definitions in reference order, install status display labels, CRC maps, backup filename helpers, patch URL/name helpers, and reference-style log message helpers. Add pure types for target, status rows, options snapshot, plan rows, execution log rows, and progress values that later services/controllers can reuse without Slint. Export the module from `src/domain/mod.rs` and add public-import assertions next to the existing domain visibility test.
Done when: Domain unit/source-contract tests prove all reference strings, CRC mappings, file groups, target labels, backup names, patch URL names, and status labels without reading `.gsd`, `.planning`, or `.audits`.

## Inputs

- `CMT/src/downgrader.py`
- `CMT/src/globals.py`
- `CMT/src/enums.py`
- `src/domain/mod.rs`
- `src/domain/settings.rs`

## Expected Output

- `src/domain/downgrader.rs`
- `src/domain/mod.rs`

## Verification

cargo test downgrader_domain

## Observability Impact

No runtime observability yet; this task adds stable labels and typed row/result structures that later logs and worker payloads can use without string matching.
