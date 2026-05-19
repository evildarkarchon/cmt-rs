//! Pure Overview diagnostics service.
//!
//! The reference Overview tab gathers discovery, binary, archive, module,
//! enablement, update, and desktop-action state inside Tk widget code. This
//! module keeps those decisions in a pure builder: all filesystem, registry,
//! process, network, desktop, and Slint work must be performed by callers and
//! injected here as typed facts.

use std::path::{Path, PathBuf};

use crate::{
    domain::{
        discovery::{
            ArchiveFormat, ArchiveRecord, ArchiveVersion, Fallout4InstallType,
            Fallout4Installation, ModuleHeaderVersion, ModuleRecord,
        },
        overview::{
            ACTION_DOWNGRADE_MANAGER_LABEL, ARCHIVE_GENERAL_LABEL, ARCHIVE_TEXTURE_LABEL,
            ArchivePanelSummary, BINARY_PANEL_TITLE, BinaryPanelSummary, BinaryStatusRow,
            MODULE_FULL_LABEL, MODULE_LIGHT_LABEL, ModulePanelSummary, OverviewActionError,
            OverviewAvailability, OverviewDeferredAction, OverviewDeferredActionKind,
            OverviewGamePathStatus, OverviewModManagerStatus, OverviewProblem,
            OverviewProblemDetail, OverviewProblemLink, OverviewProblemSource, OverviewProblemType,
            OverviewRefreshState, OverviewSnapshot, OverviewTopStatus, StatusSeverity,
            UpdateBannerState, UpdateCheckFailure, UpdateRelease,
        },
        settings::{AppSettings, UpdateSource},
    },
    platform::process::SystemMetadata,
    services::discovery::{DiscoveredModManager, DiscoveryReport},
};

const UNMANAGED_MOD_NAME: &str = "<Unmanaged>";
const VERIFY_FILES_SOLUTION: &str = "Verify files with Steam or reinstall the game.\nIf you downgraded the game you will need to do so again afterward.";
const ADDRESS_LIBRARY_SUMMARY: &str = "Address Library is a requirement for many F4SE mods and playing downgraded,\nand likely needs to be installed.";
const ADDRESS_LIBRARY_LINK: &str = "https://www.nexusmods.com/fallout4/mods/47327";
const NO_MOD_MANAGER_SUMMARY: &str = "No Mod Manager Detected";
const WRONG_BINARY_VERSION_SUMMARY: &str =
    "The version of this binary does not match your installed game version.";
const WRONG_ARCHIVE_VERSION_SUMMARY: &str =
    "The version of this archive does not match your installed game version.";
const MISSING_BINARY_SUMMARY: &str = "This file is missing from your game installation.";
const MISSING_DATA_SUMMARY: &str = "The Data folder was not found in your game install path.";
const MISSING_CCC_SUMMARY: &str = "The CC list file was not found in your game install path.\nThis is used to detect which CC modules/archives may be enabled.";
const MISSING_PLUGINS_SUMMARY: &str =
    "plugins.txt was not found.\nThis is used to detect which modules/archives are enabled.";
const ARCHIVE_UNREADABLE_SUMMARY: &str =
    "Failed to read archive due to permissions or the file is missing.";
const MODULE_UNREADABLE_SUMMARY: &str =
    "Failed to read module due to permissions or the file is missing.";
const MODULE_UNKNOWN_SOLUTION: &str = "It may be possible to open/resave this file with Creation Kit to update its format for Fallout 4.\nYou should compare the original and resaved files with xEdit to verify no undesired changes were made.";
const VORTEX_PARTIAL_SUPPORT_SUMMARY: &str = "Note: Vortex is not yet fully supported.\nOverview should be accurate but Scanner will only look in Data and not your staging folders, so it cannot yet identify the source mod for each issue.";
const MO2_WINDOWS_24H2_SUMMARY: &str = "Note: MO2 2.5.2 and earlier have issues on Windows 11 24H2+.\nPython apps such as Wrye Bash and CLASSIC may give errors\nsuch as FileNotFound or fail to detect files that are only\npresent in the VFS and not the Data folder.";
const ARCHIVE_LIMIT_SOLUTION: &str = "Archives can be unpacked or merged to reduce your total.\nNote: Do not mix texture and non-texture archives when merging.\nUnpacking is only suggested for small non-texture archives for performance reasons.\nYou can use Unpackrr to quickly unpack small archives:";
const FULL_MODULE_LIMIT_SOLUTION: &str = "Many Full modules are eligible to be flagged as Light (ESL).\n\nThis guide walks you through the process in xEdit:";
const LIGHT_MODULE_LIMIT_SOLUTION: &str = "Some plugins will need to be removed or manually merged.\nWarning: Do not use old/outdated tools like zMerge with Fallout 4 unless\nyou understand their issues and how to fix the merged plugins afterward.";
const UNPACKRR_LINK: &str = "https://www.nexusmods.com/fallout4/mods/82082";
const ESL_GUIDE_LINK: &str = "https://themidnightride.moddinglinked.com/esl.html";

/// Input contract for the pure Overview diagnostic builder.
///
/// Each field is pre-collected by another layer. The builder reads these facts
/// only; it does not attempt to repair, rescan, fetch updates, launch desktop
/// handlers, or infer file existence from paths.
pub struct OverviewDiagnosticsInput<'a> {
    /// Discovery report from the discovery service or a test fixture.
    pub discovery: &'a DiscoveryReport,
    /// Current typed application settings.
    pub settings: &'a AppSettings,
    /// Classified executable/DLL/BIN facts in display order.
    pub binaries: &'a [OverviewBinaryFact],
    /// Classified BA2 archive facts.
    pub archives: &'a [ArchiveRecord],
    /// Classified ESP/ESM/ESL module facts.
    pub modules: &'a [ModuleRecord],
    /// Injected enablement and required-file facts.
    pub enablement: &'a OverviewEnablementFacts,
    /// Update-check state supplied by a worker or test.
    pub update: &'a OverviewUpdateCheckState,
    /// Optional desktop action feedback from a previous UI action.
    pub last_desktop_action: Option<&'a OverviewDesktopActionFeedback>,
}

/// Pure builder for complete Overview snapshots.
#[derive(Debug, Default, Clone, Copy)]
pub struct OverviewDiagnostics;

impl OverviewDiagnostics {
    /// Builds a complete [`OverviewSnapshot`] from already-collected facts.
    pub fn build(input: OverviewDiagnosticsInput<'_>) -> OverviewSnapshot {
        let mut problems = Vec::new();

        let manager_status = manager_status(input.discovery, &mut problems);
        let installation = installation_or_problem(input.discovery, &mut problems);
        let effective_install_type = effective_install_type(installation, input.binaries);
        let system_metadata = system_metadata_or_problem(input.discovery, &mut problems);

        append_manager_warnings(&manager_status, system_metadata.as_ref(), &mut problems);
        append_missing_data_problem(installation, &mut problems);

        let mut binaries = build_binary_panel(
            input.binaries,
            effective_install_type,
            &input.enablement.address_library,
            &mut problems,
        );
        let archives = ArchivePanelSummary::from_records(input.archives);
        let modules = ModulePanelSummary::from_records(input.modules);

        append_enablement_problems(
            input.enablement,
            manager_status_has_manager(&manager_status),
            &mut problems,
        );
        append_archive_problems(input.archives, effective_install_type, &mut problems);
        append_module_problems(input.modules, &mut problems);
        append_count_limit_problems(&archives, &modules, &mut problems);

        binaries.title = BINARY_PANEL_TITLE;

        let update_banner = update_banner(input.settings.update_source, input.update);
        let last_action_error = last_action_error(input.last_desktop_action);
        let refresh = refresh_state(&problems);
        let top = OverviewTopStatus::new(
            manager_status,
            installation
                .map(|installation| OverviewGamePathStatus::found(installation.game_path.clone()))
                .unwrap_or_default(),
            effective_install_type,
            system_metadata,
        );

        OverviewSnapshot {
            refresh,
            top,
            binaries,
            archives,
            archive_records: input.archives.to_vec(),
            data_path: installation.and_then(|installation| installation.data_path.clone()),
            modules,
            update_banner,
            problems,
            last_action_error,
        }
    }
}

