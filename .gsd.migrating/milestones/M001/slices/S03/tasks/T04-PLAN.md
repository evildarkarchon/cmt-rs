# T04: 03-platform-discovery-background-adapters 04

**Slice:** S03 — **Milestone:** M001

## Description

Replace the inert worker boundary with reusable typed worker event contracts, cancellation states, recording/Slint handoff sinks, and a small off-UI-thread execution facade.

Purpose: Later scans, filesystem traversal, parsing, downloads, patching, and process monitoring need consistent progress/completion/cancellation/error events delivered through Slint-safe handoff.
Output: Worker event and handoff modules with unit tests that require no Slint window, plus compile-safe Slint event-loop sink code.

## Must-Haves

- [ ] "D-09: Worker events use a shared envelope with task identity/kind/status metadata plus typed payload variants."
- [ ] "D-10: Events distinguish discovery, scan, patch, download, external process, and generic/unknown task kinds."
- [ ] "D-11: Progress events support optional human-readable text plus optional current/total counts without requiring percentages, rates, or ETA."
- [ ] "D-12: Worker events distinguish cancellation request/acknowledgement from final cancelled completion."
- [ ] "D-16: External process/desktop action results can flow through worker payloads with operation kind, target, success/failure, and safe message."
- [ ] "Owned background events can be emitted through recording and Slint event-loop sinks without worker code mutating Slint models or UI handles directly."
- [ ] "Blocking work has an explicit worker facade that uses off-UI-thread execution and emits typed events."

## Files

- `src/workers/events.rs`
- `src/workers/handoff.rs`
- `src/workers/mod.rs`
