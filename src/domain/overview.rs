//! Pure Overview-tab snapshot and projection contracts.
//!
//! The reference Overview tab combines discovery results, PC metadata, update
//! checks, binary/archive/module summaries, and scanner problem records. This
//! module defines that boundary as inert Rust data so tests and later worker/UI
//! code can construct Overview state without touching Slint, the filesystem,
//! registry, process table, or network.

use std::path::{Path, PathBuf};

use crate::{
    domain::{
        discovery::{
            ArchiveFormat, ArchiveRecord, ArchiveVersion, Fallout4InstallType,
            Fallout4Installation, ModuleHeaderVersion, ModuleKind, ModuleRecord,
        },
        settings::UpdateSource,
    },
    platform::process::SystemMetadata,
    services::discovery::DiscoveredModManager,
};

/// User-facing label for the mod-manager top status row.
pub const TOP_ROW_MOD_MANAGER_LABEL: &str = "Mod Manager";
/// User-facing label for the game-path top status row.
pub const TOP_ROW_GAME_PATH_LABEL: &str = "Game Path";
/// User-facing label for the game-version top status row.
pub const TOP_ROW_VERSION_LABEL: &str = "Version";
/// User-facing label for the PC-specs top status row.
pub const TOP_ROW_PC_SPECS_LABEL: &str = "PC Specs";
/// Reference top-row order from `CMT/src/tabs/_overview.py`.
pub const TOP_STATUS_LABELS: [&str; 4] = [
    TOP_ROW_MOD_MANAGER_LABEL,
    TOP_ROW_GAME_PATH_LABEL,
    TOP_ROW_VERSION_LABEL,
    TOP_ROW_PC_SPECS_LABEL,
];

/// User-facing title for the binary summary panel.
pub const BINARY_PANEL_TITLE: &str = "Binaries (EXE/DLL/BIN)";
/// User-facing title for the archive summary panel.
pub const ARCHIVE_PANEL_TITLE: &str = "Archives (BA2)";
/// User-facing title for the module summary panel.
pub const MODULE_PANEL_TITLE: &str = "Modules (ESM/ESL/ESP)";

/// User-facing label for the Address Library binary row.
pub const BINARY_ADDRESS_LIBRARY_LABEL: &str = "Address Library";
/// User-facing archive row label for General/GNRL archives.
pub const ARCHIVE_GENERAL_LABEL: &str = "General";
/// User-facing archive row label for Texture/DX10 archives.
pub const ARCHIVE_TEXTURE_LABEL: &str = "Texture";
/// User-facing total row label used by archive and module panels.
pub const COUNT_TOTAL_LABEL: &str = "Total";
/// User-facing unreadable row label used by archive and module panels.
pub const COUNT_UNREADABLE_LABEL: &str = "Unreadable";
/// User-facing archive-version row label for v1 BA2 files.
pub const ARCHIVE_OLD_GEN_VERSION_LABEL: &str = "v1 (OG)";
/// User-facing archive-version row label for v7/v8 BA2 files.
pub const ARCHIVE_NEXT_GEN_VERSION_LABEL: &str = "v7/8 (NG)";
/// Reference archive count row order, excluding visual separators.
pub const ARCHIVE_COUNT_LABELS: [&str; 6] = [
    ARCHIVE_GENERAL_LABEL,
    ARCHIVE_TEXTURE_LABEL,
    COUNT_TOTAL_LABEL,
    COUNT_UNREADABLE_LABEL,
    ARCHIVE_OLD_GEN_VERSION_LABEL,
    ARCHIVE_NEXT_GEN_VERSION_LABEL,
];

/// User-facing module row label for full plugins.
pub const MODULE_FULL_LABEL: &str = "Full";
/// User-facing module row label for light plugins.
pub const MODULE_LIGHT_LABEL: &str = "Light";
/// User-facing module row label for HEDR v1.00 plugins.
pub const MODULE_HEDR_100_LABEL: &str = "HEDR v1.00";
/// User-facing module row label for HEDR v0.95 plugins.
pub const MODULE_HEDR_095_LABEL: &str = "HEDR v0.95";
/// User-facing module row label for unsupported/unknown HEDR versions.
pub const MODULE_HEDR_UNKNOWN_LABEL: &str = "HEDR v????";
/// Reference module count row order, excluding visual separators.
pub const MODULE_COUNT_LABELS: [&str; 7] = [
    MODULE_FULL_LABEL,
    MODULE_LIGHT_LABEL,
    COUNT_TOTAL_LABEL,
    COUNT_UNREADABLE_LABEL,
    MODULE_HEDR_100_LABEL,
    MODULE_HEDR_095_LABEL,
    MODULE_HEDR_UNKNOWN_LABEL,
];

/// Reference full-plugin limit used for Overview warning colors.
pub const MAX_MODULES_FULL: usize = 254;
/// Reference light-plugin limit used for Overview warning colors.
pub const MAX_MODULES_LIGHT: usize = 4096;
/// Reference General/GNRL BA2 limit used for Overview warning colors.
pub const MAX_ARCHIVES_GENERAL: usize = 256;
/// Reference Texture/DX10 BA2 limit used for Overview warning colors.
pub const MAX_ARCHIVES_TEXTURE: usize = 255;

/// Deferred utility button label in the binary panel.
pub const ACTION_DOWNGRADE_MANAGER_LABEL: &str = "Downgrade Manager...";
/// Deferred utility button label in the archive panel.
pub const ACTION_ARCHIVE_PATCHER_LABEL: &str = "Archive Patcher...";
/// Reference update banner heading when at least one source reports a newer version.
pub const UPDATE_AVAILABLE_HEADING: &str = "An update is available:";
/// Reference Nexus Mods project URL for update links.
pub const NEXUS_MODS_LINK: &str = "https://www.nexusmods.com/fallout4/mods/87907";
/// Reference GitHub project URL for update links.
pub const GITHUB_LINK: &str = "https://github.com/wxMichael/Collective-Modding-Toolkit";

/// Severity/color intent for Overview status rows and problem records.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StatusSeverity {
    /// Positive/healthy state, matching the reference green rows.
    Good,
    /// Caution state, matching the reference orange threshold rows.
    Warning,
    /// Error/problem state, matching the reference red rows.
    Error,
    /// Informational state used for non-problem details and links.
    Info,
    /// Present but neither good nor bad, matching the reference neutral rows.
    Neutral,
    /// Input was absent or not classified yet; this is the default safe fallback.
    #[default]
    Unknown,
}

impl StatusSeverity {
    /// Returns true when this severity should be treated as a problem signal.
    pub const fn is_problem(self) -> bool {
        matches!(self, Self::Warning | Self::Error)
    }
}

/// High-level lifecycle state for an Overview refresh.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OverviewRefreshPhase {
    /// No refresh has run yet.
    #[default]
    Idle,
    /// A worker is collecting data for the Overview snapshot.
    Loading,
    /// A refresh completed with enough data to render the main panels.
    Ready,
    /// A refresh completed with recoverable missing or malformed inputs.
    Partial,
    /// A refresh failed at the snapshot level while preserving safe diagnostics.
    Error,
}

