# Phase 3: Platform Discovery & Background Adapters - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 13
**Analogs found:** 13 / 13

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | request-response | `Cargo.toml` dependency block | exact |
| `src/domain/discovery.rs` | model | transform | `src/domain/settings.rs` + `CMT/src/game_info.py` | role-match |
| `src/domain/mod_manager.rs` | model | transform | `src/domain/settings.rs` + `CMT/src/mod_manager_info.py` | role-match |
| `src/domain/mod.rs` | config | transform | `src/domain/mod.rs` | exact |
| `src/platform/filesystem.rs` | utility/adapter | file-I/O | `src/platform/settings_store.rs` | role-match |
| `src/platform/registry.rs` | utility/adapter | request-response | `src/platform/settings_store.rs` + `CMT/src/utils.py` | role-match |
| `src/platform/process.rs` | utility/adapter | request-response | `src/platform/settings_store.rs` + `CMT/src/utils.py` | role-match |
| `src/platform/desktop.rs` | utility/adapter | request-response | `src/platform/settings_store.rs` | role-match |
| `src/platform/mod.rs` | config | transform | `src/platform/mod.rs` | exact |
| `src/services/mod.rs` | config | transform | `src/domain/mod.rs` / `src/platform/mod.rs` | exact |
| `src/services/discovery.rs` | service | request-response | `src/app/settings_controller.rs` + `CMT/src/game_info.py` | role-match |
| `src/workers/events.rs` | model | event-driven | `src/domain/settings.rs` + `src/workers/mod.rs` | role-match |
| `src/workers/handoff.rs` | service/adapter | event-driven | `src/platform/settings_store.rs` + `src/main.rs` weak-callback pattern | partial |
| `src/workers/mod.rs` | provider | event-driven | `src/workers/mod.rs` | exact |

## Pattern Assignments

### `Cargo.toml` (config, request-response)

**Analog:** `Cargo.toml`

**Dependency style pattern** (lines 7-17):
```toml
[dependencies]
anyhow = "1.0.102"
directories = "6.0.0"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
slint = "1.16.1"
thiserror = "2.0.18"
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync"] }
toml = "1.1.2"
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
```

**Apply:** keep dependency additions focused. Phase research recommends `windows-registry`, `sysinfo`, `pelite`, and `open` only behind adapters; add `tempfile` as dev-dependency if fake/integration fixtures need real temp paths.

---

### `src/domain/discovery.rs` (model, transform)

**Analogs:** `src/domain/settings.rs`, `CMT/src/game_info.py`

**Imports/model pattern** (`src/domain/settings.rs` lines 1-4, 31-45):
```rust
//! Typed settings model for reference-compatible CMT settings data.

use serde_json::{Map, Value, json};

/// Top-level application settings persisted in the reference `settings.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    pub log_level: LogLevel,
    pub update_source: UpdateSource,
    pub scanner: ScannerSettings,
    pub downgrader: DowngraderSettings,
}
```

**Reference derived-path pattern** (`CMT/src/game_info.py` lines 144-156):
```python
@game_path.setter
def game_path(self, value: Path) -> None:
	self._game_path = value
	self._game_path_sv.set(str(value))

	data_path = value / "Data"
	if is_dir(data_path):
		self._data_path = data_path
		f4se_path = data_path / "F4SE/Plugins"
		self._f4se_path = f4se_path if is_dir(f4se_path) else None
	else:
		self._data_path = None
		self._f4se_path = None
```

**Reference INI/archive suffix pattern** (`CMT/src/game_info.py` lines 82-110):
```python
docs_path = get_environment_path(CSIDL.Documents) / "My Games\\Fallout4"
for name in ("Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"):
	ini_path = docs_path / name
	if not is_file(ini_path):
		continue
...
if self.language == Language.English:
	self.ba2_suffixes: tuple[str, ...] = ("main", "textures", "voices_en")
else:
	self.ba2_suffixes = ("main", "textures", "voices_en", f"voices_{self.language}")
```

**Error/message pattern** (`CMT/src/game_info.py` lines 262-272):
```python
if not is_fo4_dir(game_path_as_path):
	if registry_path:
		msg = (
			"A Fallout 4 installation could not be found.\n\n"
			"The path set in your registry is:\n"
			f"{registry_path}\n\n"
			"If this is not correct, please run the Fallout 4 Launcher to correct it."
		)
	else:
		msg = "A Fallout 4 installation could not be found."
