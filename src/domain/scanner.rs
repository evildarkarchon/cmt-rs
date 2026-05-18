//! Pure Scanner-tab domain contract.
//!
//! The reference Tkinter Scanner tab lives in `CMT/src/tabs/_scanner.py` and
//! shares problem records with the Overview tab through `ProblemInfo` and
//! `SimpleProblemInfo`. This module freezes the Scanner labels, settings
//! projection, grouping, detail rendering, read-only action descriptors, and
//! Overview handoff as inert Rust data so later services/controllers/UI code do
//! not need to duplicate reference strings or import Slint/platform APIs.

use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use crate::domain::{
    autofix::{AutoFixOperationKey, AutoFixSelectionIdentity},
    overview::OverviewProblem,
    settings::ScannerSettings,
};

/// Reference notebook tab title.
pub const SCANNER_TAB_TITLE: &str = "Scanner";
/// Reference side-pane labelframe title.
pub const SCAN_SETTINGS_GROUP_LABEL: &str = "Scan Settings";
/// Reference scan button label while idle.
pub const SCAN_BUTTON_LABEL: &str = "Scan Game";
/// Reference scan button label while a scan is active.
pub const SCANNING_BUTTON_LABEL: &str = "Scanning...";
/// Reference collapse tree button label.
pub const COLLAPSE_ALL_LABEL: &str = "Collapse All";
/// Reference expand tree button label.
pub const EXPAND_ALL_LABEL: &str = "Expand All";
/// Reference result-count suffix.
pub const RESULT_COUNT_SUFFIX: &str = "Results ~ Select an item for details";
/// Reference first progress text shown before scanner filesystem work starts.
pub const PROGRESS_REFRESHING_OVERVIEW_TEXT: &str = "Refreshing Overview...";
/// Reference progress text shown before building the MO2 mod-file index.
pub const PROGRESS_BUILDING_MOD_INDEX_TEXT: &str = "Building mod file index...";
/// Reference progress prefix while walking top-level Data folders.
pub const PROGRESS_SCANNING_PREFIX: &str = "Scanning...";
/// Reference initial progress value immediately after Overview handoff.
pub const PROGRESS_AFTER_OVERVIEW_PERCENT: f32 = 1.0;
/// Reference completion progress value.
pub const PROGRESS_COMPLETE_PERCENT: f32 = 100.0;
/// Reference delay between Tkinter queue polls in milliseconds.
pub const PROGRESS_CHECK_DELAY_MS: u64 = 100;
/// S07 intentionally renders scan settings from persisted state without editing them inline.
pub const SCANNER_SETTINGS_READ_ONLY_IN_S07: bool = true;

/// Reference race-subgraph threshold from `CMT/src/globals.py`.
pub const RACE_SUBGRAPH_THRESHOLD: usize = 100;
/// Reference race-subgraph information text.
pub const INFO_SCAN_RACE_SUBGRAPHS: &str = "Counts race animation subgraph records (RACE \\ SADD).\nDepending on your PC, adding too many of these may result in stutter when loading Cells.\nThis issue needs more investigation as this may be mere correlation and not causation.";
/// Reference scanner Overview Issues tooltip.
pub const TOOLTIP_SCAN_OVERVIEW: &str = "Report details of issues from Overview";
/// Reference scanner Errors tooltip.
pub const TOOLTIP_SCAN_ERRORS: &str = "Check for errors in mod configuration or files.";
/// Reference scanner Wrong File Formats tooltip.
pub const TOOLTIP_SCAN_FORMATS: &str = "Check file types against a whitelist per Data folder.\ne.g. MP3 instead of XWM/WAV in Data/Sound/.";
/// Reference scanner Loose Previs tooltip.
pub const TOOLTIP_SCAN_PREVIS: &str =
    "Report loose Data/Vis/ and Data/Meshes/Precombined/ folders.";
/// Reference scanner Junk Files tooltip.
pub const TOOLTIP_SCAN_JUNK: &str =
    "Report junk files such as desktop.ini, Thumbs.db,\nand leftover fomod folders.";
/// Reference scanner Problem Overrides tooltip.
pub const TOOLTIP_SCAN_BAD_OVERRIDES: &str = "Check for overrides that typically cause issues\nsuch as outdated F4SE script files or loose AnimTextData folders.";
/// Reference scanner Race Subgraphs tooltip, including the threshold text.
pub const TOOLTIP_SCAN_RACE_SUBGRAPHS: &str = "Counts race animation subgraph records (RACE \\ SADD).\nDepending on your PC, adding too many of these may result in stutter when loading Cells.\nThis issue needs more investigation as this may be mere correlation and not causation.\n\nBase Game Count: 37\nWarning Threshold: 100";

/// Reference detail label for mod attribution.
pub const DETAIL_LABEL_MOD: &str = "Mod:";
/// Reference detail label for the problem path/name.
pub const DETAIL_LABEL_PROBLEM: &str = "Problem:";
/// Reference detail label for the problem summary.
pub const DETAIL_LABEL_SUMMARY: &str = "Summary:";
/// Reference detail label for solution text.
pub const DETAIL_LABEL_SOLUTION: &str = "Solution:";
/// Reference fallback when a problem has no solution suggestion.
pub const NO_SOLUTION_SUGGESTION: &str = "No solution suggestion.";
/// Reference missing-mod display used by the details pane when staging is enabled.
pub const MOD_NOT_AVAILABLE_LABEL: &str = "N/A";

/// Reference details action button label.
pub const ACTION_COPY_DETAILS_LABEL: &str = "Copy Details";
/// Reference file-list action button label.
pub const ACTION_FILE_LIST_LABEL: &str = "File List";
/// Read-only action label for opening a problem location.
pub const ACTION_OPEN_LOCATION_LABEL: &str = "Open Location";
/// Read-only action label for opening a solution URL.
pub const ACTION_OPEN_URL_LABEL: &str = "Open URL";
/// Read-only action label for copying a solution URL.
pub const ACTION_COPY_URL_LABEL: &str = "Copy URL";

/// Reference file-list title for Race Subgraph simple problems.
pub const RACE_SUBGRAPH_FILE_LIST_TITLE: &str = "Race Animation Subgraph Records";
/// Reference generic file-list title.
pub const GENERIC_FILE_LIST_TITLE: &str = "Files";
/// Reference file-list text for Race Subgraph simple problems.
pub const RACE_SUBGRAPH_FILE_LIST_TEXT: &str = "Counts race animation subgraph records (RACE \\ SADD).\nDepending on your PC, adding too many of these may result in stutter when loading Cells. This issue needs more investigation as this may be mere correlation and not causation.";
/// Reference Race Subgraph file-list columns.
pub const RACE_SUBGRAPH_FILE_LIST_COLUMNS: [&str; 2] = ["Records", " Module"];
/// Generic file-list columns for scanner detail dialogs.
pub const GENERIC_FILE_LIST_COLUMNS: [&str; 2] = ["Value", " File"];

/// Reference scanner category labels in display order.
pub const SCANNER_CATEGORY_LABELS: [&str; 7] = [
    "Overview Issues",
    "Errors",
    "Wrong File Formats",
    "Loose Previs",
    "Junk Files",
    "Problem Overrides",
    "Race Subgraphs",
];

