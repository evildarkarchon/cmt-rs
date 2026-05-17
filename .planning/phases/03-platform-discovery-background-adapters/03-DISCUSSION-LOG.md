# Phase 3: platform-discovery-background-adapters - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 03-platform-discovery-background-adapters
**Areas discussed:** Discovery fallback, MO2 config depth, Worker event shape, Failure reporting

---

## Discovery Fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror reference | Use manager gamePath first, then current directory, then Bethesda/GOG registry paths. Recommended for parity. | ✓ |
| Registry first | Use registry before current directory or manager context. More conventional, but diverges from the reference. | |
| You decide | Let researcher/planner choose the smallest reference-compatible order. | |

**User's choice:** Mirror reference
**Notes:** Discovery order is locked to the Python reference.

| Option | Description | Selected |
|--------|-------------|----------|
| Return recoverable | Return typed NotFound/InvalidRegistry results with reference messages; later UI decides whether to show a picker. Recommended because Phase 3 has no user-visible workflow. | ✓ |
| Include picker seam | Model a manual Fallout4.exe picker adapter now, but do not wire visible UI yet. | |
| Reference exact | Represent the full askyesno/filedialog/sys.exit flow from Python, adapted to Rust errors. | |
| You decide | Let planner choose the smallest testable contract. | |

**User's choice:** Return recoverable
**Notes:** Manual UI prompting is deferred to later visible UI phases.

| Option | Description | Selected |
|--------|-------------|----------|
| Normalize exe path | Accept either a directory or Fallout4.exe path and store the parent directory. Recommended; matches reference and helps future UI. | ✓ |
| Directory only | Only accept game directories; reject file paths as invalid. | |
| You decide | Let planner decide based on tests and adapter shape. | |

**User's choice:** Normalize exe path
**Notes:** This preserves the reference manual-selection normalization behavior.

| Option | Description | Selected |
|--------|-------------|----------|
| Allow partial state | Accept valid game directory and represent missing Data/F4SE as `None`/missing fields. Recommended; mirrors reference property behavior. | ✓ |
| Require Data | Treat missing Data as invalid discovery, even if Fallout4.exe exists. | |
| Require all paths | Require Fallout4.exe, Data, and Data/F4SE/Plugins before discovery succeeds. | |
| You decide | Let planner choose based on reference tests. | |

**User's choice:** Allow partial state
**Notes:** Valid Fallout 4 directory discovery should not fail just because derived folders are absent.

---

## MO2 Config Depth

| Option | Description | Selected |
|--------|-------------|----------|
| Game path only | Parse only enough to discover Fallout 4. Smallest Phase 3 scope, but later scanner phases must revisit MO2 parsing. | |
| Core paths now | Parse gamePath, selected_profile, mod_directory, overwrite_directory, profiles_directory, profile-local flags, and skip rules. Recommended; matches reference fields later phases need. | ✓ |
| Full reference now | Also parse custom executable tool paths and all currently modeled MO2 settings. | |
| You decide | Let researcher/planner decide how much is needed for acceptance tests. | |

**User's choice:** Core paths now
**Notes:** Phase 3 should prepare the MO2 path context later scanner/overview phases need.

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror reference | Check portable.txt/ModOrganizer.ini beside the executable first, then HKCU CurrentInstance under LOCALAPPDATA. Recommended for parity. | ✓ |
| Portable only | Support only portable MO2 in Phase 3; defer registry CurrentInstance handling. | |
| Instance only | Support only HKCU CurrentInstance; defer portable.txt behavior. | |
| You decide | Let planner pick the smallest path coverage. | |

**User's choice:** Mirror reference
**Notes:** Portable and instance-based MO2 discovery both belong in Phase 3.

| Option | Description | Selected |
|--------|-------------|----------|
| Typed error | Return a manager-specific typed error with the reference message text. Recommended; avoids panics and lets later UI show guidance. | ✓ |
| Ignore MO2 | Treat the manager as detected but continue current-directory/registry game discovery if INI is bad. | |
| Hard fail | Fail discovery immediately like the reference raises/exists in some paths. | |
| You decide | Let planner choose based on testability. | |

**User's choice:** Typed error
**Notes:** Known MO2 configuration problems should be structured and visible to later UI.

| Option | Description | Selected |
|--------|-------------|----------|
| Detect only | For Vortex, capture running manager name/path/version only; defer Vortex staging/config parsing. Recommended because the reference has a pass placeholder. | ✓ |
| Parse staging now | Research and parse Vortex staging/game paths now, beyond the current reference implementation. | |
| You decide | Let researcher decide whether Vortex parsing is necessary. | |

