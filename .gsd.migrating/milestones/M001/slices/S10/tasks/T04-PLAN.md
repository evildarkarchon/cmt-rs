---
estimated_steps: 21
estimated_files: 3
skills_used: []
---

# T04: Build Archive Patcher Slint modal contract

---
estimated_steps: 7
estimated_files: 3
skills_used:
  - write-docs
  - tdd
  - verify-before-complete
---
Why: The visible workflow must be reference-shaped before runtime wiring so source contract tests can lock labels, control order, exported UI row types, and callbacks for the Overview and Tools entrypoints.

Do:
1. Create `ui/archive_patcher_window.slint` with an exported `ArchivePatcherWindow` component and exported UI row structs for candidates, plan rows, and log rows as needed by Rust projection helpers.
2. Match the reference modal shape: title `Archive Patcher`, top desired-version group with `v1 (OG)` and `v8 (NG)` radio choices defaulting to v1, dynamic filter explanation label, `Name Filter:` entry, `Patch All`, `Restore Last Run`, `About`, candidate list, inline confirmation/plan area, bottom log/status area, and disabled/working states.
3. Implement a simple About overlay using `Bethesda Archive (BA2) Formats & Versions` and the reference body text supplied by the domain layer.
4. Import/export the component and row structs from `ui/main.slint` so `slint::include_modules!()` generates Rust types.
5. Add source-level Slint contract tests in `src/main.rs` for title, labels, default target, callback names, disabled-state properties, About title, and candidate/log/plan model surfaces.
6. Do not place patching logic in Slint; callbacks should only signal intents and display projected properties/models.
7. Keep visual changes conservative and consistent with the existing Downgrader modal style where Slint requires adaptation.

Done when: Slint contract tests prove the modal exists with the reference labels/order and exposes all properties/callbacks needed by runtime wiring.

Failure Modes Q5: Missing runtime data should display safe empty-state text and disabled write controls through properties rather than crashing Slint bindings.
Load Profile Q6: Candidate and log models should remain simple row arrays; the UI must not read filesystem state or dynamically scan directories.
Negative Tests Q7: No candidates, running state disables write controls, confirmation visible only after a plan, About overlay open/close, and Restore disabled when no manifest exists.

## Inputs

- `src/domain/archive_patcher.rs`
- `src/app/archive_patcher_controller.rs`
- `ui/main.slint`
- `ui/downgrader_window.slint`
- `CMT/src/patcher/_base.py`
- `CMT/src/patcher/_archives.py`
- `CMT/src/globals.py`

## Expected Output

- `ui/archive_patcher_window.slint`
- `ui/main.slint`
- `src/main.rs`

## Verification

cargo test s10_archive_patcher_slint_contract --quiet
cargo check --quiet

## Observability Impact

Adds visible modal state surfaces for empty/error/running/completed phases so user-facing failure is inspectable without reading logs.