/// Deterministic scanner result group order for known reference problem labels.
pub const SCANNER_PROBLEM_GROUP_LABELS: [&str; 15] = [
    "Junk File",
    "Unexpected Format",
    "Misplaced DLL",
    "Loose Previs",
    "Loose AnimTextData",
    "Invalid Archive",
    "Invalid Module",
    "Invalid Archive Name",
    "F4SE Script Override",
    "File Not Found",
    "Wrong Version",
    "Race Subgraph Record Count",
    "Limit Exceeded",
    "No Mod Manager",
    "Unknown Game Version",
];

/// Scanner category identifiers matching `CMT/src/scan_settings.py::ScanSetting`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerCategoryKind {
    /// Overview Issues category.
    OverviewIssues,
    /// Errors category.
    Errors,
    /// Wrong File Formats category.
    WrongFormat,
    /// Loose Previs category.
    LoosePrevis,
    /// Junk Files category.
    JunkFiles,
    /// Problem Overrides category.
    ProblemOverrides,
    /// Race Subgraphs category.
    RaceSubgraphs,
}

impl ScannerCategoryKind {
    /// Returns scanner categories in the reference side-pane display order.
    pub const fn reference_order() -> [Self; 7] {
        [
            Self::OverviewIssues,
            Self::Errors,
            Self::WrongFormat,
            Self::LoosePrevis,
            Self::JunkFiles,
            Self::ProblemOverrides,
            Self::RaceSubgraphs,
        ]
    }

    /// Returns the exact user-facing category label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::OverviewIssues => "Overview Issues",
            Self::Errors => "Errors",
            Self::WrongFormat => "Wrong File Formats",
            Self::LoosePrevis => "Loose Previs",
            Self::JunkFiles => "Junk Files",
            Self::ProblemOverrides => "Problem Overrides",
            Self::RaceSubgraphs => "Race Subgraphs",
        }
    }

    /// Returns the reference tooltip/help text for the category.
    pub const fn help_text(self) -> &'static str {
        match self {
            Self::OverviewIssues => TOOLTIP_SCAN_OVERVIEW,
            Self::Errors => TOOLTIP_SCAN_ERRORS,
            Self::WrongFormat => TOOLTIP_SCAN_FORMATS,
            Self::LoosePrevis => TOOLTIP_SCAN_PREVIS,
            Self::JunkFiles => TOOLTIP_SCAN_JUNK,
            Self::ProblemOverrides => TOOLTIP_SCAN_BAD_OVERRIDES,
            Self::RaceSubgraphs => TOOLTIP_SCAN_RACE_SUBGRAPHS,
        }
    }

    /// Returns whether this category is enabled in persisted settings.
    pub const fn enabled_in(self, settings: &ScannerSettings) -> bool {
        match self {
            Self::OverviewIssues => settings.overview_issues,
            Self::Errors => settings.errors,
            Self::WrongFormat => settings.wrong_format,
            Self::LoosePrevis => settings.loose_previs,
            Self::JunkFiles => settings.junk_files,
            Self::ProblemOverrides => settings.problem_overrides,
            Self::RaceSubgraphs => settings.race_subgraphs,
        }
    }
}

/// Static reference metadata for one scanner category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScannerCategoryDescriptor {
    /// Stable category kind.
    pub kind: ScannerCategoryKind,
    /// Exact reference label.
    pub label: &'static str,
    /// Exact reference help/tooltip text.
    pub help_text: &'static str,
}

/// All scanner categories in reference display order.
pub const SCANNER_CATEGORIES: [ScannerCategoryDescriptor; 7] = [
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::OverviewIssues,
        label: "Overview Issues",
        help_text: TOOLTIP_SCAN_OVERVIEW,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::Errors,
        label: "Errors",
        help_text: TOOLTIP_SCAN_ERRORS,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::WrongFormat,
        label: "Wrong File Formats",
        help_text: TOOLTIP_SCAN_FORMATS,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::LoosePrevis,
        label: "Loose Previs",
        help_text: TOOLTIP_SCAN_PREVIS,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::JunkFiles,
        label: "Junk Files",
        help_text: TOOLTIP_SCAN_JUNK,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::ProblemOverrides,
        label: "Problem Overrides",
        help_text: TOOLTIP_SCAN_BAD_OVERRIDES,
    },
    ScannerCategoryDescriptor {
        kind: ScannerCategoryKind::RaceSubgraphs,
        label: "Race Subgraphs",
        help_text: TOOLTIP_SCAN_RACE_SUBGRAPHS,
    },
];

/// Render-ready scanner category state projected from persisted settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerCategoryProjection {
    /// Stable category kind.
    pub kind: ScannerCategoryKind,
    /// Exact reference label.
    pub label: &'static str,
    /// Exact reference help/tooltip text.
    pub help_text: &'static str,
    /// Whether the category is enabled in the current settings snapshot.
    pub enabled: bool,
    /// Whether the UI should allow editing this category in S07.
    pub read_only: bool,
}

/// Projects persisted scanner settings into reference-ordered UI category rows.
pub fn scanner_category_projection(settings: &ScannerSettings) -> Vec<ScannerCategoryProjection> {
    SCANNER_CATEGORIES
        .into_iter()
        .map(|category| ScannerCategoryProjection {
            kind: category.kind,
            label: category.label,
            help_text: category.help_text,
            enabled: category.kind.enabled_in(settings),
            read_only: SCANNER_SETTINGS_READ_ONLY_IN_S07,
        })
        .collect()
}

/// Returns the exact reference result-count text for a completed scan.
pub fn scanner_result_count_text(count: usize) -> String {
    format!("{count} {RESULT_COUNT_SUFFIX}")
}

/// Returns the exact reference folder progress text for a top-level Data folder.
pub fn scanner_folder_progress_text(
    current_index: usize,
    total_folders: usize,
    folder: &str,
) -> String {
    format!(
        "{PROGRESS_SCANNING_PREFIX} {current_index}/{}: {folder}",
        total_folders.max(1)
    )
}

/// Reference scanner problem types plus custom labels from future/unknown sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScannerProblemType {
    /// Reference `Junk File` problem.
    JunkFile,
    /// Reference `Unexpected Format` problem.
    UnexpectedFormat,
    /// Reference `Misplaced DLL` problem.
    MisplacedDll,
    /// Reference `Loose Previs` problem.
    LoosePrevis,
    /// Reference `Loose AnimTextData` problem.
    LooseAnimTextData,
    /// Reference `Invalid Archive` problem.
    InvalidArchive,
    /// Reference `Invalid Module` problem.
    InvalidModule,
    /// Reference `Invalid Archive Name` problem.
    InvalidArchiveName,
    /// Reference `F4SE Script Override` problem.
    F4seScriptOverride,
    /// Reference `File Not Found` problem.
    FileNotFound,
    /// Reference `Wrong Version` problem.
    WrongVersion,
    /// Reference simple `Race Subgraph Record Count` problem.
    RaceSubgraphRecordCount,
    /// Reference simple `Limit Exceeded` problem.
    LimitExceeded,
    /// Overview handoff `No Mod Manager` problem.
    NoModManager,
    /// Overview handoff `Unknown Game Version` problem.
    UnknownGameVersion,
    /// Unknown or future problem label preserved verbatim.
    Custom(String),
}

