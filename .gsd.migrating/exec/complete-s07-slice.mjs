import { executeSliceComplete } from 'file:///C:/Users/evild/AppData/Roaming/npm/node_modules/gsd-pi/dist/resources/extensions/gsd/tools/workflow-tool-executors.js';

const narrative = [
  'S07 replaced the Scanner placeholder with a faithful read-only Scanner workflow. T01 established the pure domain contract for reference category labels, settings projection, result grouping, detail records, action descriptors, Overview handoff, and copy-details rendering. T02 added the adapter-backed ScannerScanService over the fakeable Filesystem seam, including reference classification rules, MO2 attribution, Vortex Data-only behavior, progress events, safe diagnostics, and partial-failure continuation. T03 added the Slint-free ScannerController, scanner worker payloads, scan-id stale event rejection, action feedback reduction, and save-on-scan-start settings persistence. T04 built the Scanner Slint surface with the seven reference settings labels, Scan Game/Scanning state, status/progress/result count surfaces, grouped results, details, file list, and read-only actions. T05 wired the UI callbacks to real worker scheduling, discovery/Overview collection, scanner progress/completion events, and safe clipboard/desktop actions.',
  '',
  'The completed slice keeps Auto-Fix write behavior deferred. The Scanner can read local Data and manager staging information, but it does not write, delete, rename, patch, archive, or execute user files. Read-only copy/open actions flow through fakeable adapters and report safe inline feedback. The scan service handles missing Data, missing or malformed MO2 prerequisites, unreadable child directories, unreadable module bytes, zero-result scans, stale worker events, save failures, and desktop/clipboard failures without panicking.',
  '',
  'Closeout verification found Windows-specific path display differences that were not visible in the earlier bash/WSL-targeted run. Display-only Scanner detail paths and MO2 parse user messages now normalize separators to forward slashes for deterministic UI/test strings while preserving native PathBuf values for actual file actions.'
].join('\n');

const verification = [
  'Final closeout verification was run through gsd_exec on the Windows-side cargo environment after restoring the missing T02 task summary and fixing display-only path normalization. All targeted S07 filters and full cargo gates exited 0:',
  '',
  '| # | Command | Exit Code | Verdict | Duration |',
  '|---|---------|-----------|---------|----------|',
  '| 1 | cargo test scanner_domain | 0 | pass, 7 passed | 586ms |',
  '| 2 | cargo test scanner_scan_service | 0 | pass, 15 passed | 531ms |',
  '| 3 | cargo test scanner_controller | 0 | pass, 12 passed | 527ms |',
  '| 4 | cargo test scanner_worker_payload | 0 | pass, 1 passed | 512ms |',
  '| 5 | cargo test settings_controller_saves_scanner | 0 | pass, 2 passed | 529ms |',
  '| 6 | cargo test s07_scanner_slint_contract | 0 | pass, 3 passed | 517ms |',
  '| 7 | cargo test s07_scanner_runtime_wiring | 0 | pass, 6 passed | 508ms |',
  '| 8 | cargo fmt --check | 0 | pass | 283ms |',
  '| 9 | cargo check | 0 | pass | 4603ms |',
  '| 10 | cargo test | 0 | pass, 257 passed | 573ms |',
  '| 11 | cargo clippy --all-targets --all-features | 0 | pass with non-fatal warnings | 17644ms |',
  '',
  'The verification proves the Scanner domain/service/controller/runtime contracts, source-level Slint contract, settings save-on-scan behavior, worker payload handoff, safe read-only action feedback, and final build/test/lint gates. Clippy still reports non-fatal warnings, but exits 0 under the current project lint configuration.'
].join('\n');

const operationalReadiness = [
  'Health signal: Scanner UI exposes scan status, progress text/percent, result counts, detail selection state, file-list visibility, and action feedback. ScannerScanOutput carries scan id, status kind, progress history, grouped/flat results, and ScannerScanDiagnostics counts.',
  '',
  'Failure signal: safe rows/statuses cover missing Data, missing MO2 prerequisites or modlist, unreadable directories/files, unreadable module bytes, save failures, spawn failures, stale worker events, and desktop/clipboard failures. Worker events carry scan ids so stale completions/progress/actions can be ignored.',
  '',
  'Recovery procedure: inspect ScannerScanDiagnostics rows/counts and tracing events such as scanner-scan-request, scanner-overview-refresh-phase, scanner-mo2-index-build-started/completed, scanner-data-root-progress, scanner-race-subgraph-counts, scanner-race-module-read-failure, and scanner-scan-completed. Re-run the targeted cargo filters listed in Verification to localize regressions.',
  '',
  'Monitoring gaps: there is no persistent in-app diagnostics export yet, and clippy reports non-fatal warnings for large Err variants plus the scanner traversal helper argument count.'
].join('\n');

