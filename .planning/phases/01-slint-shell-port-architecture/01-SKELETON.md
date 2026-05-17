# Phase 01 Walking Skeleton: Rust/Slint Desktop Shell

## Skeleton Goal

Build the thinnest executable desktop foundation for the Rust port: a Slint `MainWindow` titled `Collective Modding Toolkit`, launched by Rust, with six inert tabs matching the reference application order.

## Architectural Decisions Locked By This Skeleton

- **UI framework:** Slint 1.16.1.
- **UI build path:** `build.rs` compiles `ui/main.slint` using `slint-build` 1.16.1.
- **Root UI file:** `ui/main.slint` exports `MainWindow`.
- **Tab component files:** one file per tab: `overview_tab.slint`, `f4se_tab.slint`, `scanner_tab.slint`, `tools_tab.slint`, `settings_tab.slint`, and `about_tab.slint`.
- **Tab labels/order:** `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`, sourced from `CMT/src/cm_checker.py` and `CMT/src/enums.py`.
- **Rust boundaries:** `app`, `domain`, `platform`, and `workers` modules exist as documented no-op boundaries.
- **Behavior boundary:** no real diagnostics, settings persistence, game discovery, F4SE scanning, scanner traversal, tool launching, About links, update checks, archive parsing, subprocesses, network calls, or background jobs in Phase 1.
- **Reference safety:** `CMT/` is read-only; verification includes `git status --short CMT`.

## Initial File Layout

```text
Cargo.toml
Cargo.lock
build.rs
src/
  main.rs
  app/mod.rs
  domain/mod.rs
  platform/mod.rs
  workers/mod.rs
ui/
  main.slint
  overview_tab.slint
  f4se_tab.slint
  scanner_tab.slint
  tools_tab.slint
  settings_tab.slint
  about_tab.slint
```

## Verification Contract

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `git status --short CMT`

## Test Contract

The skeleton must include an automated Rust test named `shell_tab_labels_match_reference_order` asserting the canonical label array exactly equals:

```text
Overview, F4SE, Scanner, Tools, Settings, About
```

## Deferred To Later Phases

- Settings defaults and persistence.
- Platform/game discovery and registry/path adapters.
- Overview diagnostics and update panels.
- F4SE DLL compatibility scanning.
- Scanner results, filters, and fixes.
- Tools/About links and copy/open actions.
- Downgrade Manager and Archive Patcher workflows.
- Real worker runtime orchestration and Slint event-loop result handoff.

## Source Coverage Audit

| Source Type | Item | Covered By |
|-------------|------|------------|
| GOAL | Build/run Rust/Slint CMT shell with reference identity and safe boundaries | Plans 01, 02, 03 |
| REQ | FOUND-01 Slint desktop app builds | Plan 01, Plan 02 |
| REQ | FOUND-02 shell title and tabs match reference order | Plan 02, Plan 03 |
| REQ | FOUND-03 UI/app/domain/platform/workers boundaries exist | Plan 01, Plan 03 |
| REQ | FOUND-04 verification commands run | Plan 01, Plan 03 |
| REQ | FOUND-05 CMT remains unchanged | Plan 02, Plan 03 |
| REQ | SAFE-05 no blocking long-running UI work | Plan 02, Plan 03 |
| RESEARCH | External Slint compilation via build.rs | Plan 01 |
| RESEARCH | Official TabWidget static tab shell | Plan 02 |
| RESEARCH | Canonical Rust tab labels for testing | Plan 03 |
| CONTEXT | D-01 ui/main.slint plus one component file per tab | Plan 02 |
| CONTEXT | D-02 inert placeholder content only | Plan 02 |
| CONTEXT | D-03 exact tab labels/order | Plan 02, Plan 03 |
| CONTEXT | D-04 module stubs only | Plan 03 |
| CONTEXT | D-05 no domain behavior in Slint or stubs | Plans 01, 02, 03 |
| CONTEXT | D-06 stack baseline | Plan 01 |
| CONTEXT | D-07 aligned Slint versions | Plan 01 |
| CONTEXT | D-08 no scanner/archive/Fallout parser crates | Plan 01 |
| CONTEXT | D-09 automated Rust tab-order test | Plan 03 |
| CONTEXT | D-10 required Rust verification commands | Plan 03 |
| CONTEXT | D-11 git status CMT and reference citations | Plan 02, Plan 03 |
