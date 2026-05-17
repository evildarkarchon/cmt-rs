# Phase 1: Slint Shell & Port Architecture - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

## Phase Boundary

Phase 1 delivers the buildable Rust/Slint application shell only: a `Collective Modding Toolkit` window with the six reference tabs in order, inert placeholder content, and compile-ready architectural module boundaries for later port slices.

## Requirements (locked via SPEC.md)

**6 requirements are locked.** See `01-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `01-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Add Slint runtime/build dependencies needed for a buildable desktop shell.
- Add the build script and Slint UI file(s) needed to compile and launch a `MainWindow`.
- Show the `Collective Modding Toolkit` shell with tabs `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in that order.
- Provide inert placeholder content for each tab.
- Create recommended Rust module skeletons for `app`, `domain`, `platform`, and `workers` boundaries.
- Run and report the Rust verification commands for this slice.
- Verify `CMT/` remains unmodified and shell labels were checked against reference source.

**Out of scope (from SPEC.md):**
- Implementing Overview diagnostics, settings persistence, game discovery, F4SE scanning, scanner results, tool launching, or About link behavior - those are later phases.
- Performing a full tab layout audit - later tab phases inspect their own reference files in detail.
- Adding real background jobs or Slint UI-thread handoff behavior beyond the skeleton boundary - Phase 3 owns background adapters.
- Adding new product features, scanner categories, or redesigns - this phase only establishes the faithful shell foundation.
- Editing any file under `CMT/` - the reference submodule is read-only.

## Implementation Decisions

### Slint UI Shape
- **D-01:** Use `ui/main.slint` plus one component file per tab for Phase 1 rather than putting every placeholder directly in `main.slint`.
- **D-02:** Tab component files should contain inert placeholder content only. The placeholder text should be short scope notes that make clear each tab's real behavior belongs to later phases.
- **D-03:** The visible tab labels must be exactly `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About` in that order.

### Rust Skeleton
- **D-04:** Create module stubs, not real controllers or domain implementations. The expected boundaries are `app`, `domain`, `platform`, and `workers` with doc comments and minimal no-op types/functions as needed to compile.
- **D-05:** Keep domain behavior out of Slint markup and out of Phase 1 stubs. Later phases will fill the modules with settings, discovery, scanner, tool, and worker behavior.

### Dependencies And Build Setup
- **D-06:** Add the stack baseline now: Slint build/runtime dependencies plus focused baseline crates from research that establish the app foundation.
- **D-07:** Use the researched Slint version family (`slint` and `slint-build` 1.16.1 unless a current compatibility issue is discovered during planning). Keep Slint runtime and build versions aligned.
- **D-08:** Additional baseline dependencies should be limited to crates that support the architecture foundation, such as async/background orchestration, typed serialization, logging, and error handling. Do not add scanner/archive/Fallout-specific parsing crates in Phase 1.

### Verification
- **D-09:** Phase 1 must include an automated Rust test that asserts the six shell tab labels/order, in addition to manual launch or smoke verification.
- **D-10:** Phase 1 completion must run `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`.
- **D-11:** Phase 1 completion must run `git status --short CMT` and cite the CMT reference source used for shell labels/order, expected to be `CMT/src/cm_checker.py` and/or `CMT/src/enums.py`.

### the agent's Discretion
- Downstream agents may choose the exact Slint component names and Rust module file names as long as the decisions above remain true and the code compiles cleanly.
- Downstream agents may decide whether placeholder text is a single sentence or a short label per tab, provided it remains inert and scope-note oriented.

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked Phase Requirements
- `.planning/phases/01-slint-shell-port-architecture/01-SPEC.md` - Locked Phase 1 requirements, boundaries, constraints, and acceptance criteria.
- `.planning/ROADMAP.md` - Phase 1 goal, success criteria, requirement mapping, and UI hint.
- `.planning/REQUIREMENTS.md` - `FOUND-01` through `FOUND-05` and `SAFE-05` requirement definitions.

### Project Direction
- `.planning/PROJECT.md` - Project goal, core value, constraints, and read-only `CMT/` rule.
- `.planning/research/SUMMARY.md` - Research synthesis for stack, architecture, expected features, and pitfalls.
- `.planning/research/STACK.md` - Recommended Rust/Slint stack and dependency guidance.
- `.planning/research/ARCHITECTURE.md` - Recommended component boundaries and build order.

### Reference App Sources
- `CMT/src/cm_checker.py` - Reference app shell construction and tab creation order.
- `CMT/src/enums.py` - Reference `Tab` enum labels: `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
- `AGENTS.md` - Current project-specific agent instructions, including CMT read-only and verification requirements.

## Existing Code Insights

### Reusable Assets
- `Cargo.toml` - Existing Rust package metadata (`cmt-rs`, edition 2024); Phase 1 extends this with dependencies/build config.
- `Cargo.lock` - Existing lockfile; dependency changes should update it normally.
- `src/main.rs` - Current console-only entry point to replace with Slint startup.

### Established Patterns
- No Rust app architecture pattern exists yet; Phase 1 establishes the first pattern.
- The project convention is to preserve comments and add Rust doc comments to public functions/types or substantially rewritten methods.
- The project requires `CMT/` to remain read-only and reference files to be inspected before porting behavior.

### Integration Points
- `build.rs` will become the Slint compile integration point if the implementation follows the researched external `.slint` file pattern.
- `ui/main.slint` and per-tab Slint files will become the visible shell integration point.
- `src/app`, `src/domain`, `src/platform`, and `src/workers` will be future integration points for controller, domain, OS boundary, and background work phases.

## Specific Ideas

- Use scope-note placeholders in each tab, for example wording that says the tab is reserved for a later port phase rather than implying real behavior exists.
- Keep Phase 1 tests focused on the shell contract: label/order constants and compile-time structure, not GUI automation or downstream behavior.

## Deferred Ideas

None - discussion stayed within phase scope.

---

*Phase: 1-Slint Shell & Port Architecture*
*Context gathered: 2026-05-17*