/// Classified binary/executable fact supplied to [`OverviewDiagnostics`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewBinaryFact {
    /// Reference file name or relative path, such as `Fallout4.exe`.
    pub file_name: String,
    /// Optional absolute path for scanner handoff.
    pub path: Option<PathBuf>,
    /// Classified install type for this file.
    pub install_type: Fallout4InstallType,
    /// Optional display/version metadata from a file-version reader.
    pub version: Option<String>,
    /// Optional CRC/hash fallback metadata.
    pub hash: Option<String>,
}

impl OverviewBinaryFact {
    /// Creates a classified binary fact with no path or version metadata.
    pub fn new(file_name: impl Into<String>, install_type: Fallout4InstallType) -> Self {
        Self {
            file_name: file_name.into(),
            path: None,
            install_type,
            version: None,
            hash: None,
        }
    }

    /// Creates a not-found binary fact.
    pub fn missing(file_name: impl Into<String>) -> Self {
        Self::new(file_name, Fallout4InstallType::NotFound)
    }

    /// Attaches an absolute path supplied by an adapter or fixture.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Attaches optional version/hash metadata supplied by an adapter or fixture.
    pub fn with_version_metadata(
        mut self,
        version: impl Into<Option<String>>,
        hash: impl Into<Option<String>>,
    ) -> Self {
        self.version = version.into();
        self.hash = hash.into();
        self
    }
}

/// Required-file or enablement-file state supplied by a collector.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OverviewFilePresence {
    /// The file was not checked yet.
    #[default]
    Unknown,
    /// The file exists at the supplied path.
    Present(PathBuf),
    /// The file was checked and is absent.
    Missing(PathBuf),
    /// The file exists or was expected, but could not be read.
    Unreadable(PathBuf),
}

impl OverviewFilePresence {
    /// Creates a present-file fact.
    pub fn present(path: impl Into<PathBuf>) -> Self {
        Self::Present(path.into())
    }

    /// Creates a missing-file fact.
    pub fn missing(path: impl Into<PathBuf>) -> Self {
        Self::Missing(path.into())
    }

    /// Creates an unreadable-file fact.
    pub fn unreadable(path: impl Into<PathBuf>) -> Self {
        Self::Unreadable(path.into())
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Self::Unknown => None,
            Self::Present(path) | Self::Missing(path) | Self::Unreadable(path) => Some(path),
        }
    }
}

/// Address Library availability supplied by binary/F4SE collectors.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewAddressLibraryFact {
    /// Availability status shown in the binary panel.
    pub availability: OverviewAvailability,
    /// Optional absolute expected or installed path.
    pub path: Option<PathBuf>,
    /// Optional relative path, usually `F4SE/Plugins/version-...bin`.
    pub relative_path: Option<PathBuf>,
    /// Whether a missing Address Library should be reported as a problem.
    pub required: bool,
}

impl OverviewAddressLibraryFact {
    /// Creates an installed Address Library fact.
    pub fn installed(path: impl Into<PathBuf>, relative_path: impl Into<PathBuf>) -> Self {
        Self {
            availability: OverviewAvailability::Installed,
            path: Some(path.into()),
            relative_path: Some(relative_path.into()),
            required: true,
        }
    }

    /// Creates a missing Address Library fact.
    pub fn missing(path: impl Into<PathBuf>, relative_path: impl Into<PathBuf>) -> Self {
        Self {
            availability: OverviewAvailability::NotFound,
            path: Some(path.into()),
            relative_path: Some(relative_path.into()),
            required: true,
        }
    }

    /// Creates an unknown/not-checked Address Library fact.
    pub const fn unknown() -> Self {
        Self {
            availability: OverviewAvailability::Unknown,
            path: None,
            relative_path: None,
            required: false,
        }
    }
}

/// Enablement and required-file facts used by Overview diagnostics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewEnablementFacts {
    /// Fallout4.ccc state for Creation Club module/archive enablement.
    pub fallout4_ccc: OverviewFilePresence,
    /// plugins.txt state for plugin enablement.
    pub plugins_txt: OverviewFilePresence,
    /// Address Library availability for the binary panel.
    pub address_library: OverviewAddressLibraryFact,
}

/// Worker-supplied update-check state.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OverviewUpdateCheckState {
    /// No update check has been started for the configured source.
    #[default]
    NotChecked,
    /// A worker is checking the configured source.
    Checking,
    /// A check completed and returned zero or more newer releases.
    Completed {
        /// Newer releases, empty when no update is available.
        releases: Vec<UpdateRelease>,
    },
    /// A configured check failed; the reference app treats this as no banner.
    FailedSilently {
        /// Safe provider-level failure summaries.
        failures: Vec<UpdateCheckFailure>,
    },
}

/// Desktop action feedback from an earlier UI action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewDesktopActionFeedback {
    /// Deferred action that was attempted.
    pub action: OverviewDeferredActionKind,
    /// Result of handing the action to a desktop adapter.
    pub outcome: OverviewDesktopActionOutcome,
}

impl OverviewDesktopActionFeedback {
    /// Creates successful feedback for an action.
    pub const fn succeeded(action: OverviewDeferredActionKind) -> Self {
        Self {
            action,
            outcome: OverviewDesktopActionOutcome::Succeeded,
        }
    }

    /// Creates failed feedback with a safe user-facing message.
    pub fn failed(action: OverviewDeferredActionKind, safe_message: impl Into<String>) -> Self {
        Self {
            action,
            outcome: OverviewDesktopActionOutcome::Failed {
                safe_message: safe_message.into(),
            },
        }
    }
}

/// Success or failure result for a deferred desktop action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewDesktopActionOutcome {
    /// The desktop adapter accepted the action.
    Succeeded,
    /// The desktop adapter rejected the action with safe text.
    Failed {
        /// Safe UI-facing failure message.
        safe_message: String,
    },
}

fn manager_status(
    report: &DiscoveryReport,
    problems: &mut Vec<OverviewProblem>,
) -> OverviewModManagerStatus {
    match &report.mod_manager {
        Ok(Some(manager)) => OverviewModManagerStatus::detected(manager.clone()),
        Ok(None) => {
            problems.push(no_mod_manager_problem());
            OverviewModManagerStatus::NotFound
        }
        Err(error) => {
            problems.push(
                OverviewProblem::pathless(
                    OverviewProblemSource::TopStatus,
                    "Mod Manager",
                    OverviewProblemType::Custom("Mod Manager Error".to_owned()),
                    error.user_message(),
                    None::<String>,
                )
                .with_severity(StatusSeverity::Error),
            );
            OverviewModManagerStatus::NotFound
        }
    }
}