impl ScannerProblemType {
    /// Converts a user-facing problem label into the closest typed scanner problem.
    pub fn from_label(label: &str) -> Self {
        match label {
            "Junk File" => Self::JunkFile,
            "Unexpected Format" => Self::UnexpectedFormat,
            "Misplaced DLL" => Self::MisplacedDll,
            "Loose Previs" => Self::LoosePrevis,
            "Loose AnimTextData" => Self::LooseAnimTextData,
            "Invalid Archive" => Self::InvalidArchive,
            "Invalid Module" => Self::InvalidModule,
            "Invalid Archive Name" => Self::InvalidArchiveName,
            "F4SE Script Override" => Self::F4seScriptOverride,
            "File Not Found" => Self::FileNotFound,
            "Wrong Version" => Self::WrongVersion,
            "Race Subgraph Record Count" => Self::RaceSubgraphRecordCount,
            "Limit Exceeded" => Self::LimitExceeded,
            "No Mod Manager" => Self::NoModManager,
            "Unknown Game Version" => Self::UnknownGameVersion,
            custom => Self::Custom(custom.to_owned()),
        }
    }

    /// Returns the user-facing problem label.
    pub fn label(&self) -> &str {
        match self {
            Self::JunkFile => "Junk File",
            Self::UnexpectedFormat => "Unexpected Format",
            Self::MisplacedDll => "Misplaced DLL",
            Self::LoosePrevis => "Loose Previs",
            Self::LooseAnimTextData => "Loose AnimTextData",
            Self::InvalidArchive => "Invalid Archive",
            Self::InvalidModule => "Invalid Module",
            Self::InvalidArchiveName => "Invalid Archive Name",
            Self::F4seScriptOverride => "F4SE Script Override",
            Self::FileNotFound => "File Not Found",
            Self::WrongVersion => "Wrong Version",
            Self::RaceSubgraphRecordCount => "Race Subgraph Record Count",
            Self::LimitExceeded => "Limit Exceeded",
            Self::NoModManager => "No Mod Manager",
            Self::UnknownGameVersion => "Unknown Game Version",
            Self::Custom(label) => label.as_str(),
        }
    }

    fn group_rank(&self) -> usize {
        match self {
            Self::JunkFile => 0,
            Self::UnexpectedFormat => 1,
            Self::MisplacedDll => 2,
            Self::LoosePrevis => 3,
            Self::LooseAnimTextData => 4,
            Self::InvalidArchive => 5,
            Self::InvalidModule => 6,
            Self::InvalidArchiveName => 7,
            Self::F4seScriptOverride => 8,
            Self::FileNotFound => 9,
            Self::WrongVersion => 10,
            Self::RaceSubgraphRecordCount => 11,
            Self::LimitExceeded => 12,
            Self::NoModManager => 13,
            Self::UnknownGameVersion => 14,
            Self::Custom(_) => usize::MAX,
        }
    }
}

/// Reference solution text identifiers from `CMT/src/enums.py::SolutionType`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScannerSolutionKind {
    /// `These files should either be archived or deleted.`
    ArchiveOrDeleteFile,
    /// `These folders should be packed into BA2 archives.`
    ArchiveFolder,
    /// `This file should be deleted.`
    DeleteFile,
    /// Convert/delete/ignore file guidance.
    ConvertDeleteOrIgnoreFile,
    /// Delete or ignore file guidance.
    DeleteOrIgnoreFile,
    /// Delete or ignore folder guidance.
    DeleteOrIgnoreFolder,
    /// Invalid archive-name guidance.
    RenameArchive,
    /// Download-mod guidance.
    DownloadMod,
    /// Steam verification/reinstall guidance.
    VerifyFiles,
    /// Unknown file-format guidance.
    UnknownFormat,
    /// Custom scanner solution text preserved verbatim.
    Custom(String),
}

impl ScannerSolutionKind {
    /// Returns the exact reference solution string.
    pub fn as_reference_text(&self) -> &str {
        match self {
            Self::ArchiveOrDeleteFile => "These files should either be archived or deleted.",
            Self::ArchiveFolder => "These folders should be packed into BA2 archives.",
            Self::DeleteFile => "This file should be deleted.",
            Self::ConvertDeleteOrIgnoreFile => {
                "This file may need to be converted and relevant files updated for the new name.\nOtherwise it can likely be deleted or ignored."
            }
            Self::DeleteOrIgnoreFile => "It can either be deleted or ignored.",
            Self::DeleteOrIgnoreFolder => "It can either be deleted or ignored.",
            Self::RenameArchive => {
                "Archives must be named the same as a plugin with an added suffix or added to an INI."
            }
            Self::DownloadMod => "Download the mod here:",
            Self::VerifyFiles => {
                "Verify files with Steam or reinstall the game.\nIf you downgraded the game you will need to do so again afterward."
            }
            Self::UnknownFormat => "If this file type is expected here, please report it.",
            Self::Custom(text) => text.as_str(),
        }
    }

    /// Converts the solution text into an owned string for scanner result records.
    pub fn into_solution_text(self) -> String {
        match self {
            Self::Custom(text) => text,
            other => other.as_reference_text().to_owned(),
        }
    }

    /// Returns the typed Auto-Fix operation key represented by this solution kind.
    ///
    /// Custom solution text has no reference `SolutionType` registry key and must
    /// therefore stay display-only and ineligible for Auto-Fix by string matching.
    pub fn auto_fix_operation_key(&self) -> Option<AutoFixOperationKey> {
        match self {
            Self::ArchiveOrDeleteFile => Some(AutoFixOperationKey::ArchiveOrDeleteFile),
            Self::ArchiveFolder => Some(AutoFixOperationKey::ArchiveFolder),
            Self::DeleteFile => Some(AutoFixOperationKey::DeleteFile),
            Self::ConvertDeleteOrIgnoreFile => Some(AutoFixOperationKey::ConvertDeleteOrIgnoreFile),
            Self::DeleteOrIgnoreFile => Some(AutoFixOperationKey::DeleteOrIgnoreFile),
            Self::DeleteOrIgnoreFolder => Some(AutoFixOperationKey::DeleteOrIgnoreFolder),
            Self::RenameArchive => Some(AutoFixOperationKey::RenameArchive),
            Self::DownloadMod => Some(AutoFixOperationKey::DownloadMod),
            Self::VerifyFiles => Some(AutoFixOperationKey::VerifyFiles),
            Self::UnknownFormat => Some(AutoFixOperationKey::UnknownFormat),
            Self::Custom(_) => None,
        }
    }
}

/// Optional mod attribution shown when a mod-manager staging path is active.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModAttribution {
    /// User-facing mod name.
    pub name: String,
}

impl ModAttribution {
    /// Creates non-empty mod attribution.
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        if name.is_empty() {
            None
        } else {
            Some(Self { name })
        }
    }

    /// Converts an optional reference mod string into optional attribution.
    pub fn from_optional(name: Option<&str>) -> Option<Self> {
        name.and_then(Self::new)
    }

    /// Returns the display name.
    pub fn display_name(&self) -> &str {
        self.name.as_str()
    }
}

