# Phase 02: Settings & Defaults Parity - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 10
**Analogs found:** 10 / 10

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | request-response | `Cargo.toml` | exact-modification |
| `src/domain/settings.rs` | model | CRUD | `CMT/src/app_settings.py` + `src/domain/mod.rs` | role-match |
| `src/domain/mod.rs` | config | transform | `src/domain/mod.rs` | exact-modification |
| `src/platform/settings_store.rs` | service | file-I/O | `CMT/src/app_settings.py` + `CMT/src/utils.py` | role-match |
| `src/platform/mod.rs` | config | transform | `src/platform/mod.rs` | exact-modification |
| `src/app/settings_controller.rs` | controller | event-driven | `CMT/src/tabs/_settings.py` + `src/app/mod.rs` | role-match |
| `src/app/mod.rs` | config | transform | `src/app/mod.rs` | exact-modification |
| `src/main.rs` | route | event-driven | `src/main.rs` | exact-modification |
| `ui/settings_tab.slint` | component | event-driven | `ui/settings_tab.slint` + `CMT/src/tabs/_settings.py` | exact-modification |
| `src/main.rs` or module tests | test | file-I/O | `src/main.rs` tests | exact |

## Pattern Assignments

### `Cargo.toml` (config, request-response)

**Analog:** `Cargo.toml`

**Dependency pattern** (lines 7-16):
```toml
[dependencies]
anyhow = "1.0.102"
directories = "6.0.0"
serde = { version = "1.0.228", features = ["derive"] }
slint = "1.16.1"
thiserror = "2.0.18"
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync"] }
toml = "1.1.2"
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
```

**Apply:** Add `serde_json` under `[dependencies]`. If fixture ergonomics are needed, add one focused dev dependency such as `assert_fs` or `tempfile`; keep the existing direct version-string style.

---

### `src/domain/settings.rs` (model, CRUD)

**Analog:** `CMT/src/app_settings.py` for reference behavior, `src/domain/mod.rs` for Rust module documentation style.

**Rust domain boundary style** (`src/domain/mod.rs` lines 1-5):
```rust
//! Domain model boundary for future CMT behavior.
//!
//! This module is intentionally inert in Phase 1. Later port slices can add
//! typed settings, scan results, game metadata, archive information, and other
//! pure domain state here without putting that logic in Slint markup.
```

**Reference imports and constants** (`CMT/src/app_settings.py` lines 20-30):
```python
import json
import logging
from pathlib import Path
from typing import Literal, TypedDict, get_args, get_origin

from utils import get_asset_path, is_file

logger = logging.getLogger(__name__)

SETTINGS_PATH = Path("settings.json")
```

**Reference settings schema** (`CMT/src/app_settings.py` lines 43-54):
```python
class AppSettingsDict(TypedDict):
	log_level: Literal["DEBUG", "INFO", "WARNING", "ERROR"]
	update_source: Literal["nexus", "github", "both", "none"]
	scanner_OverviewIssues: bool
	scanner_Errors: bool
	scanner_WrongFormat: bool
	scanner_LoosePrevis: bool
	scanner_JunkFiles: bool
	scanner_ProblemOverrides: bool
	scanner_RaceSubgraphs: bool
	downgrader_keep_backups: bool
	downgrader_delete_deltas: bool
```

**Reference defaults** (`CMT/src/app_settings.py` lines 57-69):
```python
DEFAULT_SETTINGS: AppSettingsDict = {
	"log_level": "INFO",
	"update_source": download_source,  # type: ignore[typeddict-item]
	"scanner_OverviewIssues": True,
	"scanner_Errors": True,
	"scanner_WrongFormat": True,
	"scanner_LoosePrevis": True,
	"scanner_JunkFiles": True,
	"scanner_ProblemOverrides": True,
	"scanner_RaceSubgraphs": True,
	"downgrader_keep_backups": True,
	"downgrader_delete_deltas": True,
}
```