fn installation_or_problem<'a>(
    report: &'a DiscoveryReport,
    problems: &mut Vec<OverviewProblem>,
) -> Option<&'a Fallout4Installation> {
    match &report.game {
        Ok(installation) => Some(installation),
        Err(error) => {
            problems.push(OverviewProblem::pathless(
                OverviewProblemSource::TopStatus,
                "Game Path",
                OverviewProblemType::FileNotFound,
                error.user_message(),
                Some("Verify files with Steam or reinstall the game.".to_owned()),
            ));
            None
        }
    }
}

fn system_metadata_or_problem(
    report: &DiscoveryReport,
    problems: &mut Vec<OverviewProblem>,
) -> Option<SystemMetadata> {
    match &report.system_metadata {
        Ok(metadata) => Some(metadata.clone()),
        Err(error) => {
            problems.push(
                OverviewProblem::pathless(
                    OverviewProblemSource::TopStatus,
                    "PC Specs",
                    OverviewProblemType::Custom("System Metadata Unavailable".to_owned()),
                    error.user_message(),
                    None::<String>,
                )
                .with_severity(StatusSeverity::Warning),
            );
            None
        }
    }
}

fn effective_install_type(
    installation: Option<&Fallout4Installation>,
    binaries: &[OverviewBinaryFact],
) -> Fallout4InstallType {
    binaries
        .iter()
        .find(|fact| basename_eq(&fact.file_name, "Fallout4.exe"))
        .map(|fact| fact.install_type)
        .or_else(|| installation.map(|installation| installation.install_type))
        .unwrap_or(Fallout4InstallType::NotFound)
}

fn append_manager_warnings(
    manager_status: &OverviewModManagerStatus,
    metadata: Option<&SystemMetadata>,
    problems: &mut Vec<OverviewProblem>,
) {
    let OverviewModManagerStatus::Detected(manager) = manager_status else {
        return;
    };

    match manager.as_ref() {
        DiscoveredModManager::Vortex(_) => problems.push(
            OverviewProblem::pathless(
                OverviewProblemSource::TopStatus,
                "Vortex",
                OverviewProblemType::Custom("Partial Support".to_owned()),
                VORTEX_PARTIAL_SUPPORT_SUMMARY,
                None::<String>,
            )
            .with_severity(StatusSeverity::Warning),
        ),
        DiscoveredModManager::ModOrganizer(configuration) => {
            let version = configuration.context.manager.version;
            if metadata.is_some_and(is_windows_11_24h2_or_later)
                && version <= crate::domain::discovery::SemanticVersion::new(2, 5, 2)
            {
                problems.push(
                    OverviewProblem::pathless(
                        OverviewProblemSource::TopStatus,
                        "Windows 11 24H2 + Mod Organizer",
                        OverviewProblemType::Custom("Compatibility Warning".to_owned()),
                        MO2_WINDOWS_24H2_SUMMARY,
                        Some("Update Mod Organizer when a fixed version is available.".to_owned()),
                    )
                    .with_severity(StatusSeverity::Warning),
                );
            }
        }
    }
}

fn is_windows_11_24h2_or_later(metadata: &SystemMetadata) -> bool {
    let text = format!(
        "{} {}",
        metadata.os_name,
        metadata.os_version.as_deref().unwrap_or_default()
    );
    text.contains("Windows")
        && text.contains("11")
        && (text.contains("24H2") || text.contains("25H2"))
}

fn append_missing_data_problem(
    installation: Option<&Fallout4Installation>,
    problems: &mut Vec<OverviewProblem>,
) {
    if matches!(installation, Some(installation) if installation.data_path.is_none()) {
        problems.push(OverviewProblem::pathless(
            OverviewProblemSource::Modules,
            "Data",
            OverviewProblemType::FileNotFound,
            MISSING_DATA_SUMMARY,
            Some(VERIFY_FILES_SOLUTION.to_owned()),
        ));
    }
}

fn build_binary_panel(
    facts: &[OverviewBinaryFact],
    install_type: Fallout4InstallType,
    address_library: &OverviewAddressLibraryFact,
    problems: &mut Vec<OverviewProblem>,
) -> BinaryPanelSummary {
    let mut rows = Vec::with_capacity(facts.len());

    for fact in facts {
        let severity = binary_severity(fact, install_type);
        let mut row = BinaryStatusRow::new(binary_label(&fact.file_name), fact.install_type)
            .with_version_metadata(fact.version.clone(), fact.hash.clone());
        row.path.clone_from(&fact.path);
        row.severity = severity;
        rows.push(row);
        append_binary_problem(fact, install_type, severity, problems);
    }

    append_address_library_problem(address_library, problems);

    BinaryPanelSummary {
        title: BINARY_PANEL_TITLE,
        rows,
        address_library: address_library.availability,
        actions: vec![OverviewDeferredAction::utility(
            OverviewDeferredActionKind::OpenDowngradeManager,
            ACTION_DOWNGRADE_MANAGER_LABEL,
        )],
    }
}

fn binary_severity(fact: &OverviewBinaryFact, install_type: Fallout4InstallType) -> StatusSeverity {
    if is_compatible_binary_install_type(fact.install_type, install_type) {
        return StatusSeverity::Good;
    }

    if fact.install_type == Fallout4InstallType::NotFound
        && is_optional_missing_binary(&fact.file_name, install_type)
    {
        return StatusSeverity::Neutral;
    }

    match fact.install_type {
        Fallout4InstallType::NotFound => StatusSeverity::Error,
        Fallout4InstallType::Unknown => StatusSeverity::Unknown,
        Fallout4InstallType::Obsolete => StatusSeverity::Warning,
        Fallout4InstallType::OldGen
        | Fallout4InstallType::DownGrade
        | Fallout4InstallType::NextGen
        | Fallout4InstallType::Anniversary
        | Fallout4InstallType::NextGenAnniversary => StatusSeverity::Error,
    }
}

fn append_binary_problem(
    fact: &OverviewBinaryFact,
    install_type: Fallout4InstallType,
    severity: StatusSeverity,
    problems: &mut Vec<OverviewProblem>,
) {
    if !severity.is_problem() && fact.install_type != Fallout4InstallType::Unknown {
        return;
    }

    if basename_eq(&fact.file_name, "Fallout4.exe")
        && fact.install_type == Fallout4InstallType::Unknown
    {
        let version = fact
            .version
            .as_deref()
            .or(fact.hash.as_deref())
            .filter(|value| !value.is_empty())
            .unwrap_or("Unknown");
        problems.push(OverviewProblem::pathless(
            OverviewProblemSource::Binaries,
            "Fallout4.exe",
            OverviewProblemType::UnknownGameVersion,
            format!(
                "{version} is an unknown version.\nPossible causes:\n1. The game is an old version and should be updated.\n2. The exe file may be corrupted.\n3. The game is a new version and the Toolkit needs to be updated."
            ),
            Some("Either update the game/verify files in Steam, or report this issue.".to_owned()),
        ));
        return;
    }

    if fact.install_type == Fallout4InstallType::NotFound
        && is_optional_missing_binary(&fact.file_name, install_type)
    {
        return;
    }

    let problem = if fact.install_type == Fallout4InstallType::NotFound {
        OverviewProblemType::FileNotFound
    } else {
        OverviewProblemType::WrongVersion
    };
    let summary = if fact.install_type == Fallout4InstallType::NotFound {
        MISSING_BINARY_SUMMARY
    } else {
        WRONG_BINARY_VERSION_SUMMARY
    };

    problems.push(path_or_pathless_problem(
        OverviewProblemSource::Binaries,
        fact.path.as_deref(),
        Some(PathBuf::from(relative_binary_name(&fact.file_name))),
        relative_binary_name(&fact.file_name),
        problem,
        summary,
        None::<String>,
    ));
}