/// Extra solution/detail data attached to a scanner result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScannerExtraData {
    /// URL extra data from reference `extra_data` or Overview links.
    Url {
        /// Optional human label retained from Overview link metadata.
        label: Option<String>,
        /// URL string opened or copied by read-only actions.
        url: String,
    },
    /// Non-URL extra text from reference `extra_data`.
    Text(String),
    /// Structured Overview detail preserved for scanner details/copy text.
    Detail {
        /// Detail name.
        name: String,
        /// Detail value.
        value: String,
    },
}

impl ScannerExtraData {
    /// Creates URL extra data without a display label.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url {
            label: None,
            url: url.into(),
        }
    }

    /// Creates URL extra data with a retained display label.
    pub fn labeled_url(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self::Url {
            label: Some(label.into()),
            url: url.into(),
        }
    }

    /// Creates non-URL extra text.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Creates structured detail extra data.
    pub fn detail(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Detail {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the string appended to solution text in the details pane and copied details.
    pub fn display_text(&self) -> String {
        match self {
            Self::Url { url, .. } => url.clone(),
            Self::Text(text) => text.clone(),
            Self::Detail { name, value } => format!("{name}: {value}"),
        }
    }

    /// Returns the URL when this extra data can drive URL open/copy actions.
    pub fn url_value(&self) -> Option<&str> {
        match self {
            Self::Url { url, .. } => Some(url.as_str()),
            Self::Text(_) | Self::Detail { .. } => None,
        }
    }
}

/// Metadata for one file-list row shown by a scanner detail action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerFileListEntry {
    /// Value shown in the first column, such as a record count.
    pub value: String,
    /// Path shown in the second column.
    pub path: PathBuf,
}

impl ScannerFileListEntry {
    /// Creates a file-list row from a value and path.
    pub fn new(value: impl ToString, path: impl Into<PathBuf>) -> Self {
        Self {
            value: value.to_string(),
            path: path.into(),
        }
    }
}

/// Optional file-list metadata attached to simple scanner results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerFileList {
    /// Dialog title.
    pub title: String,
    /// Introductory dialog text.
    pub description: String,
    /// Two-column headings in reference order.
    pub columns: [String; 2],
    /// Rows to display.
    pub entries: Vec<ScannerFileListEntry>,
}

impl ScannerFileList {
    /// Creates generic file-list metadata.
    pub fn generic(entries: Vec<ScannerFileListEntry>) -> Self {
        Self {
            title: GENERIC_FILE_LIST_TITLE.to_owned(),
            description: String::new(),
            columns: GENERIC_FILE_LIST_COLUMNS.map(str::to_owned),
            entries,
        }
    }

    /// Creates Race Subgraph file-list metadata with reference title/text/columns.
    pub fn race_subgraph_records(entries: Vec<ScannerFileListEntry>) -> Self {
        Self {
            title: RACE_SUBGRAPH_FILE_LIST_TITLE.to_owned(),
            description: RACE_SUBGRAPH_FILE_LIST_TEXT.to_owned(),
            columns: RACE_SUBGRAPH_FILE_LIST_COLUMNS.map(str::to_owned),
            entries,
        }
    }

    /// Returns true when there are no rows to show.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One detail row for a selected scanner result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerDetailRecord {
    /// Reference detail label including the trailing colon.
    pub label: &'static str,
    /// Display value for the row.
    pub value: String,
}

/// Read-only action kinds exposed by scanner result details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerActionKind {
    /// Copy the selected result's details.
    CopyDetails,
    /// Open the selected result's path/location.
    OpenLocation,
    /// Open the selected result's solution URL.
    OpenSolutionUrl,
    /// Copy the selected result's solution URL.
    CopySolutionUrl,
    /// Show the selected result's file list.
    ShowFileList,
}

impl ScannerActionKind {
    /// Returns the stable UI/controller action id used by scanner intent handlers.
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::CopyDetails => "copy-details",
            Self::OpenLocation => "open-location",
            Self::OpenSolutionUrl => "open-solution-url",
            Self::CopySolutionUrl => "copy-solution-url",
            Self::ShowFileList => "show-file-list",
        }
    }

    /// Parses a stable UI/controller action id into a scanner action kind.
    pub fn from_id(action_id: &str) -> Option<Self> {
        match action_id {
            "copy-details" => Some(Self::CopyDetails),
            "open-location" => Some(Self::OpenLocation),
            "open-solution-url" => Some(Self::OpenSolutionUrl),
            "copy-solution-url" => Some(Self::CopySolutionUrl),
            "show-file-list" => Some(Self::ShowFileList),
            _ => None,
        }
    }
}

/// Target payload for a read-only scanner detail action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannerActionTarget {
    /// The UI should render and copy the selected result's details.
    DetailsText,
    /// A local path/location to open via a later platform adapter.
    Path(PathBuf),
    /// A URL to open or copy via a later service.
    Url(String),
    /// The selected result's attached file-list metadata should be shown.
    FileList,
}

/// UI-safe scanner action descriptor; executing it belongs outside the domain module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerActionDescriptor {
    /// Stable action kind.
    pub kind: ScannerActionKind,
    /// User-facing label.
    pub label: &'static str,
    /// Target data needed by future controller/service code.
    pub target: ScannerActionTarget,
    /// Whether the UI should allow the action now.
    pub enabled: bool,
    /// Whether this action is read-only with respect to user files.
    pub read_only: bool,
    /// Optional safe status text for disabled/deferred actions.
    pub status_text: Option<&'static str>,
}

impl ScannerActionDescriptor {
    fn enabled(kind: ScannerActionKind, label: &'static str, target: ScannerActionTarget) -> Self {
        Self {
            kind,
            label,
            target,
            enabled: true,
            read_only: true,
            status_text: None,
        }
    }
}

/// Safe scanner action feedback produced by copy/open/file-list workers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerActionFeedback {
    /// Optional scan id the action belonged to; stale ids are ignored by controllers.
    pub scan_id: Option<u64>,
    /// Scanner action that completed or was rejected.
    pub action: ScannerActionKind,
    /// True when the action completed successfully.
    pub succeeded: bool,
    /// User-safe feedback suitable for the Scanner status surface.
    pub safe_message: String,
    /// Optional diagnostic detail for logs/tests; not displayed as primary text.
    pub diagnostic: Option<String>,
}

impl ScannerActionFeedback {
    /// Creates successful action feedback with user-safe text.
    pub fn succeeded(
        scan_id: Option<u64>,
        action: ScannerActionKind,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            scan_id,
            action,
            succeeded: true,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Creates failed action feedback with user-safe text.
    pub fn failed(
        scan_id: Option<u64>,
        action: ScannerActionKind,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            scan_id,
            action,
            succeeded: false,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Adds optional diagnostic detail while preserving the safe message.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }

    /// Returns the safe user-facing feedback message.
    pub fn safe_message(&self) -> &str {
        self.safe_message.as_str()
    }
}

/// One scanner result row plus all details needed by later controllers/UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerResult {
    /// Problem/group type for this result.
    pub problem_type: ScannerProblemType,
    /// Tree row label for this result.
    pub tree_label: String,
    /// Detail-pane problem path/name.
    pub detail_path: String,
    /// Optional absolute path or location target.
    pub absolute_path: Option<PathBuf>,
    /// Optional source mod attribution.
    pub mod_attribution: Option<ModAttribution>,
    /// Human-readable problem summary.
    pub summary: String,
    /// Optional solution text before fallback/extra-data rendering.
    pub solution: Option<String>,
    /// Typed reference solution identity retained separately from display text.
    pub solution_kind: Option<ScannerSolutionKind>,
    /// Extra URL/text/detail data appended beneath solution text.
    pub extra_data: Vec<ScannerExtraData>,
    /// Optional file-list metadata.
    pub file_list: Option<ScannerFileList>,
}