**Validation and repair pattern** (`CMT/src/app_settings.py` lines 91-127):
```python
new_settings = [k for k in self.dict if k not in json_content]
if new_settings:
	logger.info("Settings : Adding new settings to JSON: %s", ", ".join(new_settings))
	resave = True

for k, v in json_content.items():
	if k not in self.dict:
		logger.error("Settings : Unknown setting '%s' will be removed.", k)
		resave = True
		continue

	annotation = AppSettingsDict.__annotations__.get(k)
	if not annotation:
		logger.debug("Settings : '%s' has no set type", k)
		self.dict[k] = v
	elif get_origin(annotation) is Literal:
		if v in get_args(annotation):
			logger.debug("Settings : '%s' is correct type (%s)", k, type(v).__name__)
			self.dict[k] = v
		else:
			logger.error("Settings : '%s' has invalid value '%s'. Reset to '%s'", k, v, self.dict[k])  # type: ignore[reportUnknownArgumentType]
			resave = True
	elif type(v) is annotation:
		logger.debug("Settings : '%s' is correct type (%s)", k, type(v).__name__)
		self.dict[k] = v
	else:
		logger.error(
			"Settings : '%s' has invalid type (%s) '%s'. Reset to '%s'",
			k,
			type(v).__name__,
			v,
			self.dict[k],  # type: ignore[reportUnknownArgumentType]
		)
		resave = True

if resave:
	self.save()
```

**Apply:** Implement typed Rust enums/structs with conversion from reference wire strings. Parse valid JSON through `serde_json::Value`/object inspection so valid keys are preserved independently, invalid keys default, and unknown keys are omitted on resave.

---

### `src/domain/mod.rs` (config, transform)

**Analog:** `src/domain/mod.rs`

**Module documentation and public marker pattern** (lines 1-12):
```rust
//! Domain model boundary for future CMT behavior.
//!
//! This module is intentionally inert in Phase 1. Later port slices can add
//! typed settings, scan results, game metadata, archive information, and other
//! pure domain state here without putting that logic in Slint markup.

/// No-op domain state marker reserved for future typed application data.
///
/// Constructing this marker performs no filesystem, registry, settings,
/// scanner, network, subprocess, or background work.
#[derive(Debug, Default, Clone, Copy)]
pub struct DomainState;
```

**Apply:** Add `pub mod settings;` and re-export only stable types that app/platform layers need. Preserve doc-comment style for new public types/functions.

---

### `src/platform/settings_store.rs` (service, file-I/O)

**Analog:** `CMT/src/app_settings.py` for load/save behavior, `CMT/src/utils.py` for asset path resolution, `src/platform/mod.rs` for Rust boundary style.

**Platform boundary style** (`src/platform/mod.rs` lines 1-5):
```rust
//! Platform adapter boundary for future operating-system integrations.
//!
//! This module exists so later slices can isolate filesystem, registry, process,
//! dialog, and URL-opening adapters from UI and domain code. Phase 1 keeps the
//! boundary as a no-op marker and performs no platform access.
```

**Default `download-source.txt` detection** (`CMT/src/app_settings.py` lines 31-41):
```python
source_path = get_asset_path("download-source.txt")
try:
	download_source = source_path.read_text("utf-8", "ignore").strip()
	if download_source not in {"nexus", "github"}:
		logger.error("Settings : Invalid download source: '%s'", download_source)
		download_source = "nexus"
	else:
		logger.debug("Settings : Download source: '%s'", download_source)
except:
	logger.exception("Settings : Failed to detect download source.")
	download_source = "nexus"
```

**Missing/malformed file behavior** (`CMT/src/app_settings.py` lines 72-90):
```python
class AppSettings:
	def __init__(self) -> None:
		self.dict = DEFAULT_SETTINGS.copy()

		if not is_file(SETTINGS_PATH):
			logger.info("Settings : %s not found; using defaults.", SETTINGS_PATH.name)
			self.save()
			return

		resave = False
		try:
			json_content: AppSettingsDict = json.loads(SETTINGS_PATH.read_text("utf-8"))
			if not isinstance(json_content, dict):  # type: ignore[reportUnnecessaryIsInstance]
				# File doesn't contain a JSON Object
				raise ValueError  # noqa: TRY004
		except:
			logger.exception("Settings : Failed to load %s. Settings will be reset.", SETTINGS_PATH.name)
			resave = True
```

**Save pattern** (`CMT/src/app_settings.py` lines 129-136):
```python
def save(self) -> None:
	logger.debug("Settings : Saving %s", SETTINGS_PATH.name)
	try:
		with SETTINGS_PATH.open("w", encoding="utf-8") as f:
			json.dump(self.dict, f, indent="\t")
			f.write("\n")
	except:
		logger.exception("Settings : Failed to save %s", SETTINGS_PATH.name)
```