fn is_compatible_binary_install_type(
    actual: Fallout4InstallType,
    expected: Fallout4InstallType,
) -> bool {
    actual == expected
        || (actual == Fallout4InstallType::OldGen && expected == Fallout4InstallType::DownGrade)
        || (actual == Fallout4InstallType::NextGenAnniversary
            && matches!(
                expected,
                Fallout4InstallType::NextGen | Fallout4InstallType::Anniversary
            ))
}

fn is_optional_missing_binary(file_name: &str, install_type: Fallout4InstallType) -> bool {
    basename_eq(file_name, "CreationKit.exe")
        || basename_eq(file_name, "Archive2.exe")
        || (basename_eq(file_name, "f4se_steam_loader.dll")
            && matches!(
                install_type,
                Fallout4InstallType::NextGen
                    | Fallout4InstallType::Anniversary
                    | Fallout4InstallType::NextGenAnniversary
            ))
}

fn append_address_library_problem(
    address_library: &OverviewAddressLibraryFact,
    problems: &mut Vec<OverviewProblem>,
) {
    if address_library.availability != OverviewAvailability::NotFound || !address_library.required {
        return;
    }

    let mut problem = path_or_pathless_problem(
        OverviewProblemSource::Binaries,
        address_library.path.as_deref(),
        address_library.relative_path.clone(),
        address_library
            .relative_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Address Library"),
        OverviewProblemType::FileNotFound,
        ADDRESS_LIBRARY_SUMMARY,
        Some("Download the mod here:".to_owned()),
    );
    problem.links = vec![OverviewProblemLink::new(
        Some("Address Library".to_owned()),
        ADDRESS_LIBRARY_LINK,
    )];
    problems.push(problem);
}

fn append_enablement_problems(
    enablement: &OverviewEnablementFacts,
    manager_detected: bool,
    problems: &mut Vec<OverviewProblem>,
) {
    append_required_file_problem(
        &enablement.fallout4_ccc,
        "Fallout4.ccc",
        MISSING_CCC_SUMMARY,
        Some(VERIFY_FILES_SOLUTION.to_owned()),
        problems,
    );
    append_required_file_problem(
        &enablement.plugins_txt,
        "plugins.txt",
        MISSING_PLUGINS_SUMMARY,
        Some(if manager_detected {
            "N/A".to_owned()
        } else {
            "Launch this app with your mod manager.".to_owned()
        }),
        problems,
    );
}

fn append_required_file_problem(
    presence: &OverviewFilePresence,
    display_name: &str,
    missing_summary: &str,
    solution: Option<String>,
    problems: &mut Vec<OverviewProblem>,
) {
    match presence {
        OverviewFilePresence::Missing(_) => problems.push(path_or_pathless_problem(
            OverviewProblemSource::Modules,
            presence.path(),
            Some(PathBuf::from(display_name)),
            display_name,
            OverviewProblemType::FileNotFound,
            missing_summary,
            solution,
        )),
        OverviewFilePresence::Unreadable(_) => problems.push(path_or_pathless_problem(
            OverviewProblemSource::Modules,
            presence.path(),
            Some(PathBuf::from(display_name)),
            display_name,
            OverviewProblemType::FileNotFound,
            format!("{display_name} could not be read."),
            solution,
        )),
        OverviewFilePresence::Unknown | OverviewFilePresence::Present(_) => {}
    }
}

fn append_archive_problems(
    records: &[ArchiveRecord],
    install_type: Fallout4InstallType,
    problems: &mut Vec<OverviewProblem>,
) {
    for record in records
        .iter()
        .filter(|record| record.enabled || !record.readable)
    {
        if !record.readable {
            problems.push(
                archive_problem(
                    record,
                    OverviewProblemType::InvalidArchive,
                    ARCHIVE_UNREADABLE_SUMMARY,
                )
                .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
            );
            continue;
        }

        match record.version {
            ArchiveVersion::Unknown(version) => {
                problems.push(
                    archive_problem(
                        record,
                        OverviewProblemType::InvalidArchive,
                        format!("Archive version ({version}) is not valid for Fallout 4."),
                    )
                    .with_details(vec![OverviewProblemDetail::new(
                        "Version",
                        version.to_string(),
                    )])
                    .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
                );
                continue;
            }
            ArchiveVersion::NextGen7 | ArchiveVersion::NextGen8
                if archive_version_mismatches_game(record.version, install_type) =>
            {
                problems.push(
                    archive_problem(
                        record,
                        OverviewProblemType::WrongVersion,
                        WRONG_ARCHIVE_VERSION_SUMMARY,
                    )
                    .with_details(vec![OverviewProblemDetail::new(
                        "Version",
                        record.version.as_header_value().to_string(),
                    )])
                    .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
                );
            }
            ArchiveVersion::OldGen | ArchiveVersion::NextGen7 | ArchiveVersion::NextGen8 => {}
        }

        if let ArchiveFormat::Unknown(format) = &record.format {
            problems.push(
                archive_problem(
                    record,
                    OverviewProblemType::InvalidArchive,
                    format!("Archive format ({format}) is not valid for Fallout 4."),
                )
                .with_details(vec![OverviewProblemDetail::new("Format", format.clone())])
                .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
            );
        }
    }
}

fn archive_version_mismatches_game(
    version: ArchiveVersion,
    install_type: Fallout4InstallType,
) -> bool {
    matches!(version, ArchiveVersion::NextGen7 | ArchiveVersion::NextGen8)
        && matches!(
            install_type,
            Fallout4InstallType::OldGen | Fallout4InstallType::DownGrade
        )
}

fn archive_problem(
    record: &ArchiveRecord,
    problem: OverviewProblemType,
    summary: impl Into<String>,
) -> OverviewProblem {
    path_or_pathless_problem(
        OverviewProblemSource::Archives,
        Some(&record.path),
        Some(file_name_path(&record.path)),
        display_file_name(&record.path),
        problem,
        summary,
        None::<String>,
    )
}

fn append_module_problems(records: &[ModuleRecord], problems: &mut Vec<OverviewProblem>) {
    for record in records
        .iter()
        .filter(|record| record.enabled || !record.readable)
    {
        if !record.readable {
            problems.push(
                module_problem(record, MODULE_UNREADABLE_SUMMARY, None::<String>)
                    .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
            );
            continue;
        }

        if let ModuleHeaderVersion::Unknown(version) = &record.header_version {
            let display = if version.is_empty() {
                "unknown".to_owned()
            } else {
                version.clone()
            };
            let valid_games = module_version_support_suffix(&display);
            problems.push(
                module_problem(
                    record,
                    format!("Module version ({display}) is not valid for Fallout 4.{valid_games}"),
                    Some(MODULE_UNKNOWN_SOLUTION.to_owned()),
                )
                .with_details(vec![OverviewProblemDetail::new("HEDR", display)])
                .with_mod_name(Some(UNMANAGED_MOD_NAME.to_owned())),
            );
        }
    }
}

