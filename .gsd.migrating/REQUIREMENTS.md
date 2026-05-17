# Requirements

This file is the explicit capability and coverage contract for the project.

## Active

### R012 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R013 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R014 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R015 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R016 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R017 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R018 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R019 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R020 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R021 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R022 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R023 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R024 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R025 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R026 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R027 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R028 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R029 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R030 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R031 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R032 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R033 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R034 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R035 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R036 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R037 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R038 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R039 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R040 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R041 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R042 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R043 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R044 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R045 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R046 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R047 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R048 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R049 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R050 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R051 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R052 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

### R053 — Untitled
- Class: core-capability
- Status: active
- Source: inferred

## Validated

### R001 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

### R002 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

### R003 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

### R004 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

### R005 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

### R006 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred
- Primary owning slice: S02
- Validation: S02 verifies first-run settings load reference-compatible defaults through domain and settings-store tests plus full Rust gates.

### R007 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred
- Primary owning slice: S02
- Validation: S02 verifies persisted JSON keys and values for `log_level`, `update_source`, scanner toggles, and downgrader settings through typed domain/store tests plus full Rust gates.

### R008 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred
- Primary owning slice: S02
- Validation: S02 Settings-tab Slint source contract tests verify update channel labels and order; full Rust gates passed.

### R009 — User can choose log level options matching the validated Settings contract: `Debug`, `Info`, `Warning`, and `Error`.
- Class: core-capability
- Status: validated
- Description: User can choose log level options matching the validated Settings contract: `Debug`, `Info`, `Warning`, and `Error`.
- Source: inferred
- Primary owning slice: S02
- Validation: S02 Settings source-level contract tests and full Rust verification (`cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`) passed.
- Notes: S02 intentionally includes `Warning` because the reference settings schema accepts persisted `WARNING`; this resolves the discrepancy with the Python tab source, which visibly listed only Debug/Info/Error.

### R010 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred
- Primary owning slice: S02
- Validation: S02 domain settings tests verify scanner-related defaults, including Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs; full Rust gates passed.

### R011 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred
- Primary owning slice: S02
- Validation: S02 domain and store tests verify malformed, invalid, and partial settings repair behavior, including preserving valid values and restoring defaults where needed; full Rust gates passed.

### R054 — Untitled
- Class: core-capability
- Status: validated
- Source: inferred

## Deferred

## Out of Scope

## Traceability

| ID | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|
| R001 | core-capability | validated | none | none | unmapped |
| R002 | core-capability | validated | none | none | unmapped |
| R003 | core-capability | validated | none | none | unmapped |
| R004 | core-capability | validated | none | none | unmapped |
| R005 | core-capability | validated | none | none | unmapped |
| R006 | core-capability | validated | S02 | none | S02 verifies first-run settings load reference-compatible defaults through domain and settings-store tests plus full Rust gates. |
| R007 | core-capability | validated | S02 | none | S02 verifies persisted JSON keys and values for `log_level`, `update_source`, scanner toggles, and downgrader settings through typed domain/store tests plus full Rust gates. |
| R008 | core-capability | validated | S02 | none | S02 Settings-tab Slint source contract tests verify update channel labels and order; full Rust gates passed. |
| R009 | core-capability | validated | S02 | none | S02 Settings source-level contract tests and full Rust verification (`cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features`) passed. |
| R010 | core-capability | validated | S02 | none | S02 domain settings tests verify scanner-related defaults, including Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs; full Rust gates passed. |
| R011 | core-capability | validated | S02 | none | S02 domain and store tests verify malformed, invalid, and partial settings repair behavior, including preserving valid values and restoring defaults where needed; full Rust gates passed. |
| R012 | core-capability | active | none | none | unmapped |
| R013 | core-capability | active | none | none | unmapped |
| R014 | core-capability | active | none | none | unmapped |
| R015 | core-capability | active | none | none | unmapped |
| R016 | core-capability | active | none | none | unmapped |
| R017 | core-capability | active | none | none | unmapped |
| R018 | core-capability | active | none | none | unmapped |
| R019 | core-capability | active | none | none | unmapped |
| R020 | core-capability | active | none | none | unmapped |
| R021 | core-capability | active | none | none | unmapped |
| R022 | core-capability | active | none | none | unmapped |
| R023 | core-capability | active | none | none | unmapped |
| R024 | core-capability | active | none | none | unmapped |
| R025 | core-capability | active | none | none | unmapped |
| R026 | core-capability | active | none | none | unmapped |
| R027 | core-capability | active | none | none | unmapped |
| R028 | core-capability | active | none | none | unmapped |
| R029 | core-capability | active | none | none | unmapped |
| R030 | core-capability | active | none | none | unmapped |
| R031 | core-capability | active | none | none | unmapped |
| R032 | core-capability | active | none | none | unmapped |
| R033 | core-capability | active | none | none | unmapped |
| R034 | core-capability | active | none | none | unmapped |
| R035 | core-capability | active | none | none | unmapped |
| R036 | core-capability | active | none | none | unmapped |
| R037 | core-capability | active | none | none | unmapped |
| R038 | core-capability | active | none | none | unmapped |
| R039 | core-capability | active | none | none | unmapped |
| R040 | core-capability | active | none | none | unmapped |
| R041 | core-capability | active | none | none | unmapped |
| R042 | core-capability | active | none | none | unmapped |
| R043 | core-capability | active | none | none | unmapped |
| R044 | core-capability | active | none | none | unmapped |
| R045 | core-capability | active | none | none | unmapped |
| R046 | core-capability | active | none | none | unmapped |
| R047 | core-capability | active | none | none | unmapped |
| R048 | core-capability | active | none | none | unmapped |
| R049 | core-capability | active | none | none | unmapped |
| R050 | core-capability | active | none | none | unmapped |
| R051 | core-capability | active | none | none | unmapped |
| R052 | core-capability | active | none | none | unmapped |
| R053 | core-capability | active | none | none | unmapped |
| R054 | core-capability | validated | none | none | unmapped |

## Coverage Summary

- Active requirements: 42
- Mapped to slices: 42
- Validated: 12 (R001, R002, R003, R004, R005, R006, R007, R008, R009, R010, R011, R054)
- Unmapped active requirements: 0