impl ScannerResult {
    /// Creates a simple/pathless scanner result such as reference `SimpleProblemInfo`.
    pub fn simple(
        problem_type: ScannerProblemType,
        path_label: impl Into<String>,
        summary: impl Into<String>,
        solution: Option<String>,
    ) -> Self {
        let path_label = path_label.into();
        Self {
            problem_type,
            tree_label: path_label.clone(),
            detail_path: path_label,
            absolute_path: None,
            mod_attribution: None,
            summary: summary.into(),
            solution,
            solution_kind: None,
            extra_data: Vec::new(),
            file_list: None,
        }
    }

    /// Creates a path-backed scanner result such as reference `ProblemInfo`.
    pub fn with_path(
        problem_type: ScannerProblemType,
        absolute_path: impl Into<PathBuf>,
        relative_path: impl Into<PathBuf>,
        summary: impl Into<String>,
        solution: Option<String>,
    ) -> Self {
        let absolute_path = absolute_path.into();
        let relative_path = relative_path.into();
        let detail_path = path_display_slash(&relative_path);
        let tree_label = leaf_name(&detail_path)
            .filter(|leaf| !leaf.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| path_leaf_display(&absolute_path));
        Self {
            problem_type,
            tree_label,
            detail_path,
            absolute_path: Some(absolute_path),
            mod_attribution: None,
            summary: summary.into(),
            solution,
            solution_kind: None,
            extra_data: Vec::new(),
            file_list: None,
        }
    }

    /// Attaches optional source mod attribution.
    pub fn with_mod_attribution(mut self, mod_name: impl Into<String>) -> Self {
        self.mod_attribution = ModAttribution::new(mod_name);
        self
    }

    /// Attaches extra solution/detail data.
    pub fn with_extra_data(mut self, extra_data: Vec<ScannerExtraData>) -> Self {
        self.extra_data = extra_data;
        self
    }

    /// Attaches file-list metadata.
    pub fn with_file_list(mut self, file_list: ScannerFileList) -> Self {
        self.file_list = Some(file_list);
        self
    }

    /// Attaches a typed reference solution and derives the display text from it.
    pub fn with_solution_kind(mut self, solution_kind: ScannerSolutionKind) -> Self {
        self.solution = Some(solution_kind.as_reference_text().to_owned());
        self.solution_kind = Some(solution_kind);
        self
    }

    /// Returns the typed Auto-Fix operation key for this result, if one was retained.
    pub fn auto_fix_operation_key(&self) -> Option<AutoFixOperationKey> {
        self.solution_kind
            .as_ref()
            .and_then(ScannerSolutionKind::auto_fix_operation_key)
    }

    /// Returns a deterministic owned identity for the selected result facts.
    ///
    /// The identity is computed from already-owned/displayed data only and never
    /// performs filesystem I/O. Later write-capable Auto-Fix requests can carry
    /// this value to reject stale or tampered selections before mutation.
    pub fn selection_identity(&self) -> AutoFixSelectionIdentity {
        AutoFixSelectionIdentity::from_parts(self.selection_identity_parts())
    }

    fn selection_identity_parts(&self) -> Vec<String> {
        let mut parts = vec![
            self.problem_type.label().to_owned(),
            self.tree_label.clone(),
            self.detail_path.clone(),
            self.absolute_path
                .as_ref()
                .map(|path| path_display_slash(path))
                .unwrap_or_default(),
            self.mod_attribution
                .as_ref()
                .map(|attribution| attribution.name.clone())
                .unwrap_or_default(),
            self.summary.clone(),
            self.solution.clone().unwrap_or_default(),
            self.solution_kind
                .as_ref()
                .and_then(ScannerSolutionKind::auto_fix_operation_key)
                .map(|key| key.as_id().to_owned())
                .unwrap_or_default(),
        ];
        for extra in &self.extra_data {
            parts.push(extra.display_text());
        }
        if let Some(file_list) = &self.file_list {
            parts.push(file_list.title.clone());
            parts.push(file_list.description.clone());
            parts.push(file_list.columns[0].clone());
            parts.push(file_list.columns[1].clone());
            for entry in &file_list.entries {
                parts.push(entry.value.clone());
                parts.push(path_display_slash(&entry.path));
            }
        }
        parts
    }

    /// Returns the solution text with reference fallback and extra-data appending semantics.
    pub fn solution_text(&self) -> String {
        let mut solution = self
            .solution
            .clone()
            .unwrap_or_else(|| NO_SOLUTION_SUGGESTION.to_owned());
        if !self.extra_data.is_empty() {
            let extra = self
                .extra_data
                .iter()
                .map(ScannerExtraData::display_text)
                .collect::<Vec<_>>()
                .join("\n");
            solution.push('\n');
            solution.push_str(&extra);
        }
        solution
    }

    /// Returns detail records in the order used by the reference details pane.
    pub fn detail_records(&self, include_mod: bool) -> Vec<ScannerDetailRecord> {
        let mut records = Vec::with_capacity(if include_mod { 4 } else { 3 });
        if include_mod {
            records.push(ScannerDetailRecord {
                label: DETAIL_LABEL_MOD,
                value: self.mod_display_name().to_owned(),
            });
        }
        records.push(ScannerDetailRecord {
            label: DETAIL_LABEL_PROBLEM,
            value: self.detail_path.clone(),
        });
        records.push(ScannerDetailRecord {
            label: DETAIL_LABEL_SUMMARY,
            value: self.summary.clone(),
        });
        records.push(ScannerDetailRecord {
            label: DETAIL_LABEL_SOLUTION,
            value: self.solution_text(),
        });
        records
    }

    /// Renders reference-compatible copy-details text, including the trailing newline.
    pub fn copy_details_text(&self, include_mod: bool) -> String {
        let mut details = String::new();
        for record in self.detail_records(include_mod) {
            details.push_str(record.label);
            details.push(' ');
            details.push_str(&record.value);
            details.push('\n');
        }
        details
    }

