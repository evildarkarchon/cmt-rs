# T02: Plan 02

**Slice:** S02 — **Milestone:** M001

## Description

Implement reference-compatible settings file IO around the domain model.

Purpose: Users need first-run defaults, safe repair, persistence, and test-injectable paths before the Settings tab can save choices.
Output: A platform settings store with current-directory production path, asset resolver for `download-source.txt`, and tests for missing, malformed, partial, and save-failure cases.