```

**Apply:** model `GameInstallation`, optional `data_path`/`f4se_path`, INI path/read state, archive/module sets, and `DiscoveryError { kind, user_message, diagnostic }`. Do not make missing `Data` a failure.

---

### `src/domain/mod_manager.rs` (model, transform)

**Analogs:** `src/domain/settings.rs`, `CMT/src/mod_manager_info.py`, `CMT/src/utils.py`

**Typed enum/value pattern** (`src/domain/settings.rs` lines 233-266):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub const fn as_wire_value(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        }
    }
}
```

**Reference manager detection/version fallback** (`CMT/src/utils.py` lines 175-194):
```python
def find_mod_manager() -> ModManagerInfo | None:
	pid = os.getppid()
	proc: Process | None = Process(pid)
	managers = {"ModOrganizer.exe", "Vortex.exe"}
	for _ in range(8):
		if proc is None:
			break
		with proc.oneshot():
			if proc.name() in managers:
				manager_path = Path(proc.exe())
				manager = "Mod Organizer" if proc.name() == "ModOrganizer.exe" else "Vortex"
				ver = get_file_version(manager_path)
				manager_version = Version(".".join(str(n) for n in ver[:3])) if ver else Version("0.0.0")
				return ModManagerInfo(manager, manager_path, manager_version)
			proc = proc.parent()
	return None
```

**Reference MO2 defaults/parsing pattern** (`CMT/src/mod_manager_info.py` lines 95-129, 164-195):
```python
self.mo2_settings = {
	"base_directory": ini_path.parent,
	"cache_directory": Path("%BASE_DIR%/webcache"),
	"download_directory": Path("%BASE_DIR%/downloads"),
	"mod_directory": Path("%BASE_DIR%/mods"),
	"overwrite_directory": Path("%BASE_DIR%/overwrite"),
	"profile_local_inis": False,
	"profile_local_saves": False,
	"profiles_directory": Path("%BASE_DIR%/profiles"),
	"skip_file_suffixes": (".mohidden",),
	"skip_directories": set(),
}
...
if game_name != "Fallout 4":
	msg = f"Only Fallout 4 is supported.\ngameName is '{game_name}' in INI: \n{ini_path}"
	raise ValueError(msg)

if "selected_profile" not in self.mo2_settings:
	msg = "Profile is not set in ModOrganizer.ini."
	raise ValueError(msg)
```

**Apply:** create `ManagerKind`, `ManagerContext`, `SemanticVersion`, `Mo2Context`, and parser result/error types. Keep display strings `Mod Organizer`/`Vortex` and `0.0.0` fallback exact.

---

### `src/domain/mod.rs` (config, transform)

**Analog:** `src/domain/mod.rs`

**Module boundary pattern** (lines 1-14):
```rust
//! Domain model boundary for future CMT behavior.

pub mod settings;

/// No-op domain state marker reserved for future typed application data.
#[derive(Debug, Default, Clone, Copy)]
pub struct DomainState;
```

**Apply:** add `pub mod discovery;` and `pub mod mod_manager;`. Preserve the marker unless implementation intentionally replaces it. Extend the importability test like lines 16-30 for new public domain types.

---

### `src/platform/filesystem.rs` (utility/adapter, file-I/O)

**Analog:** `src/platform/settings_store.rs`

**Trait + production/fake/static implementation pattern** (lines 13-24, 26-58, 60-79):
```rust
/// Resolves auxiliary settings assets such as `download-source.txt`.
pub trait AssetResolver {
    /// Reads the configured download source text if the asset is available.
    fn read_download_source(&self) -> io::Result<Option<String>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAssetResolver {
    download_source_path: PathBuf,
}

impl AssetResolver for FileAssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(&self.download_source_path) {
            Ok(source) => Ok(Some(source.trim().to_owned())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}
```

