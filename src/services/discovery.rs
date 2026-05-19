//! Fallout 4 and mod-manager discovery orchestration.
//!
//! The service in this module preserves the reference discovery order while
//! keeping all operating-system access behind fakeable platform traits. It never
//! opens a manual file picker: failures are returned as typed, recoverable
//! results with reference-compatible user messages and structured attempt data.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Component, Path, PathBuf},
};

use crate::{
    domain::{
        discovery::{DiscoveryError, FALLOUT4_EXECUTABLE, Fallout4Installation, SemanticVersion},
        mod_manager::{
            DetectedModManager, Mo2Configuration, Mo2ParseError, Mo2ParseErrorKind, ModManagerKind,
            ModOrganizerContext, ModOrganizerDirectories, ModOrganizerSkipRules, ModOrganizerTool,
            VortexContext,
        },
    },
    platform::{
        PlatformError, PlatformOperation,
        filesystem::Filesystem,
        process::{ProcessInspector, SystemMetadata},
        registry::{RegistryHive, RegistryReader, RegistryValueRequest},
    },
};

const PROCESS_ANCESTOR_LIMIT: usize = 8;
const MO2_REGISTRY_SUBKEY: &str = r"Software\Mod Organizer Team\Mod Organizer";
const MO2_CURRENT_INSTANCE_VALUE: &str = "CurrentInstance";
const BETHESDA_FALLOUT4_REGISTRY_SUBKEY: &str = r"SOFTWARE\WOW6432Node\Bethesda Softworks\Fallout4";
const BETHESDA_FALLOUT4_REGISTRY_VALUE: &str = "Installed Path";
const GOG_FALLOUT4_REGISTRY_SUBKEY: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games\1998527297";
const GOG_FALLOUT4_REGISTRY_VALUE: &str = "path";

/// Inputs that are environmental in production but deterministic in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRequest {
    /// Current process id, used to walk ancestors exactly like the reference app.
    pub current_process_id: Option<u32>,
    /// Current working directory candidate checked after a manager game path.
    pub current_working_directory: PathBuf,
    /// `LOCALAPPDATA` equivalent used for MO2 instance INI lookup.
    pub local_appdata: Option<PathBuf>,
}

impl DiscoveryRequest {
    /// Creates a discovery request with no current process id or local appdata.
    pub fn new(current_working_directory: impl Into<PathBuf>) -> Self {
        Self {
            current_process_id: None,
            current_working_directory: current_working_directory.into(),
            local_appdata: None,
        }
    }

    /// Adds the process id whose parent chain should be inspected for managers.
    pub const fn with_current_process_id(mut self, process_id: u32) -> Self {
        self.current_process_id = Some(process_id);
        self
    }

    /// Adds the `LOCALAPPDATA` path used for MO2 instance configuration lookup.
    pub fn with_local_appdata(mut self, local_appdata: impl Into<PathBuf>) -> Self {
        self.local_appdata = Some(local_appdata.into());
        self
    }
}

/// Full discovery report, including partial failures that later UI code can show safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryReport {
    /// Fallout 4 installation state, or a recoverable reference-compatible error.
    pub game: Result<Fallout4Installation, DiscoveryError>,
    /// Detected/parsed mod-manager state, or a manager-specific typed error.
    pub mod_manager: Result<Option<DiscoveredModManager>, ModManagerDiscoveryError>,
    /// Fakeable PC specs/system metadata for the Overview tab.
    pub system_metadata: Result<SystemMetadata, PlatformError>,
    /// Ordered Fallout 4 path attempts in the locked reference order.
    pub attempts: Vec<DiscoveryAttempt>,
    /// Ordered manager-discovery steps, useful for debugging MO2 portable/instance selection.
    pub manager_steps: Vec<ModManagerDiscoveryStep>,
}

/// Supported manager discovery results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveredModManager {
    /// Parsed Mod Organizer 2 configuration and executable identity.
    ModOrganizer(Box<Mo2Configuration>),
    /// Identity-only Vortex context. No staging/config parsing is performed.
    Vortex(VortexContext),
}

impl DiscoveredModManager {
    /// Returns the exact reference display name for the detected manager.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ModOrganizer(configuration) => configuration.context.manager.display_name(),
            Self::Vortex(context) => context.manager.display_name(),
        }
    }

    /// Returns the executable path that identified the manager.
    pub fn executable_path(&self) -> &Path {
        match self {
            Self::ModOrganizer(configuration) => &configuration.context.manager.executable_path,
            Self::Vortex(context) => &context.manager.executable_path,
        }
    }

    /// Returns the parsed/fallback manager executable version.
    pub fn version(&self) -> SemanticVersion {
        match self {
            Self::ModOrganizer(configuration) => configuration.context.manager.version,
            Self::Vortex(context) => context.manager.version,
        }
    }

    fn manager_game_path(&self) -> Option<PathBuf> {
        match self {
            Self::ModOrganizer(configuration) => configuration.context.game_path.clone(),
            Self::Vortex(_) => None,
        }
    }
}

/// Manager-specific discovery failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModManagerDiscoveryError {
    /// Process-table inspection failed before a manager could be classified.
    ProcessInspection(PlatformError),
    /// A matching manager process did not expose an executable path.
    MissingExecutablePath {
        /// Manager kind inferred from the process name.
        kind: ModManagerKind,
        /// Process id that matched the manager executable name.
        pid: u32,
    },
    /// MO2 was detected but its configuration was unavailable or invalid.
    ModOrganizer(Mo2ParseError),
}

impl ModManagerDiscoveryError {
    /// Returns safe user-facing text for this manager failure.
    pub fn user_message(&self) -> String {
        match self {
            Self::ProcessInspection(error) => error.user_message().to_owned(),
            Self::MissingExecutablePath { kind, .. } => format!(
                "{} was detected but its executable path could not be read.",
                kind.display_name()
            ),
            Self::ModOrganizer(error) => error.user_message(),
        }
    }

    /// Returns true when game discovery should stop rather than silently falling through.
    pub const fn blocks_game_discovery(&self) -> bool {
        matches!(
            self,
            Self::MissingExecutablePath { .. } | Self::ModOrganizer(_)
        )
    }

    /// Returns whether the failure should be handled without panicking.
    pub const fn is_recoverable(&self) -> bool {
        true
    }
}

impl fmt::Display for ModManagerDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

impl std::error::Error for ModManagerDiscoveryError {}

/// Source of a Fallout 4 game-path candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoverySource {
    /// `gamePath` parsed from a running supported manager.
    RunningManagerGamePath,
    /// Process current working directory.
    CurrentWorkingDirectory,
    /// Bethesda registry installed path.
    BethesdaRegistry,
    /// GOG registry installed path.
    GogRegistry,
}

impl DiscoverySource {
    /// Returns a stable, queryable label for tracing and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::RunningManagerGamePath => "running-manager-game-path",
            Self::CurrentWorkingDirectory => "current-working-directory",
            Self::BethesdaRegistry => "bethesda-registry",
            Self::GogRegistry => "gog-registry",
        }
    }
}

/// Outcome for one Fallout 4 path attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryAttemptOutcome {
    /// A source had no candidate to evaluate.
    NoCandidate,
    /// Candidate existed but did not identify a valid Fallout 4 directory.
    InvalidCandidate { reason: String },
    /// A platform adapter failed while checking this source.
    AdapterError(PlatformError),
    /// Candidate produced a valid installation state.
    Accepted,
}

/// One ordered game-path discovery attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryAttempt {
    /// Candidate source checked by this attempt.
    pub source: DiscoverySource,
    /// Candidate path, when the source supplied one.
    pub candidate: Option<PathBuf>,
    /// Result of checking the candidate.
    pub outcome: DiscoveryAttemptOutcome,
}

