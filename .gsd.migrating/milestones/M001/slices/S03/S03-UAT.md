# S03: S03 — UAT

**Milestone:** M001
**Written:** 2026-05-17T10:38:06.636Z

# UAT: S03 Platform Discovery Background Adapters

## UAT Type
Developer/API smoke test backed by fake-backed Rust unit tests. This slice intentionally exposes backend contracts and services, not live Slint controls.

## Preconditions
1. Build the Rust crate successfully.
2. Use a disposable test fixture or fake adapters for filesystem, registry, process, version, and system metadata inputs.
3. Do not require a real Fallout 4 installation, live Windows registry, running MO2/Vortex process, or desktop UI prompt.

## Steps and Expected Outcomes
1. Run the Rust verification gates.
   - Expected: `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` all pass.
2. Construct a `DiscoveryService` with fake filesystem, registry, and process/system metadata adapters.
   - Expected: construction performs no real OS queries and requires no Slint window.
3. Provide a running MO2 process ancestor with a valid `gamePath` and a separate valid current working directory.
   - Expected: discovery accepts the manager `gamePath` first and records attempts in the locked reference order.
4. Provide no manager game path but a valid current working directory.
   - Expected: discovery checks current working directory before Bethesda and GOG registry paths.
5. Provide a direct `Fallout4.exe` candidate.
   - Expected: discovery normalizes it to the parent game directory.
6. Provide a game directory with `Fallout4.exe` but missing `Data` or missing `Data/F4SE/Plugins`.
   - Expected: discovery returns a valid installation with missing derived paths represented as `None`, not as a fatal install failure.
7. Provide MO2 portable and instance inputs.
   - Expected: adjacent portable files are checked before HKCU `CurrentInstance`; parsed context includes game path, selected profile, directories, profile-local flags, skip rules, and supported custom tools.
8. Provide incomplete, missing, or non-Fallout MO2 configuration.
   - Expected: discovery returns manager-specific typed errors and does not panic or silently fall through to unrelated game-path sources.
9. Provide a Vortex process ancestor with and without version metadata.
   - Expected: discovery returns Vortex display name, executable path, parsed version when available, `0.0.0` fallback when absent, and no staging/config parsing.
10. Provide fake system metadata.
    - Expected: `DiscoveryReport.system_metadata` carries the fake PC Specs data for later Overview display.

## Edge Cases
- Registry read errors should be recoverable diagnostics when a later source can still succeed.
- Invalid Bethesda registry paths should return a reference-compatible invalid-registry error without silently falling through to GOG.
- Optional custom executable checks in MO2 should treat adapter file-check failures as absent and log diagnostics rather than failing the whole parse.

## Not Proven By This UAT
- Live Windows registry/process/desktop behavior on an end-user machine.
- Actual Slint UI display of discovery results.
- Scanner, archive, downgrader, or tool workflows that will consume these contracts in later slices.