/// Safe, user-facing refresh state displayed by the Overview UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewRefreshState {
    /// Current refresh lifecycle phase.
    pub phase: OverviewRefreshPhase,
    /// Safe status text for the UI; raw OS/network diagnostics belong in logs.
    pub message: Option<String>,
}

impl OverviewRefreshState {
    /// Creates the default idle refresh state.
    pub fn idle() -> Self {
        Self {
            phase: OverviewRefreshPhase::Idle,
            message: None,
        }
    }

    /// Creates a loading refresh state with a safe message.
    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            phase: OverviewRefreshPhase::Loading,
            message: Some(message.into()),
        }
    }

    /// Creates a ready refresh state with an optional safe message.
    pub fn ready(message: impl Into<Option<String>>) -> Self {
        Self {
            phase: OverviewRefreshPhase::Ready,
            message: message.into(),
        }
    }

    /// Creates a partial refresh state for recoverable missing/malformed inputs.
    pub fn partial(message: impl Into<String>) -> Self {
        Self {
            phase: OverviewRefreshPhase::Partial,
            message: Some(message.into()),
        }
    }

    /// Creates an error refresh state without exposing raw diagnostics.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            phase: OverviewRefreshPhase::Error,
            message: Some(message.into()),
        }
    }

    /// Returns true while a background Overview worker is expected to be active.
    pub const fn is_busy(&self) -> bool {
        matches!(self.phase, OverviewRefreshPhase::Loading)
    }

    /// Returns the severity implied by this refresh state.
    pub const fn severity(&self) -> StatusSeverity {
        match self.phase {
            OverviewRefreshPhase::Idle => StatusSeverity::Unknown,
            OverviewRefreshPhase::Loading => StatusSeverity::Info,
            OverviewRefreshPhase::Ready => StatusSeverity::Good,
            OverviewRefreshPhase::Partial => StatusSeverity::Warning,
            OverviewRefreshPhase::Error => StatusSeverity::Error,
        }
    }
}

impl Default for OverviewRefreshState {
    fn default() -> Self {
        Self::idle()
    }
}

/// Top-row status categories in the exact reference display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewTopStatusKind {
    /// Mod manager detection status.
    ModManager,
    /// Fallout 4 installation path status.
    GamePath,
    /// Fallout 4 executable version/install-type status.
    Version,
    /// PC specs/system metadata status.
    PcSpecs,
}

impl OverviewTopStatusKind {
    /// Returns top status kinds in the order rendered by the reference Overview tab.
    pub const fn reference_order() -> [Self; 4] {
        [
            Self::ModManager,
            Self::GamePath,
            Self::Version,
            Self::PcSpecs,
        ]
    }

    /// Returns the exact label text without the colon added by the UI layer.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ModManager => TOP_ROW_MOD_MANAGER_LABEL,
            Self::GamePath => TOP_ROW_GAME_PATH_LABEL,
            Self::Version => TOP_ROW_VERSION_LABEL,
            Self::PcSpecs => TOP_ROW_PC_SPECS_LABEL,
        }
    }
}

/// Render-ready top status row produced from typed Overview state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewTopStatusRow {
    /// Row category.
    pub kind: OverviewTopStatusKind,
    /// Exact reference label without trailing colon.
    pub label: &'static str,
    /// Display value for the row.
    pub value: String,
    /// Severity/color intent for the value.
    pub severity: StatusSeverity,
    /// Optional deferred action tied to the row, such as opening the game path.
    pub action: Option<OverviewDeferredAction>,
}

/// Mod-manager status captured for the top row without process access.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OverviewModManagerStatus {
    /// No supported manager was detected.
    #[default]
    NotFound,
    /// A supported manager was detected or parsed by discovery services.
    Detected(Box<DiscoveredModManager>),
}

impl OverviewModManagerStatus {
    /// Creates a detected-manager status from discovery output.
    pub fn detected(manager: DiscoveredModManager) -> Self {
        Self::Detected(Box::new(manager))
    }

    /// Returns the reference-compatible display text.
    pub fn display_text(&self) -> String {
        match self {
            Self::NotFound => "Not Found".to_owned(),
            Self::Detected(manager) => {
                let profile = match manager.as_ref() {
                    DiscoveredModManager::ModOrganizer(configuration) => {
                        configuration.context.selected_profile.as_str()
                    }
                    DiscoveredModManager::Vortex(_) => "Unknown",
                };
                format!(
                    "{} v{} [Profile: {}]",
                    manager.display_name(),
                    manager.version(),
                    profile
                )
            }
        }
    }

    /// Returns the severity used by the reference for the mod-manager row.
    pub const fn severity(&self) -> StatusSeverity {
        match self {
            Self::NotFound => StatusSeverity::Error,
            Self::Detected(_) => StatusSeverity::Neutral,
        }
    }
}

/// Game path status captured for the top row.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OverviewGamePathStatus {
    /// Discovery has not produced a usable Fallout 4 path.
    #[default]
    NotFound,
    /// Discovery produced a game path.
    Found(PathBuf),
}

impl OverviewGamePathStatus {
    /// Creates a found game-path status.
    pub fn found(path: impl Into<PathBuf>) -> Self {
        Self::Found(path.into())
    }

    /// Returns the display text for the game-path row.
    pub fn display_text(&self) -> String {
        match self {
            Self::NotFound => "Not Found".to_owned(),
            Self::Found(path) => display_path(path),
        }
    }

    /// Returns the severity for the game-path row.
    pub const fn severity(&self) -> StatusSeverity {
        match self {
            Self::NotFound => StatusSeverity::Error,
            Self::Found(_) => StatusSeverity::Neutral,
        }
    }
}

/// Top Overview rows backed by typed discovery and system metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewTopStatus {
    /// Mod-manager status.
    pub mod_manager: OverviewModManagerStatus,
    /// Fallout 4 game-path status.
    pub game_path: OverviewGamePathStatus,
    /// Fallout 4 install type/version class.
    pub version: Fallout4InstallType,
    /// Fakeable PC specs/system metadata.
    pub system_metadata: Option<SystemMetadata>,
}

impl OverviewTopStatus {
    /// Creates top status rows from already-collected typed values.
    pub fn new(
        mod_manager: OverviewModManagerStatus,
        game_path: OverviewGamePathStatus,
        version: Fallout4InstallType,
        system_metadata: Option<SystemMetadata>,
    ) -> Self {
        Self {
            mod_manager,
            game_path,
            version,
            system_metadata,
        }
    }

