---
estimated_steps: 6
estimated_files: 3
skills_used: []
---

# T04: Build Scanner Slint surface

Expected executor skills: tdd, write-docs, verify-before-complete.

Why: The placeholder Scanner tab must become the reference-shaped UI surface while keeping behavior in Rust controllers and preserving the known S07 difference of embedded panes instead of Tk floating windows.

Do: Replace `ui/scanner_tab.slint` with an embedded Scanner layout that uses standard Slint widgets consistently with the existing F4SE/Tools style. Include a `Scan Settings` group with the seven checkboxes in reference order, the `Scan Game` button, progress/status/result-count text, grouped read-only result rows with optional `Mod` column only when attribution exists, a details pane with labels `Mod:`, `Problem:`, `Summary:`, `Solution:`, a `Copy Details` button, optional `File List` affordance/text, and read-only `Open Path`, `Open URL`, and `Copy URL` buttons only when available. Do not render Auto-Fix, Fixed, or Fix Failed controls. Update `ui/main.slint` to import scanner structs, expose scanner properties/models/callbacks through `MainWindow`, and forward callbacks to the root. Add Slint source-contract/runtime projection tests in `src/main.rs` with names including `s07_scanner_slint_contract` that assert labels/order, callback names, hidden Auto-Fix text, result/detail labels, and MainWindow forwarding.

Done when: `cargo test s07_scanner_slint_contract` and `cargo check` pass with the new Slint API.

Threat Surface Q3: UI exposes only read-only open/copy callbacks and must not include any destructive or disabled no-op write action.
Negative Tests Q7: source tests assert no `Auto-Fix`, `Fixed!`, or `Fix Failed` text, all checkbox labels are in order, `Scan Game`/`Scanning...` strings exist, and detail labels/actions exist.

## Inputs

- `ui/scanner_tab.slint`
- `ui/main.slint`
- `ui/f4se_tab.slint`
- `ui/tools_tab.slint`
- `src/domain/scanner.rs`
- `src/app/scanner_controller.rs`
- `src/main.rs`

## Expected Output

- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Verification

cargo test s07_scanner_slint_contract
cargo check

## Observability Impact

Adds visible scanner progress, empty state, details, file-list, and inline action-feedback surfaces; no background wiring yet.