fn module_problem(
    record: &ModuleRecord,
    summary: impl Into<String>,
    solution: impl Into<Option<String>>,
) -> OverviewProblem {
    path_or_pathless_problem(
        OverviewProblemSource::Modules,
        Some(&record.path),
        Some(file_name_path(&record.path)),
        display_file_name(&record.path),
        OverviewProblemType::InvalidModule,
        summary,
        solution,
    )
}

fn module_version_support_suffix(version: &str) -> String {
    let games = match version {
        "1.2" | "1.3" => &["Morrowind"][..],
        "0.8" | "1.0" => &["Oblivion"],
        "0.94" => &[
            "Skyrim LE",
            "Skyrim VR",
            "Skyrim SE",
            "Fallout 3",
            "Fallout NV",
        ],
        "1.70" => &["Skyrim LE", "Skyrim VR", "Skyrim SE"],
        "1.71" => &["Skyrim SE"],
        "0.85" => &["Fallout 3"],
        "0.95" | "1.00" => &["Fallout 4 VR", "Fallout 4"],
        "1.32" | "1.33" | "1.34" => &["Fallout NV"],
        "68.0" | "216.0" | "223.0" => &["Fallout 76"],
        "0.96" => &["Starfield"],
        _ => &[],
    };

    if games.is_empty() {
        String::new()
    } else {
        format!("\nGames supporting v{version}: {}", games.join(", "))
    }
}

fn append_count_limit_problems(
    archives: &ArchivePanelSummary,
    modules: &ModulePanelSummary,
    problems: &mut Vec<OverviewProblem>,
) {
    for row in archives.rows.iter().filter(|row| {
        matches!(row.label, ARCHIVE_GENERAL_LABEL | ARCHIVE_TEXTURE_LABEL)
            && matches!(row.limit, Some(limit) if row.value > limit)
    }) {
        let file_format = if row.label == ARCHIVE_GENERAL_LABEL {
            "General"
        } else {
            "Texture"
        };
        let limit = row.limit.expect("filtered rows have limits");
        problems.push(
            OverviewProblem::pathless(
                OverviewProblemSource::CountLimit,
                format!("{} {file_format} Archives", row.value),
                OverviewProblemType::LimitExceeded,
                format!(
                    "You have {} {file_format} Archives enabled. The limit is {limit}.",
                    row.value
                ),
                Some(ARCHIVE_LIMIT_SOLUTION.to_owned()),
            )
            .with_links(vec![OverviewProblemLink::new(
                Some("Unpackrr".to_owned()),
                UNPACKRR_LINK,
            )]),
        );
    }

    for row in modules.rows.iter().filter(|row| {
        matches!(row.label, MODULE_FULL_LABEL | MODULE_LIGHT_LABEL)
            && matches!(row.limit, Some(limit) if row.value > limit)
    }) {
        let file_format = row.label;
        let limit = row.limit.expect("filtered rows have limits");
        let (solution, links) = if row.label == MODULE_FULL_LABEL {
            (
                FULL_MODULE_LIMIT_SOLUTION.to_owned(),
                vec![OverviewProblemLink::new(
                    Some("ESL guide".to_owned()),
                    ESL_GUIDE_LINK,
                )],
            )
        } else {
            (LIGHT_MODULE_LIMIT_SOLUTION.to_owned(), Vec::new())
        };
        problems.push(
            OverviewProblem::pathless(
                OverviewProblemSource::CountLimit,
                format!("{} {file_format} Modules", row.value),
                OverviewProblemType::LimitExceeded,
                format!(
                    "You have {} {file_format} Modules enabled. The limit is {limit}.",
                    row.value
                ),
                Some(solution),
            )
            .with_links(links),
        );
    }
}

fn update_banner(
    selected_source: UpdateSource,
    update: &OverviewUpdateCheckState,
) -> UpdateBannerState {
    if matches!(selected_source, UpdateSource::None) {
        return UpdateBannerState::Disabled;
    }

    match update {
        OverviewUpdateCheckState::NotChecked => UpdateBannerState::NotChecked { selected_source },
        OverviewUpdateCheckState::Checking => UpdateBannerState::Checking { selected_source },
        OverviewUpdateCheckState::Completed { releases } => {
            UpdateBannerState::available_or_no_update(selected_source, releases.clone())
        }
        OverviewUpdateCheckState::FailedSilently { failures } => {
            UpdateBannerState::failed_silently(selected_source, failures.clone())
        }
    }
}

fn last_action_error(
    feedback: Option<&OverviewDesktopActionFeedback>,
) -> Option<OverviewActionError> {
    let feedback = feedback?;
    match &feedback.outcome {
        OverviewDesktopActionOutcome::Succeeded => None,
        OverviewDesktopActionOutcome::Failed { safe_message } => Some(OverviewActionError::new(
            feedback.action,
            safe_message.clone(),
        )),
    }
}

fn refresh_state(problems: &[OverviewProblem]) -> OverviewRefreshState {
    if problems.is_empty() {
        OverviewRefreshState::ready(None::<String>)
    } else {
        OverviewRefreshState::partial("Overview refreshed with recoverable issues.")
    }
}

fn no_mod_manager_problem() -> OverviewProblem {
    OverviewProblem::pathless(
        OverviewProblemSource::TopStatus,
        "Mod Manager",
        OverviewProblemType::NoModManager,
        NO_MOD_MANAGER_SUMMARY,
        Some("Your mod manager must launch the app to be detected.".to_owned()),
    )
}

fn manager_status_has_manager(manager_status: &OverviewModManagerStatus) -> bool {
    matches!(manager_status, OverviewModManagerStatus::Detected(_))
}

fn path_or_pathless_problem(
    source: OverviewProblemSource,
    path: Option<&Path>,
    relative_path: Option<PathBuf>,
    display_path: impl Into<String>,
    problem: OverviewProblemType,
    summary: impl Into<String>,
    solution: impl Into<Option<String>>,
) -> OverviewProblem {
    match path {
        Some(path) => OverviewProblem::with_path(
            source,
            path.to_path_buf(),
            relative_path,
            problem,
            summary,
            solution,
        ),
        None => OverviewProblem::pathless(source, display_path, problem, summary, solution),
    }
}

fn file_name_path(path: &Path) -> PathBuf {
    PathBuf::from(display_file_name(path))
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn binary_label(file_name: &str) -> String {
    trim_extension(&relative_binary_name(file_name))
}

fn relative_binary_name(file_name: &str) -> String {
    file_name
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(file_name)
        .to_owned()
}

fn basename_eq(file_name: &str, expected: &str) -> bool {
    relative_binary_name(file_name).eq_ignore_ascii_case(expected)
}

fn trim_extension(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_owned())
        .unwrap_or_else(|| file_name.to_owned())
}