    /// Returns top status rows in the reference display order.
    pub fn rows(&self) -> Vec<OverviewTopStatusRow> {
        OverviewTopStatusKind::reference_order()
            .into_iter()
            .map(|kind| match kind {
                OverviewTopStatusKind::ModManager => OverviewTopStatusRow {
                    kind,
                    label: kind.label(),
                    value: self.mod_manager.display_text(),
                    severity: self.mod_manager.severity(),
                    action: None,
                },
                OverviewTopStatusKind::GamePath => OverviewTopStatusRow {
                    kind,
                    label: kind.label(),
                    value: self.game_path.display_text(),
                    severity: self.game_path.severity(),
                    action: match &self.game_path {
                        OverviewGamePathStatus::Found(path) => {
                            Some(OverviewDeferredAction::open_path(
                                OverviewDeferredActionKind::OpenGamePath,
                                TOP_ROW_GAME_PATH_LABEL,
                                path.clone(),
                            ))
                        }
                        OverviewGamePathStatus::NotFound => None,
                    },
                },
                OverviewTopStatusKind::Version => OverviewTopStatusRow {
                    kind,
                    label: kind.label(),
                    value: self.version.to_string(),
                    severity: install_type_severity(self.version),
                    action: None,
                },
                OverviewTopStatusKind::PcSpecs => OverviewTopStatusRow {
                    kind,
                    label: kind.label(),
                    value: self.pc_specs_display_text(),
                    severity: if self.system_metadata.is_some() {
                        StatusSeverity::Neutral
                    } else {
                        StatusSeverity::Unknown
                    },
                    action: None,
                },
            })
            .collect()
    }

    /// Returns a compact PC specs text derived from fakeable metadata.
    pub fn pc_specs_display_text(&self) -> String {
        let Some(metadata) = &self.system_metadata else {
            return "Unknown".to_owned();
        };

        let os = match metadata.os_version.as_deref() {
            Some(version) if !version.is_empty() => format!("{} {}", metadata.os_name, version),
            _ => metadata.os_name.clone(),
        };
        let memory = metadata
            .physical_memory_bytes
            .map(format_gib)
            .unwrap_or_else(|| "Unknown RAM".to_owned());
        let cpu = metadata
            .cpu_brand
            .clone()
            .unwrap_or_else(|| "Unknown CPU".to_owned());
        format!("{os}\n{memory}\n{cpu}")
    }
}

impl Default for OverviewTopStatus {
    fn default() -> Self {
        Self::new(
            OverviewModManagerStatus::NotFound,
            OverviewGamePathStatus::NotFound,
            Fallout4InstallType::Unknown,
            None,
        )
    }
}

/// Availability status for the Address Library row.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OverviewAvailability {
    /// The item is installed/present.
    Installed,
    /// The item is missing/not found.
    NotFound,
    /// The item has not been checked yet.
    #[default]
    Unknown,
}

impl OverviewAvailability {
    /// Returns the reference-compatible display text for availability rows.
    pub const fn display_text(self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::NotFound => "Not Found",
            Self::Unknown => "Unknown",
        }
    }

    /// Returns the severity for availability rows.
    pub const fn severity(self) -> StatusSeverity {
        match self {
            Self::Installed => StatusSeverity::Good,
            Self::NotFound => StatusSeverity::Error,
            Self::Unknown => StatusSeverity::Unknown,
        }
    }
}

/// Binary/executable row state used by the Overview binary panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryStatusRow {
    /// Reference row label, usually the file stem without extension.
    pub label: String,
    /// Optional absolute path for scanner/problem handoff.
    pub path: Option<PathBuf>,
    /// Classified install type for this binary.
    pub install_type: Fallout4InstallType,
    /// Optional version string displayed on hover by the reference UI.
    pub version: Option<String>,
    /// Optional CRC/hash fallback displayed on hover by the reference UI.
    pub hash: Option<String>,
    /// Severity/color intent for the row.
    pub severity: StatusSeverity,
}

impl BinaryStatusRow {
    /// Creates a binary status row from already-classified file information.
    pub fn new(label: impl Into<String>, install_type: Fallout4InstallType) -> Self {
        Self {
            label: label.into(),
            path: None,
            install_type,
            version: None,
            hash: None,
            severity: install_type_severity(install_type),
        }
    }

    /// Attaches a path to this binary row.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Attaches optional version/hash metadata to this binary row.
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

/// Binary panel summary, including the deferred Downgrade Manager utility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryPanelSummary {
    /// Exact reference panel title.
    pub title: &'static str,
    /// Binary rows in the order supplied by the classifier.
    pub rows: Vec<BinaryStatusRow>,
    /// Address Library row availability.
    pub address_library: OverviewAvailability,
    /// Deferred actions placed in this panel.
    pub actions: Vec<OverviewDeferredAction>,
}

impl BinaryPanelSummary {
    /// Creates an empty binary panel with reference utility placement preserved.
    pub fn empty() -> Self {
        Self {
            title: BINARY_PANEL_TITLE,
            rows: Vec::new(),
            address_library: OverviewAvailability::Unknown,
            actions: vec![OverviewDeferredAction::utility(
                OverviewDeferredActionKind::OpenDowngradeManager,
                ACTION_DOWNGRADE_MANAGER_LABEL,
            )],
        }
    }
}

impl Default for BinaryPanelSummary {
    fn default() -> Self {
        Self::empty()
    }
}

/// Count row used by archive and module Overview panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewCountRow {
    /// Exact reference row label without the trailing colon.
    pub label: &'static str,
    /// Count value to display.
    pub value: usize,
    /// Optional reference limit shown beside the count.
    pub limit: Option<usize>,
    /// Severity/color intent for the count.
    pub severity: StatusSeverity,
}

impl OverviewCountRow {
    /// Creates a count row without a limit.
    pub const fn new(label: &'static str, value: usize, severity: StatusSeverity) -> Self {
        Self {
            label,
            value,
            limit: None,
            severity,
        }
    }

    /// Creates a count row with reference 95% warning-threshold semantics.
    pub fn limited(label: &'static str, value: usize, limit: usize) -> Self {
        Self {
            label,
            value,
            limit: Some(limit),
            severity: limit_severity(value, limit),
        }
    }
}

/// Archive panel summary generated from typed archive records or fake test data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePanelSummary {
    /// Exact reference panel title.
    pub title: &'static str,
    /// Count/version rows in reference order.
    pub rows: Vec<OverviewCountRow>,
    /// Deferred actions placed in this panel.
    pub actions: Vec<OverviewDeferredAction>,
}

impl ArchivePanelSummary {
    /// Creates an archive panel with zero counts and reference row order.
    pub fn empty() -> Self {
        Self::from_counts(ArchivePanelCounts::default())
    }

    /// Creates an archive panel from explicit counts.
    pub fn from_counts(counts: ArchivePanelCounts) -> Self {
        Self {
            title: ARCHIVE_PANEL_TITLE,
            rows: vec![
                OverviewCountRow::limited(
                    ARCHIVE_GENERAL_LABEL,
                    counts.general,
                    MAX_ARCHIVES_GENERAL,
                ),
                OverviewCountRow::limited(
                    ARCHIVE_TEXTURE_LABEL,
                    counts.texture,
                    MAX_ARCHIVES_TEXTURE,
                ),
                OverviewCountRow::limited(
                    COUNT_TOTAL_LABEL,
                    counts.general + counts.texture,
                    MAX_ARCHIVES_GENERAL + MAX_ARCHIVES_TEXTURE,
                ),
                OverviewCountRow::new(
                    COUNT_UNREADABLE_LABEL,
                    counts.unreadable,
                    non_zero_error_or_neutral(counts.unreadable),
                ),
                OverviewCountRow::new(
                    ARCHIVE_OLD_GEN_VERSION_LABEL,
                    counts.old_gen_version,
                    StatusSeverity::Neutral,
                ),
                OverviewCountRow::new(
                    ARCHIVE_NEXT_GEN_VERSION_LABEL,
                    counts.next_gen_version,
                    StatusSeverity::Neutral,
                ),
            ],
            actions: vec![OverviewDeferredAction::utility(
                OverviewDeferredActionKind::OpenArchivePatcher,
                ACTION_ARCHIVE_PATCHER_LABEL,
            )],
        }
    }