    /// Returns read-only detail actions available for this result.
    pub fn read_only_actions(&self) -> Vec<ScannerActionDescriptor> {
        let mut actions = vec![ScannerActionDescriptor::enabled(
            ScannerActionKind::CopyDetails,
            ACTION_COPY_DETAILS_LABEL,
            ScannerActionTarget::DetailsText,
        )];

        if let Some(path) = &self.absolute_path {
            actions.push(ScannerActionDescriptor::enabled(
                ScannerActionKind::OpenLocation,
                ACTION_OPEN_LOCATION_LABEL,
                ScannerActionTarget::Path(path.clone()),
            ));
        }

        if let Some(url) = self.primary_solution_url() {
            actions.push(ScannerActionDescriptor::enabled(
                ScannerActionKind::OpenSolutionUrl,
                ACTION_OPEN_URL_LABEL,
                ScannerActionTarget::Url(url.to_owned()),
            ));
            actions.push(ScannerActionDescriptor::enabled(
                ScannerActionKind::CopySolutionUrl,
                ACTION_COPY_URL_LABEL,
                ScannerActionTarget::Url(url.to_owned()),
            ));
        }

        if self
            .file_list
            .as_ref()
            .is_some_and(|file_list| !file_list.is_empty())
        {
            actions.push(ScannerActionDescriptor::enabled(
                ScannerActionKind::ShowFileList,
                ACTION_FILE_LIST_LABEL,
                ScannerActionTarget::FileList,
            ));
        }

        actions
    }

    /// Returns the mod display value used by the reference details pane.
    pub fn mod_display_name(&self) -> &str {
        self.mod_attribution
            .as_ref()
            .map(ModAttribution::display_name)
            .filter(|name| !name.is_empty())
            .unwrap_or(MOD_NOT_AVAILABLE_LABEL)
    }

    /// Returns the first URL extra data value, if present.
    pub fn primary_solution_url(&self) -> Option<&str> {
        self.extra_data.iter().find_map(ScannerExtraData::url_value)
    }
}

/// A deterministic result group ready for tree/model projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerResultGroup {
    /// Problem type represented by this group.
    pub problem_type: ScannerProblemType,
    /// Group label shown in the result tree.
    pub label: String,
    /// Results in deterministic reference-compatible order.
    pub results: Vec<ScannerResult>,
}

impl ScannerResultGroup {
    fn new(problem_type: ScannerProblemType) -> Self {
        let label = problem_type.label().to_owned();
        Self {
            problem_type,
            label,
            results: Vec::new(),
        }
    }
}

/// Owned scanner completion snapshot that can cross worker/UI boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerScanSnapshot {
    /// Scan id copied from the request that produced this snapshot.
    pub scan_id: u64,
    /// Safe final status text for the Scanner status surface.
    pub status_text: String,
    /// Reference-shaped result-count text.
    pub result_count_text: String,
    /// Flat scanner results in deterministic display order.
    pub results: Vec<ScannerResult>,
    /// Grouped scanner results in deterministic reference problem order.
    pub groups: Vec<ScannerResultGroup>,
}

impl ScannerScanSnapshot {
    /// Creates an empty scanner snapshot with the supplied safe status text.
    pub fn empty(scan_id: u64, status_text: impl Into<String>) -> Self {
        Self::from_grouped(scan_id, Vec::new(), Vec::new(), status_text)
    }

    /// Creates a snapshot from flat results, computing deterministic groups.
    pub fn from_results(
        scan_id: u64,
        results: Vec<ScannerResult>,
        status_text: impl Into<String>,
    ) -> Self {
        let groups = group_scanner_results(&results);
        Self::from_grouped(scan_id, results, groups, status_text)
    }

    /// Creates a snapshot from already-grouped results.
    pub fn from_grouped(
        scan_id: u64,
        results: Vec<ScannerResult>,
        groups: Vec<ScannerResultGroup>,
        status_text: impl Into<String>,
    ) -> Self {
        Self {
            scan_id,
            result_count_text: scanner_result_count_text(results.len()),
            status_text: status_text.into(),
            results,
            groups,
        }
    }

    /// Returns the number of flat result rows in the snapshot.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

/// Groups scanner results by deterministic problem order and sorted row keys.
pub fn group_scanner_results(results: &[ScannerResult]) -> Vec<ScannerResultGroup> {
    let mut sorted = results.to_vec();
    sorted.sort_by(compare_scanner_results);

    let mut groups: Vec<ScannerResultGroup> = Vec::new();
    for result in sorted {
        if let Some(group) = groups
            .last_mut()
            .filter(|group| group.problem_type == result.problem_type)
        {
            group.results.push(result);
        } else {
            let mut group = ScannerResultGroup::new(result.problem_type.clone());
            group.results.push(result);
            groups.push(group);
        }
    }
    groups
}

/// Maps a single Overview problem into a scanner result while preserving links/details.
pub fn scanner_result_from_overview_problem(problem: &OverviewProblem) -> ScannerResult {
    let problem_type = ScannerProblemType::from_label(problem.problem.label());
    let solution = problem.solution.clone();
    let mut result = match &problem.path {
        Some(path) => {
            let relative_path = problem
                .relative_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&problem.display_path));
            ScannerResult::with_path(
                problem_type,
                path.clone(),
                relative_path,
                problem.summary.clone(),
                solution,
            )
        }
        None => ScannerResult::simple(
            problem_type,
            problem.display_path.clone(),
            problem.summary.clone(),
            solution,
        ),
    };

    result.mod_attribution = ModAttribution::from_optional(problem.mod_name.as_deref());
    result.extra_data = overview_extra_data(problem);
    result
}

/// Maps Overview problems into scanner results in their source order.
pub fn scanner_results_from_overview_problems(problems: &[OverviewProblem]) -> Vec<ScannerResult> {
    problems
        .iter()
        .map(scanner_result_from_overview_problem)
        .collect()
}

fn overview_extra_data(problem: &OverviewProblem) -> Vec<ScannerExtraData> {
    let mut extra_data = Vec::with_capacity(problem.links.len() + problem.details.len());
    for link in &problem.links {
        match &link.label {
            Some(label) => extra_data.push(ScannerExtraData::labeled_url(label, &link.url)),
            None => extra_data.push(ScannerExtraData::url(&link.url)),
        }
    }
    for detail in &problem.details {
        extra_data.push(ScannerExtraData::detail(&detail.name, &detail.value));
    }
    extra_data
}

fn compare_scanner_results(left: &ScannerResult, right: &ScannerResult) -> Ordering {
    compare_problem_types(&left.problem_type, &right.problem_type)
        .then_with(|| mod_sort_key(left).cmp(mod_sort_key(right)))
        .then_with(|| left.detail_path.cmp(&right.detail_path))
        .then_with(|| left.tree_label.cmp(&right.tree_label))
        .then_with(|| left.summary.cmp(&right.summary))
}

fn compare_problem_types(left: &ScannerProblemType, right: &ScannerProblemType) -> Ordering {
    left.group_rank()
        .cmp(&right.group_rank())
        .then_with(|| left.label().cmp(right.label()))
}

fn mod_sort_key(result: &ScannerResult) -> &str {
    result
        .mod_attribution
        .as_ref()
        .map(ModAttribution::display_name)
        .unwrap_or("")
}

fn path_display_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn path_leaf_display(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .or_else(|| {
            let display = path.display().to_string();
            leaf_name(&display).map(str::to_owned)
        })
        .unwrap_or_else(|| path.display().to_string())
}

fn leaf_name(value: &str) -> Option<&str> {
    value.rsplit(['\\', '/']).find(|part| !part.is_empty())
}

