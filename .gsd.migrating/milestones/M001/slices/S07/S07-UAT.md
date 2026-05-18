# S07: Scanner Read Only Results — UAT

**Milestone:** M001
**Written:** 2026-05-18T07:33:53.636Z

## UAT Type

Agent-verifiable integration UAT for the read-only Scanner tab. No human game installation is required; behavior is proven by fake-backed Rust tests, Slint source-contract tests, runtime wiring tests, and cargo gates.

## Preconditions

1. The app is built from the current S07 code.
2. Scanner settings are available through SettingsController.
3. Filesystem, discovery, mod-manager, Overview, clipboard, and desktop seams can be substituted by tests.

## Steps and Expected Outcomes

1. Open the Scanner tab.
   - Expected: the tab shows Scan Settings with Overview Issues, Errors, Wrong File Formats, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs in that order.
2. Disable every scanner setting and attempt to scan.
   - Expected: Scan Game is disabled or no worker is scheduled; settings are not persisted outside the scan-start flow.
3. Start a scan with enabled settings and a fake Data tree.
   - Expected: old rows/details clear, the button uses Scanning..., progress emits Refreshing Overview..., Building mod file index... when MO2 is present, and Scanning... n/N: folder for top-level Data traversal.
4. Include Overview problems, wrong-format files, loose previs folders, junk files, F4SE script overrides, invalid BA2 names, and race-subgraph-heavy modules.
   - Expected: results are gated by the corresponding toggles, grouped in reference order, sorted deterministically, and include safe summaries/solutions/details.
5. Scan with MO2 context and enabled modlist order plus overwrite.
   - Expected: staged files are attributed to the winning mod; missing modlist or prerequisites show safe Errors rows and diagnostics instead of panics.
6. Scan with Vortex context.
   - Expected: Scanner scans Data only and does not fabricate mod attribution.
7. Select a result and trigger Copy Details, Open Location, Open URL, Copy URL, and File List where available.
   - Expected: actions are read-only, go through fakeable adapters, and success/failure feedback is safe and inline. Auto-Fix, Fixed, and Fix Failed controls are absent.
8. Inject stale worker progress/completion/action events and worker spawn failures.
   - Expected: stale events are ignored, spawn failures map to safe Scanner status, and the UI remains in a recoverable state.

## Edge Cases Covered

- Missing Data returns a safe visible row/status and no traversal.
- Unreadable child directories produce Errors rows and continue siblings.
- Unreadable module bytes are diagnostic-only for race counting and do not abort the scan.
- Malformed modlist, all toggles off, unexpected extensions with and without proper replacements, invalid BA2 suffixes, already-enabled archives, zero results, save rollback, stale events, and clipboard/desktop failures are covered.

## Not Proven By This UAT

- Real end-user Fallout 4 installation performance on a large mod list.
- Future Auto-Fix write actions, which are intentionally deferred to S08.
- Human visual comparison against the original Tkinter app beyond source-contract/layout tests.
