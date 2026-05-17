---
estimated_steps: 21
estimated_files: 3
skills_used: []
---

# T02: Compute pure Overview diagnostics

---
estimated_steps: 9
estimated_files: 3
skills_used:
  - tdd
  - verify-before-complete
---
Why: The reference Overview logic mixes diagnostics with Tk widgets. This task extracts the scanner-ready decisions into a pure service before adding filesystem or Slint wiring.

Do:
1. Add `src/services/overview.rs` with a pure `OverviewDiagnostics` or equivalent builder that consumes a `DiscoveryReport`, current `AppSettings`, typed binary facts, archive/module records, INI and enablement facts, update result state, and optional desktop-action feedback.
2. Produce a complete `OverviewSnapshot` from those facts: top status rows, mod-manager summary including MO2/Vortex partial-support details, PC specs formatting, install-type/version text, Binaries/Archives/Modules panel counts, warning/error severities, deferred utility button state, and problem feed.
3. Implement reference-compatible count rules and thresholds from `CMT/src/globals.py`: archive General/Texture/Total/Unreadable/v1/v7-8 counts and module Full/Light/Total/Unreadable/HEDR v1.00/HEDR v0.95/HEDR v???? counts.
4. Implement problem creation for missing Data, no mod manager, unknown game version, missing Address Library, wrong binary/archive/module versions, missing Fallout4.ccc, missing plugins.txt, unreadable files, invalid headers, and exceeded archive/module limits.
5. Keep all filesystem-derived information as injected facts; this service must not call filesystem, process, registry, network, desktop, or Slint APIs.
6. Export the service from `src/services/mod.rs`.
7. Add focused tests using constructed facts for successful Old-Gen, Next-Gen, Anniversary, missing Data, missing enablement files, unreadable records, exceeded limits, Vortex identity-only, MO2 Windows 11 24H2 warning, and update-banner states.

Done when: fake inputs can produce the full Overview snapshot and problem feed expected by UI and Scanner without touching OS boundaries.

Requirement Impact Q4: re-assert S03 typed discovery semantics by testing valid game paths with missing optional Data/F4SE paths as non-fatal Overview states.
Failure Modes Q5: discovery errors, manager errors, and system metadata errors must degrade into safe rows/problems rather than aborting snapshot construction.
Load Profile Q6: service cost should be linear in supplied archive/module records and should not clone large path lists unnecessarily beyond the final snapshot.
Negative Tests Q7: malformed version text, unknown CRC/version, unreadable records, empty plugin/archive lists, max-count boundary, missing ccc/plugins, and failed update result.

## Inputs

- `src/domain/overview.rs`
- `src/domain/discovery.rs`
- `src/domain/mod_manager.rs`
- `src/domain/settings.rs`
- `src/services/discovery.rs`
- `CMT/src/tabs/_overview.py`
- `CMT/src/globals.py`
- `CMT/src/enums.py`
- `CMT/src/helpers.py`

## Expected Output

- `src/services/overview.rs`
- `src/services/mod.rs`
- `src/domain/overview.rs`

## Verification

cargo test overview_diagnostics

## Observability Impact

Centralizes the phase and severity values that worker logs and UI status rows can report when diagnostics degrade.
