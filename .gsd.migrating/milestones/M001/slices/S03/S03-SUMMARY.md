---
id: S03
parent: M001
milestone: M001
provides:
  - Pure Fallout 4 discovery and mod-manager domain contracts with reference-compatible messages.
  - Fakeable platform adapters for filesystem, registry, process/system/version metadata, and desktop/tool actions.
  - A discovery orchestration service for Fallout 4, MO2, Vortex, and system metadata.
  - Typed worker event, cancellation, handoff, and off-UI-thread execution contracts.
requires:
  []
affects:
  - S04
  - S05
  - S06
  - S07
  - S08
  - S09
  - S10
key_files:
  - src/domain/discovery.rs
  - src/domain/mod_manager.rs
  - src/domain/mod.rs
  - src/platform/mod.rs
  - src/platform/filesystem.rs
  - src/platform/registry.rs
  - src/platform/process.rs
  - src/platform/desktop.rs
  - src/services/mod.rs
  - src/services/discovery.rs
  - src/workers/events.rs
  - src/workers/handoff.rs
  - src/workers/mod.rs
  - src/main.rs
  - Cargo.toml
  - Cargo.lock
key_decisions:
  - Discovery and mod-manager domain contracts remain pure and do not touch filesystem, registry, process, Slint, or desktop state.
  - Platform seams are fakeable traits with typed platform failures and cfg-gated real adapters.
  - Discovery orchestration is UI-prompt-free and preserves the reference search order.
  - Manager-specific MO2 failures block silent fallback so incomplete or non-Fallout manager state remains visible.
  - WorkerRuntime uses the active Tokio runtime handle and emits owned WorkerEvent envelopes through handoff sinks.
patterns_established:
  - Keep domain contracts pure, platform facts injectable, and orchestration in services.
  - Separate user-safe messages from diagnostic details for discovery, platform, manager, and worker failures.
  - Use fake-backed tests for OS-dependent discovery behavior instead of querying the real host.
  - Send owned worker events through recording or Slint event-loop sinks rather than mutating UI state from background work.
observability_surfaces:
  - Structured tracing for discovery manager detection, registry read failures, system metadata failures, MO2 parsing, and worker lifecycle/failure paths.
  - Ordered discovery attempts and manager-discovery steps in DiscoveryReport for later UI/diagnostic display.
  - RecordingEventSink for inspectable worker event sequences in tests and diagnostics.
drill_down_paths:
  - .gsd/milestones/M001/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S03/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S03/tasks/T03-SUMMARY.md
  - .gsd/milestones/M001/slices/S03/tasks/T04-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-05-17T10:38:06.635Z
blocker_discovered: false
---

# S03: S03

**Established typed Fallout 4 discovery contracts, fakeable platform seams, a tested discovery orchestration service, and reusable worker event handoff foundations without launching the GUI.**

## What Happened

S03 created the Phase 3 backend foundation for later Overview, F4SE, Scanner, Tools, Downgrader, and Archive Patcher work. The domain layer now represents Fallout 4 installation state, optional Data and Data/F4SE/Plugins paths, archive/module/INI records, semantic versions, install-type labels, Vortex identity-only context, MO2 directory/profile/skip-rule context, MO2 configuration output, and typed discovery/MO2 errors with reference-compatible user messages.

The platform layer now exposes injectable traits for filesystem, registry, process/system/version metadata, and desktop/tool actions. Real adapters are implemented through standard Rust crates/native APIs where available, return typed unsupported behavior on unsupported platforms, and remain replaceable with fakes in tests.

The service layer now contains DiscoveryService, which orchestrates injected platform adapters without UI prompts. Game discovery preserves the locked reference search order: running manager game path, current working directory, Bethesda registry path, then GOG registry path. Direct Fallout4.exe candidates normalize to their parent directory, valid game directories can produce partial derived state when Data or Data/F4SE/Plugins are absent, registry and not-found failures remain typed/recoverable, and discovery reports include ordered attempts plus fake-backed system metadata for later PC Specs display.

MO2 discovery checks adjacent portable files before HKCU CurrentInstance and LOCALAPPDATA instance configuration, then falls back to portable INI. It parses gamePath, selected_profile, mod/overwrite/profiles/cache/download directories, profile-local flags, skip rules, and supported custom executable paths. Missing, incomplete, or non-Fallout MO2 state returns manager-specific typed errors instead of panicking or silently falling through. Vortex detection returns display name, executable path, parsed or fallback version, and deliberately performs no staging/config parsing.

