# M001: Initial Port

**Vision:** Port the existing CMT Collective Modding Toolkit desktop application to a faithful Rust and Slint implementation that preserves the original Tkinter workflows, tab order, labels, defaults, validation behavior, and user-facing messages while keeping the Rust codebase buildable, testable, and responsive.

## Success Criteria

- The Rust crate launches a Slint desktop app identified as Collective Modding Toolkit with tabs ordered Overview, F4SE, Scanner, Tools, Settings, and About.
- Reference settings defaults, persistence keys, repair behavior, and visible Settings labels are preserved and verified.
- Shared platform discovery, filesystem/process adapter, and worker handoff seams support later tabs without blocking the Slint UI thread.
- The planned Overview, F4SE, Scanner, Tools, Settings, About, Downgrade Manager, and Archive Patcher workflows are tracked as vertical slices with dependencies and safety gates.
- Relevant Rust quality gates pass for completed slices and CMT remains a read-only reference submodule.

## Slices

- [x] **S01: Slint Shell Port Architecture** `risk:medium` `depends:[]`
  > After this: Developer can run the CMT Slint shell with the reference window title, six tabs in reference order, and no-op Rust module seams ready for later slices.

- [x] **S02: S02** `risk:medium` `depends:[]`
  > After this: User can open Settings, see reference-labeled Update Channel and Log Level choices, and have selected values load, persist, repair, and revert safely on save failure.

- [x] **S03: S03** `risk:medium` `depends:[]`
  > After this: Domain and platform tests prove Fallout 4 discovery contracts, fakeable filesystem/registry/process/desktop seams, and worker event handoff without launching the GUI.

- [x] **S04: S04** `risk:medium` `depends:[]`
  > After this: User can see Overview game, binary, archive, module, and update status panels populated from typed discovery and diagnostics.

- [x] **S05: S05** `risk:medium` `depends:[]`
  > After this: User can open Tools and About, see reference groupings/attribution, launch static links or utility entry points, and receive visible failure feedback.

- [x] **S06: S06** `risk:medium` `depends:[]`
  > After this: User can inspect F4SE plugin DLL compatibility in a reference-shaped table without blocking the UI.

- [x] **S07: S07** `risk:medium` `depends:[]`
  > After this: User can run Scanner, see progress, grouped read-only results, details, and copy/open actions while the UI remains responsive.

- [x] **S08: S08** `risk:medium` `depends:[]`
  > After this: User sees supported auto-fix actions on Scanner results and receives Fixed or Fix Failed feedback without blocking the UI.

- [ ] **S09: S09** `risk:medium` `depends:[]`
  > After this: User can open and run Downgrade Manager from Overview or Tools with backup and delta cleanup preferences respected and visible status/errors.

- [ ] **S10: Archive Patcher Workflow** `risk:medium` `depends:[S09]`
  > After this: User can open and run Archive Patcher operations through validated, fail-closed write plans that protect user files.

## Boundary Map

| Boundary | Owned by | Notes |
| --- | --- | --- |
| Slint UI markup | ui/*.slint | Presents layout, labels, tab order, and callback surfaces only. |
| App/controller bridge | src/app and src/main.rs | Loads state, binds Slint callbacks, and projects domain data to UI properties/models. |
| Domain models/services | src/domain and src/services | Own settings, discovery, scanner, archive, module, and tool semantics in testable Rust. |
| Platform adapters | src/platform | Own filesystem, registry, process, desktop open, and version metadata boundaries. |
| Workers | src/workers | Own background execution, progress, cancellation, and Slint-safe event-loop handoff. |