**Test fixture pattern** (`src/platform/settings_store.rs` lines 400-417):
```rust
fn isolated_settings_path(case_name: &str) -> (PathBuf, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cmt-rs-{case_name}-{unique}"));
    fs::create_dir_all(&root).expect("test temp directory should be created");
    let settings_path = root.join("settings.json");
    (root, settings_path)
}
```

**Apply:** define a small `FileSystem` trait (`is_file`, `is_dir`, `read_text`, `read_dir_sorted`) plus real and fake implementations. Sort enumeration deterministically; tests must not depend on the real Fallout 4 filesystem.

---

### `src/platform/registry.rs` (utility/adapter, request-response)

**Analogs:** `src/platform/settings_store.rs`, `CMT/src/utils.py`

**Adapter trait style** (`src/platform/settings_store.rs` lines 18-24):
```rust
pub trait AssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>>;
}
```

**Reference registry lookup pattern** (`CMT/src/utils.py` lines 272-283):
```python
def get_registry_value(key: int, subkey: str, value_name: str) -> str | None:
	try:
		with winreg.OpenKey(key, subkey) as reg_handle:
			value, value_type = winreg.QueryValueEx(reg_handle, value_name)

		if value and value_type == winreg.REG_SZ and isinstance(value, str):
			return value
	except OSError:
		pass
	return None
```

**Reference registry keys** (`CMT/src/game_info.py` lines 221-229):
```python
registry_path = get_registry_value(
	winreg.HKEY_LOCAL_MACHINE,
	R"SOFTWARE\WOW6432Node\Bethesda Softworks\Fallout4",
	"Installed Path",
) or get_registry_value(
	winreg.HKEY_LOCAL_MACHINE,
	R"SOFTWARE\WOW6432Node\GOG.com\Games\1998527297",
	"path",
)
```

**Apply:** expose `RegistryAdapter::get_string(root, subkey, value)` returning `Result<Option<String>, RegistryError>`. Real non-Windows implementation should return typed unsupported errors where direct registry access is requested; fake adapter drives tests.

---

### `src/platform/process.rs` (utility/adapter, request-response)

**Analogs:** `CMT/src/utils.py`, `src/platform/settings_store.rs`

**Reference process walk/version metadata** (`CMT/src/utils.py` lines 175-194, 210-222):
```python
for _ in range(8):
	if proc is None:
		break
	with proc.oneshot():
		if proc.name() in managers:
			manager_path = Path(proc.exe())
			manager = "Mod Organizer" if proc.name() == "ModOrganizer.exe" else "Vortex"
			ver = get_file_version(manager_path)
			manager_version = Version(".".join(str(n) for n in ver[:3])) if ver else Version("0.0.0")
			return ModManagerInfo(manager, manager_path, manager_version)
		proc = proc.parent()

def get_file_version(path: Path) -> tuple[int, int, int, int] | None:
	try:
		info = win32api.GetFileVersionInfo(str(path), "\\")
	except:
		return None
```

**Error logging without UI leakage pattern** (`src/app/settings_controller.rs` lines 101-118):
```rust
match self.store.save(&candidate) {
    Ok(()) => {
        self.last_persisted = candidate;
        visible_value(&self.last_persisted)
    }
    Err(error) => {
        tracing::error!(
            path = %self.store.settings_path().display(),
            %error,
            "Settings : Failed to save settings; reverting UI selection"
        );
        match visible_value(&candidate) {
            "debug" | "info" | "warning" | "error" => previous_log_level,
            _ => previous_update_source,
        }
    }
}
```

**Apply:** define `ProcessAdapter` with process list/parent metadata and version metadata. Keep raw OS errors in diagnostics/tracing; return safe typed results. Use fakes for Mod Organizer, Vortex, unknown processes, and version fallback tests.

---

### `src/platform/desktop.rs` (utility/adapter, request-response)

**Analog:** `src/platform/settings_store.rs`

**Adapter result style** (`src/platform/settings_store.rs` lines 119-128, 224-234):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSettings {
    pub settings: AppSettings,
    pub diagnostics: Vec<RepairDiagnostic>,
    pub reset_to_defaults: bool,
}