const uatContent = [
  '## UAT Type',
  '',
  'Agent-verifiable integration UAT for the read-only Scanner tab. No human game installation is required; behavior is proven by fake-backed Rust tests, Slint source-contract tests, runtime wiring tests, and cargo gates.',
  '',
  '## Preconditions',
  '',
  '1. The app is built from the current S07 code.',
  '2. Scanner settings are available through SettingsController.',
  '3. Filesystem, discovery, mod-manager, Overview, clipboard, and desktop seams can be substituted by tests.',
  '',
  '## Steps and Expected Outcomes',
  '',
  '1. Open the Scanner tab.',
  '   - Expected: the tab shows Scan Settings with Overview Issues, Errors, Wrong File Formats, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs in that order.',
  '2. Disable every scanner setting and attempt to scan.',
  '   - Expected: Scan Game is disabled or no worker is scheduled; settings are not persisted outside the scan-start flow.',
  '3. Start a scan with enabled settings and a fake Data tree.',
  '   - Expected: old rows/details clear, the button uses Scanning..., progress emits Refreshing Overview..., Building mod file index... when MO2 is present, and Scanning... n/N: folder for top-level Data traversal.',
  '4. Include Overview problems, wrong-format files, loose previs folders, junk files, F4SE script overrides, invalid BA2 names, and race-subgraph-heavy modules.',
  '   - Expected: results are gated by the corresponding toggles, grouped in reference order, sorted deterministically, and include safe summaries/solutions/details.',
  '5. Scan with MO2 context and enabled modlist order plus overwrite.',
  '   - Expected: staged files are attributed to the winning mod; missing modlist or prerequisites show safe Errors rows and diagnostics instead of panics.',
  '6. Scan with Vortex context.',
  '   - Expected: Scanner scans Data only and does not fabricate mod attribution.',
  '7. Select a result and trigger Copy Details, Open Location, Open URL, Copy URL, and File List where available.',
  '   - Expected: actions are read-only, go through fakeable adapters, and success/failure feedback is safe and inline. Auto-Fix, Fixed, and Fix Failed controls are absent.',
  '8. Inject stale worker progress/completion/action events and worker spawn failures.',
  '   - Expected: stale events are ignored, spawn failures map to safe Scanner status, and the UI remains in a recoverable state.',
  '',
  '## Edge Cases Covered',
  '',
  '- Missing Data returns a safe visible row/status and no traversal.',
  '- Unreadable child directories produce Errors rows and continue siblings.',
  '- Unreadable module bytes are diagnostic-only for race counting and do not abort the scan.',
  '- Malformed modlist, all toggles off, unexpected extensions with and without proper replacements, invalid BA2 suffixes, already-enabled archives, zero results, save rollback, stale events, and clipboard/desktop failures are covered.',
  '',
  '## Not Proven By This UAT',
  '',
  '- Real end-user Fallout 4 installation performance on a large mod list.',
  '- Future Auto-Fix write actions, which are intentionally deferred to S08.',
  '- Human visual comparison against the original Tkinter app beyond source-contract/layout tests.'
].join('\n');