**Asset resolver pattern** (`CMT/src/utils.py` lines 197-200):
```python
def get_asset_path(relative_path: str) -> Path:
	# PyInstaller EXEs extract to a temp folder and store the path in sys._MEIPASS
	base_path = Path(str(getattr(sys, "_MEIPASS", False) or "."))
	return base_path / "assets" / relative_path
```

**Apply:** Put current-directory `settings.json` path and injectable test paths here, not in Slint. Add an asset resolver trait/struct that can read `download-source.txt` and fall back to `nexus` when missing or invalid.

---

### `src/platform/mod.rs` (config, transform)

**Analog:** `src/platform/mod.rs`

**No-op marker and doc comment pattern** (lines 7-12):
```rust
/// No-op platform services marker reserved for future OS-facing adapters.
///
/// Constructing this marker does not read paths, query the registry, inspect the
/// environment, launch processes, or disclose filesystem state.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformServices;
```

**Apply:** Add `pub mod settings_store;` and re-export `SettingsStore`/asset resolver types if app startup needs them. Update stale comments if `PlatformServices` stops being fully no-op.

---

### `src/app/settings_controller.rs` (controller, event-driven)

**Analog:** `CMT/src/tabs/_settings.py` for immediate radio-save behavior, `src/app/mod.rs` for Rust app-boundary style.

**App boundary style** (`src/app/mod.rs` lines 1-7):
```rust
//! Application-facing shell contracts for the Rust/Slint port.
//!
//! The labels below are copied from the reference `Tab` enum in
//! `CMT/src/enums.py` and the creation order in `CMT/src/cm_checker.py`. They
//! intentionally remain static in Phase 1 so tests can lock the shell identity
//! without launching GUI automation or wiring real tab behavior.
```

**Reference enum wire values** (`CMT/src/tabs/_settings.py` lines 33-43):
```python
class UpdateMode(StrEnum):
	DontCheck = "none"
	NexusModsOnly = "nexus"
	GitHubOnly = "github"
	GitHubAndNexusMods = "both"


class LogLevel(StrEnum):
	DEBUG = "DEBUG"
	INFO = "INFO"
	ERROR = "ERROR"
```

**Reference initial binding** (`CMT/src/tabs/_settings.py` lines 46-52):
```python
class SettingsTab(CMCTabFrame):
	def __init__(self, cmc: CMCheckerInterface, notebook: ttk.Notebook) -> None:
		super().__init__(cmc, notebook, "Settings")

		self.sv_setting_update_source = StringVar(value=cmc.settings.dict["update_source"])
		self.sv_setting_log_level = StringVar(value=cmc.settings.dict["log_level"])
```

**Reference immediate-save callback** (`CMT/src/tabs/_settings.py` lines 97-105):
```python
def update_setting(s: str = action, v: Variable = var) -> None:
	self.on_radio_change(s, v)
radio.configure(command=update_setting)
radio.pack(anchor=W, side=TOP)
ToolTip(radio, tooltip)

def on_radio_change(self, setting: str, variable: Variable) -> None:
	self.cmc.settings.dict[setting] = variable.get()
	self.cmc.settings.save()
```

**Apply:** Bind Slint callbacks to update typed settings and persist immediately. Keep a last-persisted copy so save failure can set Slint properties back and expose the UI-SPEC error text.

---

### `src/app/mod.rs` (config, transform)

**Analog:** `src/app/mod.rs`

**Constants and doc comments pattern** (lines 8-19):
```rust
/// Reference shell tab labels in their display order.
pub const SHELL_TAB_LABELS: [&str; 6] =
    ["Overview", "F4SE", "Scanner", "Tools", "Settings", "About"];

/// Returns the canonical shell tab labels in reference display order.
///
/// The labels match `CMT/src/enums.py` and the notebook construction order in
/// `CMT/src/cm_checker.py`. The function performs no GUI, filesystem, settings,
/// scanner, network, subprocess, or background work.
pub const fn shell_tab_labels() -> [&'static str; 6] {
    SHELL_TAB_LABELS
}
```

**Controller marker pattern** (lines 21-27):
```rust
/// No-op application controller boundary reserved for future UI orchestration.
///
/// Phase 1 deliberately keeps this type inert: it owns no settings, platform
/// adapters, worker handles, or Slint component references, and constructing it
/// has no side effects.
#[derive(Debug, Default, Clone, Copy)]
pub struct ShellController;
```

