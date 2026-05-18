---
id: S06
milestone: M001
status: draft
---

# S06: F4SE Diagnostics — Context Draft

## Goal

Deliver a non-blocking, reference-shaped F4SE diagnostics tab that scans `Data/F4SE/Plugins` DLLs and displays compatibility in the `DLL`, `OG`, `NG`, `AE`, and `Your Game` table.

## Confirmed Human Decisions So Far

- Scan UX: use lazy auto-scan on first F4SE tab open, with the reference loading text `Scanning DLLs...`; do not add a manual refresh button in S06.
- Bad DLLs: keep unreadable or unclassifiable DLL rows visible, continue scanning other DLLs, and surface safe inline unknown/warning feedback instead of crashing or hiding the file.
- Detail scope: keep this slice to the reference-shaped table plus `F4SE DLLs` legend; do not add selected-row details, copy, or open actions in S06.
- Compatibility strictness: do not guess compatibility from names or loose heuristics. Show support only when the chosen compatibility source proves it; otherwise show unknown/warning.
- Unknown game state: if the current game version is not classifiable as OG/NG/AE, keep DLL facts visible and render `Your Game` as unknown/warning rather than a false hard failure.
- Empty plugins folder: when `Data/F4SE/Plugins` exists but contains no DLLs, render the normal empty table and legend rather than an error.

## Open Items To Resolve

- The user described a known-compatible list/certification model, but inspected reference source currently shows export/version-data inspection in `CMT/src/utils.py::parse_dll`; clarify whether S06 should use current source behavior only or introduce a Rust-owned compatibility list if one is supplied.
- Final context still needs explicit integration points and scope boundaries after wrap-up confirmation.
