# S09: Downgrade Manager Workflow — UAT

**Milestone:** M001
**Written:** 2026-05-19T00:45:03.970Z

# S09 UAT: Downgrade Manager Workflow

## UAT Type

Manual desktop workflow smoke test plus sandbox-backed destructive-path confidence. The automated slice verification proves mutation safety with fake/sandbox files; manual UAT should focus on the visible modal workflow and non-blocking UI behavior.

## Preconditions

1. Run a current build of the Rust/Slint CMT app from a workspace that can discover or be configured with a Fallout 4 installation path.
2. Use a disposable Fallout 4 copy or sandbox fixture for any confirmed patch run; do not run against the user's only live game install during UAT.
3. Ensure the app settings contain known Downgrader options for `Keep Backups` and `Delete Patches` so persistence can be observed.
4. Network access is available if the confirmed run needs to download delta patches.

## Steps and Expected Outcomes

1. Open the app and use the Overview `Downgrade Manager...` action.
   - Expected: A separate fixed-shape window titled `Downgrader` opens without blocking the main UI thread.
   - Expected: The modal shows `Current Game`, `Current Creation Kit`, `Desired Version`, `Options`, `Patch\n All`, `About`, a bottom log area, and a progress bar.

2. Close the modal, then open Tools and use the `Downgrade Manager` utility entry.
   - Expected: The same `Downgrader` window opens from Tools.
   - Expected: Archive Patcher remains deferred/disabled for S10 and is not accidentally enabled.

3. Inspect the current status rows.
   - Expected: The six managed files are represented in reference order: `Fallout4.exe`, `Fallout4Launcher.exe`, `steam_api64.dll`, `CreationKit.exe`, `Archive2.exe`, and `Archive2Interop.dll`.
   - Expected: Status labels use the reference vocabulary where applicable: `Old-Gen`, `Next-Gen`, `Anniversary`, `Obsolete`, `Unknown`, and `Not Found`.

4. Toggle `Old-Gen` / `Next-Gen`, `Keep Backups`, and `Delete Patches`, then click `Patch\n All` once.
   - Expected: The app saves the Downgrader option snapshot used for the workflow.
   - Expected: No file mutation starts on the first click.
   - Expected: An inline plan appears inside the same modal listing skip, restore, backup, download, patch, and cleanup actions as applicable.

5. Use the inline confirmation action to run the reviewed plan.
   - Expected: The patch action becomes disabled while work is running.
   - Expected: Attempts to close the modal or press Escape while running are blocked.
   - Expected: The log receives reference-style rows such as `Skipped {file}: Already ...`, `Skipped {file}: Not Found.`, `Skipped {file}: Unsupported Version.`, `Patched {file}`, or `Failed patching {file}`.
   - Expected: Progress text/percent updates while downloads or patch application are active.

6. Let the workflow finish.
   - Expected: The patch action is re-enabled, the modal can be closed, status rows refresh from disk, and Overview refreshes with the current settings snapshot.
   - Expected: If `Delete Patches` is enabled, downloaded/local delta files used by the workflow are cleaned up after successful patching.
   - Expected: If `Keep Backups` is disabled, no longer-needed backup files are removed according to the plan; if enabled, compatible backups remain.

7. Open the `About` action in the Downgrader modal.
   - Expected: A real modal overlay displays the preserved `About Downgrading Fallout 4 & Creation Kit` title and body copy rather than a deferred/no-op log line.

## Edge Cases to Exercise

1. Missing game root: opening/running the workflow should show a safe failure and never offer destructive mutation.
2. Missing managed file: the row should log `Skipped {file}: Not Found.` and the rest of the queue should continue.
3. Already-target file: the row should be skipped without backup/download/mutation.
4. Unsupported/Unknown/Anniversary/Obsolete CRC: the row should fail closed with a skip or safe failure instead of force-patching.
5. Backup present and valid for the desired target: the plan should restore from backup instead of downloading a delta.
6. Backup present but invalid: the plan should clean up invalid backup state safely before choosing the next action.
7. Download/hash/apply failure: the active file should remain intact, usable backups should be preserved, and a `Failed patching {file}` row should be visible.
8. Files change between preview and confirmation: the confirmed run should abort with the plan-changed safe message and require a new preview.

## Not Proven By This UAT

- Real GitHub availability or bandwidth for every delta asset under all network conditions.
- Real-world downgrade success against every possible modded Fallout 4 installation layout.
- Archive Patcher behavior, which remains scoped to S10.
- Anniversary/AE as a selectable target, game-only mode, cancellation, or offline/manual patch selection; these are explicitly out of scope for S09.
