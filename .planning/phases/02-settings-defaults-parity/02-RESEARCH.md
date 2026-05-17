# Phase 02: Settings & Defaults Parity - Research

**Researched:** 2026-05-17 [VERIFIED: system date]
**Domain:** Rust/Slint settings persistence, reference-default parity, and Settings-tab UI controls [VERIFIED: .planning/ROADMAP.md]
**Confidence:** HIGH [VERIFIED: local project files + Context7 docs + crates.io search]

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**6 requirements are locked.** See `02-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `02-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Typed Rust settings model for all Phase 2 `SET-*` keys.
- Reference-compatible `settings.json` load, validation, defaulting, and save behavior.
- `download-source.txt` default detection for `update_source` with fallback to `nexus`.
- Settings-tab Update Channel and Log Level visible controls with reference labels and persisted values.
- Tests or source-level checks that prove defaults, validation, persistence keys, and Settings-tab labels match the reference.
- Confirmation that `CMT/` remains read-only during the implementation.

**Out of scope (from SPEC.md):**
- Scanner-tab checkbox UI for scanner settings - scanner UI behavior belongs to the scanner phase; Phase 2 only persists the values and defaults.
- Running scanner diagnostics - this phase defines settings consumed by later scanner behavior only.
- Platform/game/mod-manager discovery - Phase 3 owns discovery and background adapter seams.
- Performing update checks or downloads - Phase 2 stores `update_source`; later phases act on it.
- Downgrader/archive patching behavior - Phase 2 stores backup and delta cleanup preferences only.
- Migrating to a new TOML settings format - this phase explicitly uses reference-compatible `settings.json`.
- Adding new settings not present in the reference app - this phase preserves reference parity rather than expanding product behavior.

### the agent's Discretion
- Downstream agents may choose exact Rust type/module names and Slint component structure as long as the decisions above, SPEC.md, and project module-boundary rules remain satisfied.
- Downstream agents may choose the logging facade or error type style that best fits the current crate, provided invalid/repair events are observable in logs or testable diagnostics.

### Deferred Ideas (OUT OF SCOPE)
None - discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SET-01 | User settings load with reference-compatible defaults when no settings file exists. [VERIFIED: .planning/REQUIREMENTS.md] | Use `AppSettings::load_from_paths` defaults-first behavior and save defaults on missing file. [VERIFIED: CMT/src/app_settings.py] |
| SET-02 | User settings persist `log_level`, `update_source`, scanner toggles, `downgrader_keep_backups`, and `downgrader_delete_deltas`. [VERIFIED: .planning/REQUIREMENTS.md] | Preserve the reference JSON keys exactly and serialize JSON through `serde_json`. [VERIFIED: CMT/src/app_settings.py] [CITED: docs.rs/serde_json/1.0.149] |
| SET-03 | User can choose update channel options matching the reference labels. [VERIFIED: .planning/REQUIREMENTS.md] | Render labels in this order: `All: GitHub & Nexus Mods`, `Early: GitHub`, `Stable: Nexus Mods`, `Never: Don't Check`. [VERIFIED: CMT/src/tabs/_settings.py] |
| SET-04 | User can choose log level options matching the reference labels. [VERIFIED: .planning/REQUIREMENTS.md] | Render labels in this order: `Debug`, `Info`, `Error`. [VERIFIED: CMT/src/tabs/_settings.py] |
| SET-05 | Scanner-related settings default to enabled for Overview Issues, Errors, Wrong Format, Loose Previs, Junk Files, Problem Overrides, and Race Subgraphs. [VERIFIED: .planning/REQUIREMENTS.md] | Default all seven persisted `scanner_*` keys to `true`. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: CMT/src/scan_settings.py] |
| SET-06 | Invalid or incomplete settings files fail safely by preserving valid values and falling back to documented defaults for invalid values. [VERIFIED: .planning/REQUIREMENTS.md] | Implement per-key validation for syntactically valid JSON and defaults-only reset for malformed JSON. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |
</phase_requirements>

## Summary

