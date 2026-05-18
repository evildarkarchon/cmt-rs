---
estimated_steps: 21
estimated_files: 4
skills_used: []
---

# T03: Replace Tools and About Slint placeholders with reference-shaped tabs

---
estimated_steps: 8
estimated_files: 4
skills_used:
  - verify-before-complete
  - tdd
---
Why: Users need to see the reference-shaped Tools and About surfaces, not inert placeholders. The Slint markup should stay declarative and call back with stable ids while Rust owns action definitions and state transitions.

Do:
1. Replace `ui/tools_tab.slint` with a conservative port of the reference three labelframe-style groups using `GroupBox`, `Button`, helper/status text, dark palette continuity, and source labels/order from `src/domain/tools.rs` tests. Preserve multi-line button labels where practical.
2. Keep `Downgrade Manager` and `Archive Patcher` visible in `Toolkit Utilities` but disabled, with status text that makes the S09/S10 deferral explicit.
3. Replace `ui/about_tab.slint` with the reference title split (`Collective Modding\nToolkit`), icon/logo imagery from `../resources/images/*`, version/credit text, and Nexus/Discord/GitHub rows with `Open Link`, `Open Invite`, `Copy Link`, and `Copy Invite` buttons.
4. Add inline safe-error banners/properties for Tools and About. About copy buttons should bind to copy-label/enabled properties and use a 3000ms `Timer` per copied state (or one equivalent timer mechanism) to invoke a reset callback after success.
5. Update `ui/main.slint` to expose/forward Tools/About properties and callbacks using stable string ids, e.g. tool action requested, about open requested, about copy requested, and about copy-label reset requested. Do not put static URLs in Slint.
6. Update source-contract tests in `src/main.rs`: remove Tools/About from the inert placeholder set, assert Tools/About component exports, group labels/order, button labels/order, disabled utility markers, image resource references, callback/property forwarding, safe-error banners, copy timers/reset callbacks, and absence of direct `https://` URL opens in Slint.

Done when: Slint compiles against the copied resources, Tools/About no longer contain placeholder text, and source-contract tests lock the S05 visual/action surface.

Threat Surface (Q3): UI can only emit stable ids and cannot supply arbitrary URLs. Disabled utilities must remain disabled in Slint as well as fail-closed in reducers.
Requirement Impact (Q4): re-verifies the shell/tab contract because `ui/main.slint` changes; Settings and Overview forwarding tests must remain intact.
Failure Modes (Q5): missing image resources should be caught by `cargo check`/source tests; users still see text/buttons even if an image fails to render; unknown callback ids are handled in T02/T04 reducers.
Load Profile (Q6): static markup with around a dozen buttons and four images; no dynamic filesystem/network load.
Negative Tests (Q7): source tests should fail if placeholder text remains, if a utility button is enabled, if a callback forwarding line is missing, if image paths point into `CMT/`, or if Slint embeds raw URLs.

## Inputs

- `src/domain/tools.rs`
- `resources/images/icon-256.png`
- `resources/images/logo-nexusmods.png`
- `resources/images/logo-discord.png`
- `resources/images/logo-github.png`
- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Expected Output

- `ui/tools_tab.slint`
- `ui/about_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Verification

cargo test s05_slint_contract

## Observability Impact

Adds visible inline failure surfaces and copy-reset UI state; no background/runtime logs are wired until T04.
