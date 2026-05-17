# T01: Plan 01

**Slice:** S01 — **Milestone:** M001

## Description

Establish the buildable Rust/Slint walking skeleton: Cargo knows about Slint, build.rs compiles an external Slint file, and src/main.rs launches a generated MainWindow instead of printing Hello World.

Purpose: This creates the first executable vertical slice for the port foundation while keeping the UI inert and deferring tab behavior to later plans and phases.
Output: Updated Cargo dependency baseline, Slint build script, Rust entry point, and minimal external MainWindow file.