**Apply:** Add `pub mod settings_controller;`. If `ShellController` becomes stateful, rewrite the no-op doc comment rather than leaving it inaccurate.

---

### `src/main.rs` (route, event-driven)

**Analog:** `src/main.rs`

**Generated Slint module and launch pattern** (lines 1-10):
```rust
pub mod app;
pub mod domain;
pub mod platform;
pub mod workers;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = MainWindow::new()?;
    app.run()
}
```

**Source-level test include pattern** (lines 22-60):
```rust
const MAIN_SLINT: &str = include_str!("../ui/main.slint");
const TAB_COMPONENTS: [(&str, &str, &str, &str); 6] = [
    (
        "ui/overview_tab.slint",
        "OverviewTab",
        "Overview",
        include_str!("../ui/overview_tab.slint"),
    ),
    // ...
    (
        "ui/settings_tab.slint",
        "SettingsTab",
        "Settings",
        include_str!("../ui/settings_tab.slint"),
    ),
];
```

**Apply:** Load settings before showing `MainWindow`, initialize Settings-tab properties, then connect callbacks. Keep JSON/filesystem logic delegated to domain/platform/app modules.

---

### `ui/settings_tab.slint` (component, event-driven)

**Analog:** `ui/settings_tab.slint` for local formatting, `CMT/src/tabs/_settings.py` for reference controls.

**Current Slint component pattern** (`ui/settings_tab.slint` lines 1-20):
```slint
export component SettingsTab inherits Rectangle {
    VerticalLayout {
        padding: 24px;
        spacing: 16px;
        alignment: center;

        Text {
            text: "Settings";
            font-size: 18px;
            font-weight: 600;
            horizontal-alignment: center;
        }

        Text {
            text: "Settings behavior is reserved for a later port phase.";
            font-size: 14px;
            horizontal-alignment: center;
        }
    }
}
```

**Reference option group structure** (`CMT/src/tabs/_settings.py` lines 59-85):
```python
options_radios = {
	"Update Channel": (
		TOOLTIP_UPDATE_SOURCE,
		1,
		0,
		self.sv_setting_update_source,
		"update_source",
		(
			("All: GitHub & Nexus Mods", UpdateMode.GitHubAndNexusMods),
			("Early: GitHub", UpdateMode.GitHubOnly),
			("Stable: Nexus Mods", UpdateMode.NexusModsOnly),
			("Never: Don't Check", UpdateMode.DontCheck),
		),
	),
	"Log Level": (
		TOOLTIP_LOG_LEVEL,
		2,
		0,
		self.sv_setting_log_level,
		"log_level",
		(
			("Debug", LogLevel.DEBUG),
			("Info", LogLevel.INFO),
			("Error", LogLevel.ERROR),
		),
	),
}
```

**Reference radio construction** (`CMT/src/tabs/_settings.py` lines 87-101):
```python
for name, (tooltip, column, row, var, action, options) in options_radios.items():
	frame = ttk.Labelframe(self, text=name, padding=5)
	frame.grid(column=column, row=row, padx=5, pady=5, sticky=NSEW)
	for text, value in options:
		radio = ttk.Radiobutton(
			frame,
			value=value,
			variable=var,
			text=text,
		)
```

**Tooltip copy source** (`CMT/src/globals.py` lines 301-307):
```python
TOOLTIP_UPDATE_SOURCE = """GitHub will always have the latest release.
Nexus Mods releases may be delayed due to
their review process or to await more testing."""
TOOLTIP_LOG_LEVEL = """Sets the minimum importance level for messages sent to the log file.
DEBUG: Extra verbose.
INFO: Default.
ERROR: Only log errors."""
```

**Apply:** Import Slint standard widgets as needed, replace centered placeholder with top-aligned content, expose `in-out` selected-value properties plus callbacks, and keep labels/order exactly as shown above. Use UI-SPEC error copy only for save failure: `Could not save settings. Your previous setting was restored.`

---

### `src/main.rs` or module tests (test, file-I/O)

**Analog:** `src/main.rs` tests.

**Source parsing helper** (lines 62-72):
```rust
fn slint_string_property_values(source: &str, property: &str) -> Vec<String> {
    let prefix = format!("{property}:");

    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix(&prefix))
        .filter_map(|value| value.trim().trim_end_matches(';').strip_prefix('"'))
        .filter_map(|value| value.strip_suffix('"'))
        .map(String::from)
        .collect()
}
```