#[cfg(test)]
mod overview_diagnostics {
    use super::*;
    use crate::{
        domain::{
            discovery::{
                ArchiveFormat, DiscoveryError, ModuleHeaderVersion, ModuleKind, SemanticVersion,
            },
            mod_manager::{
                DetectedModManager, Mo2Configuration, ModManagerKind, ModOrganizerContext,
                ModOrganizerDirectories, VortexContext,
            },
            overview::{
                ARCHIVE_NEXT_GEN_VERSION_LABEL, ARCHIVE_OLD_GEN_VERSION_LABEL, COUNT_TOTAL_LABEL,
                COUNT_UNREADABLE_LABEL, MAX_ARCHIVES_GENERAL, MAX_MODULES_LIGHT, NEXUS_MODS_LINK,
                OverviewDeferredActionTarget, OverviewProblemType, OverviewRefreshPhase,
                UpdateProvider,
            },
            settings::UpdateSource,
        },
        platform::{PlatformError, PlatformErrorKind, PlatformOperation},
        services::discovery::{ModManagerDiscoveryError, ModManagerDiscoveryStep},
    };

    fn settings(update_source: UpdateSource) -> AppSettings {
        AppSettings {
            update_source,
            ..AppSettings::default()
        }
    }

    fn input<'a>(
        discovery: &'a DiscoveryReport,
        settings: &'a AppSettings,
        binaries: &'a [OverviewBinaryFact],
        archives: &'a [ArchiveRecord],
        modules: &'a [ModuleRecord],
        enablement: &'a OverviewEnablementFacts,
        update: &'a OverviewUpdateCheckState,
    ) -> OverviewDiagnosticsInput<'a> {
        OverviewDiagnosticsInput {
            discovery,
            settings,
            binaries,
            archives,
            modules,
            enablement,
            update,
            last_desktop_action: None,
        }
    }

    fn report(
        installation: Fallout4Installation,
        manager: Option<DiscoveredModManager>,
        metadata: SystemMetadata,
    ) -> DiscoveryReport {
        DiscoveryReport {
            game: Ok(installation),
            mod_manager: Ok(manager),
            system_metadata: Ok(metadata),
            attempts: Vec::new(),
            manager_steps: Vec::<ModManagerDiscoveryStep>::new(),
        }
    }

    fn error_report() -> DiscoveryReport {
        DiscoveryReport {
            game: Err(DiscoveryError::invalid_registry_path("D:/Moved/Fallout 4")),
            mod_manager: Err(ModManagerDiscoveryError::ProcessInspection(
                PlatformError::new(
                    PlatformOperation::ListProcesses,
                    "process table",
                    PlatformErrorKind::CommandFailed,
                    "Process inspection failed.",
                ),
            )),
            system_metadata: Err(PlatformError::new(
                PlatformOperation::ReadSystemMetadata,
                "system metadata",
                PlatformErrorKind::CommandFailed,
                "System metadata read failed.",
            )),
            attempts: Vec::new(),
            manager_steps: Vec::new(),
        }
    }

    fn installation(install_type: Fallout4InstallType) -> Fallout4Installation {
        let mut installation = Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            Some("C:/Games/Fallout 4/Data/F4SE/Plugins"),
        );
        installation.install_type = install_type;
        installation
    }

    fn installation_without_data(install_type: Fallout4InstallType) -> Fallout4Installation {
        let mut installation = Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            None::<PathBuf>,
            None::<PathBuf>,
        );
        installation.install_type = install_type;
        installation
    }

    fn windows_metadata(version: &str) -> SystemMetadata {
        SystemMetadata::new(
            "Windows",
            Some(version),
            "x86_64",
            Some("Example CPU"),
            Some(32 * 1024 * 1024 * 1024),
            Some(16),
        )
    }

    fn mo2(version: SemanticVersion) -> DiscoveredModManager {
        let manager = DetectedModManager::mod_organizer("C:/Modding/MO2/ModOrganizer.exe", version);
        let context = ModOrganizerContext::new(
            manager,
            "Default",
            ModOrganizerDirectories::reference_defaults("C:/Modding/MO2"),
        )
        .with_game_path("C:/Games/Fallout 4");
        DiscoveredModManager::ModOrganizer(Box::new(Mo2Configuration::new(context)))
    }

    fn vortex() -> DiscoveredModManager {
        DiscoveredModManager::Vortex(VortexContext::new(
            "C:/Program Files/Black Tree Gaming/Vortex/Vortex.exe",
            None,
        ))
    }

    fn enabled_files() -> OverviewEnablementFacts {
        OverviewEnablementFacts {
            fallout4_ccc: OverviewFilePresence::present("C:/Games/Fallout 4/Fallout4.ccc"),
            plugins_txt: OverviewFilePresence::present(
                "C:/Users/Example/AppData/Local/Fallout4/plugins.txt",
            ),
            address_library: OverviewAddressLibraryFact::installed(
                "C:/Games/Fallout 4/Data/F4SE/Plugins/version-1-10-163-0.bin",
                "F4SE/Plugins/version-1-10-163-0.bin",
            ),
        }
    }

    fn row_value(rows: &[OverviewCountRow], label: &str) -> usize {
        rows.iter()
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("missing row {label}"))
            .value
    }

    use crate::domain::overview::OverviewCountRow;

    #[test]
    fn overview_diagnostics_successful_old_gen_snapshot_has_reference_counts_and_no_problems() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let settings = settings(UpdateSource::Nexus);
        let binaries = vec![
            OverviewBinaryFact::new("Fallout4.exe", Fallout4InstallType::OldGen)
                .with_path("C:/Games/Fallout 4/Fallout4.exe")
                .with_version_metadata(Some("1.10.163.0".to_owned()), None::<String>),
            OverviewBinaryFact::new("steam_api64.dll", Fallout4InstallType::OldGen),
        ];
        let archives = vec![
            ArchiveRecord::new(
                "C:/Games/Fallout 4/Data/Fallout4 - Main.ba2",
                ArchiveFormat::General,
                ArchiveVersion::OldGen,
                true,
            ),
            ArchiveRecord::new(
                "C:/Games/Fallout 4/Data/Fallout4 - Textures1.ba2",
                ArchiveFormat::DirectX10,
                ArchiveVersion::OldGen,
                true,
            ),
        ];
        let modules = vec![
            ModuleRecord::new(
                "C:/Games/Fallout 4/Data/Fallout4.esm",
                ModuleKind::Full,
                ModuleHeaderVersion::Version100,
                true,
            ),
            ModuleRecord::new(
                "C:/Games/Fallout 4/Data/Example.esl",
                ModuleKind::Light,
                ModuleHeaderVersion::Version095,
                true,
            ),
        ];
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::Completed { releases: vec![] };

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Ready);
        assert_eq!(snapshot.top.version, Fallout4InstallType::OldGen);
        assert_eq!(snapshot.binaries.rows[0].label, "Fallout4");
        assert_eq!(snapshot.binaries.rows[0].severity, StatusSeverity::Good);
        assert_eq!(
            snapshot.binaries.address_library,
            OverviewAvailability::Installed
        );
        assert_eq!(row_value(&snapshot.archives.rows, ARCHIVE_GENERAL_LABEL), 1);
        assert_eq!(row_value(&snapshot.archives.rows, ARCHIVE_TEXTURE_LABEL), 1);
        assert_eq!(
            row_value(&snapshot.archives.rows, ARCHIVE_OLD_GEN_VERSION_LABEL),
            2
        );
        assert_eq!(row_value(&snapshot.modules.rows, MODULE_FULL_LABEL), 1);
        assert_eq!(row_value(&snapshot.modules.rows, MODULE_LIGHT_LABEL), 1);
        assert!(snapshot.problems.is_empty());
        assert!(matches!(
            snapshot.update_banner,
            UpdateBannerState::NoUpdate {
                selected_source: UpdateSource::Nexus
            }
        ));
    }

    #[test]
    fn overview_diagnostics_next_gen_allows_missing_steam_loader_and_missing_f4se_path() {
        let mut installation = installation(Fallout4InstallType::NextGen);
        installation.f4se_plugins_path = None;
        let discovery = report(
            installation,
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("11 23H2"),
        );
        let settings = settings(UpdateSource::Github);
        let binaries = vec![
            OverviewBinaryFact::new("Fallout4.exe", Fallout4InstallType::NextGen),
            OverviewBinaryFact::missing("f4se_steam_loader.dll"),
        ];
        let archives = vec![ArchiveRecord::new(
            "C:/Games/Fallout 4/Data/Fallout4 - Main.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Ready);
        assert_eq!(snapshot.top.version, Fallout4InstallType::NextGen);
        assert_eq!(snapshot.binaries.rows[1].severity, StatusSeverity::Neutral);
        assert_eq!(
            row_value(&snapshot.archives.rows, ARCHIVE_NEXT_GEN_VERSION_LABEL),
            1
        );
        assert!(snapshot.problems.is_empty());
        assert!(matches!(
            snapshot.update_banner,
            UpdateBannerState::NotChecked {
                selected_source: UpdateSource::Github
            }
        ));
    }

    #[test]
    fn overview_diagnostics_anniversary_snapshot_projects_version_and_available_update() {
        let discovery = report(
            installation(Fallout4InstallType::Anniversary),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("11 23H2"),
        );
        let settings = settings(UpdateSource::Both);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::Anniversary,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::Completed {
            releases: vec![UpdateRelease::new(UpdateProvider::Github, "0.7.1")],
        };

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Ready);
        assert_eq!(snapshot.top.version, Fallout4InstallType::Anniversary);
        assert_eq!(
            snapshot.update_banner.heading(),
            Some("An update is available:")
        );
        let UpdateBannerState::Available { releases, .. } = snapshot.update_banner else {
            panic!("expected available update banner");
        };
        assert_eq!(releases[0].display_label(), "v0.7.1 (GitHub)");
        assert_eq!(
            releases[0].action.target,
            OverviewDeferredActionTarget::Url(crate::domain::overview::GITHUB_LINK.to_owned())
        );
    }

    #[test]
    fn overview_diagnostics_missing_data_is_partial_not_fatal_even_without_f4se_path() {
        let discovery = report(
            installation_without_data(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::OldGen,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = OverviewEnablementFacts::default();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        let data_problem = snapshot
            .problems
            .iter()
            .find(|problem| problem.display_path == "Data")
            .expect("missing Data should produce a problem");
        assert_eq!(data_problem.problem, OverviewProblemType::FileNotFound);
        assert_eq!(data_problem.summary, MISSING_DATA_SUMMARY);
        assert!(matches!(
            snapshot.update_banner,
            UpdateBannerState::Disabled
        ));
    }

    #[test]
    fn overview_diagnostics_missing_enablement_files_and_address_library_are_reported() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::OldGen,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = OverviewEnablementFacts {
            fallout4_ccc: OverviewFilePresence::missing("C:/Games/Fallout 4/Fallout4.ccc"),
            plugins_txt: OverviewFilePresence::missing(
                "C:/Users/Example/AppData/Local/Fallout4/plugins.txt",
            ),
            address_library: OverviewAddressLibraryFact::missing(
                "C:/Games/Fallout 4/Data/F4SE/Plugins/version-1-10-163-0.bin",
                "F4SE/Plugins/version-1-10-163-0.bin",
            ),
        };
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        assert!(snapshot.problems.iter().any(|problem| {
            problem.display_path.ends_with("Fallout4.ccc") && problem.summary == MISSING_CCC_SUMMARY
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.display_path.ends_with("plugins.txt")
                && problem.summary == MISSING_PLUGINS_SUMMARY
                && problem.solution.as_deref() == Some("N/A")
        }));
        let address_problem = snapshot
            .problems
            .iter()
            .find(|problem| problem.summary == ADDRESS_LIBRARY_SUMMARY)
            .expect("missing Address Library should be reported");
        assert_eq!(address_problem.problem, OverviewProblemType::FileNotFound);
        assert_eq!(address_problem.links[0].url, ADDRESS_LIBRARY_LINK);
    }

    #[test]
    fn overview_diagnostics_unreadable_invalid_and_wrong_version_records_create_problems() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![
            OverviewBinaryFact::new("Fallout4.exe", Fallout4InstallType::Unknown)
                .with_version_metadata(
                    Some("not-a-version".to_owned()),
                    Some("FFFFFFFF".to_owned()),
                ),
        ];
        let archives = vec![
            ArchiveRecord::unreadable("C:/Games/Fallout 4/Data/Unreadable.ba2"),
            ArchiveRecord::new(
                "C:/Games/Fallout 4/Data/BadVersion.ba2",
                ArchiveFormat::General,
                ArchiveVersion::Unknown(99),
                true,
            ),
            ArchiveRecord::new(
                "C:/Games/Fallout 4/Data/BadFormat.ba2",
                ArchiveFormat::Unknown("NOPE".to_owned()),
                ArchiveVersion::OldGen,
                true,
            ),
            ArchiveRecord::new(
                "C:/Games/Fallout 4/Data/NextGenOnOld.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
        ];
        let modules = vec![
            ModuleRecord::unreadable("C:/Games/Fallout 4/Data/Unreadable.esp"),
            ModuleRecord::new(
                "C:/Games/Fallout 4/Data/BadHeader.esp",
                ModuleKind::Full,
                ModuleHeaderVersion::Unknown("0.94".to_owned()),
                true,
            ),
        ];
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::UnknownGameVersion
                && problem
                    .summary
                    .contains("not-a-version is an unknown version")
        }));
        assert_eq!(
            row_value(&snapshot.archives.rows, COUNT_UNREADABLE_LABEL),
            3
        );
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::InvalidArchive
                && problem.summary == ARCHIVE_UNREADABLE_SUMMARY
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::InvalidArchive
                && problem.summary == "Archive version (99) is not valid for Fallout 4."
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::InvalidArchive
                && problem.summary == "Archive format (NOPE) is not valid for Fallout 4."
        }));
        assert_eq!(row_value(&snapshot.modules.rows, COUNT_UNREADABLE_LABEL), 1);
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::InvalidModule
                && problem.summary.contains("Games supporting v0.94")
        }));
    }

    #[test]
    fn overview_diagnostics_old_gen_rejects_next_gen_archive_versions() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::OldGen,
        )];
        let archives = vec![ArchiveRecord::new(
            "C:/Games/Fallout 4/Data/NextGenOnOld.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        let archive_problem = snapshot
            .problems
            .iter()
            .find(|problem| problem.problem == OverviewProblemType::WrongVersion)
            .expect("known Old-Gen installs should reject v7/v8 archives");
        assert_eq!(archive_problem.summary, WRONG_ARCHIVE_VERSION_SUMMARY);
        assert_eq!(
            archive_problem.relative_path.as_deref(),
            Some(Path::new("NextGenOnOld.ba2"))
        );
    }

    #[test]
    fn overview_diagnostics_exceeded_limits_create_reference_problem_feed_entries() {
        let discovery = report(
            installation(Fallout4InstallType::NextGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("11 23H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::NextGen,
        )];
        let archives = (0..=MAX_ARCHIVES_GENERAL)
            .map(|index| {
                ArchiveRecord::new(
                    format!("C:/Games/Fallout 4/Data/General{index}.ba2"),
                    ArchiveFormat::General,
                    ArchiveVersion::OldGen,
                    true,
                )
            })
            .collect::<Vec<_>>();
        let modules = (0..=MAX_MODULES_LIGHT)
            .map(|index| {
                ModuleRecord::new(
                    format!("C:/Games/Fallout 4/Data/Light{index}.esl"),
                    ModuleKind::Light,
                    ModuleHeaderVersion::Version100,
                    true,
                )
            })
            .collect::<Vec<_>>();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::LimitExceeded
                && problem.display_path == "257 General Archives"
                && problem.links[0].url == UNPACKRR_LINK
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::LimitExceeded
                && problem.display_path == "4097 Light Modules"
                && problem.solution.as_deref() == Some(LIGHT_MODULE_LIMIT_SOLUTION)
        }));
        assert_eq!(row_value(&snapshot.archives.rows, COUNT_TOTAL_LABEL), 257);
        assert_eq!(row_value(&snapshot.modules.rows, COUNT_TOTAL_LABEL), 4097);
    }

    #[test]
    fn overview_diagnostics_vortex_identity_only_is_detected_with_partial_support_warning() {
        let discovery = report(
            installation(Fallout4InstallType::NextGen),
            Some(vortex()),
            windows_metadata("11 23H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::NextGen,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(
            snapshot.top.mod_manager.display_text(),
            "Vortex v0.0.0 [Profile: Unknown]"
        );
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::Custom("Partial Support".to_owned())
                && problem.severity == StatusSeverity::Warning
        }));
    }

    #[test]
    fn overview_diagnostics_mo2_windows_11_24h2_warning_matches_reference_threshold() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 2))),
            windows_metadata("11 24H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::OldGen,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::Custom("Compatibility Warning".to_owned())
                && problem.summary == MO2_WINDOWS_24H2_SUMMARY
                && problem.severity == StatusSeverity::Warning
        }));
    }

    #[test]
    fn overview_diagnostics_update_banner_states_follow_settings_and_worker_result() {
        let discovery = report(
            installation(Fallout4InstallType::OldGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("10 22H2"),
        );
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::OldGen,
        )];
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = enabled_files();

        let none_settings = settings(UpdateSource::None);
        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &none_settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &OverviewUpdateCheckState::Completed {
                releases: vec![UpdateRelease::new(UpdateProvider::NexusMods, "9.9.9")],
            },
        ));
        assert!(matches!(
            snapshot.update_banner,
            UpdateBannerState::Disabled
        ));

        let both_settings = settings(UpdateSource::Both);
        let checking = OverviewDiagnostics::build(input(
            &discovery,
            &both_settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &OverviewUpdateCheckState::Checking,
        ));
        assert!(matches!(
            checking.update_banner,
            UpdateBannerState::Checking {
                selected_source: UpdateSource::Both
            }
        ));

        let failed = OverviewDiagnostics::build(input(
            &discovery,
            &both_settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &OverviewUpdateCheckState::FailedSilently {
                failures: vec![UpdateCheckFailure::new(
                    UpdateProvider::NexusMods,
                    "request timed out",
                )],
            },
        ));
        let UpdateBannerState::FailedSilently { failures, .. } = failed.update_banner else {
            panic!("expected failed silently update state");
        };
        assert_eq!(failures[0].summary, "request timed out");
    }

    #[test]
    fn overview_diagnostics_discovery_manager_and_system_errors_degrade_to_safe_snapshot() {
        let discovery = error_report();
        let settings = settings(UpdateSource::Github);
        let binaries = Vec::new();
        let archives = Vec::new();
        let modules = Vec::new();
        let enablement = OverviewEnablementFacts::default();
        let update = OverviewUpdateCheckState::NotChecked;
        let feedback = OverviewDesktopActionFeedback::failed(
            OverviewDeferredActionKind::OpenGamePath,
            "Path open failed.",
        );

        let snapshot = OverviewDiagnostics::build(OverviewDiagnosticsInput {
            discovery: &discovery,
            settings: &settings,
            binaries: &binaries,
            archives: &archives,
            modules: &modules,
            enablement: &enablement,
            update: &update,
            last_desktop_action: Some(&feedback),
        });

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Partial);
        assert_eq!(snapshot.top.game_path.display_text(), "Not Found");
        assert_eq!(snapshot.top.version, Fallout4InstallType::NotFound);
        assert_eq!(snapshot.top.pc_specs_display_text(), "Unknown");
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::FileNotFound
                && problem.summary.contains("The path set in your registry")
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::Custom("Mod Manager Error".to_owned())
                && problem.summary == "Process inspection failed."
        }));
        assert!(snapshot.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::Custom("System Metadata Unavailable".to_owned())
                && problem.summary == "System metadata read failed."
        }));
        assert_eq!(
            snapshot
                .last_action_error
                .as_ref()
                .map(|error| &error.summary),
            Some(&"Path open failed.".to_owned())
        );
    }

    #[test]
    fn overview_diagnostics_boundary_counts_do_not_create_limit_problems() {
        let discovery = report(
            installation(Fallout4InstallType::NextGen),
            Some(mo2(SemanticVersion::new(2, 5, 3))),
            windows_metadata("11 23H2"),
        );
        let settings = settings(UpdateSource::None);
        let binaries = vec![OverviewBinaryFact::new(
            "Fallout4.exe",
            Fallout4InstallType::NextGen,
        )];
        let archives = (0..MAX_ARCHIVES_GENERAL)
            .map(|index| {
                ArchiveRecord::new(
                    format!("C:/Games/Fallout 4/Data/General{index}.ba2"),
                    ArchiveFormat::General,
                    ArchiveVersion::OldGen,
                    true,
                )
            })
            .collect::<Vec<_>>();
        let modules = Vec::new();
        let enablement = enabled_files();
        let update = OverviewUpdateCheckState::NotChecked;

        let snapshot = OverviewDiagnostics::build(input(
            &discovery,
            &settings,
            &binaries,
            &archives,
            &modules,
            &enablement,
            &update,
        ));

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Ready);
        assert_eq!(
            row_value(&snapshot.archives.rows, ARCHIVE_GENERAL_LABEL),
            256
        );
        assert!(
            !snapshot
                .problems
                .iter()
                .any(|problem| problem.problem == OverviewProblemType::LimitExceeded)
        );
    }

    #[test]
    fn overview_diagnostics_imports_reference_mod_manager_kind_for_public_surface() {
        assert_eq!(ModManagerKind::Vortex.display_name(), "Vortex");
        assert_eq!(
            NEXUS_MODS_LINK,
            "https://www.nexusmods.com/fallout4/mods/87907"
        );
    }
}