pub fn save(&self, settings: &AppSettings) -> io::Result<()> {
    let mut json =
        serde_json::to_string_pretty(&settings.to_json_value()).map_err(io::Error::other)?;
    json.push('\n');
    fs::write(&self.paths.settings_path, json)
}
```

**Apply:** use typed `DesktopAction`, `DesktopActionResult`, and `DesktopActionError` for open URL, open path, and launch external tool. Do not show dialogs in adapters; surface success/failure as values for worker/controller code.

---

### `src/platform/mod.rs` (config, transform)

**Analog:** `src/platform/mod.rs`

**Module boundary pattern** (lines 1-14):
```rust
//! Platform adapter boundary for future operating-system integrations.

pub mod settings_store;

/// No-op platform services marker reserved for future OS-facing adapters.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformServices;
```

**Apply:** add `pub mod filesystem;`, `pub mod registry;`, `pub mod process;`, and `pub mod desktop;` (or combine desktop into process only if planner chooses one file). Keep platform side effects isolated here.

---

### `src/services/mod.rs` (config, transform)

**Analog:** `src/domain/mod.rs` / `src/platform/mod.rs`

**Module declaration pattern** (`src/domain/mod.rs` lines 1-8):
```rust
//! Domain model boundary for future CMT behavior.

pub mod settings;
```

**Apply:** create `src/services/mod.rs` with a doc comment describing orchestration-only code and `pub mod discovery;`. Also add `pub mod services;` to `src/main.rs` near lines 1-4 if the crate exposes services from the binary root.

---

### `src/services/discovery.rs` (service, request-response)

**Analogs:** `src/app/settings_controller.rs`, `CMT/src/game_info.py`

**Controller/service orchestration pattern** (`src/app/settings_controller.rs` lines 20-42):
```rust
impl<R: AssetResolver> SettingsController<R> {
    pub fn load(store: SettingsStore<R>) -> io::Result<Self> {
        let loaded = store.load()?;

        Ok(Self::from_settings(store, loaded.settings))
    }

    pub fn from_settings(store: SettingsStore<R>, settings: AppSettings) -> Self {
        Self {
            store,
            last_persisted: settings,
        }
    }
}
```

**Reference discovery order** (`CMT/src/game_info.py` lines 175-229, 258-275):
```python
def find_path(self) -> None:
	if self.manager is not None:
		...
		if self.manager.game_path:
			self.game_path = self.manager.game_path
			return

	if is_fo4_dir(Path.cwd()):
		self.game_path = Path.cwd()
		return

	registry_path = get_registry_value(...Bethesda...) or get_registry_value(...GOG...)
	game_path_as_path = Path(game_path)
	if is_file(game_path_as_path):
		game_path_as_path = game_path_as_path.parent

	if not is_fo4_dir(game_path_as_path):
		...
	self.game_path = game_path_as_path
```

**Apply:** orchestrate adapters in the locked order: manager game path, current working directory, Bethesda/GOG registry. Accept direct `Fallout4.exe` inputs by normalizing to parent. Return result values, never show UI or exit.

---

### `src/workers/events.rs` (model, event-driven)

**Analogs:** `src/domain/settings.rs`, `src/workers/mod.rs`

**Typed aggregate pattern** (`src/domain/settings.rs` lines 202-222):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRepairResult {
    pub settings: AppSettings,
    pub diagnostics: Vec<RepairDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairDiagnostic {
    MissingKey { key: String },
    InvalidValue { key: String },
    InvalidType { key: String },
    UnknownKey { key: String },
}
```

**Existing worker boundary** (`src/workers/mod.rs` lines 1-13):
```rust
//! Worker orchestration boundary for future long-running CMT tasks.
//!
//! Future phases can place scan, patch, download, and subprocess orchestration
//! behind this module so slow work stays off the Slint UI thread.

/// No-op worker runtime marker reserved for future background orchestration.
#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerRuntime;
```

**Apply:** define `WorkerEvent`, `TaskId`, `TaskKind`, `TaskStatus`, `Progress`, `WorkerPayload`, and typed error/action result payloads. Include discovery, scan, patch, download, external process, cancellation, and generic/unknown variants now even if only test-constructed.

---

### `src/workers/handoff.rs` (service/adapter, event-driven)

**Analogs:** `src/platform/settings_store.rs`, `src/main.rs`

**Trait/sink pattern** (`src/platform/settings_store.rs` lines 13-24):
```rust
pub trait AssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>>;
}
```

