---
estimated_steps: 20
estimated_files: 6
skills_used: []
---

# T01: Capture Tools and About reference contracts and assets

---
estimated_steps: 7
estimated_files: 6
skills_used:
  - verify-before-complete
  - tdd
---
Why: S05 fidelity depends on freezing the Python reference labels, URLs, help text, attribution text, utility disabled state, and image names before wiring any UI behavior. The Rust app must not depend on mutable `CMT/` paths at runtime.

Do:
1. Re-check the read-only reference files listed in Inputs only as source material; do not edit anything under `CMT/`.
2. Add `src/domain/tools.rs` with typed action/link ids, ordered Tool group/entry definitions, About link definitions, reference constants (`Collective Modding Toolkit`, `0.6.1`, Nexus/Discord/GitHub URLs), tooltip/help text, URL host hint text, utility deferred metadata, and Rust-owned image resource path constants.
3. Export the module from `src/domain/mod.rs` and add a public import smoke test for the new domain types.
4. Copy the four required reference images into `resources/images/` as Rust-owned assets: `icon-256.png`, `logo-nexusmods.png`, `logo-discord.png`, and `logo-github.png`.
5. Add unit tests whose names include `s05_reference_contract` asserting exact group labels/order, button labels/order including multi-line labels, URLs, help text/hint mapping, About title/credit/open/copy labels, and disabled/deferred utility metadata.

Done when: the domain contract is the single Rust source of truth for S05 action ids/URLs/text, the copied assets exist outside `CMT/`, and the focused contract tests pass.

Threat Surface (Q3): static external URLs are introduced but no user-supplied URL is accepted; utility actions must remain disabled and represented as internal deferred metadata only.
Requirement Impact (Q4): no active requirement ids were preloaded; re-verify shell import tests and S04/S02 compile surfaces because `src/domain/mod.rs` changes.
Failure Modes (Q5): if a reference image is missing, fail the test/copy step instead of silently referencing `CMT/`; if a URL/action id is malformed, domain tests should fail before UI wiring.
Load Profile (Q6): static contract only; cost is constant-size arrays and four image resources.
Negative Tests (Q7): include tests for URL hint fallback (`Open website`), utility entries disabled, unique action ids, and exact About copy labels (`Copy Link`, `Copy Invite`).

## Inputs

- `CMT/src/tabs/_tools.py`
- `CMT/src/tabs/_about.py`
- `CMT/src/globals.py`
- `CMT/src/assets/images/icon-256.png`
- `CMT/src/assets/images/logo-nexusmods.png`
- `CMT/src/assets/images/logo-discord.png`
- `CMT/src/assets/images/logo-github.png`
- `src/domain/mod.rs`

## Expected Output

- `src/domain/tools.rs`
- `src/domain/mod.rs`
- `resources/images/icon-256.png`
- `resources/images/logo-nexusmods.png`
- `resources/images/logo-discord.png`
- `resources/images/logo-github.png`

## Verification

cargo test s05_reference_contract

## Observability Impact

Adds no runtime signals yet, but creates stable action ids and labels that later logs/UI states can reference unambiguously.