#[cfg(test)]
mod scanner_domain {
    use super::*;
    use crate::domain::overview::{
        OverviewProblem, OverviewProblemDetail, OverviewProblemLink, OverviewProblemSource,
        OverviewProblemType,
    };

    fn result(
        problem_type: ScannerProblemType,
        detail_path: &str,
        mod_name: Option<&str>,
    ) -> ScannerResult {
        let mut result = ScannerResult::simple(
            problem_type,
            detail_path,
            format!("Summary for {detail_path}"),
            Some("Solution".to_owned()),
        );
        if let Some(mod_name) = mod_name {
            result = result.with_mod_attribution(mod_name);
        }
        result
    }

    #[test]
    fn scanner_domain_label_order_matches_reference_inputs() {
        let category_labels: Vec<&str> = ScannerCategoryKind::reference_order()
            .into_iter()
            .map(ScannerCategoryKind::label)
            .collect();
        assert_eq!(category_labels, SCANNER_CATEGORY_LABELS);
        assert_eq!(SCANNER_CATEGORIES[0].help_text, TOOLTIP_SCAN_OVERVIEW);
        assert_eq!(SCANNER_CATEGORIES[6].help_text, TOOLTIP_SCAN_RACE_SUBGRAPHS);

        assert_eq!(
            SCANNER_PROBLEM_GROUP_LABELS,
            [
                "Junk File",
                "Unexpected Format",
                "Misplaced DLL",
                "Loose Previs",
                "Loose AnimTextData",
                "Invalid Archive",
                "Invalid Module",
                "Invalid Archive Name",
                "F4SE Script Override",
                "File Not Found",
                "Wrong Version",
                "Race Subgraph Record Count",
                "Limit Exceeded",
                "No Mod Manager",
                "Unknown Game Version",
            ]
        );
        assert_eq!(
            ScannerSolutionKind::DownloadMod.as_reference_text(),
            "Download the mod here:"
        );
        assert_eq!(RACE_SUBGRAPH_THRESHOLD, 100);
    }

    #[test]
    fn scanner_domain_default_category_projection_from_scanner_settings() {
        let projection = scanner_category_projection(&ScannerSettings::default());
        assert_eq!(projection.len(), SCANNER_CATEGORY_LABELS.len());
        assert_eq!(
            projection.iter().map(|row| row.label).collect::<Vec<_>>(),
            SCANNER_CATEGORY_LABELS
        );
        assert!(projection.iter().all(|row| row.enabled));
        assert!(projection.iter().all(|row| row.read_only));

        let settings = ScannerSettings {
            overview_issues: true,
            errors: false,
            wrong_format: true,
            loose_previs: false,
            junk_files: true,
            problem_overrides: false,
            race_subgraphs: true,
        };
        let projection = scanner_category_projection(&settings);
        let enabled: Vec<(ScannerCategoryKind, bool)> = projection
            .into_iter()
            .map(|row| (row.kind, row.enabled))
            .collect();
        assert_eq!(
            enabled,
            vec![
                (ScannerCategoryKind::OverviewIssues, true),
                (ScannerCategoryKind::Errors, false),
                (ScannerCategoryKind::WrongFormat, true),
                (ScannerCategoryKind::LoosePrevis, false),
                (ScannerCategoryKind::JunkFiles, true),
                (ScannerCategoryKind::ProblemOverrides, false),
                (ScannerCategoryKind::RaceSubgraphs, true),
            ]
        );
    }

    #[test]
    fn scanner_domain_result_count_and_progress_text_include_zero() {
        assert_eq!(
            scanner_result_count_text(0),
            "0 Results ~ Select an item for details"
        );
        assert_eq!(
            scanner_result_count_text(1),
            "1 Results ~ Select an item for details"
        );
        assert_eq!(
            scanner_folder_progress_text(2, 4, "meshes"),
            "Scanning... 2/4: meshes"
        );
        assert_eq!(
            scanner_folder_progress_text(1, 0, "Data"),
            "Scanning... 1/1: Data"
        );
        assert_eq!(PROGRESS_REFRESHING_OVERVIEW_TEXT, "Refreshing Overview...");
        assert_eq!(
            PROGRESS_BUILDING_MOD_INDEX_TEXT,
            "Building mod file index..."
        );
    }

    #[test]
    fn scanner_domain_grouping_is_deterministic_and_empty_sets_are_stable() {
        assert!(group_scanner_results(&[]).is_empty());

        let results = vec![
            result(
                ScannerProblemType::Custom("Zed Custom".to_owned()),
                "zeta.txt",
                None,
            ),
            result(
                ScannerProblemType::LimitExceeded,
                "300 General Archives",
                None,
            ),
            result(ScannerProblemType::JunkFile, "Data/fomod", Some("B Mod")),
            result(ScannerProblemType::FileNotFound, "plugins.txt", None),
            result(
                ScannerProblemType::JunkFile,
                "Data/desktop.ini",
                Some("A Mod"),
            ),
            result(
                ScannerProblemType::Custom("Alpha Custom".to_owned()),
                "alpha.txt",
                None,
            ),
        ];

        let groups = group_scanner_results(&results);
        assert_eq!(
            groups
                .iter()
                .map(|group| group.label.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Junk File",
                "File Not Found",
                "Limit Exceeded",
                "Alpha Custom",
                "Zed Custom",
            ]
        );
        assert_eq!(
            groups[0]
                .results
                .iter()
                .map(|result| result.mod_display_name())
                .collect::<Vec<_>>(),
            vec!["A Mod", "B Mod"]
        );
        assert_eq!(groups[0].results[0].detail_path, "Data/desktop.ini");
    }

    #[test]
    fn scanner_domain_overview_problem_mapping_preserves_urls_details_and_pathless_limits() {
        let pathless_limit = OverviewProblem::pathless(
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

        let mapped = scanner_result_from_overview_problem(&pathless_limit);
        assert_eq!(mapped.problem_type, ScannerProblemType::LimitExceeded);
        assert_eq!(mapped.absolute_path, None);
        assert_eq!(mapped.detail_path, "300 General Archives");
        assert_eq!(mapped.tree_label, "300 General Archives");
        assert_eq!(
            mapped.extra_data,
            vec![
                ScannerExtraData::labeled_url(
                    "Unpackrr",
                    "https://www.nexusmods.com/fallout4/mods/82082"
                ),
                ScannerExtraData::detail("Limit", "256"),
            ]
        );
        assert!(mapped.solution_text().contains("https://www.nexusmods.com"));
        assert!(mapped.solution_text().contains("Limit: 256"));
        assert!(
            mapped
                .read_only_actions()
                .iter()
                .any(|action| action.kind == ScannerActionKind::OpenSolutionUrl)
        );

        let no_path = OverviewProblem::pathless(
            OverviewProblemSource::TopStatus,
            "Collective Modding Toolkit.exe",
            OverviewProblemType::NoModManager,
            "No Mod Manager Detected",
            Some("Your mod manager must launch the app to be detected.".to_owned()),
        );
        let mapped_no_path = scanner_result_from_overview_problem(&no_path);
        assert_eq!(mapped_no_path.absolute_path, None);
        assert!(
            !mapped_no_path
                .read_only_actions()
                .iter()
                .any(|action| action.kind == ScannerActionKind::OpenLocation)
        );
    }

    #[test]
    fn scanner_domain_copy_details_text_handles_mod_and_missing_solution() {
        let result = ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            "C:/Games/Fallout 4/Data/fomod",
            "fomod",
            "This is a junk folder not used by the game or mod managers.",
            Some(ScannerSolutionKind::DeleteOrIgnoreFolder.into_solution_text()),
        )
        .with_mod_attribution("Example Mod");

        assert_eq!(
            result.copy_details_text(true),
            "Mod: Example Mod\nProblem: fomod\nSummary: This is a junk folder not used by the game or mod managers.\nSolution: It can either be deleted or ignored.\n"
        );
        assert_eq!(
            result.copy_details_text(false),
            "Problem: fomod\nSummary: This is a junk folder not used by the game or mod managers.\nSolution: It can either be deleted or ignored.\n"
        );

        let missing_solution = ScannerResult::simple(
            ScannerProblemType::FileNotFound,
            "plugins.txt",
            "plugins.txt was not found.",
            None,
        );
        assert_eq!(missing_solution.mod_display_name(), MOD_NOT_AVAILABLE_LABEL);
        assert_eq!(
            missing_solution.copy_details_text(true),
            "Mod: N/A\nProblem: plugins.txt\nSummary: plugins.txt was not found.\nSolution: No solution suggestion.\n"
        );
    }