The worker layer now contains reusable owned event contracts, cancellation tokens, recording and Slint event-loop handoff sinks, typed handoff errors, and a Tokio spawn_blocking facade for slow future work. Worker lifecycle, cancellation, failure, panic, and handoff-failure paths emit structured logs and owned events rather than blocking or mutating UI state directly.

## Verification

All four S03 tasks are complete and the recorded current-host verification gates passed via closeout-safe GSD verification evidence: cargo fmt --check passed, cargo check passed, cargo test passed with 87 tests, and cargo clippy --all-targets --all-features passed. Task-level coverage includes pure domain error/message contracts, fake platform adapter behavior, discovery ordering, path normalization, optional derived paths, registry error handling, MO2 portable/instance ordering and typed failures, Vortex parsed/fallback versions, fake-backed system metadata inclusion, worker event payloads, recording/Slint handoff sinks, cancellation flows, safe failure events, and off-calling-thread blocking execution.

## Requirements Advanced

- Later Overview work can consume typed installation, manager, and system metadata state without direct OS queries in tests. — 
- Later F4SE/Scanner work can distinguish valid game paths from missing optional Data or Data/F4SE/Plugins folders. — 
- Later tools and background workflows can use typed platform and worker boundaries instead of blocking Slint callbacks. — 

## Requirements Validated

- Discovery ordering, not-found behavior, direct executable normalization, partial derived paths, MO2 parsing, MO2 portable/instance order, MO2 typed failures, Vortex identity-only detection, and fake-backed PC Specs metadata are covered by src/services/discovery.rs unit tests. — 
- Platform adapter fake/real boundary behavior is covered by platform unit tests. — 
- Worker event and handoff behavior is covered by worker unit tests. — 

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

None.

## Operational Readiness

None.

## Deviations

The closeout was resumed after an interruption; the prior session had already produced the slice summary/UAT drafts and closeout-safe verification evidence, and this completion canonicalized them through gsd_slice_complete.

## Known Limitations

No live Slint UI integration or real-host Fallout 4/MO2/Vortex probing is exposed yet; this slice provides contracts, adapters, service orchestration, and fake-backed tests for later vertical slices. Real Windows registry/process/desktop behavior is typechecked through cfg-gated code but should still be exercised on a Windows host when those workflows are connected to the UI.

## Follow-ups

Connect Overview/F4SE/Scanner tabs to DiscoveryService through the worker facade in later slices. Runtime-test Windows-specific real adapters on a Windows host before relying on live registry/process/desktop behavior in user workflows. Preserve ordered discovery attempts and manager steps as user-visible diagnostics when the UI surfaces discovery failures.

## Files Created/Modified

- `src/domain/discovery.rs` — Fallout 4 installation, derived paths, archive/module/INI records, semantic versions, install types, and discovery errors.
- `src/domain/mod_manager.rs` — MO2/Vortex identity, MO2 directories/profile/skip-rule context, configuration shape, and typed manager errors.
- `src/domain/mod.rs` — Domain module exports and import smoke coverage.
- `src/platform/mod.rs` — Shared platform error/operation contract and adapter module exports.
- `src/platform/filesystem.rs` — Fakeable filesystem adapter trait and real/fake implementations.
- `src/platform/registry.rs` — Fakeable registry reader trait and cfg-gated real registry behavior.
- `src/platform/process.rs` — Fakeable process, executable-version, and system metadata trait plus real/fake implementations.
- `src/platform/desktop.rs` — Fakeable desktop URL/path/tool action contract and real/fake implementations.
- `src/services/mod.rs` — Service-layer exports.
- `src/services/discovery.rs` — Fallout 4, MO2, Vortex, registry, and system metadata discovery orchestration with fake-backed tests.
- `src/workers/events.rs` — Worker task, status, payload, progress, cancellation, external-action, and failure contracts.
- `src/workers/handoff.rs` — Recording and Slint event-loop worker event sinks.
- `src/workers/mod.rs` — Tokio blocking worker facade, cancellation handle, and lifecycle logging.
- `src/main.rs` — Module exports/importability coverage for service and worker boundaries.
- `Cargo.toml` — Dependencies required for platform adapters, async worker facade, and tests.
- `Cargo.lock` — Resolved dependency updates for S03 platform/service/worker dependencies.