**Slint weak callback boundary pattern** (`src/main.rs` lines 33-60):
```rust
fn bind_settings_callbacks(app: &MainWindow, controller: SettingsController<FileAssetResolver>) {
    let controller = Rc::new(RefCell::new(controller));

    app.on_update_source_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);

        move |selected| {
            let visible_value = controller
                .borrow_mut()
                .select_update_source(selected.as_str());
            if let Some(app) = app.upgrade() {
                app.set_update_source(visible_value.into());
            }
        }
    });
}
```

**Apply:** define `EventSink: Send + Sync + 'static`, `RecordingEventSink` for tests, and `SlintEventLoopSink` using `slint::invoke_from_event_loop` or app-layer `Weak::upgrade_in_event_loop`. Worker/core code should emit owned `WorkerEvent` and should not import generated Slint component types.

---

### `src/workers/mod.rs` (provider, event-driven)

**Analog:** `src/workers/mod.rs`

**Existing seam pattern** (lines 1-13):
```rust
//! Worker orchestration boundary for future long-running CMT tasks.
//!
//! Future phases can place scan, patch, download, and subprocess orchestration
//! behind this module so slow work stays off the Slint UI thread.

#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerRuntime;
```

**Apply:** replace/extend the inert marker with `pub mod events; pub mod handoff;` and a lightweight facade only if needed. Keep construction side-effect-free unless planner explicitly creates a Tokio runtime in this phase.

## Shared Patterns

### Public Rust API Documentation
**Source:** `src/domain/settings.rs` and `src/platform/settings_store.rs` throughout; examples at `src/domain/settings.rs` lines 31-45 and `src/platform/settings_store.rs` lines 13-24.  
**Apply to:** all public structs, enums, traits, and public methods in new Phase 3 files.

```rust
/// Filesystem-backed load/save boundary for [`crate::domain::settings::AppSettings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsStore<R = FileAssetResolver> {
    paths: SettingsPaths<R>,
}
```

### Fakeable Adapter Boundary
**Source:** `src/platform/settings_store.rs` lines 13-24, 60-79, 137-146.  
**Apply to:** filesystem, registry, process, desktop adapters and discovery service tests.

```rust
pub trait AssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAssetResolver {
    download_source: Option<String>,
}
```

### Safe Error Reporting + Diagnostics
**Source:** `src/app/settings_controller.rs` lines 101-118 and `src/domain/settings.rs` lines 202-222.  
**Apply to:** discovery, MO2 parser, platform adapters, desktop actions, worker errors.

```rust
Err(error) => {
    tracing::error!(
        path = %self.store.settings_path().display(),
        %error,
        "Settings : Failed to save settings; reverting UI selection"
    );
    // Preserve the pre-save snapshot: callers reset the Slint property to this value.
    match visible_value(&candidate) {
        "debug" | "info" | "warning" | "error" => previous_log_level,
        _ => previous_update_source,
    }
}
```

### Reference Fallout 4 Validation
**Source:** `CMT/src/utils.py` lines 171-172 and `CMT/src/game_info.py` lines 258-275.  
**Apply to:** discovery candidate normalization and validation.

```python
def is_fo4_dir(path: Path) -> bool:
	return is_dir(path) and is_file(path / "Fallout4.exe")
```

### Slint Boundary / No Cross-Thread UI Mutation
**Source:** `src/main.rs` lines 33-60 and `03-RESEARCH.md` Slint handoff recommendation.  
**Apply to:** worker handoff and any future background callbacks.

```rust
let app = app.as_weak();
...
if let Some(app) = app.upgrade() {
    app.set_update_source(visible_value.into());
}
```

## No Analog Found

All proposed files have usable local analogs. The weakest local match is `src/workers/handoff.rs`; use the research Slint `invoke_from_event_loop` example plus the existing weak-handle callback boundary in `src/main.rs`.

## Metadata

**Analog search scope:** `src/**/*.rs`, `CMT/src/game_info.py`, `CMT/src/utils.py`, `CMT/src/mod_manager_info.py`, `Cargo.toml`  
**Files scanned:** 14  
**Pattern extraction date:** 2026-05-17
