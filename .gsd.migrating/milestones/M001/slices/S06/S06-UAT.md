# S06: F4SE Diagnostics — UAT

**Milestone:** M001
**Written:** 2026-05-18T04:50:51.705Z

## UAT Type

Manual exploratory/reference-parity UAT for the desktop F4SE diagnostics tab.

## Preconditions

- Build and launch the Rust/Slint Collective Modding Toolkit from this slice.
- Have a test Fallout 4 installation or fakeable discovered environment available.
- For positive-path inspection, ensure `Data/F4SE/Plugins` exists and contains representative direct-child DLLs, including at least one normal F4SE plugin DLL and optionally an `msdia*.dll` helper.
- For missing/empty edge cases, use a disposable test install or temporary copy so folders can be renamed/emptied safely.

## Steps and Expected Outcomes

1. Launch the app and confirm the main tab order remains `Overview`, `F4SE`, `Scanner`, `Tools`, `Settings`, `About`.
   - Expected: The F4SE tab is present in the reference position and no scan visibly blocks startup.
2. Open the `F4SE` tab for the first time.
   - Expected: The tab shows the heading `F4SE DLLs` and the loading text `Scanning DLLs...` while work runs off the UI thread.
3. Wait for the scan to complete.
   - Expected: A read-only table appears with columns `DLL`, `OG`, `NG`, `AE`, and `Your Game`, followed by the reference F4SE DLL legend text and icon meanings.
4. Inspect rows for direct child DLLs under `Data/F4SE/Plugins`.
   - Expected: Each scanned DLL appears by filename; compatibility is shown only where exports/version data prove support; unknown or inconclusive support remains warning/unknown rather than falsely compatible.
5. Include an `msdia*.dll` helper in the plugin folder and reopen/relaunch for a scan.
   - Expected: The `msdia*` helper is ignored and does not appear as a compatibility row.
6. Put a DLL inside a nested subfolder under `Data/F4SE/Plugins`.
   - Expected: Nested DLLs are not scanned because S06 only scans direct children.
7. Test an empty `Data/F4SE/Plugins` folder.
   - Expected: The tab shows the normal empty table/empty state plus the legend, not a hard error.
8. Test missing `Data` or missing `Data/F4SE/Plugins`.
   - Expected: The visible error is `Data folder not found` or `Data/F4SE/Plugins folder not found`; when no mod manager is detected, the message appends `Try launching via your mod manager.`
9. Test with an unknown/unclassifiable current game version.
   - Expected: DLL facts still display, and the `Your Game` column/status shows a warning explanation rather than a misleading hard failure.
10. Leave the tab and return to it.
    - Expected: The initial scan is not repeatedly rescheduled by normal tab switching in this slice.

## Edge Cases to Exercise

- Unreadable DLL file.
- Malformed/non-PE DLL bytes.
- DLL with F4SE load/preload but no query/version facts.
- DLL with NG/AE-compatible versions that do not match the known reference mappings.
- Worker spawn or scan failure in a fake-backed test harness.

## Not Proven By This UAT

- Exhaustive compatibility for every real-world F4SE plugin.
- A curated compatibility database or filename/mod-name heuristics; these are intentionally out of scope.
- Manual refresh, selected-row details, copy/open-location actions, or scanner auto-fix workflows.
- Long-duration performance with very large plugin directories beyond the automated non-blocking/worker tests.
- Human accessibility review of the rendered Slint table.