Phase 02 should implement a reference-compatible JSON settings subsystem, not a new configuration design. [VERIFIED: 02-CONTEXT.md] The reference stores settings in current-directory `settings.json`, loads a full default map first, repairs missing/unknown/invalid values, saves defaults on first run, and writes repaired settings without surfacing success or repair messages in the UI. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md]

The Settings tab scope is intentionally narrow: expose only the visible Update Channel and Log Level radio groups in this phase, while still persisting scanner and downgrader preference keys for later consumers. [VERIFIED: 02-CONTEXT.md] The scanner checkbox UI belongs to a later scanner phase, but the underlying scanner defaults must exist now because `ScanSettings` reads and writes `scanner_{ScanSetting.name}` keys. [VERIFIED: CMT/src/scan_settings.py] [VERIFIED: 02-CONTEXT.md]

**Primary recommendation:** Build a typed Rust `AppSettings` model plus an injectable `SettingsStore`/asset resolver, add `serde_json = "1.0.149"`, replace the Settings placeholder with Slint radio groups, and verify behavior with filesystem fixture tests plus source-level Slint label tests. [VERIFIED: Cargo.toml] [VERIFIED: crates.io cargo search] [CITED: docs.rs/serde_json/1.0.149] [CITED: docs.slint.dev/latest/docs/rust/slint/index.html]

## Project Constraints (from AGENTS.md)

