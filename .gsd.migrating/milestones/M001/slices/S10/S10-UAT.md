# S10: Archive Patcher Workflow — UAT

**Milestone:** M001
**Written:** 2026-05-19T04:10:12.536Z

## UAT: S10 Archive Patcher Workflow

**UAT Type:** Manual destructive-safety smoke test on a sandbox Fallout 4 `Data` directory, backed by the automated Rust/unit/source-contract verification listed in the slice summary.

### Preconditions

1. Run the Rust/Slint app from a build produced after S10.
2. Configure or fake discovery so Overview has a valid Fallout 4 `Data` root and enabled BA2 archive records.
3. Use disposable/sandbox BA2 fixtures or backed-up archives only: include at least one enabled `v1 (OG)` BA2 and one enabled `v7` or `v8 (NG)` BA2.
4. Ensure the app-owned config/manifest location is writable.

### Steps and Expected Outcomes

1. **Open from Overview.**
   - Action: In the Overview tab, click `Archive Patcher...`.
   - Expected: A modal titled `Archive Patcher` opens. `v1 (OG)` is selected by default, write controls are safe, and the candidate list is populated from enabled `v7`/`v8` Overview archive records only.

2. **Verify filtering and target inversion.**
   - Action: Type a mixed-case substring into `Name Filter:`.
   - Expected: Candidate rows update by case-insensitive basename filtering and status/log text reports `Showing N files to be patched.` or `Nothing to do!` as appropriate.
   - Action: Switch to `v8 (NG)`.
   - Expected: Candidates now come from enabled `v1 (OG)` Overview archive records only; no UI-side directory scan is required.

3. **Verify About content.**
   - Action: Click `About`.
   - Expected: The About overlay/dialog shows the reference `Bethesda Archive (BA2) Formats & Versions` title/text and can be dismissed without changing candidates.

4. **Verify confirmation before mutation.**
   - Action: With at least one candidate visible, click `Patch All`.
   - Expected: A read-only preview/confirmation plan is shown before any archive bytes change. Cancelling/leaving the plan state does not patch files.

5. **Confirm patch execution.**
   - Action: Confirm the preview plan.
   - Expected: Patch controls disable while the operation runs, close/Escape is blocked during mutation, log/progress rows stream updates, each valid file logs `Patched to v<target>: <file>`, invalid files are skipped with clear failure messages, and the final log reports `Patching complete. N Successful, M Failed.` Overview refreshes after completion and archive counts/candidates reflect the new header versions.

6. **Restore the latest run.**
   - Action: Reopen Archive Patcher if needed and click `Restore Last Run`.
   - Expected: Restore uses the latest header manifest, writes only entries whose current path/header still match the expected patched state, skips stale/moved/malformed entries safely, streams restore logs, and refreshes Overview after completion.

7. **Open from Tools.**
   - Action: In the Tools tab, click `Archive Patcher`.
   - Expected: The same live modal opens rather than deferred/unavailable feedback, using the same Overview archive source and safe state.

### Edge Cases to Exercise

- Missing or unavailable discovery/Overview archive data opens a safe empty/error modal with write controls disabled and an actionable message.
- Non-`BTDX`, short-header, unknown-version, or unknown-format files are skipped and logged without writing.
- Missing files, permission/in-use failures, and byte-write failures are logged per file and do not stop later files.
- Digest mismatch between preview and confirmation aborts before writing.
- Restore skips manifest entries whose files moved or whose current header no longer matches the expected patched state.

### Not Proven By This UAT

- Real-world performance on very large user mod lists or multi-GB archives beyond the bounded header-write behavior.
- Historical restore browsing or multi-manifest management.
- Cancellation during patch/restore; S10 intentionally has no cancel button.
- Patching every `.ba2` under `Data` regardless of Overview-enabled state.
- Exposing `v7` as a target version.
