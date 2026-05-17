# T04: Plan 04

**Slice:** S02 — **Milestone:** M001

## Description

Wire the Settings UI to persisted settings with immediate save and fail-safe UI state.

Purpose: Phase 2 is complete only when visible Settings choices are backed by `settings.json` persistence and save failures do not leave the UI lying about persisted state.
Output: App/controller wiring that loads settings at startup, binds Slint callbacks, persists `update_source` and `log_level`, and reverts UI properties on save errors.