- `CMT/` is a read-only reference submodule and must not be edited, formatted, moved, deleted, or generated into. [VERIFIED: AGENTS.md]
- Implement new code in the Rust project outside `CMT/`. [VERIFIED: AGENTS.md]
- Inspect relevant original files in `CMT/src/` before porting behavior and preserve labels, tab structure, ordering, defaults, validation rules, and user-facing messages unless there is a clear documented reason to diverge. [VERIFIED: AGENTS.md]
- Prefer Slint `.slint` files for UI structure and styling, with Rust handling application state, filesystem work, parsing, and command execution. [VERIFIED: AGENTS.md]
- Keep UI and domain logic separated enough that non-UI behavior can be tested without launching a window. [VERIFIED: AGENTS.md]
- Avoid blocking the Slint UI thread; slow work must run off-thread and marshal results back through Slint-safe callbacks or event-loop APIs. [VERIFIED: AGENTS.md]
- Use typed Rust models for app state instead of unstructured strings or maps. [VERIFIED: AGENTS.md]
- Add Rust doc comments to public functions, public types, and methods that are added or substantially rewritten. [VERIFIED: AGENTS.md]
- Avoid `unwrap()` and `expect()` in production paths unless the invariant is obvious or documented. [VERIFIED: AGENTS.md]
- Run `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features` before considering the implementation slice complete. [VERIFIED: AGENTS.md]
- Do not commit unless explicitly asked by the user, despite GSD `commit_docs` being true. [VERIFIED: AGENTS.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Settings defaults and validation | Rust domain | Platform/filesystem adapter | The settings rules are non-UI behavior that must be unit-tested without launching Slint. [VERIFIED: AGENTS.md] [VERIFIED: 02-CONTEXT.md] |
| `settings.json` read/write | Platform/filesystem adapter | Rust domain | The file path is current-directory by default but tests need injectable paths. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |
| `download-source.txt` default detection | Platform/asset resolver | Rust domain | The reference reads an asset and falls back to `nexus`; the context requires an asset resolver abstraction. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |
| Settings-tab radio controls | Slint UI | Rust app/controller | Slint owns visible labels and selection widgets; Rust owns persistence callbacks. [VERIFIED: ui/settings_tab.slint] [CITED: docs.slint.dev/latest/docs/rust/slint/index.html] |
| Scanner and downgrader preference consumption | Later domain phases | Settings model | Phase 2 stores these values only; scanner diagnostics and downgrader behavior are out of scope. [VERIFIED: 02-CONTEXT.md] |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust stable, edition 2024 | rustc 1.95.0 available; edition 2024 in crate | Typed settings model, validation, tests | The crate already uses edition 2024 and the local toolchain supports it. [VERIFIED: Cargo.toml] [VERIFIED: rustc --version] |
| Slint | 1.16.1 | Settings-tab controls and top-level properties/callbacks | The project uses Slint and generated Rust exposes top-level property getters/setters plus `on_<callback>` handlers. [VERIFIED: Cargo.toml] [CITED: docs.slint.dev/latest/docs/rust/slint/index.html] |
| serde | 1.0.228 | Serialize/deserialize typed settings values | The crate already depends on `serde` with `derive`, and Serde supports field defaults and variant renames. [VERIFIED: Cargo.toml] [CITED: serde.rs/attributes.html] |
| serde_json | 1.0.149 | Reference-compatible `settings.json` parsing and writing | The phase explicitly requires JSON parity; `serde_json` supports untyped `Value` inspection and pretty JSON output. [VERIFIED: crates.io cargo search] [CITED: docs.rs/serde_json/1.0.149] |
| tracing | 0.1.44 | Log repair, reset, and save-failure diagnostics | The context requires repair events to be observable in logs or diagnostics and the crate already depends on `tracing`. [VERIFIED: Cargo.toml] [VERIFIED: 02-CONTEXT.md] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| thiserror | 2.0.18 | Domain/settings error enums | Use for load/save/validation error reporting that tests can assert. [VERIFIED: Cargo.toml] |
| anyhow | 1.0.102 | Startup/app-controller error context | Use at app boundaries, not inside the core settings validation API. [VERIFIED: Cargo.toml] |
| assert_fs | 1.1.3 | Temporary settings-file fixtures | Add as a dev-dependency if tests need ergonomic temp directories and JSON file assertions. [VERIFIED: crates.io cargo search] |
| tempfile | 3.27.0 | Temporary settings-file fixtures | Add as a dev-dependency if a smaller temp-file dependency is preferred over `assert_fs`. [VERIFIED: prior STACK.md crates.io check] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `settings.json` + `serde_json` | `toml` primary config | TOML was recommended for future human-editable config, but Phase 02 explicitly requires reference-compatible `settings.json`. [VERIFIED: STACK.md] [VERIFIED: 02-CONTEXT.md] |
| Per-key `serde_json::Value` validation | Direct `serde_json::from_str::<AppSettings>` | Direct struct deserialization is concise but can fail the whole file or silently default fields in ways that do not match the reference per-key repair semantics. [VERIFIED: CMT/src/app_settings.py] [CITED: docs.rs/serde_json/1.0.149] |
| Slint source-level label tests | Full GUI automation | Context locks source-level assertions for Phase 2 and explicitly excludes full GUI automation. [VERIFIED: 02-CONTEXT.md] |

**Installation:**
```bash
cargo add serde_json
cargo add --dev assert_fs
```

**Version verification:** `serde_json = "1.0.149"` and `assert_fs = "1.1.3"` were verified with `cargo search` on 2026-05-17. [VERIFIED: crates.io cargo search]

## Architecture Patterns

### System Architecture Diagram

```text
Application startup
  |
  v
SettingsStore::load(default path: settings.json, injectable in tests)
  |
  +--> Missing file? -------- yes --> build defaults from download-source.txt --> save defaults --> AppSettings
  |                              [VERIFIED: CMT/src/app_settings.py]
  |
  +--> Malformed/non-object JSON? yes --> log parse/reset --> save defaults --> AppSettings
  |                              [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md]
  |
  +--> Valid JSON object --> validate known keys one-by-one
           |                    [VERIFIED: CMT/src/app_settings.py]
           +--> valid value: preserve
           +--> missing/invalid/unknown: default or remove, mark resave
           v
       repaired AppSettings --> optional resave --> bind selected values to SettingsTab
                                                [CITED: docs.slint.dev/latest/docs/rust/slint/index.html]

SettingsTab radio selection
  |
  v
Slint callback --> Rust controller updates one setting --> save immediately
  |                                             [VERIFIED: CMT/src/tabs/_settings.py]
  +--> save ok: quiet UI
  +--> save fail: revert to last persisted value + log failure
        [VERIFIED: 02-CONTEXT.md]
```

### Recommended Project Structure

```text
src/
├── domain/
│   ├── mod.rs                 # existing domain boundary [VERIFIED: src/domain/mod.rs]
│   └── settings.rs            # AppSettings, enums, defaults, validation [RECOMMENDED]
├── platform/
│   ├── mod.rs                 # existing platform boundary [VERIFIED: src/platform/mod.rs]
│   └── settings_store.rs      # injectable file IO + asset resolver [RECOMMENDED]
├── app/
│   ├── mod.rs                 # existing app boundary [VERIFIED: src/app/mod.rs]
│   └── settings_controller.rs # Slint callback binding and save/revert policy [RECOMMENDED]
└── main.rs                    # load settings before or during MainWindow wiring [VERIFIED: src/main.rs]
ui/
└── settings_tab.slint         # replace placeholder with reference radio groups [VERIFIED: ui/settings_tab.slint]
```

### Pattern 1: Defaults-First Settings Load
**What:** Construct defaults first, then overlay only valid JSON values. [VERIFIED: CMT/src/app_settings.py]
**When to use:** Always use for Phase 02 load because missing keys and invalid values must fall back independently. [VERIFIED: 02-CONTEXT.md]
**Example:**
```rust
/// Loads settings using reference-compatible defaults and per-key repair.
/// Malformed JSON resets to defaults, while valid JSON objects preserve valid keys.
pub fn load_settings(path: &Path, assets: &dyn AssetResolver) -> SettingsLoadResult {
    let mut settings = AppSettings::defaults_with_download_source(assets);
    // Parse as serde_json::Value first so each key can be validated independently.
    // Source: CMT/src/app_settings.py and docs.rs/serde_json/1.0.149.
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    settings.apply_json_object(value.as_object());
    settings
}
```

### Pattern 2: String Enums with Reference Wire Values
**What:** Use enums or const-backed conversion methods for persisted strings: `update_source` values `both`, `github`, `nexus`, `none`; `log_level` values `DEBUG`, `INFO`, `ERROR` for UI controls. [VERIFIED: CMT/src/tabs/_settings.py]
**When to use:** Use for all UI-facing choices and JSON validation; do not persist display labels. [VERIFIED: CMT/src/tabs/_settings.py]
**Example:**
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateSource {
    Both,
    Github,
    Nexus,
    None,
}

impl UpdateSource {
    /// Returns the reference JSON value used by CMT/src/tabs/_settings.py.
    pub const fn as_json_value(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Github => "github",
            Self::Nexus => "nexus",
            Self::None => "none",
        }
    }
}
```

### Pattern 3: Slint Properties and Save Callbacks
**What:** Expose top-level Slint properties for selected `update_source` and `log_level`, plus callbacks for radio selection. [CITED: docs.slint.dev/latest/docs/rust/slint/index.html]
**When to use:** Use at the UI boundary so Rust can persist immediately and revert on save failure. [VERIFIED: 02-CONTEXT.md]
**Example:**
```slint
// Source: docs.slint.dev standard widget docs and CMT/src/tabs/_settings.py labels.
import { GroupBox, RadioButton } from "std-widgets.slint";

