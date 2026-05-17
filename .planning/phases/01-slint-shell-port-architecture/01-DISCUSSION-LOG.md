# Phase 1: Slint Shell & Port Architecture - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 1-Slint Shell & Port Architecture
**Areas discussed:** UI file shape, Rust skeleton, Version/deps, Verification

---

## Gray Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| All recommended | Discuss the few decisions most likely to affect downstream planning | |
| UI file shape | How to organize Slint files and tab placeholders | yes |
| Rust skeleton | How much app/domain/platform/workers structure to create now | yes |
| Version/deps | Whether to lock exact Slint versions and build setup now | yes |
| Verification | How strict the CMT label/cleanliness checks should be | yes |

**User's choice:** UI file shape, Rust skeleton, Version/deps, Verification
**Notes:** SPEC.md already locked requirements; discussion focused only on implementation decisions.

---

## UI Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Single main.slint | Keep all tabs/placeholders in one UI file for the shell phase | |
| Main plus tabs | Create main.slint plus one component file per tab now | yes |
| Hybrid | main.slint with small reusable placeholder component only | |

**User's choice:** Main plus tabs
**Notes:** Placeholder content should use scope notes rather than domain behavior.

---

## Rust Skeleton

| Option | Description | Selected |
|--------|-------------|----------|
| Module stubs | Create modules with doc comments and minimal no-op types only | yes |
| Controller shell | Create app controller wiring now, even if callbacks are inert | |
| Directory markers | Only mod.rs files and TODO-level structure | |

**User's choice:** Module stubs
**Notes:** This keeps Phase 1 small while establishing boundaries for later phases.

---

## Dependencies

| Option | Description | Selected |
|--------|-------------|----------|
| Exact Slint 1.16.1 | Use slint/slint-build 1.16.1 exactly; defer other deps | |
| Compatible Slint 1.16 | Use semver-compatible 1.16 versions; defer other deps | |
| Add stack baseline | Add Slint plus serde/tracing/tokio skeleton deps now | yes |

**User's choice:** Add stack baseline
**Notes:** Scanner/archive/Fallout-specific parsing dependencies remain deferred.

---

## Verification

| Option | Description | Selected |
|--------|-------------|----------|
| CMT status check | Require `git status --short CMT` plus cite CMT tab source files | |
| Automated label test | Also require a Rust test asserting the six shell tab labels/order | yes |
| Manual only | Manual launch plus CMT status check is enough | |

**User's choice:** Automated label test
**Notes:** Normal cargo checks and CMT cleanliness check remain mandatory.

---

## Placeholders

| Option | Description | Selected |
|--------|-------------|----------|
| Plain tab names | Minimal inert content: tab name only | |
| Scope notes | Short text saying behavior is intentionally future-phase work | yes |
| Reference notes | Mention the matching CMT source file for each tab | |

**User's choice:** Scope notes
**Notes:** Avoid implying real tab behavior exists in Phase 1.

## the agent's Discretion

- Exact Slint component names and Rust module file names are left to downstream agents.
- Exact placeholder sentence wording is flexible if it remains short, inert, and scope-note oriented.

## Deferred Ideas

None.
