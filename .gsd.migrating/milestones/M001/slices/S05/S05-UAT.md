# S05: Tools Shell, Links & About — UAT

**Milestone:** M001
**Written:** 2026-05-18T02:18:27.909Z

## UAT Type

Manual desktop smoke test plus automated contract/regression coverage for failure paths.

## Preconditions

- Build from `J:/cmt-rs` with the S05 implementation present.
- A normal desktop session is available with a default browser and clipboard service for positive-path manual checks.
- Do not modify files under `CMT/`; it remains reference-only.

## Steps

1. Launch the Rust app, for example with `cargo run` from the project root.
2. Confirm the window is titled `Collective Modding Toolkit` and the tab order remains `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
3. Open the `Tools` tab.
4. Verify the group order is `Toolkit Utilities`, `Other CM Authors' Tools`, then `Other Useful Tools`.
5. Verify `Downgrade Manager` and `Archive Patcher` are visible in `Toolkit Utilities` but disabled/deferred with clear S09/S10-style status text; clicking them must not start downloads, backups, patch plans, archive writes, or modal workflows.
6. Verify the external tool buttons preserve the reference labels/order and show their helper text. Click representative enabled entries and confirm either the appropriate static URL is opened by the OS or a visible inline failure message appears without freezing or crashing the app.
7. Open the `About` tab.
8. Verify the CMT title, version/credit attribution, and Nexus Mods, Discord, and GitHub rows are visible. Logos should render from the Rust-owned resources when available; text/buttons must remain usable if an image is unavailable.
9. Click each About `Open Link` action. Confirm it opens the corresponding static link or displays a safe inline error if the desktop open operation fails.
10. Click each About `Copy Link` action. Confirm the clicked copy button briefly changes to `Copied!`, is then reset to its original label after the timer, and the copied URL can be pasted into a scratch field/document when the platform clipboard succeeds.
11. Repeat an open/copy action while the OS browser or clipboard is unavailable, if practical in the test environment. Confirm the app shows safe visible feedback and remains responsive.

## Expected Outcomes

- Tools and About are no longer inert placeholders.
- Tools group labels, button ordering, visible utility entries, and help text match the reference-shaped S05 contract.
- Destructive utility workflows are fail-closed and cannot run in S05.
- About attribution, static link actions, copied-link feedback, and image identity are visible.
- Link/copy failures are surfaced through inline status/error feedback rather than panics, silent failures, or blocked UI.

## Edge Cases

- Desktop open adapter failure: user sees a safe inline failure message and may retry after fixing browser/default-app configuration.
- Clipboard adapter failure or unsupported clipboard: user sees a safe inline failure message and the copy button does not pretend success.
- Unknown or mismatched callback id: automated tests prove it is rejected before platform adapters are invoked.
- Missing About image resource: the tab should remain text/action usable rather than breaking the entire surface.

## Not Proven By This UAT

- Live Downgrade Manager behavior, downloads, backups, delta cleanup, or version switching; this remains S09.
- Live Archive Patcher parsing, write plans, backups, or archive mutation; this remains S10.
- F4SE diagnostics and Scanner result workflows; these remain later slices.
- Final installer/package resource validation outside the current Cargo/Slint project layout.
- Exhaustive real-world OS/browser/clipboard failure matrices beyond the automated fake-adapter tests and practical manual checks.