export component SettingsTab inherits Rectangle {
    in-out property <string> update-source;
    callback update-source-selected(string);

    GroupBox {
        title: "Update Channel";
        VerticalLayout {
            RadioButton { text: "All: GitHub & Nexus Mods"; checked: root.update-source == "both"; }
            RadioButton { text: "Early: GitHub"; checked: root.update-source == "github"; }
            RadioButton { text: "Stable: Nexus Mods"; checked: root.update-source == "nexus"; }
            RadioButton { text: "Never: Don't Check"; checked: root.update-source == "none"; }
        }
    }
}
```

### Anti-Patterns to Avoid
- **Deserializing directly into a final struct and accepting the result wholesale:** This loses the reference's per-key repair and unknown-key cleanup behavior. [VERIFIED: CMT/src/app_settings.py]
- **Persisting Slint labels instead of reference JSON values:** The reference persists enum/string values such as `nexus`, not labels such as `Stable: Nexus Mods`. [VERIFIED: CMT/src/tabs/_settings.py]
- **Moving settings to OS config directories in this phase:** Current-directory `settings.json` is locked for reference parity. [VERIFIED: 02-CONTEXT.md]
- **Adding scanner checkbox UI now:** Scanner-tab checkbox UI is explicitly out of scope for Phase 02. [VERIFIED: 02-CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing/writing | Manual string parsing or manual JSON formatting | `serde_json` | `serde_json` parses into `Value` and serializes pretty JSON safely. [CITED: docs.rs/serde_json/1.0.149] |
| Slint/Rust event connection | Ad hoc global mutable state | Generated Slint property getters/setters and `on_<callback>` handlers | Slint exposes generated Rust APIs for top-level properties and callbacks. [CITED: docs.slint.dev/latest/docs/rust/slint/index.html] |
| Temporary settings tests | Writing real repository `settings.json` | Injectable paths plus `assert_fs` or `tempfile` | Context requires tests to avoid touching real top-level settings. [VERIFIED: 02-CONTEXT.md] |
| Validation reporting | UI dialogs/toasts for successful repair | Logs/testable diagnostics only | Reference and context require quiet UI repair behavior. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |

**Key insight:** The complex part is not storing values; it is reproducing reference-compatible partial repair, resave, and quiet UI behavior while keeping tests isolated from real user files. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Direct Struct Deserialization Changes Repair Semantics
**What goes wrong:** A single invalid enum can reject the whole file or a missing field can be defaulted without triggering the required repair save. [VERIFIED: CMT/src/app_settings.py]
**Why it happens:** Serde defaults are convenient but the reference validates each key and tracks whether a resave is needed. [VERIFIED: CMT/src/app_settings.py] [CITED: serde.rs/attr-default.html]
**How to avoid:** Parse to `serde_json::Value`, require an object, and validate known keys one at a time. [VERIFIED: CMT/src/app_settings.py] [CITED: docs.rs/serde_json/1.0.149]
**Warning signs:** Tests assert only final in-memory defaults and not whether repaired JSON drops unknown keys. [VERIFIED: 02-CONTEXT.md]

### Pitfall 2: `download-source.txt` Default Is Not Always Hard-Coded `nexus`
**What goes wrong:** The Rust port always defaults `update_source` to `nexus` and ignores packaged asset intent. [VERIFIED: CMT/src/app_settings.py]
**Why it happens:** The fallback is `nexus`, but the primary default is the asset file if it contains `nexus` or `github`. [VERIFIED: CMT/src/app_settings.py]
**How to avoid:** Add an asset resolver abstraction and tests for asset values `github`, `nexus`, invalid text, and read failure. [VERIFIED: 02-CONTEXT.md] [VERIFIED: CMT/src/app_settings.py]
**Warning signs:** Default tests do not inject an asset resolver. [VERIFIED: 02-CONTEXT.md]

### Pitfall 3: UI Labels Drift From Reference
**What goes wrong:** Labels are modernized or reordered and no longer match the Tkinter Settings tab. [VERIFIED: CMT/src/tabs/_settings.py]
**Why it happens:** The current Slint file is only a placeholder. [VERIFIED: ui/settings_tab.slint]
**How to avoid:** Add source-level tests against `ui/settings_tab.slint` for group titles and radio labels in order. [VERIFIED: 02-CONTEXT.md]
**Warning signs:** The plan relies only on manual visual inspection. [VERIFIED: 02-CONTEXT.md]

### Pitfall 4: Save Failure Leaves UI Showing an Unsaved Value
**What goes wrong:** The user selects a radio option, save fails, and the UI keeps showing a value that was not persisted. [VERIFIED: 02-CONTEXT.md]
**Why it happens:** The reference writes immediately, but the Rust plan must add explicit failure handling for persistent state parity. [VERIFIED: CMT/src/tabs/_settings.py] [VERIFIED: 02-CONTEXT.md]
**How to avoid:** Keep `last_persisted_settings` in the controller and reset Slint properties after a save error. [VERIFIED: 02-CONTEXT.md] [CITED: docs.slint.dev/latest/docs/rust/slint/index.html]
**Warning signs:** Callback code mutates Slint state before confirming save and has no revert path. [VERIFIED: 02-CONTEXT.md]

## Code Examples

### Per-Key Validation Skeleton
```rust
/// Applies a JSON object using reference-compatible per-key repair semantics.
/// Unknown keys are ignored in memory and removed when repaired settings are saved.
pub fn apply_json_object(&mut self, object: &serde_json::Map<String, serde_json::Value>) -> RepairReport {
    let mut report = RepairReport::default();
    for (key, value) in object {
        match key.as_str() {
            "log_level" => self.apply_log_level(value, &mut report),
            "update_source" => self.apply_update_source(value, &mut report),
            "scanner_OverviewIssues" => self.apply_bool(key, value, &mut report),
            "scanner_Errors" => self.apply_bool(key, value, &mut report),
            "scanner_WrongFormat" => self.apply_bool(key, value, &mut report),
            "scanner_LoosePrevis" => self.apply_bool(key, value, &mut report),
            "scanner_JunkFiles" => self.apply_bool(key, value, &mut report),
            "scanner_ProblemOverrides" => self.apply_bool(key, value, &mut report),
            "scanner_RaceSubgraphs" => self.apply_bool(key, value, &mut report),
            "downgrader_keep_backups" => self.apply_bool(key, value, &mut report),
            "downgrader_delete_deltas" => self.apply_bool(key, value, &mut report),
            _ => report.removed_unknown_keys.push(key.clone()),
        }
    }
    report
}
```
Source: reference key list and validation behavior. [VERIFIED: CMT/src/app_settings.py]

### Rust-to-Slint Callback Binding Pattern
```rust
let weak = app.as_weak();
app.on_update_source_selected(move |selected| {
    if let Some(app) = weak.upgrade() {
        // Persist immediately to match the Tkinter Radiobutton command behavior.
        // On error, reset the Slint property to the last persisted value.
        controller.save_update_source_or_revert(&app, selected.to_string());
    }
});
```
Source: Slint generated callbacks and weak-handle guidance. [CITED: docs.slint.dev/latest/docs/rust/slint/index.html]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Python `TypedDict` + runtime `typing.Literal` checks | Rust enums/typed struct plus `serde_json::Value` validation | During Rust port Phase 02 [VERIFIED: ROADMAP.md] | Preserve reference wire values while gaining compile-time types. [VERIFIED: CMT/src/app_settings.py] |
| Tkinter `ttk.Radiobutton` command | Slint radio controls with Rust callbacks | During Rust port Phase 02 [VERIFIED: ROADMAP.md] | Keep immediate-save behavior without Tkinter runtime. [VERIFIED: CMT/src/tabs/_settings.py] [CITED: docs.slint.dev/latest/docs/rust/slint/index.html] |
| Placeholder Settings tab | Reference-labeled Settings tab controls | During Rust port Phase 02 [VERIFIED: ui/settings_tab.slint] | Replaces inert placeholder with actual Update Channel and Log Level choices. [VERIFIED: 02-CONTEXT.md] |

**Deprecated/outdated:**
- Do not follow the earlier stack preference for TOML primary settings in this phase; Phase 02 explicitly uses reference-compatible `settings.json`. [VERIFIED: STACK.md] [VERIFIED: 02-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|

**If this table is empty:** All claims in this research were verified or cited — no user confirmation needed. [VERIFIED: research source tags]

## Open Questions (RESOLVED)

1. **Should `WARNING` remain a valid loaded `log_level` even though the Phase 02 UI exposes only Debug, Info, and Error?**
    - What we know: `AppSettingsDict` accepts `WARNING`, but the reference Settings tab only exposes `DEBUG`, `INFO`, and `ERROR`. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: CMT/src/tabs/_settings.py]
   - Resolution: Treat `WARNING` as unsupported for Phase 02 Rust settings and repair it to the documented default `INFO`. The phase SPEC explicitly defines visible persisted values as `DEBUG`, `INFO`, and `ERROR`, and says unsupported loaded values fall back to `INFO` unless a later requirement adds `WARNING` UI parity. [VERIFIED: 02-SPEC.md]
   - Planning impact: Domain validation, controller persistence, and tests should only accept uppercase `DEBUG`, `INFO`, and `ERROR` as valid persisted `log_level` values. [VERIFIED: 02-SPEC.md]

2. **Should JSON output match Python tab indentation exactly?**
    - What we know: The reference writes JSON with `indent="\t"` and trailing newline. [VERIFIED: CMT/src/app_settings.py]
   - Resolution: Use `serde_json::to_string_pretty` plus newline for stable readable output and assert parsed content, not whitespace, indentation, or object key ordering. [CITED: docs.rs/serde_json/1.0.149] [VERIFIED: 02-CONTEXT.md]
   - Planning impact: Tests must parse JSON and assert exact keys/values rather than exact formatting. [VERIFIED: 02-CONTEXT.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust compiler | Build/tests | ✓ | rustc 1.95.0 | — [VERIFIED: rustc --version] |
| Cargo | Dependency/test commands | ✓ | cargo 1.95.0 | — [VERIFIED: cargo --version] |
| crates.io access | Add/verify `serde_json` and dev test crate | ✓ | `cargo search` returned versions | Use existing lockfile/cache if offline later. [VERIFIED: cargo search] |

**Missing dependencies with no fallback:** None found. [VERIFIED: environment audit]

**Missing dependencies with fallback:** None found. [VERIFIED: environment audit]

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness via `cargo test`; no separate `tests/` directory exists yet. [VERIFIED: current tests list] |
| Config file | `Cargo.toml` with existing dependencies; no test-specific config file. [VERIFIED: Cargo.toml] |
| Quick run command | `cargo test settings --lib` [VERIFIED: Cargo test harness convention] |
| Full suite command | `cargo test` plus project gates `cargo fmt --check`, `cargo check`, and `cargo clippy --all-targets --all-features`. [VERIFIED: AGENTS.md] |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| SET-01 | Missing settings file creates reference-compatible defaults. [VERIFIED: .planning/REQUIREMENTS.md] | unit/filesystem fixture | `cargo test settings_missing_file_defaults --lib` | ❌ Wave 0 [VERIFIED: current tests list] |
| SET-02 | All required keys persist with reference JSON names. [VERIFIED: .planning/REQUIREMENTS.md] | unit/filesystem fixture | `cargo test settings_persist_reference_keys --lib` | ❌ Wave 0 [VERIFIED: current tests list] |
| SET-03 | Update Channel labels and values match reference order. [VERIFIED: .planning/REQUIREMENTS.md] | source-level Slint contract | `cargo test settings_tab_update_channel_labels --lib` | ❌ Wave 0 [VERIFIED: ui/settings_tab.slint] |
| SET-04 | Log Level labels and values match reference order. [VERIFIED: .planning/REQUIREMENTS.md] | source-level Slint contract | `cargo test settings_tab_log_level_labels --lib` | ❌ Wave 0 [VERIFIED: ui/settings_tab.slint] |
| SET-05 | Scanner defaults for all seven categories are enabled. [VERIFIED: .planning/REQUIREMENTS.md] | unit | `cargo test scanner_settings_defaults_enabled --lib` | ❌ Wave 0 [VERIFIED: current tests list] |
| SET-06 | Malformed JSON resets; partial invalid JSON preserves valid values, repairs invalid values, removes unknown keys. [VERIFIED: .planning/REQUIREMENTS.md] | unit/filesystem fixture | `cargo test settings_repair --lib` | ❌ Wave 0 [VERIFIED: current tests list] |

### Sampling Rate
- **Per task commit:** `cargo test settings --lib` and `cargo fmt --check`. [VERIFIED: AGENTS.md]
- **Per wave merge:** `cargo test` and `cargo check`. [VERIFIED: AGENTS.md]
- **Phase gate:** `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy --all-targets --all-features`. [VERIFIED: AGENTS.md]

### Wave 0 Gaps
- [ ] `src/domain/settings.rs` tests or equivalent module tests — covers SET-01, SET-02, SET-05, SET-06. [VERIFIED: current tests list]
- [ ] `src/platform/settings_store.rs` tests or equivalent injectable IO tests — covers missing file, save failure, and asset resolver fallback. [VERIFIED: 02-CONTEXT.md]
- [ ] `ui/settings_tab.slint` source-level assertions — covers SET-03 and SET-04. [VERIFIED: 02-CONTEXT.md]
- [ ] Dev fixture dependency decision: add `assert_fs = "1.1.3"` or use `tempfile`. [VERIFIED: crates.io cargo search] [VERIFIED: STACK.md]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No authentication is in scope for local settings parity. [VERIFIED: 02-CONTEXT.md] |
| V3 Session Management | no | No sessions are in scope. [VERIFIED: 02-CONTEXT.md] |
| V4 Access Control | no | No multi-user authorization boundary is in scope. [VERIFIED: 02-CONTEXT.md] |
| V5 Input Validation | yes | Validate JSON shape, known keys, literal values, and boolean types before applying settings. [VERIFIED: CMT/src/app_settings.py] |
| V6 Cryptography | no | No secrets or cryptographic operations are in scope. [VERIFIED: 02-CONTEXT.md] |

### Known Threat Patterns for Rust Local Settings

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed or non-object settings file | Tampering | Reset to defaults, log, save defaults, and continue. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |
| Unknown settings keys | Tampering | Ignore in memory and remove on resave. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md] |
| Invalid enum/string values | Tampering | Preserve valid keys and default invalid keys individually. [VERIFIED: CMT/src/app_settings.py] |
| Save failure after UI selection | Integrity/Repudiation | Revert UI to last persisted value and log failure. [VERIFIED: 02-CONTEXT.md] |

## Sources

### Primary (HIGH confidence)
- `CMT/src/app_settings.py` — settings path, defaults, `download-source.txt`, validation, repair, save behavior. [VERIFIED: local file]
- `CMT/src/tabs/_settings.py` — Update Channel and Log Level labels, order, persisted values, immediate save callback. [VERIFIED: local file]
- `CMT/src/scan_settings.py` — scanner setting names and labels. [VERIFIED: local file]
- `.planning/phases/02-settings-defaults-parity/02-CONTEXT.md` — locked decisions and phase boundary. [VERIFIED: local file]
- `.planning/phases/02-settings-defaults-parity/02-SPEC.md` — acceptance criteria and scope. [VERIFIED: local file]
- `.planning/REQUIREMENTS.md` — SET-01 through SET-06. [VERIFIED: local file]
- `AGENTS.md` — project constraints and verification gates. [VERIFIED: local file]
- Context7 `/websites/slint_dev_rust_slint` and `/websites/slint_dev_slint` — Slint Rust properties/callbacks and standard widgets. [CITED: docs.slint.dev]
- Context7 `/websites/rs_serde_json_1_0_149_serde_json` and `/websites/serde_rs` — JSON `Value`, pretty serialization, Serde attributes/defaults. [CITED: docs.rs] [CITED: serde.rs]
- `cargo search serde_json --limit 1` and `cargo search assert_fs --limit 1` — crate version verification. [VERIFIED: crates.io cargo search]

### Secondary (MEDIUM confidence)
- Existing `STACK.md` embedded in `AGENTS.md` — prior stack research for `tempfile`, `assert_fs`, Slint, Serde, and tracing. [VERIFIED: AGENTS.md]

### Tertiary (LOW confidence)
- None. [VERIFIED: sources used]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions were checked from `Cargo.toml`, Context7 docs, and `cargo search`. [VERIFIED: Cargo.toml] [CITED: docs.rs] [VERIFIED: crates.io cargo search]
- Architecture: HIGH — phase context and AGENTS.md explicitly define Rust domain/platform/UI boundaries. [VERIFIED: 02-CONTEXT.md] [VERIFIED: AGENTS.md]
- Pitfalls: HIGH — pitfalls are derived from concrete reference behavior and locked Phase 02 decisions. [VERIFIED: CMT/src/app_settings.py] [VERIFIED: 02-CONTEXT.md]

**Research date:** 2026-05-17 [VERIFIED: system date]
**Valid until:** 2026-06-16 for stable local reference behavior; re-check crate versions if dependency edits occur after that date. [VERIFIED: local source stability] [ASSUMED]