    /// Creates an archive panel by counting supplied archive records.
    pub fn from_records(records: &[ArchiveRecord]) -> Self {
        let mut counts = ArchivePanelCounts::default();

        for record in records
            .iter()
            .filter(|record| record.enabled || !record.readable)
        {
            let invalid_or_unreadable = !record.readable
                || matches!(record.format, ArchiveFormat::Unknown(_))
                || matches!(record.version, ArchiveVersion::Unknown(_));
            if invalid_or_unreadable {
                counts.unreadable += 1;
                continue;
            }

            match record.format {
                ArchiveFormat::General => counts.general += 1,
                ArchiveFormat::DirectX10 => counts.texture += 1,
                ArchiveFormat::Unknown(_) => {}
            }

            match record.version {
                ArchiveVersion::OldGen => counts.old_gen_version += 1,
                ArchiveVersion::NextGen7 | ArchiveVersion::NextGen8 => counts.next_gen_version += 1,
                ArchiveVersion::Unknown(_) => {}
            }
        }

        Self::from_counts(counts)
    }
}

impl Default for ArchivePanelSummary {
    fn default() -> Self {
        Self::empty()
    }
}

/// Explicit archive counts used by pure tests and future projection code.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ArchivePanelCounts {
    /// Enabled General/GNRL BA2 count.
    pub general: usize,
    /// Enabled Texture/DX10 BA2 count.
    pub texture: usize,
    /// Unreadable or invalid enabled BA2 count.
    pub unreadable: usize,
    /// v1 BA2 count.
    pub old_gen_version: usize,
    /// v7/v8 BA2 count.
    pub next_gen_version: usize,
}

/// Module panel summary generated from typed module records or fake test data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulePanelSummary {
    /// Exact reference panel title.
    pub title: &'static str,
    /// Count/header rows in reference order.
    pub rows: Vec<OverviewCountRow>,
    /// Deferred actions placed in this panel.
    pub actions: Vec<OverviewDeferredAction>,
}

impl ModulePanelSummary {
    /// Creates a module panel with zero counts and reference row order.
    pub fn empty() -> Self {
        Self::from_counts(ModulePanelCounts::default())
    }

    /// Creates a module panel from explicit counts.
    pub fn from_counts(counts: ModulePanelCounts) -> Self {
        Self {
            title: MODULE_PANEL_TITLE,
            rows: vec![
                OverviewCountRow::limited(MODULE_FULL_LABEL, counts.full, MAX_MODULES_FULL),
                OverviewCountRow::limited(MODULE_LIGHT_LABEL, counts.light, MAX_MODULES_LIGHT),
                OverviewCountRow::limited(
                    COUNT_TOTAL_LABEL,
                    counts.full + counts.light,
                    MAX_MODULES_FULL + MAX_MODULES_LIGHT,
                ),
                OverviewCountRow::new(
                    COUNT_UNREADABLE_LABEL,
                    counts.unreadable,
                    non_zero_error_or_neutral(counts.unreadable),
                ),
                OverviewCountRow::new(
                    MODULE_HEDR_100_LABEL,
                    counts.hedr_100,
                    StatusSeverity::Neutral,
                ),
                OverviewCountRow::new(
                    MODULE_HEDR_095_LABEL,
                    counts.hedr_095,
                    StatusSeverity::Neutral,
                ),
                OverviewCountRow::new(
                    MODULE_HEDR_UNKNOWN_LABEL,
                    counts.hedr_unknown,
                    non_zero_error_or_neutral(counts.hedr_unknown),
                ),
            ],
            actions: Vec::new(),
        }
    }

    /// Creates a module panel by counting supplied module records.
    pub fn from_records(records: &[ModuleRecord]) -> Self {
        let mut counts = ModulePanelCounts::default();

        for record in records
            .iter()
            .filter(|record| record.enabled || !record.readable)
        {
            if !record.readable {
                counts.unreadable += 1;
                continue;
            }

            match record.kind {
                ModuleKind::Full => counts.full += 1,
                ModuleKind::Light => counts.light += 1,
            }

            match &record.header_version {
                ModuleHeaderVersion::Version100 => counts.hedr_100 += 1,
                ModuleHeaderVersion::Version095 => counts.hedr_095 += 1,
                ModuleHeaderVersion::Unknown(_) => counts.hedr_unknown += 1,
            }
        }

        Self::from_counts(counts)
    }
}

impl Default for ModulePanelSummary {
    fn default() -> Self {
        Self::empty()
    }
}

/// Explicit module counts used by pure tests and future projection code.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModulePanelCounts {
    /// Enabled full plugin count.
    pub full: usize,
    /// Enabled light plugin count.
    pub light: usize,
    /// Unreadable or structurally invalid plugin count.
    pub unreadable: usize,
    /// HEDR v1.00 plugin count.
    pub hedr_100: usize,
    /// HEDR v0.95 plugin count.
    pub hedr_095: usize,
    /// Unsupported/unknown HEDR plugin count.
    pub hedr_unknown: usize,
}

/// Update provider names used by the reference update banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateProvider {
    /// Nexus Mods update source.
    NexusMods,
    /// GitHub update source.
    Github,
}

impl UpdateProvider {
    /// Returns the exact provider label used in the update banner.
    pub const fn label(self) -> &'static str {
        match self {
            Self::NexusMods => "NexusMods",
            Self::Github => "GitHub",
        }
    }

    /// Returns the reference URL opened for this provider.
    pub const fn url(self) -> &'static str {
        match self {
            Self::NexusMods => NEXUS_MODS_LINK,
            Self::Github => GITHUB_LINK,
        }
    }
}

/// A single newer release shown in the update banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRelease {
    /// Source that reported the newer version.
    pub provider: UpdateProvider,
    /// Version text without the leading `v`.
    pub version: String,
    /// Deferred open-link action for this release.
    pub action: OverviewDeferredAction,
}

impl UpdateRelease {
    /// Creates a release entry with the reference provider URL action.
    pub fn new(provider: UpdateProvider, version: impl Into<String>) -> Self {
        let version = version.into();
        Self {
            provider,
            action: OverviewDeferredAction::open_url(
                OverviewDeferredActionKind::OpenUpdateProvider(provider),
                format!("v{version} ({})", provider.label()),
                provider.url(),
            ),
            version,
        }
    }

    /// Returns the exact link label shape used by the reference banner.
    pub fn display_label(&self) -> String {
        format!("v{} ({})", self.version, self.provider.label())
    }
}

/// Non-fatal update-check failure retained for observability without showing a banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckFailure {
    /// Provider that failed.
    pub provider: UpdateProvider,
    /// Safe diagnostic summary for logs or later UI diagnostics.
    pub summary: String,
}