impl DiscoveryAttempt {
    fn no_candidate(source: DiscoverySource) -> Self {
        Self {
            source,
            candidate: None,
            outcome: DiscoveryAttemptOutcome::NoCandidate,
        }
    }

    fn invalid(source: DiscoverySource, candidate: PathBuf, reason: impl Into<String>) -> Self {
        Self {
            source,
            candidate: Some(candidate),
            outcome: DiscoveryAttemptOutcome::InvalidCandidate {
                reason: reason.into(),
            },
        }
    }

    fn adapter_error(
        source: DiscoverySource,
        candidate: Option<PathBuf>,
        error: PlatformError,
    ) -> Self {
        Self {
            source,
            candidate,
            outcome: DiscoveryAttemptOutcome::AdapterError(error),
        }
    }

    fn accepted(source: DiscoverySource, candidate: PathBuf) -> Self {
        Self {
            source,
            candidate: Some(candidate),
            outcome: DiscoveryAttemptOutcome::Accepted,
        }
    }
}

/// MO2/manager discovery stages in the order they were checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModManagerDiscoveryStage {
    /// Process parent-chain inspection for MO2/Vortex.
    ProcessAncestry,
    /// Adjacent `ModOrganizer.ini` in the manager executable directory.
    PortableIni,
    /// Adjacent `portable.txt` in the manager executable directory.
    PortableMarker,
    /// HKCU `CurrentInstance` registry lookup.
    CurrentInstanceRegistry,
    /// `LOCALAPPDATA/ModOrganizer/<CurrentInstance>/ModOrganizer.ini` lookup.
    LocalAppDataInstanceIni,
    /// Final adjacent portable INI fallback used by the reference app.
    FallbackPortableIni,
    /// Vortex identity-only detection.
    VortexIdentity,
}

/// Outcome for one manager discovery step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModManagerDiscoveryStepOutcome {
    /// Step found the target it was checking.
    Found,
    /// Step checked but the target was absent.
    Missing,
    /// Step was intentionally skipped because an earlier input was absent.
    Skipped,
    /// Step hit an adapter or parse error.
    Error,
}

/// One ordered manager discovery step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModManagerDiscoveryStep {
    /// Discovery stage that ran.
    pub stage: ModManagerDiscoveryStage,
    /// Human-readable target, such as a path, registry value, or process chain.
    pub target: String,
    /// Step result.
    pub outcome: ModManagerDiscoveryStepOutcome,
}

impl ModManagerDiscoveryStep {
    fn new(
        stage: ModManagerDiscoveryStage,
        target: impl Into<String>,
        outcome: ModManagerDiscoveryStepOutcome,
    ) -> Self {
        Self {
            stage,
            target: target.into(),
            outcome,
        }
    }
}

/// Discovery orchestration over fakeable platform adapters.
pub struct DiscoveryService<'a, F, R, P>
where
    F: Filesystem + ?Sized,
    R: RegistryReader + ?Sized,
    P: ProcessInspector + ?Sized,
{
    filesystem: &'a F,
    registry: &'a R,
    process: &'a P,
}