**User's choice:** Detect only
**Notes:** Vortex stays aligned with the current reference placeholder.

---

## Worker Event Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Envelope plus payload | Use a generic task envelope with task kind/id/status plus typed payload variants. Recommended; reusable and still domain-safe. | ✓ |
| Generic only | Use only strings/counts/errors with no domain payloads. Simpler, but later phases may need refactoring. | |
| Domain events only | Define separate event enums per workflow without a shared envelope. | |
| You decide | Let planner choose the event architecture. | |

**User's choice:** Envelope plus payload
**Notes:** Shared event shape must still support domain-specific data.

| Option | Description | Selected |
|--------|-------------|----------|
| All named kinds | Discovery, scan, patch, download, external process, and generic/unknown. Recommended; matches SPEC acceptance and later phases. | ✓ |
| Discovery only | Only define discovery now and add task kinds as later phases arrive. | |
| No enum kinds | Use string task names rather than a typed task-kind enum. | |
| You decide | Let planner infer kinds from SPEC. | |

**User's choice:** All named kinds
**Notes:** The task-kind contract should be broad enough for downstream phases.

| Option | Description | Selected |
|--------|-------------|----------|
| Text plus counts | Allow optional progress text plus optional current/total counts. Recommended; covers scanners/downloads without forcing percentages. | ✓ |
| Text only | Only human-readable progress messages for now. | |
| Full metrics | Include percent, bytes, rates, ETA, and nested step data now. | |
| You decide | Let planner pick based on minimal tests. | |

**User's choice:** Text plus counts
**Notes:** Percentages/rates/ETA are not required in Phase 3.

| Option | Description | Selected |
|--------|-------------|----------|
| Requested and completed | Represent cancellation request/acknowledgement and final cancelled completion separately. Recommended; avoids confusing pending cancel with stopped work. | ✓ |
| Final only | Only emit a final Cancelled result when work stops. | |
| Error variant | Treat cancellation as an error result. | |
| You decide | Let planner decide from worker patterns. | |

**User's choice:** Requested and completed
**Notes:** Pending cancellation and completed cancellation should be distinguishable.

---

## Failure Reporting

| Option | Description | Selected |
|--------|-------------|----------|
| Typed plus message | Return typed error kinds with reference-compatible user messages. Recommended; supports UI branching and parity. | ✓ |
| Messages only | Return only exact user-facing strings; simple but brittle for later UI behavior. | |
| Typed only | Return categories only and let later UI compose all messages. | |
| You decide | Let planner choose the error contract. | |

**User's choice:** Typed plus message
**Notes:** Known reference messages remain part of the contract.

| Option | Description | Selected |
|--------|-------------|----------|
| Known messages only | Use exact/reference-style messages for known cases; keep raw IO/process details for diagnostics/logging. Recommended. | ✓ |
| Always include raw | Include raw OS error/path details in every user-facing adapter error. | |
| Never include raw | Hide all paths/errors from user-facing messages, even for invalid registry guidance. | |
| You decide | Let planner choose per adapter. | |

**User's choice:** Known messages only
**Notes:** Raw details belong in logs unless the reference message intentionally includes them.

| Option | Description | Selected |
|--------|-------------|----------|
| Typed unsupported | Return explicit UnsupportedPlatform errors for real Windows-only operations, while fake-backed tests still run cross-platform. Recommended. | ✓ |
| Compile out APIs | Hide Windows-only discovery/process APIs entirely on non-Windows targets. | |
| Panic unreachable | Assume the app only runs on Windows and panic if called elsewhere. | |
| You decide | Let planner decide from cfg design. | |

**User's choice:** Typed unsupported
**Notes:** Public models and fake-backed tests should not require Windows.

| Option | Description | Selected |
|--------|-------------|----------|
| Action result events | Return typed action results/events with success/failure, operation kind, target, and safe message. Recommended; fits worker event contract. | ✓ |
| Log only | Only log launch/open failures in Phase 3. | |
| Immediate dialogs | Have adapters trigger UI dialogs directly on failure. | |
| You decide | Let planner integrate failures with the chosen worker shape. | |

**User's choice:** Action result events
**Notes:** Adapters should not own direct UI dialogs.

---

## the agent's Discretion

No selected discussion area was delegated to the agent.

## Deferred Ideas

None.