impl UpdateCheckFailure {
    /// Creates a safe update-check failure record.
    pub fn new(provider: UpdateProvider, summary: impl Into<String>) -> Self {
        Self {
            provider,
            summary: summary.into(),
        }
    }
}

/// Reference update banner state.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum UpdateBannerState {
    /// Update checks are disabled by settings (`update_source = none`).
    #[default]
    Disabled,
    /// A configured source exists, but no check has run yet.
    NotChecked {
        /// Selected update source from settings.
        selected_source: UpdateSource,
    },
    /// A worker is checking configured update sources.
    Checking {
        /// Selected update source from settings.
        selected_source: UpdateSource,
    },
    /// Checks completed without newer versions.
    NoUpdate {
        /// Selected update source from settings.
        selected_source: UpdateSource,
    },
    /// At least one configured source reported a newer version.
    Available {
        /// Selected update source from settings.
        selected_source: UpdateSource,
        /// Release entries in banner display order.
        releases: Vec<UpdateRelease>,
    },
    /// One or more configured checks failed, matching the reference silent failure behavior.
    FailedSilently {
        /// Selected update source from settings.
        selected_source: UpdateSource,
        /// Safe per-source failure summaries for logs/diagnostics.
        failures: Vec<UpdateCheckFailure>,
    },
}

impl UpdateBannerState {
    /// Creates the initial banner state for a selected update source.
    pub fn from_update_source(selected_source: UpdateSource) -> Self {
        if matches!(selected_source, UpdateSource::None) {
            Self::Disabled
        } else {
            Self::NotChecked { selected_source }
        }
    }

    /// Creates an available banner, or no-update state when no releases are present.
    pub fn available_or_no_update(
        selected_source: UpdateSource,
        releases: Vec<UpdateRelease>,
    ) -> Self {
        if releases.is_empty() {
            Self::NoUpdate { selected_source }
        } else {
            Self::Available {
                selected_source,
                releases,
            }
        }
    }

    /// Creates a silent-failure state for configured sources.
    pub fn failed_silently(
        selected_source: UpdateSource,
        failures: Vec<UpdateCheckFailure>,
    ) -> Self {
        Self::FailedSilently {
            selected_source,
            failures,
        }
    }

    /// Returns true only when the reference banner should be visible.
    pub fn is_visible(&self) -> bool {
        matches!(self, Self::Available { releases, .. } if !releases.is_empty())
    }

    /// Returns the update banner heading when visible.
    pub fn heading(&self) -> Option<&'static str> {
        self.is_visible().then_some(UPDATE_AVAILABLE_HEADING)
    }
}

/// Deferred action categories that later UI code can bind without invoking work inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewDeferredActionKind {
    /// Open the discovered game path in the desktop shell.
    OpenGamePath,
    /// Show detected mod-manager details.
    OpenModManagerDetails,
    /// Open an update provider link.
    OpenUpdateProvider(UpdateProvider),
    /// Open the Downgrade Manager utility.
    OpenDowngradeManager,
    /// Open the Archive Patcher utility.
    OpenArchivePatcher,
    /// Show invalid module-version details.
    ShowInvalidModuleVersions,
    /// Open a problem-specific external link.
    OpenProblemLink,
}

/// Target data needed to execute a deferred action later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewDeferredActionTarget {
    /// No external target; the action opens an internal utility/dialog.
    Internal,
    /// Desktop path to open.
    Path(PathBuf),
    /// URL to open.
    Url(String),
}

/// UI-safe action descriptor; executing the action belongs outside the domain module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewDeferredAction {
    /// Typed action kind.
    pub kind: OverviewDeferredActionKind,
    /// Button/link label.
    pub label: String,
    /// Target needed by a later UI or platform adapter.
    pub target: OverviewDeferredActionTarget,
    /// Whether the UI should enable this action.
    pub enabled: bool,
}

impl OverviewDeferredAction {
    /// Creates an enabled internal utility action.
    pub fn utility(kind: OverviewDeferredActionKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            target: OverviewDeferredActionTarget::Internal,
            enabled: true,
        }
    }

    /// Creates an enabled desktop-path action.
    pub fn open_path(
        kind: OverviewDeferredActionKind,
        label: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            target: OverviewDeferredActionTarget::Path(path.into()),
            enabled: true,
        }
    }

    /// Creates an enabled URL action.
    pub fn open_url(
        kind: OverviewDeferredActionKind,
        label: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            target: OverviewDeferredActionTarget::Url(url.into()),
            enabled: true,
        }
    }
}

/// Problem sources used by scanner-ready Overview records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewProblemSource {
    /// Problem originated in top-level discovery or top rows.
    TopStatus,
    /// Problem originated in binary/executable classification.
    Binaries,
    /// Problem originated in archive classification.
    Archives,
    /// Problem originated in module/plugin classification.
    Modules,
    /// Problem originated in count limit checks.
    CountLimit,
    /// Problem originated in update-check state.
    Updates,
    /// Problem was supplied by scanner integration.
    Scanner,
}

/// Reference-compatible problem labels used by Overview and scanner handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewProblemType {
    /// Reference `Invalid Archive` problem.
    InvalidArchive,
    /// Reference `Invalid Module` problem.
    InvalidModule,
    /// Reference `File Not Found` problem.
    FileNotFound,
    /// Reference `Wrong Version` problem.
    WrongVersion,
    /// Reference `Limit Exceeded` simple problem.
    LimitExceeded,
    /// Overview-specific no-manager problem.
    NoModManager,
    /// Overview-specific unknown game-version problem.
    UnknownGameVersion,
    /// Any later scanner/reference problem not yet modeled as a variant.
    Custom(String),
}

impl OverviewProblemType {
    /// Returns the user-facing problem label.
    pub fn label(&self) -> &str {
        match self {
            Self::InvalidArchive => "Invalid Archive",
            Self::InvalidModule => "Invalid Module",
            Self::FileNotFound => "File Not Found",
            Self::WrongVersion => "Wrong Version",
            Self::LimitExceeded => "Limit Exceeded",
            Self::NoModManager => "No Mod Manager",
            Self::UnknownGameVersion => "Unknown Game Version",
            Self::Custom(label) => label.as_str(),
        }
    }
}

/// Optional link metadata attached to an Overview problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewProblemLink {
    /// Optional display label for the link.
    pub label: Option<String>,
    /// URL to open via a later desktop adapter.
    pub url: String,
}

impl OverviewProblemLink {
    /// Creates a problem link with an optional display label.
    pub fn new(label: impl Into<Option<String>>, url: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            url: url.into(),
        }
    }
}

/// Optional structured detail metadata attached to an Overview problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewProblemDetail {
    /// Detail name or column heading.
    pub name: String,
    /// Detail value.
    pub value: String,
}

