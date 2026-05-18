---
estimated_steps: 13
estimated_files: 5
skills_used: []
---

# T05: Create Slint modal source contract

---
estimated_steps: 7
estimated_files: 5
skills_used:
  - write-docs
  - tdd
  - make-interfaces-feel-better
---
Why: The user-facing Downgrader must look and behave like the reference modal before runtime code can wire real actions to it.
Do: Add `ui/downgrader_window.slint` and import it through `ui/main.slint` so Slint generates the component. Build a conservative fixed-shape window titled `Downgrader` near the reference 600x334 proportions, with `Current Game`, `Current Creation Kit`, `Desired Version`, `Options`, `Patch\n All`, `About`, bottom log, progress bar, and an inline plan/confirmation area that stays within the same modal rather than becoming a redesigned wizard. Preserve row labels and display names including Archive2 basename display. Expose properties and callbacks needed by the controller projection: grouped status rows, selected target, `Keep Backups`, `Delete Patches`, plan rows, plan visibility, confirmation state, log rows/text, progress percent/text, patch/about enabled state, close blocked state, target/option callbacks, patch requested, confirm requested if needed, about requested, and close requested. Update Overview and Tools Slint surfaces so they can forward Downgrade Manager open requests while Archive Patcher remains deferred. Add source-contract tests in `src/main.rs` or the nearest existing source-contract test module for labels, titles, callback names, deferred Archive Patcher text, and no accidental Anniversary target option.
Failure Modes Q5: Slint compile errors or unsupported close interception must be surfaced early with `cargo check`; if exact Tk-style modality is unavailable, document the practical difference in code comments/tests and still block close/Escape while running through available Slint callbacks.
Negative Tests Q7: Source tests must reject missing required labels, accidental `Anniversary` target selection, absent inline plan confirmation copy, or re-enabled Archive Patcher.
Done when: Slint compiles and source-contract tests prove the reference-shaped modal and entrypoint callback surfaces.

## Inputs

- `src/domain/downgrader.rs`
- `src/app/downgrader_controller.rs`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
- `CMT/src/downgrader.py`
- `CMT/src/globals.py`
- `CMT/src/modal_window.py`

## Expected Output

- `ui/downgrader_window.slint`
- `ui/main.slint`
- `ui/overview_tab.slint`
- `ui/tools_tab.slint`
- `src/main.rs`

## Verification

cargo test s09_downgrader_slint_contract
cargo check

## Observability Impact

Adds visible status, plan, log, and progress surfaces that expose user-safe failure state during destructive work.
