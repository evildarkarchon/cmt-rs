---
estimated_steps: 23
estimated_files: 11
skills_used: []
---

# T02: Add fakeable clipboard, action services, reducers, and worker payloads

---
estimated_steps: 9
estimated_files: 11
skills_used:
  - verify-before-complete
  - tdd
  - rust-async-patterns
---
Why: Tools/About open and copy actions must be testable and safe without launching a browser or relying on a host clipboard. This task establishes the Slint-free domain/controller/platform layer before UI and main.rs composition.

Do:
1. Add a small `src/platform/clipboard.rs` boundary with a `ClipboardActions` trait, typed `ClipboardActionResult`, fake-friendly failure mapping, and a production adapter. Prefer the focused `arboard` dependency for production clipboard writes; add it to `Cargo.toml` and let `Cargo.lock` update through Cargo.
2. Extend `PlatformOperation` with a clipboard-copy operation and safe success/failure labels while keeping diagnostics separate from UI text.
3. Add `src/services/tools.rs` (or equivalently named action service) that executes domain action requests through injected `DesktopActions` and `ClipboardActions`, rejects unknown/disabled/internal utility ids fail-closed, and returns safe feedback objects containing surface/action identity.
4. Add `src/app/tools_controller.rs` and `src/app/about_controller.rs` as pure reducers with render-ready state: last safe error, disabled utility status, and About copy labels/enabled states. Copy success must set only the targeted button to `Copied!`; reset transitions restore the original reference copy label.
5. Extend `WorkerPayload` and exports in `src/workers/events.rs` / `src/workers/mod.rs` with Tools/About action-completion payloads carrying owned data only. Do not embed Slint handles, borrowed strings, or platform adapters in worker events.
6. Export new modules from `src/app/mod.rs`, `src/platform/mod.rs`, and `src/services/mod.rs`.
7. Add focused tests whose names include `s05_actions` covering open success, desktop open failure, clipboard success, clipboard failure, unsupported platform/adapter failure, unknown action ids, disabled utility rejection, copy-label reset, and worker payload round-tripping.

Done when: action behavior can be fully exercised with fakes, no UI callback performs desktop/clipboard work directly, and all failure paths produce safe user-facing messages plus diagnostic-only details.

Threat Surface (Q3): Slint callback strings must be treated as untrusted ids and parsed against known enums; no arbitrary URL/text copied from UI should reach desktop/clipboard adapters.
Requirement Impact (Q4): supports D021/D022 and S04 decisions D018-D020 by preserving typed controller + fakeable adapter seams; re-run S03/S04 relevant tests through the focused action suite and full cargo test later.
Failure Modes (Q5): desktop adapter unsupported/command failure -> safe last-action error; clipboard unavailable/permission/OS failure -> safe About error; worker spawn failures will be surfaced in T04; lock poisoning should log and avoid panic.
Load Profile (Q6): per click is one worker event and one desktop/clipboard adapter call; rapid clicks can queue multiple URL opens, but copy buttons should disable during the `Copied!` interval.
Negative Tests (Q7): unknown ids, disabled utilities, failed desktop opens, failed clipboard construction/set_text, empty clipboard text rejected if impossible by domain contract, and malformed worker payload ignored by the wrong reducer.

## Inputs

- `src/domain/tools.rs`
- `src/platform/desktop.rs`
- `src/platform/mod.rs`
- `src/services/mod.rs`
- `src/app/mod.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`
- `Cargo.toml`

## Expected Output

- `Cargo.toml`
- `Cargo.lock`
- `src/platform/mod.rs`
- `src/platform/clipboard.rs`
- `src/services/mod.rs`
- `src/services/tools.rs`
- `src/app/mod.rs`
- `src/app/tools_controller.rs`
- `src/app/about_controller.rs`
- `src/workers/events.rs`
- `src/workers/mod.rs`

## Verification

cargo test s05_actions

## Observability Impact

Defines structured action identities and safe feedback types so runtime wiring can log action scheduling, adapter failure kind, and reducer transitions without exposing raw OS diagnostics to users.