impl OverviewProblemDetail {
    /// Creates a structured problem detail.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Scanner-ready Overview problem record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewProblem {
    /// Source area that produced the problem.
    pub source: OverviewProblemSource,
    /// Reference-compatible problem type/label.
    pub problem: OverviewProblemType,
    /// Optional absolute path when one exists.
    pub path: Option<PathBuf>,
    /// Display path/name retained even for pathless simple problems.
    pub display_path: String,
    /// Optional relative path matching the reference `ProblemInfo.relative_path`.
    pub relative_path: Option<PathBuf>,
    /// Optional source mod name; absent for unmanaged/pathless problems.
    pub mod_name: Option<String>,
    /// Human-readable summary of the issue.
    pub summary: String,
    /// Optional suggested solution text.
    pub solution: Option<String>,
    /// Optional links carried from reference `extra_data` values.
    pub links: Vec<OverviewProblemLink>,
    /// Optional structured details, such as HEDR/version detail rows.
    pub details: Vec<OverviewProblemDetail>,
    /// Severity/color intent for this problem.
    pub severity: StatusSeverity,
}

impl OverviewProblem {
    /// Creates a problem without an absolute path.
    pub fn pathless(
        source: OverviewProblemSource,
        display_path: impl Into<String>,
        problem: OverviewProblemType,
        summary: impl Into<String>,
        solution: impl Into<Option<String>>,
    ) -> Self {
        Self {
            source,
            problem,
            path: None,
            display_path: display_path.into(),
            relative_path: None,
            mod_name: None,
            summary: summary.into(),
            solution: solution.into(),
            links: Vec::new(),
            details: Vec::new(),
            severity: StatusSeverity::Error,
        }
    }

    /// Creates a problem with an absolute path and optional relative path.
    pub fn with_path(
        source: OverviewProblemSource,
        path: impl Into<PathBuf>,
        relative_path: impl Into<Option<PathBuf>>,
        problem: OverviewProblemType,
        summary: impl Into<String>,
        solution: impl Into<Option<String>>,
    ) -> Self {
        let path = path.into();
        Self {
            display_path: display_path(&path),
            source,
            problem,
            relative_path: relative_path.into(),
            path: Some(path),
            mod_name: None,
            summary: summary.into(),
            solution: solution.into(),
            links: Vec::new(),
            details: Vec::new(),
            severity: StatusSeverity::Error,
        }
    }

    /// Attaches an optional source mod name.
    pub fn with_mod_name(mut self, mod_name: impl Into<Option<String>>) -> Self {
        self.mod_name = mod_name.into();
        self
    }

    /// Attaches problem links.
    pub fn with_links(mut self, links: Vec<OverviewProblemLink>) -> Self {
        self.links = links;
        self
    }

    /// Attaches structured problem details.
    pub fn with_details(mut self, details: Vec<OverviewProblemDetail>) -> Self {
        self.details = details;
        self
    }

    /// Overrides the default error severity.
    pub const fn with_severity(mut self, severity: StatusSeverity) -> Self {
        self.severity = severity;
        self
    }
}

/// Safe last-action error retained for Overview diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewActionError {
    /// Deferred action that failed.
    pub action: OverviewDeferredActionKind,
    /// Safe user-facing error summary.
    pub summary: String,
}

impl OverviewActionError {
    /// Creates a safe last-action error record.
    pub fn new(action: OverviewDeferredActionKind, summary: impl Into<String>) -> Self {
        Self {
            action,
            summary: summary.into(),
        }
    }
}

/// Complete, UI-independent Overview snapshot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewSnapshot {
    /// Last refresh lifecycle state.
    pub refresh: OverviewRefreshState,
    /// Top status rows derived from discovery and system metadata.
    pub top: OverviewTopStatus,
    /// Binary panel state.
    pub binaries: BinaryPanelSummary,
    /// Archive panel state.
    pub archives: ArchivePanelSummary,
    /// Module panel state.
    pub modules: ModulePanelSummary,
    /// Update banner state.
    pub update_banner: UpdateBannerState,
    /// Scanner-ready problem records.
    pub problems: Vec<OverviewProblem>,
    /// Last safe action error, such as a desktop-open failure.
    pub last_action_error: Option<OverviewActionError>,
}

impl OverviewSnapshot {
    /// Creates an empty snapshot with labels/actions present and no OS access.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a loading snapshot.
    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            refresh: OverviewRefreshState::loading(message),
            ..Self::default()
        }
    }

    /// Creates a partial snapshot from already-safe data.
    pub fn partial(message: impl Into<String>, problems: Vec<OverviewProblem>) -> Self {
        Self {
            refresh: OverviewRefreshState::partial(message),
            problems,
            ..Self::default()
        }
    }

    /// Creates an error snapshot from a safe message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            refresh: OverviewRefreshState::error(message),
            ..Self::default()
        }
    }

    /// Creates a snapshot from typed discovery results without reading the host OS.
    pub fn from_discovery_parts(
        installation: Option<Fallout4Installation>,
        mod_manager: Option<DiscoveredModManager>,
        system_metadata: Option<SystemMetadata>,
        update_source: UpdateSource,
    ) -> Self {
        let mut snapshot = Self {
            update_banner: UpdateBannerState::from_update_source(update_source),
            ..Self::default()
        };

        snapshot.top.system_metadata = system_metadata;

        if let Some(manager) = mod_manager {
            snapshot.top.mod_manager = OverviewModManagerStatus::detected(manager);
        } else {
            snapshot.problems.push(OverviewProblem::pathless(
                OverviewProblemSource::TopStatus,
                TOP_ROW_MOD_MANAGER_LABEL,
                OverviewProblemType::NoModManager,
                "No Mod Manager Detected",
                Some("Your mod manager must launch the app to be detected.".to_owned()),
            ));
        }

        if let Some(installation) = installation {
            snapshot.top.game_path = OverviewGamePathStatus::found(installation.game_path.clone());
            snapshot.top.version = installation.install_type;
            snapshot.archives = ArchivePanelSummary::from_records(&installation.archives);
            snapshot.modules = ModulePanelSummary::from_records(&installation.modules);

            if installation.data_path.is_none() {
                snapshot.problems.push(OverviewProblem::pathless(
                    OverviewProblemSource::Modules,
                    "Data",
                    OverviewProblemType::FileNotFound,
                    "The Data folder was not found in your game install path.",
                    Some("Verify files with Steam or reinstall the game.\nIf you downgraded the game you will need to do so again afterward.".to_owned()),
                ));
            }
        } else {
            snapshot.top.game_path = OverviewGamePathStatus::NotFound;
            snapshot.top.version = Fallout4InstallType::NotFound;
            snapshot.problems.push(OverviewProblem::pathless(
                OverviewProblemSource::TopStatus,
                TOP_ROW_GAME_PATH_LABEL,
                OverviewProblemType::FileNotFound,
                "A Fallout 4 installation could not be found.",
                Some("Verify files with Steam or reinstall the game.".to_owned()),
            ));
        }

        snapshot.refresh = if snapshot.problems.is_empty() {
            OverviewRefreshState::ready(None::<String>)
        } else {
            OverviewRefreshState::partial("Overview refreshed with recoverable issues.")
        };
        snapshot
    }

    /// Returns all panel utility labels in their reference placement order.
    pub fn deferred_action_labels(&self) -> Vec<&str> {
        self.binaries
            .actions
            .iter()
            .chain(self.archives.actions.iter())
            .chain(self.modules.actions.iter())
            .map(|action| action.label.as_str())
            .collect()
    }
}

