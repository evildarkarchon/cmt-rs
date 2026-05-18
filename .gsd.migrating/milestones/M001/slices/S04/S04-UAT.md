# S04: Overview Diagnostics & Updates — UAT

**Milestone:** M001
**Written:** 2026-05-18T00:23:25.036Z

## UAT Type

Manual desktop acceptance / exploratory verification. This UAT is designed for a normal developer machine and may be run with or without a discovered Fallout 4 install; fake-backed automated tests provide the exhaustive edge coverage.

## Preconditions

1. The repository is checked out at the S04-complete state.
2. Rust/Cargo dependencies are available.
3. Optional: a Fallout 4 install or fixture directory is available so Overview can show discovered paths and counts; without one, the no-install inline state is still valid.
4. Optional: network access is available for update-link behavior; no-update and failed update checks are expected to stay silent in the UI.

## Steps and Expected Outcomes

1. Run `cargo run` and wait for the Slint window.
   - Expected: the app opens as `Collective Modding Toolkit` and the first tab is `Overview`.
2. Inspect the top Overview area.
   - Expected: `Refresh` and `Open Game Path` controls are present, and the Status panel is populated from typed rows for `Mod Manager`, `Game Path`, `Version`, and `PC Specs`.
   - Expected: if no game path is discovered, the path state is inline and `Open Game Path` is disabled rather than showing a modal warning.
3. Click `Refresh`.
   - Expected: the refresh message changes to a busy/loading state, the UI remains responsive, and it later resolves to ready, partial, or safe error text.
   - Expected: repeated Refresh clicks do not apply stale results out of order.
4. Inspect the three diagnostic panels.
   - Expected: `Binaries (EXE/DLL/BIN)` shows binary/install-type rows plus Address Library status and version/hash details when available.
   - Expected: `Archives (BA2)` shows General, Texture, Total, Unreadable, `v1 (OG)`, and `v7/8 (NG)` counts/status.
   - Expected: `Modules (ESM/ESL/ESP)` shows Full, Light, Total, Unreadable, `HEDR v1.00`, `HEDR v0.95`, and `HEDR v????` counts/status.
5. Inspect the Problems area.
   - Expected: it shows `Problems: 0` and `No problems detected.` when clean, or safe inline rows when discovery/diagnostic issues exist.
   - Expected: missing Data, missing `Fallout4.ccc`, missing `plugins.txt`, unreadable/invalid archives/modules, unknown binary versions, and exceeded limits appear as inline problem/status rows rather than modal interruptions.
6. Exercise update-source behavior from Settings.
   - Set Update Channel to `None`, return to Overview, and refresh.
     - Expected: no update request is needed and no green update banner is shown.
   - Set Update Channel to `Nexus`, `GitHub`, or `Both`, return to Overview, and refresh.
     - Expected: selected source(s) are eligible for checks; a green banner appears only if a newer version is found. Equal/older, malformed, failed, or offline checks stay silent in the UI.
7. If a game path is discovered, click `Open Game Path`.
   - Expected: the folder opens through the OS desktop adapter, or a safe visible last-action error appears if opening fails.
8. If an update banner is visible, click each enabled update-link button.
   - Expected: the selected Nexus/GitHub link opens through the OS desktop adapter, or a safe visible last-action error appears if opening fails.
9. Inspect deferred utility controls.
   - Expected: `Downgrade Manager...` and `Archive Patcher...` are visible in the reference-aligned panel positions but disabled/deferred with explanatory text.

## Edge Cases to Try

- Launch without a Fallout 4 install or with an invalid configured/current directory.
- Use a Data folder with unreadable or malformed `.ba2`, `.esm`, `.esl`, or `.esp` files.
- Remove `Fallout4.ccc`, `plugins.txt`, or Address Library files from a fixture copy.
- Simulate offline update checks or blocked desktop opens.
- Refresh repeatedly while a previous refresh is still in flight.

## Expected Operational Signals

- Health signal: Overview refresh message, populated diagnostic panels, problem count, and successful cargo gates.
- Failure signal: safe refresh error text, safe last-action error banner, problem-feed rows, and structured tracing events for refresh, collection, update, desktop action, and worker handoff failures.
- Recovery procedure: click Refresh after fixing local files/settings; change Update Channel to `None` to skip network checks; use panel/problem details to identify missing or unreadable files.
- Monitoring gaps: no persistent telemetry dashboard and no automated pixel-perfect visual comparison against the Python reference; live network/provider behavior remains environment-dependent.

## Not Proven By This UAT

- Scanner UI rendering or auto-fix actions.
- Live Downgrade Manager behavior, downloads, backups, or mutation workflows.
- Live Archive Patcher write/backup behavior.
- Exhaustive validation against every real Fallout 4 mod loadout.
- Pixel-perfect parity with the Tkinter Overview under all DPI/theme combinations.