    #[test]
    fn scanner_domain_extra_data_and_custom_problem_labels_are_preserved() {
        let custom = ScannerProblemType::from_label("Mystery Problem");
        assert_eq!(
            custom,
            ScannerProblemType::Custom("Mystery Problem".to_owned())
        );
        assert_eq!(custom.label(), "Mystery Problem");

        let non_url = ScannerResult::simple(
            custom.clone(),
            "Mystery Item",
            "A future scanner reported a problem this build does not model.",
            None,
        )
        .with_extra_data(vec![ScannerExtraData::text("Extra non-URL context")]);
        assert_eq!(
            non_url.solution_text(),
            "No solution suggestion.\nExtra non-URL context"
        );
        assert!(
            !non_url
                .read_only_actions()
                .iter()
                .any(|action| matches!(action.kind, ScannerActionKind::OpenSolutionUrl))
        );

        let with_url = ScannerResult::simple(
            custom,
            "Linked Item",
            "A problem with an external reference.",
            Some(ScannerSolutionKind::DownloadMod.into_solution_text()),
        )
        .with_extra_data(vec![ScannerExtraData::url("https://example.invalid/mod")])
        .with_file_list(ScannerFileList::race_subgraph_records(vec![
            ScannerFileListEntry::new(101, "C:/Games/Fallout 4/Data/Example.esp"),
        ]));

        let actions = with_url.read_only_actions();
        assert!(
            actions
                .iter()
                .any(|action| action.kind == ScannerActionKind::OpenSolutionUrl)
        );
        assert!(
            actions
                .iter()
                .any(|action| action.kind == ScannerActionKind::CopySolutionUrl)
        );
        assert!(
            actions
                .iter()
                .any(|action| action.kind == ScannerActionKind::ShowFileList)
        );
        assert_eq!(ScannerActionKind::from_id("auto-fix"), None);
        assert!(actions.iter().all(|action| action.label != "Auto-Fix"));
        assert_eq!(
            with_url
                .file_list
                .as_ref()
                .map(|file_list| file_list.title.as_str()),
            Some(RACE_SUBGRAPH_FILE_LIST_TITLE)
        );
    }

    #[test]
    fn scanner_autofix_domain_typed_solution_identity_drives_operation_key() {
        let result = ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            "C:/Games/Fallout 4/Data/desktop.ini",
            "desktop.ini",
            "This is a junk file not used by the game or mod managers.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::DeleteOrIgnoreFile);

        assert_eq!(
            result.solution.as_deref(),
            Some(ScannerSolutionKind::DeleteOrIgnoreFile.as_reference_text())
        );
        assert_eq!(
            result.solution_kind,
            Some(ScannerSolutionKind::DeleteOrIgnoreFile)
        );
        assert_eq!(
            result.auto_fix_operation_key(),
            Some(AutoFixOperationKey::DeleteOrIgnoreFile)
        );
    }

    #[test]
    fn scanner_autofix_domain_string_only_solutions_are_not_eligible_by_matching() {
        let same_display_text = ScannerSolutionKind::DeleteOrIgnoreFile
            .as_reference_text()
            .to_owned();
        let display_only = ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            "C:/Games/Fallout 4/Data/desktop.ini",
            "desktop.ini",
            "This is a junk file not used by the game or mod managers.",
            Some(same_display_text),
        );
        assert_eq!(display_only.solution_kind, None);
        assert_eq!(display_only.auto_fix_operation_key(), None);

        let custom_kind = ScannerResult::simple(
            ScannerProblemType::Custom("Future Problem".to_owned()),
            "future.dat",
            "Future scanner output.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::Custom("Custom guidance".to_owned()));
        assert_eq!(custom_kind.solution.as_deref(), Some("Custom guidance"));
        assert_eq!(custom_kind.auto_fix_operation_key(), None);
    }

    #[test]
    fn scanner_autofix_domain_selection_identity_is_stable_and_changes_with_facts() {
        let baseline = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            "C:/Games/Fallout 4/Data/Sound/example.mp3",
            "Sound/example.mp3",
            "Format not in whitelist for sound.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::ConvertDeleteOrIgnoreFile);
        let same = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            "C:/Games/Fallout 4/Data/Sound/example.mp3",
            "Sound/example.mp3",
            "Format not in whitelist for sound.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::ConvertDeleteOrIgnoreFile);
        let changed_summary = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            "C:/Games/Fallout 4/Data/Sound/example.mp3",
            "Sound/example.mp3",
            "Format not in whitelist for meshes.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::ConvertDeleteOrIgnoreFile);
        let changed_solution = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            "C:/Games/Fallout 4/Data/Sound/example.mp3",
            "Sound/example.mp3",
            "Format not in whitelist for sound.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::DeleteOrIgnoreFile);
        let changed_path = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            "C:/Games/Fallout 4/Data/Sound/other.mp3",
            "Sound/other.mp3",
            "Format not in whitelist for sound.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::ConvertDeleteOrIgnoreFile);

        assert_eq!(baseline.selection_identity(), same.selection_identity());
        assert_ne!(
            baseline.selection_identity(),
            changed_summary.selection_identity()
        );
        assert_ne!(
            baseline.selection_identity(),
            changed_solution.selection_identity()
        );
        assert_ne!(
            baseline.selection_identity(),
            changed_path.selection_identity()
        );
    }

    #[test]
    fn scanner_autofix_domain_old_deferred_read_only_action_is_absent() {
        let result = ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            "C:/Games/Fallout 4/Data/desktop.ini",
            "desktop.ini",
            "This is a junk file not used by the game or mod managers.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::DeleteOrIgnoreFile);
        let actions = result.read_only_actions();

        assert_eq!(ScannerActionKind::from_id("auto-fix"), None);
        assert!(actions.iter().all(|action| action.label != "Auto-Fix"));
        assert!(actions.iter().all(|action| action.read_only));
    }
}