**Contract assertion style** (lines 125-152):
```rust
for (file, component, label, source) in TAB_COMPONENTS {
    assert_eq!(
        source.matches("export component ").count(),
        1,
        "{file} should export exactly one component"
    );
    assert!(
        source.contains(&format!("export component {component}")),
        "{file} should export {component}"
    );
    assert!(
        source.contains(&format!("text: \"{label}\";")),
        "{file} should keep the reference tab heading"
    );
    // ... prohibited marker checks ...
}
```

**Boundary construction test** (lines 155-161):
```rust
#[test]
fn shell_contract_boundary_markers_construct_as_no_ops() {
    let _controller = ShellController;
    let _domain = DomainState;
    let _platform = PlatformServices;
    let _workers = WorkerRuntime;
}
```

**Apply:** Follow this low-cost source-level style for `ui/settings_tab.slint` label/order tests. Put settings load/save tests near the settings module or platform store with injectable paths so tests never touch repository `settings.json`.

## Shared Patterns

### Reference Fidelity Comments
**Source:** `src/app/mod.rs` lines 1-6 and `ui/main.slint` line 14
**Apply to:** New settings constants, UI labels, and controller wiring.
```rust
//! The labels below are copied from the reference `Tab` enum in
//! `CMT/src/enums.py` and the creation order in `CMT/src/cm_checker.py`. They
//! intentionally remain static in Phase 1 so tests can lock the shell identity
//! without launching GUI automation or wiring real tab behavior.
```
```slint
// The tab labels and order mirror CMT/src/cm_checker.py and CMT/src/enums.py.
```

### Rust Public API Documentation
**Source:** `src/app/mod.rs` lines 12-17, `src/domain/mod.rs` lines 7-11, `src/platform/mod.rs` lines 7-10
**Apply to:** Public settings types/functions and store/controller APIs.
```rust
/// Returns the canonical shell tab labels in reference display order.
///
/// The labels match `CMT/src/enums.py` and the notebook construction order in
/// `CMT/src/cm_checker.py`. The function performs no GUI, filesystem, settings,
/// scanner, network, subprocess, or background work.
pub const fn shell_tab_labels() -> [&'static str; 6] {
    SHELL_TAB_LABELS
}
```

### Quiet Repair Logging
**Source:** `CMT/src/app_settings.py` lines 87-99 and 111-124
**Apply to:** Settings load/repair paths.
```python
except:
	logger.exception("Settings : Failed to load %s. Settings will be reset.", SETTINGS_PATH.name)
	resave = True
# ...
if k not in self.dict:
	logger.error("Settings : Unknown setting '%s' will be removed.", k)
	resave = True
	continue
# ...
logger.error(
	"Settings : '%s' has invalid type (%s) '%s'. Reset to '%s'",
	k,
	type(v).__name__,
	v,
	self.dict[k],
)
```

### Immediate Save With Revert-On-Failure Extension
**Source:** `CMT/src/tabs/_settings.py` lines 103-105 and `02-CONTEXT.md` lines 49-52
**Apply to:** Settings-tab radio callbacks.
```python
def on_radio_change(self, setting: str, variable: Variable) -> None:
	self.cmc.settings.dict[setting] = variable.get()
	self.cmc.settings.save()
```
Context adds: save failures after radio selection should revert to the last persisted value and log the failure.

### Source-Level UI Contract Tests
**Source:** `src/main.rs` lines 22-60 and 125-152
**Apply to:** Settings-tab label/order tests.
```rust
const MAIN_SLINT: &str = include_str!("../ui/main.slint");
// ...
assert!(
    source.contains(&format!("text: \"{label}\";")),
    "{file} should keep the reference tab heading"
);
```

## No Analog Found

All proposed Phase 02 files have at least a strong role-match analog. There is no existing Rust settings implementation, so the planner should treat `CMT/src/app_settings.py` and `CMT/src/tabs/_settings.py` as behavior analogs while copying Rust style from the existing Phase 1 Rust modules.

## Metadata

**Analog search scope:** `src/**/*.rs`, `ui/**/*.slint`, `Cargo.toml`, `build.rs`, and read-only reference files under `CMT/src/`.
**Files scanned:** 20
**Pattern extraction date:** 2026-05-17
**Read-only reference status:** `CMT/` was read only; no source files were modified.
