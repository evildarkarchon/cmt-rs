# S08: Scanner Auto Fix Actions — UAT

**Milestone:** M001
**Written:** 2026-05-18T09:47:38.170Z

## UAT Type

Contract and runtime integration UAT with fake-backed Auto-Fix operations. Production filesystem mutation is intentionally not part of this slice.

## Preconditions

1. Build and run from the Rust project outside `CMT/`; treat `CMT/` as read-only reference material.
2. Use the production runtime registry for normal-user checks; it must contain no registered Auto-Fix operations.
3. Use the fake registry/runtime tests for supported-operation lifecycle checks.

## Steps and Expected Outcomes

1. Open the Rust CMT app and navigate to the Scanner tab.
   - Expected: The Scanner tab still behaves as in S07 for read-only scan results.
   - Expected: Unsupported results show no `Auto-Fix` button and no disabled placeholder.

2. Select Scanner results with typed or display-only solution text under the production registry.
   - Expected: Because the production registry is empty, no normal result exposes a mutating Auto-Fix action.
   - Expected: Display strings are not used to infer Auto-Fix eligibility.

3. Exercise a fake registered operation through the runtime wiring test path.
   - Expected: The selected detail area shows `Auto-Fix`, then `Fixing...`, then `Fixed!` on success.
   - Expected: Inline result details use the `Auto-Fix Results` heading/copy.
   - Expected: The matching row records fixed/check state only after success.

4. Exercise a fake operation failure.
   - Expected: The selected detail area transitions from `Fixing...` to `Fix Failed`.
   - Expected: Safe failure text is visible inline; raw diagnostics are retained for tests/logs rather than primary UI copy.

5. Invoke stale, unsupported, tampered, unconfirmed, missing-target, or failed-precondition requests.
   - Expected: Requests fail closed before any mutating operation runner is called.
   - Expected: The UI receives safe `Fix Failed` feedback instead of mutating files or trusting stale scan-time facts.

6. Deliver a stale worker completion after the selected result or scan id has changed.
   - Expected: The controller ignores the completion and does not overwrite the newer selected-result state.

7. Run the closeout verification suite.
   - Expected: Targeted Auto-Fix domain, service, controller, worker-payload, Slint-contract, and runtime-wiring tests pass.
   - Expected: `cargo fmt --check`, `cargo check`, full `cargo test`, and `cargo clippy --all-targets --all-features` pass.

## Edge Cases Covered

- Empty production registry.
- Unsupported solution/result type.
- Display-only solution strings.
- Stale scan id/result identity.
- Tampered callback/result index/operation key.
- Missing target path and failed preconditions.
- Declined or missing confirmation.
- Worker spawn/failure mapping to safe UI feedback.
- Stale completion ignored after newer selection/scan state.

## Not Proven By This UAT

- Real delete, rename, move, archive, backup, restore, or repair operations.
- Actual user filesystem mutation.
- Downgrade Manager or Archive Patcher behavior.
- Live filesystem monitoring or automatic rescan after a fix.
- Pixel-perfect visual comparison against the Python/Tkinter UI.
