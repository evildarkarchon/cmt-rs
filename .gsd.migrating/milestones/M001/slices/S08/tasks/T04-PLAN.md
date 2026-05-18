---
estimated_steps: 20
estimated_files: 3
skills_used: []
---

# T04: Expose gated Auto Fix UI and runtime scheduling

Expected executor skills_used: rust-async-patterns, tdd, observability, verify-before-complete.

Why: The slice is only complete when the real Slint surface remains hidden for the empty production registry, tampered callbacks fail closed, and fake-backed runtime tests prove the worker path can show Auto-Fix lifecycle feedback inline.

Do:
1. Extend ui/scanner_tab.slint with Auto-Fix properties and an auto-fix-requested callback. The button must be rendered only when the selected detail says it is visible; do not show disabled placeholders for unsupported results. Use the exact labels Auto-Fix, Fixing..., Fixed!, Fix Failed, and an inline Auto-Fix Results section.
2. Extend ScannerResultUiRow with a fixed/check state and render a conservative checkmark or equivalent marker for fake successful fixes without changing unsupported result rows.
3. Forward the new properties and callback through ui/main.slint.
4. Update src/main.rs ScannerUiProjection and projection helpers to map controller Auto-Fix state, row fixed state, inline result details, and button visibility/enabled/label values into Slint properties.
5. Bind the new callback in bind_scanner_callbacks. It should ask the controller for an Auto-Fix worker request, apply fail-closed feedback immediately when none is returned, and schedule a WorkerTaskKind::Patch worker carrying owned request data when one is returned.
6. Execute production Auto-Fix workers through AutoFixService::production or an equivalent empty registry. Since production has no registered operations, normal users must see no button; tests may call runtime helpers with fake registries to prove success and failure.
7. Update source-contract tests in src/main.rs from the S07 prohibition to S08 assertions: labels exist only in the gated Auto-Fix UI, MainWindow forwards the callback/properties, unsupported production projection hides the button, fake projection shows the button, inline Auto-Fix Results details render, and no disabled placeholder appears for unsupported rows.
8. Add runtime tests named with the s08_scanner_autofix_runtime_wiring filter for empty production hidden state, tampered callback rejection, fake worker success, fake worker failure, worker spawn/failure safe feedback, and stale completion ignoring.

Failure Modes Q5:
| Dependency | On error | On timeout | On malformed response |
| --- | --- | --- | --- |
| Worker spawn | Mark selected Auto-Fix as Fix Failed and show safe start-failed feedback | Not applicable to spawn | Not applicable |
| AutoFixService execution | Return Fix Failed with safe Auto-Fix Results details | Future timeout maps to Fix Failed without mutation | Malformed request is rejected before operation execution |
| Slint callback | Hidden/tampered callback updates safe feedback and schedules no worker | Not applicable | Invalid selection index or no selected result is rejected |

Load Profile Q6: one blocking worker is scheduled per accepted click; repeated clicks while Fixing... must be disabled/rejected. No production filesystem scan or mutation happens in S08.

Negative Tests Q7: production empty registry hides the button; unsupported selected result has no placeholder; hidden callback cannot mutate or schedule a fake operation; stale worker completion does not overwrite a newer selection; fake success/failure labels and inline details match reference copy.

Done when targeted S08 runtime and Slint contract tests pass and full cargo fmt/check/test/clippy gates are clean under the current project lint configuration.

## Inputs

- `src/domain/autofix.rs`
- `src/domain/scanner.rs`
- `src/services/autofix.rs`
- `src/app/scanner_controller.rs`
- `src/workers/events.rs`
- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Expected Output

- `ui/scanner_tab.slint`
- `ui/main.slint`
- `src/main.rs`

## Verification

cargo test s08_scanner_autofix_slint_contract
cargo test s08_scanner_autofix_runtime_wiring
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features

## Observability Impact

Runtime wiring should emit structured s08-scanner-autofix-schedule, spawn-failed, worker-started, completed, failed, rejected, and stale-ignored events. UI inspection surfaces are the button label/enabled state, inline Auto-Fix Results details, row fixed marker, and safe action feedback.