fn install_type_severity(install_type: Fallout4InstallType) -> StatusSeverity {
    match install_type {
        Fallout4InstallType::OldGen
        | Fallout4InstallType::DownGrade
        | Fallout4InstallType::NextGen
        | Fallout4InstallType::Anniversary
        | Fallout4InstallType::NextGenAnniversary => StatusSeverity::Good,
        Fallout4InstallType::Obsolete => StatusSeverity::Warning,
        Fallout4InstallType::Unknown => StatusSeverity::Unknown,
        Fallout4InstallType::NotFound => StatusSeverity::Error,
    }
}

fn limit_severity(value: usize, limit: usize) -> StatusSeverity {
    let warn_limit = (limit * 95) / 100;
    if value < warn_limit {
        StatusSeverity::Good
    } else if value <= limit {
        StatusSeverity::Warning
    } else {
        StatusSeverity::Error
    }
}

fn non_zero_error_or_neutral(value: usize) -> StatusSeverity {
    if value == 0 {
        StatusSeverity::Neutral
    } else {
        StatusSeverity::Error
    }
}

fn format_gib(bytes: u64) -> String {
    let gib = ((bytes as f64) / 1024_f64.powi(3)).round() as u64;
    format!("{gib}GB RAM")
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod overview_domain {
    use super::*;
    use crate::domain::discovery::{
        ArchiveFormat, ArchiveVersion, Fallout4Installation, SemanticVersion,
    };
    use crate::domain::mod_manager::{
        DetectedModManager, Mo2Configuration, ModOrganizerContext, ModOrganizerDirectories,
    };

    fn row_labels(rows: &[OverviewCountRow]) -> Vec<&'static str> {
        rows.iter().map(|row| row.label).collect()
    }

    fn fake_system_metadata() -> SystemMetadata {
        SystemMetadata::new(
            "Windows",
            Some("11 24H2"),
            "x86_64",
            Some("Example CPU"),
            Some(32 * 1024 * 1024 * 1024),
            Some(16),
        )
    }

    fn fake_mo2() -> DiscoveredModManager {
        let manager = DetectedModManager::mod_organizer(
            "C:/Modding/MO2/ModOrganizer.exe",
            SemanticVersion::new(2, 5, 2),
        );
        let context = ModOrganizerContext::new(
            manager,
            "Default",
            ModOrganizerDirectories::reference_defaults("C:/Modding/MO2"),
        );
        DiscoveredModManager::ModOrganizer(Box::new(Mo2Configuration::new(context)))
    }

    #[test]
    fn label_order_matches_reference_overview_domain_contract() {
        let top_labels: Vec<&str> = OverviewTopStatusKind::reference_order()
            .into_iter()
            .map(OverviewTopStatusKind::label)
            .collect();
        assert_eq!(top_labels, TOP_STATUS_LABELS);

        let snapshot = OverviewSnapshot::empty();
        assert_eq!(snapshot.binaries.title, "Binaries (EXE/DLL/BIN)");
        assert_eq!(snapshot.archives.title, "Archives (BA2)");
        assert_eq!(snapshot.modules.title, "Modules (ESM/ESL/ESP)");
        assert_eq!(row_labels(&snapshot.archives.rows), ARCHIVE_COUNT_LABELS);
        assert_eq!(row_labels(&snapshot.modules.rows), MODULE_COUNT_LABELS);
        assert_eq!(
            snapshot.deferred_action_labels(),
            vec!["Downgrade Manager...", "Archive Patcher..."]
        );
    }

    #[test]
    fn default_loading_partial_and_error_overview_domain_states_are_explicit() {
        let empty = OverviewSnapshot::empty();
        assert_eq!(empty.refresh.phase, OverviewRefreshPhase::Idle);
        assert_eq!(empty.top.game_path.display_text(), "Not Found");
        assert_eq!(empty.top.mod_manager.display_text(), "Not Found");
        assert_eq!(empty.top.version, Fallout4InstallType::Unknown);
        assert!(!empty.refresh.is_busy());

        let loading = OverviewSnapshot::loading("Refreshing Overview...");
        assert_eq!(loading.refresh.phase, OverviewRefreshPhase::Loading);
        assert!(loading.refresh.is_busy());
        assert_eq!(loading.refresh.severity(), StatusSeverity::Info);

        let partial_problem = OverviewProblem::pathless(
            OverviewProblemSource::Modules,
            "Data",
            OverviewProblemType::FileNotFound,
            "The Data folder was not found in your game install path.",
            Some("Verify files".to_owned()),
        );
        let partial = OverviewSnapshot::partial(
            "Overview refreshed with recoverable issues.",
            vec![partial_problem],
        );
        assert_eq!(partial.refresh.phase, OverviewRefreshPhase::Partial);
        assert_eq!(partial.refresh.severity(), StatusSeverity::Warning);
        assert_eq!(partial.problems[0].display_path, "Data");

        let error = OverviewSnapshot::error("Overview refresh failed.");
        assert_eq!(error.refresh.phase, OverviewRefreshPhase::Error);
        assert_eq!(error.refresh.severity(), StatusSeverity::Error);
        assert_eq!(
            error.refresh.message.as_deref(),
            Some("Overview refresh failed.")
        );
    }

    #[test]
    fn discovery_parts_reuse_existing_types_in_overview_domain_snapshot() {
        let mut installation = Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            None::<PathBuf>,
        );
        installation.install_type = Fallout4InstallType::OldGen;
        installation.archives.push(ArchiveRecord::new(
            "C:/Games/Fallout 4/Data/Fallout4 - Textures.ba2",
            ArchiveFormat::DirectX10,
            ArchiveVersion::NextGen8,
            true,
        ));
        installation.modules.push(ModuleRecord::new(
            "C:/Games/Fallout 4/Data/Example.esl",
            ModuleKind::Light,
            ModuleHeaderVersion::Version100,
            true,
        ));

        let snapshot = OverviewSnapshot::from_discovery_parts(
            Some(installation),
            Some(fake_mo2()),
            Some(fake_system_metadata()),
            UpdateSource::Nexus,
        );

        assert_eq!(snapshot.refresh.phase, OverviewRefreshPhase::Ready);
        assert_eq!(
            snapshot.top.mod_manager.display_text(),
            "Mod Organizer v2.5.2 [Profile: Default]"
        );
        assert_eq!(snapshot.top.game_path.display_text(), "C:/Games/Fallout 4");
        assert_eq!(snapshot.top.version, Fallout4InstallType::OldGen);
        assert!(snapshot.top.pc_specs_display_text().contains("32GB RAM"));
        assert_eq!(snapshot.archives.rows[1].value, 1);
        assert_eq!(snapshot.modules.rows[1].value, 1);
        assert!(matches!(
            snapshot.update_banner,
            UpdateBannerState::NotChecked {
                selected_source: UpdateSource::Nexus
            }
        ));
        assert!(snapshot.problems.is_empty());
    }

    #[test]
    fn missing_game_path_and_data_marker_are_safe_overview_domain_states() {
        let missing_game =
            OverviewSnapshot::from_discovery_parts(None, None, None, UpdateSource::None);
        assert_eq!(missing_game.refresh.phase, OverviewRefreshPhase::Partial);
        assert_eq!(missing_game.top.game_path.display_text(), "Not Found");
        assert_eq!(missing_game.top.version, Fallout4InstallType::NotFound);
        assert!(matches!(
            missing_game.update_banner,
            UpdateBannerState::Disabled
        ));
        assert!(missing_game.problems.iter().any(|problem| {
            problem.source == OverviewProblemSource::TopStatus
                && problem.problem == OverviewProblemType::FileNotFound
                && problem.display_path == TOP_ROW_GAME_PATH_LABEL
        }));
        assert!(missing_game.problems.iter().any(|problem| {
            problem.problem == OverviewProblemType::NoModManager
                && problem.display_path == TOP_ROW_MOD_MANAGER_LABEL
        }));

        let mut installation = Fallout4Installation::new("C:/Games/Fallout 4");
        installation.install_type = Fallout4InstallType::NextGen;
        let missing_data = OverviewSnapshot::from_discovery_parts(
            Some(installation),
            Some(fake_mo2()),
            None,
            UpdateSource::Github,
        );
        assert_eq!(missing_data.refresh.phase, OverviewRefreshPhase::Partial);
        let data_problem = missing_data
            .problems
            .iter()
            .find(|problem| problem.display_path == "Data")
            .expect("missing Data marker should create a problem");
        assert_eq!(data_problem.source, OverviewProblemSource::Modules);
        assert_eq!(data_problem.problem.label(), "File Not Found");
        assert_eq!(
            data_problem.summary,
            "The Data folder was not found in your game install path."
        );
    }

    #[test]
    fn update_banner_overview_domain_states_match_reference_semantics() {
        let disabled = UpdateBannerState::from_update_source(UpdateSource::None);
        assert!(matches!(disabled, UpdateBannerState::Disabled));
        assert!(!disabled.is_visible());
        assert_eq!(disabled.heading(), None);

        let available = UpdateBannerState::available_or_no_update(
            UpdateSource::Both,
            vec![
                UpdateRelease::new(UpdateProvider::NexusMods, "0.7.0"),
                UpdateRelease::new(UpdateProvider::Github, "0.7.1"),
            ],
        );
        assert!(available.is_visible());
        assert_eq!(available.heading(), Some("An update is available:"));
        let UpdateBannerState::Available { releases, .. } = available else {
            panic!("expected available update banner");
        };
        assert_eq!(releases[0].display_label(), "v0.7.0 (NexusMods)");
        assert_eq!(releases[1].display_label(), "v0.7.1 (GitHub)");
        assert_eq!(
            releases[1].action.target,
            OverviewDeferredActionTarget::Url(GITHUB_LINK.to_owned())
        );

        let no_update = UpdateBannerState::available_or_no_update(UpdateSource::Github, Vec::new());
        assert!(matches!(
            no_update,
            UpdateBannerState::NoUpdate {
                selected_source: UpdateSource::Github
            }
        ));
        assert!(!no_update.is_visible());

        let failed = UpdateBannerState::failed_silently(
            UpdateSource::Nexus,
            vec![UpdateCheckFailure::new(
                UpdateProvider::NexusMods,
                "request timed out",
            )],
        );
        assert!(!failed.is_visible());
        let UpdateBannerState::FailedSilently { failures, .. } = failed else {
            panic!("expected silent failure state");
        };
        assert_eq!(failures[0].summary, "request timed out");
    }

    #[test]
    fn problem_feed_records_carry_scanner_ready_overview_domain_fields() {
        let path_problem = OverviewProblem::with_path(
            OverviewProblemSource::Archives,
            "C:/Games/Fallout 4/Data/Broken.ba2",
            Some(PathBuf::from("Broken.ba2")),
            OverviewProblemType::InvalidArchive,
            "Archive is either corrupt or not in Bethesda Archive 2 format.",
            None::<String>,
        )
        .with_mod_name(Some("<Unmanaged>".to_owned()))
        .with_details(vec![OverviewProblemDetail::new("Version", "99")]);

        assert_eq!(path_problem.source, OverviewProblemSource::Archives);
        assert_eq!(
            path_problem.path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4/Data/Broken.ba2"))
        );
        assert_eq!(
            path_problem.relative_path.as_deref(),
            Some(Path::new("Broken.ba2"))
        );
        assert_eq!(path_problem.problem.label(), "Invalid Archive");
        assert_eq!(
            path_problem.summary,
            "Archive is either corrupt or not in Bethesda Archive 2 format."
        );
        assert_eq!(path_problem.solution, None);
        assert_eq!(path_problem.details[0].value, "99");

        let pathless = OverviewProblem::pathless(
            OverviewProblemSource::CountLimit,
            "300 General Archives",
            OverviewProblemType::LimitExceeded,
            "You have 300 General Archives enabled. The limit is 256.",
            Some("Archives can be unpacked or merged to reduce your total.".to_owned()),
        )
        .with_links(vec![OverviewProblemLink::new(
            Some("Unpackrr".to_owned()),
            "https://www.nexusmods.com/fallout4/mods/82082",
        )])
        .with_details(vec![OverviewProblemDetail::new("Limit", "256")]);

        assert_eq!(pathless.path, None);
        assert_eq!(pathless.display_path, "300 General Archives");
        assert_eq!(pathless.problem.label(), "Limit Exceeded");
        assert_eq!(
            pathless.solution.as_deref(),
            Some("Archives can be unpacked or merged to reduce your total.")
        );
        assert_eq!(pathless.links[0].label.as_deref(), Some("Unpackrr"));
        assert_eq!(pathless.details[0].name, "Limit");
    }

    #[test]
    fn archive_and_module_record_counts_keep_reference_order_and_severity() {
        let archives = ArchivePanelSummary::from_counts(ArchivePanelCounts {
            general: 243,
            texture: 256,
            unreadable: 1,
            old_gen_version: 2,
            next_gen_version: 3,
        });
        assert_eq!(row_labels(&archives.rows), ARCHIVE_COUNT_LABELS);
        assert_eq!(archives.rows[0].severity, StatusSeverity::Warning);
        assert_eq!(archives.rows[1].severity, StatusSeverity::Error);
        assert_eq!(archives.rows[3].severity, StatusSeverity::Error);
        assert_eq!(archives.rows[4].value, 2);
        assert_eq!(archives.rows[5].value, 3);

        let modules = ModulePanelSummary::from_records(&[
            ModuleRecord::new(
                "C:/Games/Fallout 4/Data/Full.esp",
                ModuleKind::Full,
                ModuleHeaderVersion::Version095,
                true,
            ),
            ModuleRecord::new(
                "C:/Games/Fallout 4/Data/Light.esl",
                ModuleKind::Light,
                ModuleHeaderVersion::Unknown("0.94".to_owned()),
                true,
            ),
            ModuleRecord::unreadable("C:/Games/Fallout 4/Data/Broken.esp"),
        ]);
        assert_eq!(row_labels(&modules.rows), MODULE_COUNT_LABELS);
        assert_eq!(modules.rows[0].value, 1);
        assert_eq!(modules.rows[1].value, 1);
        assert_eq!(modules.rows[3].value, 1);
        assert_eq!(modules.rows[5].value, 1);
        assert_eq!(modules.rows[6].value, 1);
        assert_eq!(modules.rows[6].severity, StatusSeverity::Error);
    }
}
