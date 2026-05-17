---
estimated_steps: 19
estimated_files: 4
skills_used: []
---

# T06: Replace Overview Slint placeholder

---
estimated_steps: 9
estimated_files: 4
skills_used:
  - tdd
  - verify-before-complete
---
Why: The slice is only user-visible when the Slint tab presents the typed snapshot with reference-shaped layout and callbacks.

Do:
1. Replace `ui/overview_tab.slint` placeholder with a conservative reference-shaped layout: top status block, Refresh control, game-path open affordance, update banner, Binaries (EXE/DLL/BIN), Archives (BA2), Modules (ESM/ESL/ESP), deferred Downgrade Manager... and Archive Patcher... controls, and inline problem/action status text.
2. Add Slint properties/models/callbacks required by the controller while keeping domain logic in Rust: refresh requested, open game path, open Nexus link, open GitHub link, panel rows, update banner state, deferred action enabled/text, and last safe error.
3. Update `ui/main.slint` to bind Overview properties and callbacks through `MainWindow` similarly to Settings.
4. Update `src/main.rs` source-contract tests: remove Overview from inert placeholder assertions, assert reference labels/order and callback/property forwarding, keep F4SE/Scanner/Tools/About inert until later slices, and assert Settings labels remain unchanged.
5. Adjust `src/app/overview_controller.rs` only as needed for generated Slint type projection; do not move diagnostics into Slint.
6. Run and fix the full quality gates.

Done when: cargo check builds generated Slint code, source tests lock the Overview labels and bindings, and the Overview tab is populated from controller state rather than placeholder text.

Threat Surface Q3: Slint must display safe strings from Rust only; never render raw network bodies or diagnostics intended for logs.
Failure Modes Q5: loading, no game, partial discovery, refresh failure, and open-link failure must be visible inline without modal interruption.
Negative Tests Q7: source-contract tests assert no placeholder scope note remains for Overview and deferred mutation controls are disabled or explanatory.

## Inputs

- `ui/overview_tab.slint`
- `ui/main.slint`
- `src/main.rs`
- `src/app/overview_controller.rs`
- `src/domain/overview.rs`
- `src/services/overview.rs`
- `src/services/overview_collector.rs`
- `src/services/update.rs`
- `CMT/src/tabs/_overview.py`
- `CMT/src/cm_checker.py`

## Expected Output

- `ui/overview_tab.slint`
- `ui/main.slint`
- `src/main.rs`
- `src/app/overview_controller.rs`

## Verification

cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Observability Impact

Makes refresh state, update availability, deferred workflow status, and safe action failures visible in the UI for manual and automated inspection.