const params = {
  milestoneId: 'M001',
  sliceId: 'S07',
  sliceTitle: 'Scanner Read Only Results',
  oneLiner: 'Delivered the reference-shaped read-only Scanner tab, adapter-backed scan engine, controller/worker handoff, Slint surface, runtime wiring, and safe diagnostics/actions while keeping Auto-Fix deferred.',
  narrative,
  verification,
  uatContent,
  operationalReadiness,
  deviations: 'Added display-only forward-slash path normalization in src/domain/scanner.rs and src/domain/mod_manager.rs during closeout after Windows-side verification exposed platform separator drift. This preserves native PathBuf action targets and only changes deterministic UI/test strings.',
  knownLimitations: 'Auto-Fix write controls remain intentionally absent/deferred to S08. cargo clippy exits 0 but reports non-fatal warnings for large Err variants and the private scanner traversal helper argument count. Real-world performance on very large mod lists was not measured in this slice.',
  followUps: 'S08 should implement Auto-Fix write actions using the read-only Scanner result/action contract established here. Consider addressing the non-fatal clippy warnings in a cleanup slice if they become noisy.',
  provides: [
    'Reference-shaped read-only Scanner tab UI and state contract',
    'Adapter-backed ScannerScanService with MO2 attribution, Vortex Data-only handling, progress, diagnostics, and reference rule classification',
    'Scanner controller and worker payloads for scan progress/completion, stale-event rejection, and safe read-only actions'
  ],
  requires: [
    { slice: 'S02', provides: 'Main shell, settings persistence baseline, and tab wiring patterns' },
    { slice: 'S03', provides: 'Discovery and Mod Organizer/Vortex context contracts' },
    { slice: 'S04', provides: 'Overview problem feed and worker/event handoff pattern' },
    { slice: 'S05', provides: 'Safe desktop/clipboard action adapter patterns' },
    { slice: 'S06', provides: 'F4SE worker/status/runtime wiring patterns' }
  ],
  affects: ['S08 Auto-Fix write actions', 'S09 Downgrade Manager', 'S10 Archive Patcher'],
  keyFiles: [
    'src/domain/scanner.rs',
    'src/services/scanner.rs',
    'src/services/mod.rs',
    'src/app/scanner_controller.rs',
    'src/app/mod.rs',
    'src/app/settings_controller.rs',
    'src/workers/events.rs',
    'src/workers/mod.rs',
    'ui/scanner_tab.slint',
    'ui/main.slint',
    'src/main.rs',
    'src/domain/mod_manager.rs',
    'Cargo.toml'
  ],
  keyDecisions: [
    'Kept Scanner S07 read-only and omitted Auto-Fix, Fixed, and Fix Failed controls until S08.',
    'Separated pure Scanner domain data, adapter-backed scan service, Slint-free controller, owned worker payloads, and Slint projection/runtime wiring.',
    'Used Filesystem::read_dir recursion for scanner traversal instead of walk_dir so pruning, progress, and unreadable-child continuation are explicit.',
    'Built MO2 attribution from enabled modlist order plus overwrite; Vortex remains Data-only without fabricated mod names.',
    'Normalized display-only scanner/MO2 paths to forward slashes for deterministic cross-platform UI/test strings while retaining native PathBuf action targets.'
  ],
  patternsEstablished: [
    'Read-only scanner rows/actions are pure domain descriptors first, then adapted by controller/UI/runtime layers.',
    'Background scanner work returns owned scan snapshots and action feedback tagged with scan ids for stale-event rejection.',
    'Recoverable filesystem/manager failures surface as safe diagnostics and optional Errors-category rows rather than panics.',
    'Scanner settings are transient in the controller and persisted only at Scan Game start through SettingsController.'
  ],
  observabilitySurfaces: [
    'Scanner UI status, progress, result count, details, file-list visibility, and action-feedback surfaces',
    'ScannerScanDiagnostics counts for indexed mods/files, traversed folders/files, skipped folders, partial read failures, race-subgraph totals, rows by problem type, and safe diagnostic rows',
    'Structured tracing for scan request, overview handoff, MO2 index build, Vortex Data-only skip, Data root progress, race-subgraph counts/read failures, completion counts, stale events, spawn failures, and read-only action failures',
    'Worker events carry scan ids and safe messages for localization without exposing raw diagnostics as primary UI text'
  ],
  drillDownPaths: [
    '.gsd/milestones/M001/slices/S07/tasks/T01-SUMMARY.md',
    '.gsd/milestones/M001/slices/S07/tasks/T02-SUMMARY.md',
    '.gsd/milestones/M001/slices/S07/tasks/T03-SUMMARY.md',
    '.gsd/milestones/M001/slices/S07/tasks/T04-SUMMARY.md',
    '.gsd/milestones/M001/slices/S07/tasks/T05-SUMMARY.md'
  ],
  filesModified: [
    { path: 'src/domain/scanner.rs', description: 'Scanner domain contract, result/detail/action models, copy-details rendering, scan snapshots, and display-only slash normalization for detail paths.' },
    { path: 'src/services/scanner.rs', description: 'Adapter-backed read-only Scanner scan service, reference rule classification, MO2 index, Vortex Data-only behavior, progress events, and diagnostics.' },
    { path: 'src/services/mod.rs', description: 'Exports the scanner service module.' },
    { path: 'src/app/scanner_controller.rs', description: 'Slint-free Scanner controller for settings, scan lifecycle, grouped results, details, stale events, and action feedback.' },
    { path: 'src/app/mod.rs', description: 'Exports the Scanner controller.' },
    { path: 'src/app/settings_controller.rs', description: 'Adds save-on-scan-start scanner settings persistence.' },
    { path: 'src/workers/events.rs', description: 'Adds owned scanner worker payloads and action feedback events.' },
    { path: 'src/workers/mod.rs', description: 'Wires scanner worker task identifiers and payload handling.' },
    { path: 'ui/scanner_tab.slint', description: 'Reference-shaped read-only Scanner tab UI.' },
    { path: 'ui/main.slint', description: 'MainWindow Scanner properties, models, and callback forwarding.' },
    { path: 'src/main.rs', description: 'Runtime Scanner projection, scan scheduling, progress/completion handling, and read-only copy/open actions.' },
    { path: 'src/domain/mod_manager.rs', description: 'Display-only slash normalization for MO2 parse user-message paths.' },
    { path: 'Cargo.toml', description: 'rust-version corrected to the edition-2024 floor during S07 final gates.' }
  ],
  actorName: 'gsd-auto-fix',
  triggerReason: 'S07 had all tasks complete but missing slice summary/UAT artifact after T02 summary recovery and Windows-side verification.'
};

const result = await executeSliceComplete(params, process.cwd());
console.log(JSON.stringify(result, null, 2));
if (result?.isError) {
  process.exit(1);
}