impl<'a, F, R, P> DiscoveryService<'a, F, R, P>
where
    F: Filesystem + ?Sized,
    R: RegistryReader + ?Sized,
    P: ProcessInspector + ?Sized,
{
    /// Creates a discovery service from injected platform adapters.
    pub const fn new(filesystem: &'a F, registry: &'a R, process: &'a P) -> Self {
        Self {
            filesystem,
            registry,
            process,
        }
    }

    /// Runs manager, game-path, and system metadata discovery without UI prompts.
    pub fn discover(&self, request: &DiscoveryRequest) -> DiscoveryReport {
        let system_metadata = self.process.system_metadata().inspect_err(|error| {
            tracing::warn!(
                event = "discovery-system-metadata-error",
                operation = %error.operation,
                kind = ?error.kind,
                target = %error.target,
                "System metadata discovery failed"
            );
        });

        let mut manager_steps = Vec::new();
        let mod_manager = self.discover_mod_manager(request, &mut manager_steps);
        let manager_game_path = match &mod_manager {
            Ok(Some(manager)) => manager.manager_game_path(),
            Ok(None) => None,
            Err(error) if error.blocks_game_discovery() => {
                tracing::warn!(
                    event = "discovery-manager-blocked-game-discovery",
                    error = %error,
                    "Manager-specific discovery failure blocked fallback probing"
                );
                let attempts = vec![DiscoveryAttempt::adapter_error(
                    DiscoverySource::RunningManagerGamePath,
                    None,
                    PlatformError::command_failed(
                        PlatformOperation::ReadFile,
                        "mod-manager discovery",
                        error.to_string(),
                    ),
                )];
                return DiscoveryReport {
                    game: Err(DiscoveryError::fallout4_not_found(Some(error.to_string()))),
                    mod_manager,
                    system_metadata,
                    attempts,
                    manager_steps,
                };
            }
            Err(error) => {
                tracing::warn!(
                    event = "discovery-manager-nonblocking-error",
                    error = %error,
                    "Manager discovery failed; continuing fallback game-path probing"
                );
                None
            }
        };

        let (game, attempts) = self.discover_game(manager_game_path, request);

        DiscoveryReport {
            game,
            mod_manager,
            system_metadata,
            attempts,
            manager_steps,
        }
    }

    fn discover_mod_manager(
        &self,
        request: &DiscoveryRequest,
        steps: &mut Vec<ModManagerDiscoveryStep>,
    ) -> Result<Option<DiscoveredModManager>, ModManagerDiscoveryError> {
        let Some(manager) = self.detect_running_manager(request, steps)? else {
            return Ok(None);
        };

        match manager.kind {
            ModManagerKind::ModOrganizer => self
                .discover_mod_organizer(manager, request, steps)
                .map(|configuration| {
                    Some(DiscoveredModManager::ModOrganizer(Box::new(configuration)))
                })
                .map_err(ModManagerDiscoveryError::ModOrganizer),
            ModManagerKind::Vortex => {
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::VortexIdentity,
                    manager.executable_path.display().to_string(),
                    ModManagerDiscoveryStepOutcome::Found,
                ));
                Ok(Some(DiscoveredModManager::Vortex(VortexContext {
                    manager,
                })))
            }
        }
    }

    fn detect_running_manager(
        &self,
        request: &DiscoveryRequest,
        steps: &mut Vec<ModManagerDiscoveryStep>,
    ) -> Result<Option<DetectedModManager>, ModManagerDiscoveryError> {
        let Some(current_pid) = request.current_process_id else {
            steps.push(ModManagerDiscoveryStep::new(
                ModManagerDiscoveryStage::ProcessAncestry,
                "current process id unavailable",
                ModManagerDiscoveryStepOutcome::Skipped,
            ));
            tracing::debug!(
                event = "mod-manager-detection-skipped",
                reason = "missing-current-process-id"
            );
            return Ok(None);
        };

        let processes = self.process.list_processes().map_err(|error| {
            steps.push(ModManagerDiscoveryStep::new(
                ModManagerDiscoveryStage::ProcessAncestry,
                "process table",
                ModManagerDiscoveryStepOutcome::Error,
            ));
            ModManagerDiscoveryError::ProcessInspection(error)
        })?;
        let processes_by_pid = processes
            .into_iter()
            .map(|process| (process.pid, process))
            .collect::<BTreeMap<_, _>>();

        let mut maybe_pid = processes_by_pid
            .get(&current_pid)
            .and_then(|process| process.parent_pid);

        for depth in 0..PROCESS_ANCESTOR_LIMIT {
            let Some(pid) = maybe_pid else {
                break;
            };
            let Some(process) = processes_by_pid.get(&pid) else {
                break;
            };
            if let Some(kind) = manager_kind_from_process_name(&process.name) {
                let Some(executable_path) = process.executable_path.clone() else {
                    steps.push(ModManagerDiscoveryStep::new(
                        ModManagerDiscoveryStage::ProcessAncestry,
                        format!("pid {pid} {}", process.name),
                        ModManagerDiscoveryStepOutcome::Error,
                    ));
                    return Err(ModManagerDiscoveryError::MissingExecutablePath { kind, pid });
                };
                let version = self.manager_version_or_zero(&executable_path);
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::ProcessAncestry,
                    format!("pid {pid} {}", executable_path.display()),
                    ModManagerDiscoveryStepOutcome::Found,
                ));
                tracing::info!(
                    event = "mod-manager-detected",
                    manager = kind.display_name(),
                    executable_path = %executable_path.display(),
                    version = %version,
                    depth,
                    "Detected supported mod manager in process ancestry"
                );
                return Ok(Some(DetectedModManager::new(
                    kind,
                    executable_path,
                    version,
                )));
            }
            maybe_pid = process.parent_pid;
        }

        steps.push(ModManagerDiscoveryStep::new(
            ModManagerDiscoveryStage::ProcessAncestry,
            format!("parent chain for pid {current_pid}"),
            ModManagerDiscoveryStepOutcome::Missing,
        ));
        Ok(None)
    }

    fn manager_version_or_zero(&self, executable_path: &Path) -> SemanticVersion {
        match self.process.file_version(executable_path) {
            Ok(Some(version)) => version.semantic,
            Ok(None) => SemanticVersion::zero(),
            Err(error) => {
                tracing::warn!(
                    event = "manager-version-metadata-error",
                    operation = %error.operation,
                    kind = ?error.kind,
                    target = %error.target,
                    "Manager version metadata unavailable; using 0.0.0 fallback"
                );
                SemanticVersion::zero()
            }
        }
    }

    fn discover_mod_organizer(
        &self,
        manager: DetectedModManager,
        request: &DiscoveryRequest,
        steps: &mut Vec<ModManagerDiscoveryStep>,
    ) -> Result<Mo2Configuration, Mo2ParseError> {
        let manager_directory = manager
            .executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let portable_ini_path = manager_directory.join("ModOrganizer.ini");
        let portable_ini_exists = self.mo2_file_exists(
            &portable_ini_path,
            steps,
            ModManagerDiscoveryStage::PortableIni,
        )?;
        let portable_txt_path = manager_directory.join("portable.txt");
        let portable_txt_exists = self.mo2_file_exists(
            &portable_txt_path,
            steps,
            ModManagerDiscoveryStage::PortableMarker,
        )?;

        if portable_txt_exists {
            if !portable_ini_exists {
                return Err(Mo2ParseError::portable_marker_without_ini());
            }
            return self.parse_mo2_ini(manager, &portable_ini_path, Some(portable_txt_path), true);
        }

        if let Some(current_instance) = self.read_mo2_current_instance(steps) {
            if let Some(local_appdata) = &request.local_appdata {
                let instance_ini_path = local_appdata
                    .join("ModOrganizer")
                    .join(current_instance)
                    .join("ModOrganizer.ini");
                let instance_ini_exists = self.mo2_file_exists(
                    &instance_ini_path,
                    steps,
                    ModManagerDiscoveryStage::LocalAppDataInstanceIni,
                )?;
                if instance_ini_exists {
                    let configuration =
                        self.parse_mo2_ini(manager.clone(), &instance_ini_path, None, false)?;
                    if configuration.context.game_path.is_some() {
                        return Ok(configuration);
                    }
                }
            } else {
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::LocalAppDataInstanceIni,
                    "LOCALAPPDATA unavailable",
                    ModManagerDiscoveryStepOutcome::Skipped,
                ));
            }
        }

        if !portable_ini_exists {
            steps.push(ModManagerDiscoveryStep::new(
                ModManagerDiscoveryStage::FallbackPortableIni,
                portable_ini_path.display().to_string(),
                ModManagerDiscoveryStepOutcome::Missing,
            ));
            return Err(Mo2ParseError::missing_mod_organizer_ini());
        }

        steps.push(ModManagerDiscoveryStep::new(
            ModManagerDiscoveryStage::FallbackPortableIni,
            portable_ini_path.display().to_string(),
            ModManagerDiscoveryStepOutcome::Found,
        ));
        self.parse_mo2_ini(manager, &portable_ini_path, None, true)
    }

    fn mo2_file_exists(
        &self,
        path: &Path,
        steps: &mut Vec<ModManagerDiscoveryStep>,
        stage: ModManagerDiscoveryStage,
    ) -> Result<bool, Mo2ParseError> {
        match self.filesystem.is_file(path) {
            Ok(exists) => {
                steps.push(ModManagerDiscoveryStep::new(
                    stage,
                    path.display().to_string(),
                    if exists {
                        ModManagerDiscoveryStepOutcome::Found
                    } else {
                        ModManagerDiscoveryStepOutcome::Missing
                    },
                ));
                Ok(exists)
            }
            Err(error) => {
                steps.push(ModManagerDiscoveryStep::new(
                    stage,
                    path.display().to_string(),
                    ModManagerDiscoveryStepOutcome::Error,
                ));
                Err(mo2_missing_ini_with_diagnostic(error))
            }
        }
    }

    fn read_mo2_current_instance(
        &self,
        steps: &mut Vec<ModManagerDiscoveryStep>,
    ) -> Option<String> {
        let request = RegistryValueRequest::new(
            RegistryHive::CurrentUser,
            MO2_REGISTRY_SUBKEY,
            MO2_CURRENT_INSTANCE_VALUE,
        );
        match self.registry.read_string_value(&request) {
            Ok(Some(value)) if !value.is_empty() => {
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::CurrentInstanceRegistry,
                    request.target(),
                    ModManagerDiscoveryStepOutcome::Found,
                ));
                Some(value)
            }
            Ok(_) => {
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::CurrentInstanceRegistry,
                    request.target(),
                    ModManagerDiscoveryStepOutcome::Missing,
                ));
                None
            }
            Err(error) => {
                tracing::warn!(
                    event = "mo2-current-instance-registry-error",
                    operation = %error.operation,
                    kind = ?error.kind,
                    target = %error.target,
                    "MO2 CurrentInstance registry lookup failed; falling back to portable INI"
                );
                steps.push(ModManagerDiscoveryStep::new(
                    ModManagerDiscoveryStage::CurrentInstanceRegistry,
                    request.target(),
                    ModManagerDiscoveryStepOutcome::Error,
                ));
                None
            }
        }
    }

    fn parse_mo2_ini(
        &self,
        manager: DetectedModManager,
        ini_path: &Path,
        portable_txt_path: Option<PathBuf>,
        portable: bool,
    ) -> Result<Mo2Configuration, Mo2ParseError> {
        let text = self
            .filesystem
            .read_to_string(ini_path)
            .map_err(mo2_missing_ini_with_diagnostic)?;
        let values = parse_mo2_ini_values(&text);
        let game_name = values
            .general
            .get("gameName")
            .map(String::as_str)
            .unwrap_or("Fallout 4");
        if game_name != "Fallout 4" {
            return Err(Mo2ParseError::unsupported_game_name(game_name, ini_path));
        }

        let Some(selected_profile) = values.general.get("selected_profile").cloned() else {
            return Err(Mo2ParseError::missing_selected_profile());
        };

        let base_directory = values
            .settings
            .get("base_directory")
            .map(|value| PathBuf::from(unwrap_mo2_value(value)))
            .unwrap_or_else(|| ini_path.parent().map(Path::to_path_buf).unwrap_or_default());
        let cache_directory = resolve_mo2_path(
            values.settings.get("cache_directory"),
            &base_directory,
            "%BASE_DIR%/webcache",
        );
        let download_directory = resolve_mo2_path(
            values.settings.get("download_directory"),
            &base_directory,
            "%BASE_DIR%/downloads",
        );
        let mod_directory = resolve_mo2_path(
            values.settings.get("mod_directory"),
            &base_directory,
            "%BASE_DIR%/mods",
        );
        let overwrite_directory = resolve_mo2_path(
            values.settings.get("overwrite_directory"),
            &base_directory,
            "%BASE_DIR%/overwrite",
        );
        let profiles_directory = resolve_mo2_path(
            values.settings.get("profiles_directory"),
            &base_directory,
            "%BASE_DIR%/profiles",
        );

        if path_is_empty(&mod_directory)
            || path_is_empty(&profiles_directory)
            || path_is_empty(&overwrite_directory)
        {
            return Err(Mo2ParseError::missing_stage_settings(
                Some(mod_directory),
                Some(profiles_directory),
                Some(selected_profile),
                Some(overwrite_directory),
            ));
        }

        let directories = ModOrganizerDirectories::new(
            base_directory,
            cache_directory,
            download_directory,
            mod_directory,
            overwrite_directory,
            profiles_directory,
        );
        let profile_local_inis = values
            .settings
            .get("profile_local_inis")
            .is_some_and(|value| parse_mo2_bool(value));
        let profile_local_saves = values
            .settings
            .get("profile_local_saves")
            .is_some_and(|value| parse_mo2_bool(value));
        let skip_file_suffixes = values
            .settings
            .get("skip_file_suffixes")
            .map(|value| parse_csv_list(&unwrap_mo2_value(value)))
            .unwrap_or_else(|| vec![".mohidden".to_owned()]);
        let skip_directories = values
            .settings
            .get("skip_directories")
            .map(|value| parse_csv_list(&unwrap_mo2_value(value)))
            .unwrap_or_default();
        let skip_rules = ModOrganizerSkipRules::new(skip_file_suffixes, skip_directories);
        let game_path = values
            .general
            .get("gamePath")
            .map(|value| resolve_mo2_path(Some(value), &directories.base_directory, ""));

        let mut context = ModOrganizerContext::new(manager, selected_profile, directories)
            .with_source_paths(Some(ini_path.to_path_buf()), portable_txt_path, portable)
            .with_profile_local_flags(profile_local_inis, profile_local_saves)
            .with_skip_rules(skip_rules);
        if let Some(game_path) = game_path {
            context = context.with_game_path(game_path);
        }

        let mut configuration = Mo2Configuration::new(context);
        for value in values.custom_executable_binaries {
            self.add_mo2_custom_executable(&mut configuration, &unwrap_mo2_value(&value));
        }

        tracing::info!(
            event = "mo2-configuration-parsed",
            ini_path = %ini_path.display(),
            portable,
            has_game_path = configuration.context.game_path.is_some(),
            selected_profile = %configuration.context.selected_profile,
            "Parsed Mod Organizer configuration"
        );
        Ok(configuration)
    }

    fn add_mo2_custom_executable(&self, configuration: &mut Mo2Configuration, value: &str) {
        let value_lower = value.to_ascii_lowercase();
        for tool in [ModOrganizerTool::XEdit, ModOrganizerTool::BSArch] {
            if tool
                .executable_suffixes()
                .iter()
                .any(|suffix| value_lower.ends_with(suffix))
            {
                let executable_path = PathBuf::from(value);
                if self.lenient_is_file(&executable_path) {
                    configuration
                        .executables
                        .entry(tool)
                        .or_default()
                        .insert(executable_path.clone());
                }
                if tool == ModOrganizerTool::XEdit {
                    let bsarch_path = executable_path.with_file_name("BSArch.exe");
                    if self.lenient_is_file(&bsarch_path) {
                        configuration
                            .executables
                            .entry(ModOrganizerTool::BSArch)
                            .or_default()
                            .insert(bsarch_path);
                    }
                }
                break;
            }
        }
    }

    fn lenient_is_file(&self, path: &Path) -> bool {
        match self.filesystem.is_file(path) {
            Ok(is_file) => is_file,
            Err(error) => {
                tracing::warn!(
                    event = "filesystem-file-check-error",
                    operation = %error.operation,
                    kind = ?error.kind,
                    target = %error.target,
                    "Treating optional file check as absent"
                );
                false
            }
        }
    }

    fn discover_game(
        &self,
        manager_game_path: Option<PathBuf>,
        request: &DiscoveryRequest,
    ) -> (
        Result<Fallout4Installation, DiscoveryError>,
        Vec<DiscoveryAttempt>,
    ) {
        let mut attempts = Vec::new();
        let mut diagnostics = Vec::new();

        if let Some(candidate) = manager_game_path {
            match self.evaluate_game_candidate(DiscoverySource::RunningManagerGamePath, candidate) {
                CandidateEvaluation::Accepted(installation, attempt) => {
                    attempts.push(attempt);
                    return (Ok(*installation), attempts);
                }
                CandidateEvaluation::Rejected(attempt) => attempts.push(attempt),
                CandidateEvaluation::AdapterError(attempt, diagnostic) => {
                    diagnostics.push(diagnostic);
                    attempts.push(attempt);
                }
            }
        } else {
            attempts.push(DiscoveryAttempt::no_candidate(
                DiscoverySource::RunningManagerGamePath,
            ));
        }

        match self.evaluate_game_candidate(
            DiscoverySource::CurrentWorkingDirectory,
            request.current_working_directory.clone(),
        ) {
            CandidateEvaluation::Accepted(installation, attempt) => {
                attempts.push(attempt);
                return (Ok(*installation), attempts);
            }
            CandidateEvaluation::Rejected(attempt) => attempts.push(attempt),
            CandidateEvaluation::AdapterError(attempt, diagnostic) => {
                diagnostics.push(diagnostic);
                attempts.push(attempt);
            }
        }

        if let Some(outcome) = self.evaluate_registry_candidate(
            DiscoverySource::BethesdaRegistry,
            RegistryValueRequest::new(
                RegistryHive::LocalMachine,
                BETHESDA_FALLOUT4_REGISTRY_SUBKEY,
                BETHESDA_FALLOUT4_REGISTRY_VALUE,
            ),
            &mut diagnostics,
        ) {
            match outcome {
                RegistryCandidateOutcome::Accepted(installation, attempt) => {
                    attempts.push(attempt);
                    return (Ok(*installation), attempts);
                }
                RegistryCandidateOutcome::Rejected { attempt, error } => {
                    attempts.push(attempt);
                    return (Err(error), attempts);
                }
                RegistryCandidateOutcome::NoCandidate(attempt)
                | RegistryCandidateOutcome::AdapterError(attempt) => attempts.push(attempt),
            }
        }

        if let Some(outcome) = self.evaluate_registry_candidate(
            DiscoverySource::GogRegistry,
            RegistryValueRequest::new(
                RegistryHive::LocalMachine,
                GOG_FALLOUT4_REGISTRY_SUBKEY,
                GOG_FALLOUT4_REGISTRY_VALUE,
            ),
            &mut diagnostics,
        ) {
            match outcome {
                RegistryCandidateOutcome::Accepted(installation, attempt) => {
                    attempts.push(attempt);
                    return (Ok(*installation), attempts);
                }
                RegistryCandidateOutcome::Rejected { attempt, error } => {
                    attempts.push(attempt);
                    return (Err(error), attempts);
                }
                RegistryCandidateOutcome::NoCandidate(attempt)
                | RegistryCandidateOutcome::AdapterError(attempt) => attempts.push(attempt),
            }
        }

        let diagnostic = (!diagnostics.is_empty()).then(|| diagnostics.join("; "));
        (
            Err(DiscoveryError::fallout4_not_found(diagnostic)),
            attempts,
        )
    }

    fn evaluate_registry_candidate(
        &self,
        source: DiscoverySource,
        request: RegistryValueRequest,
        diagnostics: &mut Vec<String>,
    ) -> Option<RegistryCandidateOutcome> {
        match self.registry.read_string_value(&request) {
            Ok(Some(value)) if !value.is_empty() => {
                let candidate = PathBuf::from(value);
                Some(
                    match self.evaluate_game_candidate(source, candidate.clone()) {
                        CandidateEvaluation::Accepted(installation, attempt) => {
                            RegistryCandidateOutcome::Accepted(installation, attempt)
                        }
                        CandidateEvaluation::Rejected(attempt) => {
                            RegistryCandidateOutcome::Rejected {
                                attempt,
                                error: DiscoveryError::invalid_registry_path(candidate),
                            }
                        }
                        CandidateEvaluation::AdapterError(attempt, diagnostic) => {
                            diagnostics.push(diagnostic);
                            RegistryCandidateOutcome::Rejected {
                                attempt,
                                error: DiscoveryError::invalid_registry_path(candidate),
                            }
                        }
                    },
                )
            }
            Ok(_) => Some(RegistryCandidateOutcome::NoCandidate(
                DiscoveryAttempt::no_candidate(source),
            )),
            Err(error) => {
                tracing::warn!(
                    event = "game-registry-read-error",
                    source = source.label(),
                    operation = %error.operation,
                    kind = ?error.kind,
                    target = %error.target,
                    "Registry game-path lookup failed; treating source as absent"
                );
                diagnostics.push(format!(
                    "{} registry lookup failed: {}",
                    source.label(),
                    error
                ));
                Some(RegistryCandidateOutcome::AdapterError(
                    DiscoveryAttempt::adapter_error(source, None, error),
                ))
            }
        }
    }

    fn evaluate_game_candidate(
        &self,
        source: DiscoverySource,
        candidate: PathBuf,
    ) -> CandidateEvaluation {
        tracing::debug!(
            event = "fallout4-discovery-candidate",
            source = source.label(),
            candidate = %candidate.display(),
            "Evaluating Fallout 4 path candidate"
        );
        let normalized = match self.normalize_game_candidate(&candidate) {
            Ok(path) => path,
            Err(error) => {
                let diagnostic = format!(
                    "{} candidate metadata failed for {}: {}",
                    source.label(),
                    candidate.display(),
                    error
                );
                return CandidateEvaluation::AdapterError(
                    DiscoveryAttempt::adapter_error(source, Some(candidate), error),
                    diagnostic,
                );
            }
        };

        match self.is_fo4_dir(&normalized) {
            Ok(true) => {
                let installation = self.installation_for_game_path(normalized.clone());
                tracing::info!(
                    event = "fallout4-discovery-found",
                    source = source.label(),
                    game_path = %installation.game_path.display(),
                    has_data_path = installation.data_path.is_some(),
                    has_f4se_plugins_path = installation.f4se_plugins_path.is_some(),
                    "Accepted Fallout 4 installation candidate"
                );
                CandidateEvaluation::Accepted(
                    Box::new(installation),
                    DiscoveryAttempt::accepted(source, normalized),
                )
            }
            Ok(false) => CandidateEvaluation::Rejected(DiscoveryAttempt::invalid(
                source,
                normalized,
                "candidate is not a Fallout 4 directory",
            )),
            Err(error) => {
                let diagnostic = format!(
                    "{} candidate validation failed for {}: {}",
                    source.label(),
                    normalized.display(),
                    error
                );
                CandidateEvaluation::AdapterError(
                    DiscoveryAttempt::adapter_error(source, Some(normalized), error),
                    diagnostic,
                )
            }
        }
    }

    fn normalize_game_candidate(&self, candidate: &Path) -> Result<PathBuf, PlatformError> {
        let normalized = normalize_lexically(candidate);
        if self.filesystem.is_file(&normalized)? {
            Ok(normalized
                .parent()
                .map(normalize_lexically)
                .unwrap_or_else(|| PathBuf::from(".")))
        } else {
            Ok(normalized)
        }
    }

    fn is_fo4_dir(&self, path: &Path) -> Result<bool, PlatformError> {
        if !self.filesystem.is_dir(path)? {
            return Ok(false);
        }
        self.filesystem.is_file(&path.join(FALLOUT4_EXECUTABLE))
    }

    fn installation_for_game_path(&self, game_path: PathBuf) -> Fallout4Installation {
        let data_candidate = game_path.join("Data");
        let data_path = self.optional_directory(&data_candidate);
        let f4se_plugins_path = data_path
            .as_ref()
            .and_then(|data_path| self.optional_directory(&data_path.join("F4SE").join("Plugins")));

        Fallout4Installation::with_optional_paths(game_path, data_path, f4se_plugins_path)
    }

    fn optional_directory(&self, path: &Path) -> Option<PathBuf> {
        match self.filesystem.is_dir(path) {
            Ok(true) => Some(path.to_path_buf()),
            Ok(false) => None,
            Err(error) => {
                tracing::warn!(
                    event = "derived-path-check-error",
                    operation = %error.operation,
                    kind = ?error.kind,
                    target = %error.target,
                    "Optional derived path could not be checked; representing it as missing"
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CandidateEvaluation {
    Accepted(Box<Fallout4Installation>, DiscoveryAttempt),
    Rejected(DiscoveryAttempt),
    AdapterError(DiscoveryAttempt, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RegistryCandidateOutcome {
    Accepted(Box<Fallout4Installation>, DiscoveryAttempt),
    Rejected {
        attempt: DiscoveryAttempt,
        error: DiscoveryError,
    },
    NoCandidate(DiscoveryAttempt),
    AdapterError(DiscoveryAttempt),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RawMo2IniValues {
    general: BTreeMap<String, String>,
    settings: BTreeMap<String, String>,
    custom_executable_binaries: Vec<String>,
}

fn parse_mo2_ini_values(text: &str) -> RawMo2IniValues {
    let mut values = RawMo2IniValues::default();
    let mut section: Option<&str> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            section = match line {
                "[General]" => Some("General"),
                "[Settings]" => Some("Settings"),
                "[customExecutables]" => Some("customExecutables"),
                _ => None,
            };
            continue;
        }
        let Some(active_section) = section else {
            continue;
        };
        let Some((setting, value)) = line.split_once('=') else {
            continue;
        };
        match active_section {
            "General" if matches!(setting, "gameName" | "gamePath" | "selected_profile") => {
                values
                    .general
                    .insert(setting.to_owned(), unwrap_mo2_value(value));
            }
            "Settings" if is_supported_mo2_setting(setting) => {
                values
                    .settings
                    .insert(setting.to_owned(), unwrap_mo2_value(value));
            }
            "customExecutables" if setting.ends_with("binary") => {
                values.custom_executable_binaries.push(value.to_owned());
            }
            _ => {}
        }
    }

    values
}

fn is_supported_mo2_setting(setting: &str) -> bool {
    matches!(
        setting,
        "base_directory"
            | "cache_directory"
            | "download_directory"
            | "mod_directory"
            | "overwrite_directory"
            | "profile_local_inis"
            | "profile_local_saves"
            | "profiles_directory"
            | "skip_file_suffixes"
            | "skip_directories"
    )
}

fn unwrap_mo2_value(value: &str) -> String {
    value
        .strip_prefix("@ByteArray(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value)
        .to_owned()
}

fn resolve_mo2_path(value: Option<&String>, base_directory: &Path, default: &str) -> PathBuf {
    let value = value.map(String::as_str).unwrap_or(default);
    let value = unwrap_mo2_value(value);
    if value.contains("%BASE_DIR%") {
        let relative = value
            .replace("%BASE_DIR%", "")
            .trim_start_matches(['/', '\\'])
            .to_owned();
        if relative.is_empty() {
            base_directory.to_path_buf()
        } else {
            base_directory.join(relative)
        }
    } else {
        PathBuf::from(value)
    }
}

fn parse_mo2_bool(value: &str) -> bool {
    matches!(
        unwrap_mo2_value(value).to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

fn parse_csv_list(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut after_delimiter = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            after_delimiter = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == ',' {
            push_csv_item(&mut items, &current);
            current.clear();
            after_delimiter = true;
            continue;
        }
        if after_delimiter && character == ' ' {
            continue;
        }
        after_delimiter = false;
        current.push(character);
    }
    if escaped {
        current.push('\\');
    }
    push_csv_item(&mut items, &current);
    items
}

fn push_csv_item(items: &mut Vec<String>, item: &str) {
    let item = item.trim();
    if !item.is_empty() {
        items.push(item.to_owned());
    }
}

fn path_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

fn manager_kind_from_process_name(name: &str) -> Option<ModManagerKind> {
    if name.eq_ignore_ascii_case(ModManagerKind::ModOrganizer.executable_name()) {
        Some(ModManagerKind::ModOrganizer)
    } else if name.eq_ignore_ascii_case(ModManagerKind::Vortex.executable_name()) {
        Some(ModManagerKind::Vortex)
    } else {
        None
    }
}

fn mo2_missing_ini_with_diagnostic(error: PlatformError) -> Mo2ParseError {
    Mo2ParseError {
        kind: Mo2ParseErrorKind::MissingModOrganizerIni,
        diagnostic: Some(error.to_string()),
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::{cell::RefCell, collections::BTreeMap};

    use crate::platform::{
        PlatformErrorKind,
        filesystem::{FileMetadata, FileType},
        process::{ProcessInfo, VersionMetadata},
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
    }

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            self.nodes.insert(path.into(), FakeNode::Directory);
            self
        }

        fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl AsRef<[u8]>) -> Self {
            self.nodes
                .insert(path.into(), FakeNode::File(bytes.as_ref().to_vec()));
            self
        }

        fn with_game_dir(self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.with_dir(path.clone())
                .with_file(path.join(FALLOUT4_EXECUTABLE), b"exe")
        }

        fn node(
            &self,
            path: &Path,
            operation: PlatformOperation,
        ) -> Result<&FakeNode, PlatformError> {
            self.nodes.get(path).ok_or_else(|| {
                PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )
            })
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> Result<FileMetadata, PlatformError> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata::new(FileType::File, bytes.len() as u64)),
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
            }
        }

        fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, PlatformError> {
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.clone()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
            }
        }

        fn read_to_string(&self, path: &Path) -> Result<String, PlatformError> {
            String::from_utf8(self.read_bytes(path)?).map_err(|error| {
                PlatformError::parse_error(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    error.to_string(),
                )
            })
        }

        fn read_dir(
            &self,
            path: &Path,
        ) -> Result<Vec<crate::platform::filesystem::DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(Vec::new())
        }

        fn walk_dir(
            &self,
            path: &Path,
        ) -> Result<Vec<crate::platform::filesystem::DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(Vec::new())
        }
    }

    #[derive(Debug, Default)]
    struct FakeRegistry {
        values: BTreeMap<RegistryValueRequest, Result<Option<String>, PlatformError>>,
        reads: RefCell<Vec<RegistryValueRequest>>,
    }

    impl FakeRegistry {
        fn with_value(mut self, request: RegistryValueRequest, value: Option<&str>) -> Self {
            self.values
                .insert(request, Ok(value.map(std::string::ToString::to_string)));
            self
        }

        fn with_error(mut self, request: RegistryValueRequest) -> Self {
            let target = request.target();
            self.values.insert(
                request,
                Err(PlatformError::new(
                    PlatformOperation::ReadRegistry,
                    target,
                    PlatformErrorKind::PermissionDenied,
                    "Registry access failed.",
                )),
            );
            self
        }
    }

    impl RegistryReader for FakeRegistry {
        fn read_string_value(
            &self,
            request: &RegistryValueRequest,
        ) -> Result<Option<String>, PlatformError> {
            self.reads.borrow_mut().push(request.clone());
            self.values.get(request).cloned().unwrap_or(Ok(None))
        }
    }

    #[derive(Debug, Clone)]
    struct FakeProcessInspector {
        processes: Result<Vec<ProcessInfo>, PlatformError>,
        versions: BTreeMap<PathBuf, Result<Option<VersionMetadata>, PlatformError>>,
        system_metadata: Result<SystemMetadata, PlatformError>,
    }

    impl Default for FakeProcessInspector {
        fn default() -> Self {
            Self {
                processes: Ok(Vec::new()),
                versions: BTreeMap::new(),
                system_metadata: Ok(fake_system_metadata()),
            }
        }
    }

    impl FakeProcessInspector {
        fn with_processes(mut self, processes: Vec<ProcessInfo>) -> Self {
            self.processes = Ok(processes);
            self
        }

        fn with_version(
            mut self,
            path: impl Into<PathBuf>,
            version: Option<SemanticVersion>,
        ) -> Self {
            self.versions.insert(
                path.into(),
                Ok(version.map(|version| VersionMetadata::new(version, Some(version.to_string())))),
            );
            self
        }
    }

    impl ProcessInspector for FakeProcessInspector {
        fn list_processes(&self) -> Result<Vec<ProcessInfo>, PlatformError> {
            self.processes.clone()
        }

        fn file_version(&self, path: &Path) -> Result<Option<VersionMetadata>, PlatformError> {
            self.versions.get(path).cloned().unwrap_or(Ok(None))
        }

        fn system_metadata(&self) -> Result<SystemMetadata, PlatformError> {
            self.system_metadata.clone()
        }
    }

    fn fake_system_metadata() -> SystemMetadata {
        SystemMetadata::new(
            "Windows 11 Pro",
            Some("24H2"),
            "x86_64",
            Some("Fake Ryzen"),
            Some(32 * 1024 * 1024 * 1024),
            Some(16),
        )
    }

    fn service_report(
        filesystem: &FakeFilesystem,
        registry: &FakeRegistry,
        process: &FakeProcessInspector,
        request: DiscoveryRequest,
    ) -> DiscoveryReport {
        DiscoveryService::new(filesystem, registry, process).discover(&request)
    }

    fn bethesda_request() -> RegistryValueRequest {
        RegistryValueRequest::new(
            RegistryHive::LocalMachine,
            BETHESDA_FALLOUT4_REGISTRY_SUBKEY,
            BETHESDA_FALLOUT4_REGISTRY_VALUE,
        )
    }

    fn gog_request() -> RegistryValueRequest {
        RegistryValueRequest::new(
            RegistryHive::LocalMachine,
            GOG_FALLOUT4_REGISTRY_SUBKEY,
            GOG_FALLOUT4_REGISTRY_VALUE,
        )
    }

    fn mo2_current_instance_request() -> RegistryValueRequest {
        RegistryValueRequest::new(
            RegistryHive::CurrentUser,
            MO2_REGISTRY_SUBKEY,
            MO2_CURRENT_INSTANCE_VALUE,
        )
    }

    fn mo2_processes(executable_path: &str) -> Vec<ProcessInfo> {
        vec![
            ProcessInfo::new(100, Some(50), "cmt-rs.exe", Some("C:/Tools/cmt-rs.exe")),
            ProcessInfo::new(50, Some(10), "ModOrganizer.exe", Some(executable_path)),
        ]
    }

    fn vortex_processes(executable_path: &str) -> Vec<ProcessInfo> {
        vec![
            ProcessInfo::new(100, Some(70), "cmt-rs.exe", Some("C:/Tools/cmt-rs.exe")),
            ProcessInfo::new(70, Some(10), "Vortex.exe", Some(executable_path)),
        ]
    }

    fn valid_mo2_ini(game_path: &str) -> String {
        format!(
            "[General]\n\
             gameName=Fallout 4\n\
             gamePath={game_path}\n\
             selected_profile=Default\n\
             [Settings]\n\
             mod_directory=%BASE_DIR%/mods\n\
             overwrite_directory=%BASE_DIR%/overwrite\n\
             profiles_directory=%BASE_DIR%/profiles\n"
        )
    }

    #[test]
    fn manager_game_path_is_checked_before_cwd_and_registry() {
        let filesystem = FakeFilesystem::default()
            .with_file("C:/MO2/ModOrganizer.ini", valid_mo2_ini("C:/Games/Managed"))
            .with_game_dir("C:/Games/Managed")
            .with_game_dir("C:/Games/Cwd")
            .with_game_dir("C:/Games/Registry");
        let registry =
            FakeRegistry::default().with_value(bethesda_request(), Some("C:/Games/Registry"));
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"))
            .with_version(
                "C:/MO2/ModOrganizer.exe",
                Some(SemanticVersion::new(2, 5, 2)),
            );

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd").with_current_process_id(100),
        );

        let installation = report.game.expect("manager game path should win");
        assert_eq!(installation.game_path, PathBuf::from("C:/Games/Managed"));
        assert_eq!(report.attempts.len(), 1);
        assert_eq!(
            report.attempts[0].source,
            DiscoverySource::RunningManagerGamePath
        );
        assert_eq!(
            report.attempts[0].outcome,
            DiscoveryAttemptOutcome::Accepted
        );
        assert_eq!(
            registry.reads.borrow().as_slice(),
            &[mo2_current_instance_request()],
            "game registry should not be queried after manager hit"
        );
        let manager = report
            .mod_manager
            .expect("manager discovery should succeed")
            .expect("manager should be detected");
        assert_eq!(manager.display_name(), "Mod Organizer");
        assert_eq!(manager.version(), SemanticVersion::new(2, 5, 2));
    }

    #[test]
    fn cwd_is_checked_before_bethesda_and_gog_registry_paths() {
        let filesystem = FakeFilesystem::default()
            .with_game_dir("C:/Games/Cwd")
            .with_game_dir("C:/Games/Bethesda")
            .with_game_dir("C:/Games/Gog");
        let registry = FakeRegistry::default()
            .with_value(bethesda_request(), Some("C:/Games/Bethesda"))
            .with_value(gog_request(), Some("C:/Games/Gog"));
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd"),
        );

        assert_eq!(
            report.game.expect("cwd should be accepted").game_path,
            PathBuf::from("C:/Games/Cwd")
        );
        assert_eq!(
            report
                .attempts
                .iter()
                .map(|attempt| attempt.source)
                .collect::<Vec<_>>(),
            vec![
                DiscoverySource::RunningManagerGamePath,
                DiscoverySource::CurrentWorkingDirectory,
            ]
        );
        assert!(
            registry.reads.borrow().is_empty(),
            "registry must not run after cwd hit"
        );
    }

    #[test]
    fn not_found_result_is_recoverable_and_never_requires_file_picker_ui() {
        let filesystem = FakeFilesystem::default();
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing"),
        );

        let error = report.game.expect_err("no candidates should be found");
        assert_eq!(
            error.kind,
            crate::domain::discovery::DiscoveryErrorKind::Fallout4NotFound
        );
        assert_eq!(
            error.user_message(),
            "A Fallout 4 installation could not be found."
        );
        assert!(error.is_recoverable());
        assert!(!error.requires_manual_file_picker());
        assert_eq!(
            report
                .attempts
                .iter()
                .map(|attempt| attempt.source)
                .collect::<Vec<_>>(),
            vec![
                DiscoverySource::RunningManagerGamePath,
                DiscoverySource::CurrentWorkingDirectory,
                DiscoverySource::BethesdaRegistry,
                DiscoverySource::GogRegistry,
            ]
        );
    }

    #[test]
    fn direct_fallout4_exe_candidate_normalizes_to_parent_game_directory() {
        let filesystem = FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4")
            .with_file("C:/Games/Fallout 4/Fallout4.exe", b"exe");
        let registry = FakeRegistry::default()
            .with_value(bethesda_request(), Some("C:/Games/Fallout 4/Fallout4.exe"));
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing"),
        );

        let installation = report
            .game
            .expect("registry exe should normalize to parent");
        assert_eq!(installation.game_path, PathBuf::from("C:/Games/Fallout 4"));
        assert_eq!(
            report.attempts.last().expect("registry attempt").source,
            DiscoverySource::BethesdaRegistry
        );
    }

    #[test]
    fn valid_game_directory_can_return_partial_missing_derived_paths() {
        let filesystem = FakeFilesystem::default().with_game_dir("C:/Games/Fallout 4");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Fallout 4"),
        );

        let installation = report
            .game
            .expect("game executable is enough for install state");
        assert_eq!(installation.game_path, PathBuf::from("C:/Games/Fallout 4"));
        assert_eq!(installation.data_path, None);
        assert_eq!(installation.f4se_plugins_path, None);
    }

    #[test]
    fn valid_game_directory_can_return_data_without_f4se_plugins() {
        let filesystem = FakeFilesystem::default()
            .with_game_dir("C:/Games/Fallout 4")
            .with_dir("C:/Games/Fallout 4/Data");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Fallout 4"),
        );

        let installation = report.game.expect("game should be accepted");
        assert_eq!(
            installation.data_path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4/Data"))
        );
        assert_eq!(installation.f4se_plugins_path, None);
    }

    #[test]
    fn mo2_discovery_parses_paths_flags_skip_rules_and_custom_tools() {
        let mo2_ini = "[General]\n\
                       gameName=Fallout 4\n\
                       gamePath=C:/Games/Fallout 4\n\
                       selected_profile=Survival\n\
                       [Settings]\n\
                       base_directory=C:/MO2Base\n\
                       mod_directory=%BASE_DIR%/staging\n\
                       overwrite_directory=C:/Overwrites\n\
                       profiles_directory=%BASE_DIR%/profiles\n\
                       profile_local_inis=true\n\
                       profile_local_saves=false\n\
                       skip_file_suffixes=.MOHIDDEN, .Bak\n\
                       skip_directories=Cache, Temp\n\
                       [customExecutables]\n\
                       1\\binary=C:/Tools/FO4Edit.exe\n";
        let filesystem = FakeFilesystem::default()
            .with_file("C:/MO2/ModOrganizer.ini", mo2_ini)
            .with_file("C:/Tools/FO4Edit.exe", b"tool")
            .with_file("C:/Tools/BSArch.exe", b"tool")
            .with_game_dir("C:/Games/Fallout 4");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing").with_current_process_id(100),
        );

        let Some(DiscoveredModManager::ModOrganizer(configuration)) =
            report.mod_manager.expect("MO2 should parse")
        else {
            panic!("expected MO2 configuration");
        };
        let context = &configuration.context;
        assert_eq!(
            context.game_path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4"))
        );
        assert_eq!(context.selected_profile, "Survival");
        assert_eq!(context.mod_directory(), Path::new("C:/MO2Base/staging"));
        assert_eq!(context.overwrite_directory(), Path::new("C:/Overwrites"));
        assert_eq!(
            context.profiles_directory(),
            Path::new("C:/MO2Base/profiles")
        );
        assert!(context.profile_local_inis);
        assert!(!context.profile_local_saves);
        assert_eq!(context.skip_rules.file_suffixes, vec![".mohidden", ".bak"]);
        assert_eq!(
            context.skip_rules.directories,
            BTreeSet::from(["cache".to_owned(), "temp".to_owned()])
        );
        assert!(
            configuration.executables[&ModOrganizerTool::XEdit]
                .contains(&PathBuf::from("C:/Tools/FO4Edit.exe"))
        );
        assert!(
            configuration.executables[&ModOrganizerTool::BSArch]
                .contains(&PathBuf::from("C:/Tools/BSArch.exe"))
        );
    }

    #[test]
    fn mo2_instance_discovery_checks_adjacent_files_before_hkcu_current_instance() {
        let filesystem = FakeFilesystem::default()
            .with_file(
                "C:/Users/Example/AppData/Local/ModOrganizer/Fallout4/ModOrganizer.ini",
                valid_mo2_ini("C:/Games/Fallout 4"),
            )
            .with_game_dir("C:/Games/Fallout 4");
        let registry =
            FakeRegistry::default().with_value(mo2_current_instance_request(), Some("Fallout4"));
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing")
                .with_current_process_id(100)
                .with_local_appdata("C:/Users/Example/AppData/Local"),
        );

        assert!(report.game.is_ok());
        let stages = report
            .manager_steps
            .iter()
            .map(|step| step.stage)
            .collect::<Vec<_>>();
        assert_eq!(
            stages,
            vec![
                ModManagerDiscoveryStage::ProcessAncestry,
                ModManagerDiscoveryStage::PortableIni,
                ModManagerDiscoveryStage::PortableMarker,
                ModManagerDiscoveryStage::CurrentInstanceRegistry,
                ModManagerDiscoveryStage::LocalAppDataInstanceIni,
            ]
        );
    }

    #[test]
    fn mo2_portable_marker_without_ini_returns_manager_specific_error_and_does_not_fall_through() {
        let filesystem = FakeFilesystem::default()
            .with_file("C:/MO2/portable.txt", b"portable")
            .with_game_dir("C:/Games/Cwd");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd").with_current_process_id(100),
        );

        let manager_error = report.mod_manager.expect_err("MO2 error should surface");
        assert!(manager_error.is_recoverable());
        assert!(manager_error.blocks_game_discovery());
        assert!(matches!(
            manager_error,
            ModManagerDiscoveryError::ModOrganizer(Mo2ParseError {
                kind: Mo2ParseErrorKind::PortableMarkerWithoutIni,
                ..
            })
        ));
        assert!(report.game.is_err());
        assert_eq!(
            report.attempts.len(),
            1,
            "cwd fallback must not run after blocking MO2 error"
        );
    }

    #[test]
    fn mo2_non_fallout_game_name_returns_typed_manager_error() {
        let filesystem = FakeFilesystem::default().with_file(
            "C:/MO2/ModOrganizer.ini",
            "[General]\n\
             gameName=Skyrim Special Edition\n\
             selected_profile=Default\n",
        );
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing").with_current_process_id(100),
        );

        let manager_error = report
            .mod_manager
            .expect_err("unsupported game should fail");
        assert!(matches!(
            manager_error,
            ModManagerDiscoveryError::ModOrganizer(Mo2ParseError {
                kind: Mo2ParseErrorKind::UnsupportedGameName { .. },
                ..
            })
        ));
        assert_eq!(
            manager_error.user_message(),
            "Only Fallout 4 is supported.\ngameName is 'Skyrim Special Edition' in INI: \nC:/MO2/ModOrganizer.ini"
        );
    }

    #[test]
    fn mo2_missing_selected_profile_returns_typed_manager_error() {
        let filesystem = FakeFilesystem::default().with_file(
            "C:/MO2/ModOrganizer.ini",
            "[General]\n\
             gameName=Fallout 4\n\
             gamePath=C:/Games/Fallout 4\n",
        );
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(mo2_processes("C:/MO2/ModOrganizer.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing").with_current_process_id(100),
        );

        let manager_error = report.mod_manager.expect_err("missing profile should fail");
        assert!(matches!(
            manager_error,
            ModManagerDiscoveryError::ModOrganizer(Mo2ParseError {
                kind: Mo2ParseErrorKind::MissingSelectedProfile,
                ..
            })
        ));
        assert_eq!(
            manager_error.user_message(),
            "Profile is not set in ModOrganizer.ini."
        );
    }

    #[test]
    fn vortex_detection_is_identity_only_and_uses_parsed_version() {
        let filesystem = FakeFilesystem::default().with_game_dir("C:/Games/Cwd");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(vortex_processes("C:/Vortex/Vortex.exe"))
            .with_version("C:/Vortex/Vortex.exe", Some(SemanticVersion::new(1, 12, 3)));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd").with_current_process_id(100),
        );

        let Some(DiscoveredModManager::Vortex(context)) = report
            .mod_manager
            .expect("Vortex should not require config")
        else {
            panic!("expected Vortex context");
        };
        assert_eq!(context.manager.display_name(), "Vortex");
        assert_eq!(
            context.manager.executable_path,
            PathBuf::from("C:/Vortex/Vortex.exe")
        );
        assert_eq!(context.manager.version, SemanticVersion::new(1, 12, 3));
        assert!(!context.parses_staging_or_config());
    }

    #[test]
    fn vortex_detection_uses_zero_version_fallback_when_metadata_is_absent() {
        let filesystem = FakeFilesystem::default().with_game_dir("C:/Games/Cwd");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default()
            .with_processes(vortex_processes("C:/Vortex/Vortex.exe"));

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd").with_current_process_id(100),
        );

        let Some(manager) = report.mod_manager.expect("Vortex should be detected") else {
            panic!("expected Vortex manager");
        };
        assert_eq!(manager.display_name(), "Vortex");
        assert_eq!(manager.version(), SemanticVersion::zero());
    }

    #[test]
    fn registry_read_errors_are_recoverable_diagnostics_and_gog_can_still_be_checked() {
        let filesystem = FakeFilesystem::default();
        let registry = FakeRegistry::default()
            .with_error(bethesda_request())
            .with_value(gog_request(), None);
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing"),
        );

        let error = report
            .game
            .expect_err("registry failure alone should be not-found");
        assert_eq!(
            error.user_message(),
            "A Fallout 4 installation could not be found."
        );
        assert!(
            error
                .diagnostic
                .as_deref()
                .expect("registry failure diagnostic")
                .contains("bethesda-registry registry lookup failed")
        );
        assert_eq!(
            report
                .attempts
                .iter()
                .map(|attempt| attempt.source)
                .collect::<Vec<_>>(),
            vec![
                DiscoverySource::RunningManagerGamePath,
                DiscoverySource::CurrentWorkingDirectory,
                DiscoverySource::BethesdaRegistry,
                DiscoverySource::GogRegistry,
            ]
        );
    }

    #[test]
    fn invalid_bethesda_registry_path_returns_reference_registry_error_without_gog_fallthrough() {
        let filesystem = FakeFilesystem::default().with_game_dir("C:/Games/Gog");
        let registry = FakeRegistry::default()
            .with_value(bethesda_request(), Some("C:/Broken/Fallout 4"))
            .with_value(gog_request(), Some("C:/Games/Gog"));
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Missing"),
        );

        let error = report
            .game
            .expect_err("invalid Bethesda value should stop registry chain");
        assert!(matches!(
            error.kind,
            crate::domain::discovery::DiscoveryErrorKind::InvalidRegistryPath { .. }
        ));
        assert!(
            error
                .user_message()
                .contains("The path set in your registry is:\nC:/Broken/Fallout 4")
        );
        assert_eq!(registry.reads.borrow().as_slice(), &[bethesda_request()]);
    }

    #[test]
    fn report_includes_fake_backed_system_metadata() {
        let filesystem = FakeFilesystem::default().with_game_dir("C:/Games/Cwd");
        let registry = FakeRegistry::default();
        let process = FakeProcessInspector::default();

        let report = service_report(
            &filesystem,
            &registry,
            &process,
            DiscoveryRequest::new("C:/Games/Cwd"),
        );

        let metadata = report
            .system_metadata
            .expect("fake metadata should be included");
        assert_eq!(metadata.os_name, "Windows 11 Pro");
        assert_eq!(metadata.os_version.as_deref(), Some("24H2"));
        assert_eq!(metadata.cpu_brand.as_deref(), Some("Fake Ryzen"));
        assert_eq!(metadata.logical_cpu_count, Some(16));
    }
}
