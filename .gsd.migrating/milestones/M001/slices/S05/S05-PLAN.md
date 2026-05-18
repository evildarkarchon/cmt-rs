# S05: Tools Shell, Links & About

**Goal:** Deliver reference-shaped Tools and About tabs with preserved groupings, attribution, static link actions, copy-link feedback, disabled/deferred utility entries, Rust-owned image resources, and visible safe failure feedback through fakeable action boundaries.
**Demo:** User can open Tools and About, see reference groupings/attribution, launch static links or utility entry points, and receive visible failure feedback.

## Must-Haves

- Tools tab replaces the inert placeholder with the reference group labels and button order: Toolkit Utilities; Other CM Authors' Tools; Other Useful Tools.
- Downgrade Manager and Archive Patcher are visible in Tools but disabled/deferred; no S05 path can start downloads, backups, patch plans, archive writes, or modal mutation workflows.
- Tools external-link buttons and About open/copy buttons call Rust callbacks by stable action id; URLs are resolved in Rust domain/action code, not by direct Slint/browser calls.
- About tab shows reference title/version/credit text, Nexus/Discord/GitHub action rows, and Rust-owned copies of icon-256.png, logo-nexusmods.png, logo-discord.png, and logo-github.png while retaining visible text/buttons if images are unavailable.
- Copy-link success changes the relevant button label to Copied! and disables it briefly before restoring the original label; copy/open failures surface safe inline status text.
- Unit/source-contract tests cover reference labels/order/URLs/help text, deferred utility state, unknown action ids, desktop-open failure mapping, clipboard failure mapping, Slint callback forwarding, and resource references.
- Full verification gates pass: cargo fmt --check, cargo check, cargo test, cargo clippy --all-targets --all-features, plus a CMT cleanliness check.

## Proof Level

- This slice proves: Integration proof. Executors should prove the real Rust entrypoint compiles with Slint resource references and callback wiring, while tests use fake desktop/clipboard adapters and source-contract assertions instead of launching a browser, requiring a real clipboard, or mutating Fallout 4 files. No human UAT is required for slice completion, though a later visual comparison pass remains useful.

## Integration Closure

Consumes S04 safe desktop-action and worker-handoff patterns plus existing Slint shell forwarding. Introduces Tools/About domain contracts, clipboard boundary, reducers, worker payloads, Slint properties/callbacks, and main.rs composition. Leaves live Downgrade Manager and Archive Patcher workflows intentionally deferred to S09/S10, and leaves F4SE/Scanner behavior to later slices.

## Verification

- Adds visible Tools/About last-action error banners, About copy-label success state, disabled utility status text, and structured tracing around action scheduling, adapter success/failure, worker handoff failure, unknown action ids, and copy-label reset. Raw adapter diagnostics stay in logs/tests; UI status remains safe and non-secret.

## Tasks

- [x] **T01: Capture Tools and About reference contracts and assets** `est:1h`
  ---
  estimated_steps: 7
  estimated_files: 6
  skills_used:
    - verify-before-complete
    - tdd
  ---
  Why: S05 fidelity depends on freezing the Python reference labels, URLs, help text, attribution text, utility disabled state, and image names before wiring any UI behavior. The Rust app must not depend on mutable `CMT/` paths at runtime.
  - Files: `src/domain/tools.rs`, `src/domain/mod.rs`, `resources/images/icon-256.png`, `resources/images/logo-nexusmods.png`, `resources/images/logo-discord.png`, `resources/images/logo-github.png`
  - Verify: cargo test s05_reference_contract

- [x] **T02: Add fakeable clipboard, action services, reducers, and worker payloads** `est:2h`
  ---
  estimated_steps: 9
  estimated_files: 11
  skills_used:
    - verify-before-complete
    - tdd
    - rust-async-patterns
  ---
  Why: Tools/About open and copy actions must be testable and safe without launching a browser or relying on a host clipboard. This task establishes the Slint-free domain/controller/platform layer before UI and main.rs composition.
  - Files: `Cargo.toml`, `Cargo.lock`, `src/platform/mod.rs`, `src/platform/clipboard.rs`, `src/services/mod.rs`, `src/services/tools.rs`, `src/app/mod.rs`, `src/app/tools_controller.rs`, `src/app/about_controller.rs`, `src/workers/events.rs`, `src/workers/mod.rs`
  - Verify: cargo test s05_actions

- [x] **T03: Replace Tools and About Slint placeholders with reference-shaped tabs** `est:2h`
  ---
  estimated_steps: 8
  estimated_files: 4
  skills_used:
    - verify-before-complete
    - tdd
  ---
  Why: Users need to see the reference-shaped Tools and About surfaces, not inert placeholders. The Slint markup should stay declarative and call back with stable ids while Rust owns action definitions and state transitions.
  - Files: `ui/tools_tab.slint`, `ui/about_tab.slint`, `ui/main.slint`, `src/main.rs`
  - Verify: cargo test s05_slint_contract

- [x] **T04: Wire Tools and About callbacks through workers in the real app entrypoint** `est:2h`
  ---
  estimated_steps: 10
  estimated_files: 5
  skills_used:
    - verify-before-complete
    - rust-async-patterns
    - tdd
  ---
  Why: The slice is only useful if the real `MainWindow` composes the new reducers, adapters, worker handoff, and Slint properties. This task closes the runtime loop while keeping potentially slow or failing external actions off the Slint event thread.
  - Files: `src/main.rs`, `src/app/tools_controller.rs`, `src/app/about_controller.rs`, `src/services/tools.rs`, `src/workers/events.rs`
  - Verify: cargo test s05_runtime_wiring

- [x] **T05: Run full S05 verification gates and CMT cleanliness check** `est:45m`
  ---
  estimated_steps: 5
  estimated_files: 0
  skills_used:
    - verify-before-complete
    - test
  ---
  Why: S05 touches Slint resources, Rust action boundaries, workers, Cargo dependencies, and main entrypoint wiring. Closeout must prove the whole crate still builds/tests/lints and the read-only reference submodule stayed untouched.
  - Verify: cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
git status --short CMT

## Files Likely Touched

- src/domain/tools.rs
- src/domain/mod.rs
- resources/images/icon-256.png
- resources/images/logo-nexusmods.png
- resources/images/logo-discord.png
- resources/images/logo-github.png
- Cargo.toml
- Cargo.lock
- src/platform/mod.rs
- src/platform/clipboard.rs
- src/services/mod.rs
- src/services/tools.rs
- src/app/mod.rs
- src/app/tools_controller.rs
- src/app/about_controller.rs
- src/workers/events.rs
- src/workers/mod.rs
- ui/tools_tab.slint
- ui/about_tab.slint
- ui/main.slint
- src/main.rs
