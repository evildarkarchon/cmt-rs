//! Adapter-backed Scanner read-only scan service.
//!
//! The reference Scanner tab performs a read-only filesystem walk over `Data`,
//! optionally attributes files to Mod Organizer 2 staging folders, folds in
//! Overview problems, and counts race subgraph records in enabled modules. This
//! service keeps that behavior outside Slint and UI controllers while preserving
//! the reference categories, messages, pruning rules, and recoverable-error
//! continuation model.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use tracing::{debug, info, info_span, warn};

use crate::{
    domain::{
        discovery::{
            ArchiveRecord, Fallout4IniFiles, Fallout4Installation, IniDocument, ModuleRecord,
        },
        mod_manager::{ModManagerContext, ModOrganizerContext},
        overview::OverviewProblem,
        scanner::{
            INFO_SCAN_RACE_SUBGRAPHS, PROGRESS_AFTER_OVERVIEW_PERCENT,
            PROGRESS_BUILDING_MOD_INDEX_TEXT, PROGRESS_COMPLETE_PERCENT,
            PROGRESS_REFRESHING_OVERVIEW_TEXT, RACE_SUBGRAPH_THRESHOLD, ScannerExtraData,
            ScannerFileList, ScannerFileListEntry, ScannerProblemType, ScannerResult,
            ScannerResultGroup, ScannerSolutionKind, group_scanner_results,
            scanner_folder_progress_text, scanner_result_from_overview_problem,
        },
        settings::ScannerSettings,
    },
    platform::{
        PlatformError, PlatformErrorKind,
        filesystem::{DirectoryEntry, FileType, Filesystem},
    },
};

const IGNORE_FOLDERS: [&str; 4] = ["bodyslide", "fo4edit", "robco_patcher", "source"];
const JUNK_FILES: [&str; 3] = ["thumbs.db", "desktop.ini", ".ds_store"];
const JUNK_FILE_SUFFIXES: [&str; 2] = [".tmp", ".bak"];
const DEFAULT_SKIP_FILE_SUFFIXES: [&str; 1] = [".vortex_backup"];
const SADD_BYTES: &[u8] = b"\x00SADD";

const TEXTURE_PROPER_FORMATS: [&str; 1] = ["dds"];
const SOUND_PROPER_FORMATS: [&str; 2] = ["wav", "xwm"];

const F4SE_SCRIPT_NAMES: [&str; 29] = [
    "actor.pex",
    "actorbase.pex",
    "armor.pex",
    "armoraddon.pex",
    "cell.pex",
    "component.pex",
    "constructibleobject.pex",
    "defaultobject.pex",
    "encounterzone.pex",
    "equipslot.pex",
    "f4se.pex",
    "favoritesmanager.pex",
    "form.pex",
    "game.pex",
    "headpart.pex",
    "input.pex",
    "instancedata.pex",
    "location.pex",
    "matswap.pex",
    "math.pex",
    "miscobject.pex",
    "objectmod.pex",
    "objectreference.pex",
    "perk.pex",
    "scriptobject.pex",
    "ui.pex",
    "utility.pex",
    "watertype.pex",
    "weapon.pex",
];

const ARCHIVE_NAME_WHITELIST: [&str; 39] = [
    "creationkit - shaders.ba2",
    "creationkit - textures.ba2",
    "fallout4 - animations.ba2",
    "fallout4 - interface.ba2",
    "fallout4 - materials.ba2",
    "fallout4 - meshes.ba2",
    "fallout4 - meshesextra.ba2",
    "fallout4 - misc.ba2",
    "fallout4 - nvflex.ba2",
    "fallout4 - shaders.ba2",
    "fallout4 - sounds.ba2",
    "fallout4 - startup.ba2",
    "fallout4 - textures1.ba2",
    "fallout4 - textures2.ba2",
    "fallout4 - textures3.ba2",
    "fallout4 - textures4.ba2",
    "fallout4 - textures5.ba2",
    "fallout4 - textures6.ba2",
    "fallout4 - textures7.ba2",
    "fallout4 - textures8.ba2",
    "fallout4 - textures9.ba2",
    "fallout4 - texturespatch.ba2",
    "fallout4 - voices.ba2",
    "dlcultrahighresolution - textures01.ba2",
    "dlcultrahighresolution - textures02.ba2",
    "dlcultrahighresolution - textures03.ba2",
    "dlcultrahighresolution - textures04.ba2",
    "dlcultrahighresolution - textures05.ba2",
    "dlcultrahighresolution - textures06.ba2",
    "dlcultrahighresolution - textures07.ba2",
    "dlcultrahighresolution - textures08.ba2",
    "dlcultrahighresolution - textures09.ba2",
    "dlcultrahighresolution - textures10.ba2",
    "dlcultrahighresolution - textures11.ba2",
    "dlcultrahighresolution - textures12.ba2",
    "dlcultrahighresolution - textures13.ba2",
    "dlcultrahighresolution - textures14.ba2",
    "dlcultrahighresolution - textures15.ba2",
    "dlcultrahighresolution - textures16.ba2",
];

const RACE_SUBGRAPH_SOLUTION: &str = "IF you are experiencing stutter when moving between cells, removing some of these mods could alleviate performance issues.\nMerging them may also reduce stutter.";
const F4SE_SCRIPT_OVERRIDE_SUMMARY: &str = "This is an override of an F4SE script. This could break F4SE if they aren't the same version or this mod isn't intended to override F4SE files.";
const F4SE_SCRIPT_OVERRIDE_SOLUTION: &str = "Check if this mod is supposed to override F4SE Scripts.\nIf this is a script extender/library or requires one, this is likely intentional but it must support your game version explicitly.\nOtherwise, this mod or file may need to be deleted.";
const LOOSE_PREVIS_SUMMARY: &str = "Loose previs files should be archived so they only win conflicts according to their plugin's load order.\nLoose previs files are also not supported by PJM's Previs Scripts.";
const LOOSE_ANIM_TEXT_DATA_SUMMARY: &str =
    "The existence of unpacked AnimTextData may cause the game to crash.";
const JUNK_FOLDER_SUMMARY: &str = "This is a junk folder not used by the game or mod managers.";
const JUNK_FILE_SUMMARY: &str = "This is a junk file not used by the game or mod managers.";
const INVALID_ARCHIVE_NAME_SUMMARY: &str =
    "This is not a valid archive name and won't be loaded by the game.";
const DATA_FOLDER_NOT_FOUND_SUMMARY: &str = "Data folder not found";

/// Input request for one read-only scanner pass.
///
/// The request owns optional path values and borrows already-collected facts so
/// callers can build it from discovery/overview workers without giving this
/// service permission to refresh those systems or mutate settings.
#[derive(Debug, Clone)]
pub struct ScannerScanRequest<'a> {
    /// Monotonic scan id assigned by the caller; copied into progress and diagnostics.
    pub scan_id: u64,
    /// Persisted scanner settings snapshot used for category gating.
    pub settings: &'a ScannerSettings,
    /// Optional discovered installation, used for Data fallback and INI archive suffixes.
    pub installation: Option<&'a Fallout4Installation>,
    /// Optional Data path override supplied by discovery or a test fixture.
    pub data_path: Option<PathBuf>,
    /// Overview problems collected before this scan request.
    pub overview_problems: &'a [OverviewProblem],
    /// Enabled/collected module records used for race-subgraph counting.
    pub enabled_modules: &'a [ModuleRecord],
    /// Enabled/collected archive records used to avoid flagging already-loaded BA2 names.
    pub enabled_archives: &'a [ArchiveRecord],
    /// Optional manager context; MO2 enables staged attribution, Vortex is Data-only.
    pub mod_manager: Option<&'a ModManagerContext>,
}

impl<'a> ScannerScanRequest<'a> {
    /// Creates a scanner request with no paths or collected facts attached.
    pub const fn new(scan_id: u64, settings: &'a ScannerSettings) -> Self {
        Self {
            scan_id,
            settings,
            installation: None,
            data_path: None,
            overview_problems: &[],
            enabled_modules: &[],
            enabled_archives: &[],
            mod_manager: None,
        }
    }

    /// Adds a discovered installation; its Data path is used when no override is supplied.
    pub const fn with_installation(mut self, installation: &'a Fallout4Installation) -> Self {
        self.installation = Some(installation);
        self
    }

    /// Adds or overrides the Data path for traversal.
    pub fn with_data_path(mut self, data_path: impl Into<PathBuf>) -> Self {
        self.data_path = Some(data_path.into());
        self
    }

    /// Adds Overview problems for optional scanner handoff.
    pub const fn with_overview_problems(
        mut self,
        overview_problems: &'a [OverviewProblem],
    ) -> Self {
        self.overview_problems = overview_problems;
        self
    }

    /// Adds collected module records for race-subgraph counting.
    pub const fn with_enabled_modules(mut self, enabled_modules: &'a [ModuleRecord]) -> Self {
        self.enabled_modules = enabled_modules;
        self
    }

    /// Adds collected archive records for enabled-archive name checks.
    pub const fn with_enabled_archives(mut self, enabled_archives: &'a [ArchiveRecord]) -> Self {
        self.enabled_archives = enabled_archives;
        self
    }

    /// Adds optional manager context for staged attribution rules.
    pub const fn with_mod_manager(mut self, mod_manager: &'a ModManagerContext) -> Self {
        self.mod_manager = Some(mod_manager);
        self
    }

    fn effective_data_path(&self) -> Option<&Path> {
        self.data_path.as_deref().or_else(|| {
            self.installation
                .and_then(|installation| installation.data_path.as_deref())
        })
    }
}

/// A scanner progress phase safe to expose in logs or worker events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScannerScanPhase {
    /// Overview handoff phase before filesystem work.
    OverviewRefresh,
    /// Enabled module byte scan for race animation subgraph records.
    RaceSubgraphs,
    /// MO2 staged file index construction.
    ModIndex,
    /// Data path validation before traversal.
    DataValidation,
    /// Data tree traversal using direct `read_dir` calls.
    DataTraversal,
    /// Final grouping/counting phase.
    Complete,
}

impl ScannerScanPhase {
    /// Returns a stable lowercase label for tracing and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OverviewRefresh => "overview_refresh",
            Self::RaceSubgraphs => "race_subgraphs",
            Self::ModIndex => "mod_index",
            Self::DataValidation => "data_validation",
            Self::DataTraversal => "data_traversal",
            Self::Complete => "complete",
        }
    }
}

/// One safe progress event emitted during a scan.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannerProgressEvent {
    /// Scan id copied from the request.
    pub scan_id: u64,
    /// Phase that emitted this progress event.
    pub phase: ScannerScanPhase,
    /// Safe user-facing progress text.
    pub safe_message: String,
    /// Percent completion in the same 0-100 range as the reference progress bar.
    pub percent: f32,
    /// Optional top-level Data folder currently being scanned.
    pub folder: Option<String>,
    /// One-based top-level folder index, when available.
    pub folder_index: Option<usize>,
    /// Total top-level folders observed at Data root, when available.
    pub folder_total: Option<usize>,
}

impl ScannerProgressEvent {
    fn new(
        scan_id: u64,
        phase: ScannerScanPhase,
        safe_message: impl Into<String>,
        percent: f32,
    ) -> Self {
        Self {
            scan_id,
            phase,
            safe_message: safe_message.into(),
            percent,
            folder: None,
            folder_index: None,
            folder_total: None,
        }
    }

    fn folder(
        scan_id: u64,
        folder_index: usize,
        folder_total: usize,
        folder: impl Into<String>,
    ) -> Self {
        let folder = folder.into();
        let percent = if folder_total == 0 {
            PROGRESS_AFTER_OVERVIEW_PERCENT
        } else {
            (folder_index as f32 / folder_total as f32) * 100.0
        };
        Self {
            scan_id,
            phase: ScannerScanPhase::DataTraversal,
            safe_message: scanner_folder_progress_text(folder_index, folder_total, &folder),
            percent,
            folder: Some(folder),
            folder_index: Some(folder_index),
            folder_total: Some(folder_total),
        }
    }
}

/// Final lifecycle classification for a scan pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerScanStatusKind {
    /// At least one enabled category ran and no recoverable input failures were observed.
    Completed,
    /// Enabled work completed while one or more recoverable failures were captured.
    CompletedWithRecoverableIssues,
    /// No scanner categories were enabled in the settings snapshot.
    NoEnabledCategories,
    /// Data scan work was requested, but no usable Data folder was available.
    MissingData,
}

/// Safe final status for controllers and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerScanStatus {
    /// Machine-readable status kind.
    pub kind: ScannerScanStatusKind,
    /// Safe message suitable for UI status text.
    pub safe_message: String,
}

impl ScannerScanStatus {
    fn new(kind: ScannerScanStatusKind, safe_message: impl Into<String>) -> Self {
        Self {
            kind,
            safe_message: safe_message.into(),
        }
    }
}

/// Safe scanner diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScannerScanDiagnosticKind {
    /// A required Data path was absent or not a directory.
    MissingData,
    /// MO2 staged scanning was requested but required context was incomplete.
    MissingMo2Prerequisite,
    /// The selected MO2 profile did not contain a readable `modlist.txt`.
    MissingMo2Modlist,
    /// A directory could not be read; siblings were still scanned.
    UnreadableDirectory,
    /// A file could not be read; scanner work continued.
    UnreadableFile,
    /// A path or text input was malformed.
    InvalidInput,
    /// Work was intentionally skipped due to settings or unsupported manager scope.
    Skipped,
}

/// A safe diagnostic attached to a scanner run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerScanDiagnostic {
    /// Phase that observed the diagnostic.
    pub phase: ScannerScanPhase,
    /// Typed diagnostic category.
    pub kind: ScannerScanDiagnosticKind,
    /// Local path involved in the diagnostic, if one was selected/discovered.
    pub path: Option<PathBuf>,
    /// User-safe message; raw OS details remain in tracing diagnostics.
    pub safe_message: String,
}

/// Aggregate observability metadata for a scanner run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScannerScanDiagnostics {
    /// Scan id copied from the request.
    pub scan_id: u64,
    /// Overview problems copied into scanner results before category grouping.
    pub overview_problem_count: usize,
    /// Number of MO2 mod roots indexed.
    pub indexed_mod_count: usize,
    /// Number of MO2 relative folders indexed.
    pub indexed_folder_count: usize,
    /// Number of MO2 relative files indexed.
    pub indexed_file_count: usize,
    /// Number of root-level MO2 modules indexed.
    pub indexed_module_count: usize,
    /// Number of root-level MO2 archives indexed.
    pub indexed_archive_count: usize,
    /// Number of Data directories successfully read.
    pub traversed_folder_count: usize,
    /// Number of Data files considered after skip-suffix filtering.
    pub traversed_file_count: usize,
    /// Number of folders pruned by ignore, whitelist, or problem-folder rules.
    pub skipped_directory_count: usize,
    /// Count of recoverable file/directory read failures.
    pub partial_read_failure_count: usize,
    /// Total SADD records counted across enabled readable modules.
    pub race_subgraph_record_count: usize,
    /// Number of enabled modules contributing at least one SADD record.
    pub race_subgraph_module_count: usize,
    /// Final row counts by scanner problem label.
    pub rows_by_problem_type: BTreeMap<String, usize>,
    /// Safe per-path diagnostics for later logs/status panes/tests.
    pub errors: Vec<ScannerScanDiagnostic>,
}

impl ScannerScanDiagnostics {
    fn new(scan_id: u64) -> Self {
        Self {
            scan_id,
            ..Self::default()
        }
    }

    fn record_error(
        &mut self,
        phase: ScannerScanPhase,
        kind: ScannerScanDiagnosticKind,
        path: impl Into<Option<PathBuf>>,
        safe_message: impl Into<String>,
    ) {
        if matches!(
            kind,
            ScannerScanDiagnosticKind::UnreadableDirectory
                | ScannerScanDiagnosticKind::UnreadableFile
                | ScannerScanDiagnosticKind::MissingMo2Modlist
                | ScannerScanDiagnosticKind::MissingMo2Prerequisite
        ) {
            self.partial_read_failure_count += 1;
        }
        self.errors.push(ScannerScanDiagnostic {
            phase,
            kind,
            path: path.into(),
            safe_message: safe_message.into(),
        });
    }

    fn record_platform_error(
        &mut self,
        phase: ScannerScanPhase,
        path: impl Into<Option<PathBuf>>,
        error: &PlatformError,
    ) {
        let kind = diagnostic_kind_from_platform_error(error);
        self.record_error(phase, kind, path, error.user_message().to_owned());
    }
}

/// Complete scanner output ready for controller/model projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannerScanOutput {
    /// Scan id copied from the request.
    pub scan_id: u64,
    /// Final flat results before UI model projection.
    pub results: Vec<ScannerResult>,
    /// Deterministic grouped results.
    pub groups: Vec<ScannerResultGroup>,
    /// Safe final status.
    pub status: ScannerScanStatus,
    /// Progress events emitted synchronously by the service.
    pub progress: Vec<ScannerProgressEvent>,
    /// Structured diagnostics and counts.
    pub diagnostics: ScannerScanDiagnostics,
}

/// Read-only scanner service over an injected filesystem adapter.
#[derive(Debug)]
pub struct ScannerScanService<'a, F: Filesystem + ?Sized> {
    filesystem: &'a F,
}

impl<'a, F: Filesystem + ?Sized> ScannerScanService<'a, F> {
    /// Creates a scanner service without touching the filesystem.
    pub const fn new(filesystem: &'a F) -> Self {
        Self { filesystem }
    }

    /// Runs one read-only scanner pass.
    ///
    /// The method is intentionally infallible. Missing folders, unreadable
    /// children, malformed MO2 inputs, and unreadable module bytes are converted
    /// into safe diagnostics and (when the `Errors` scanner setting is enabled)
    /// scanner result rows while sibling work continues.
    pub fn scan(&self, request: ScannerScanRequest<'_>) -> ScannerScanOutput {
        self.scan_with_progress(request, |_| {})
    }

    /// Runs one read-only scanner pass and invokes `on_progress` as each safe
    /// progress event is observed.
    ///
    /// The returned output still contains the full progress history for tests
    /// and diagnostics; the callback exists for UI/runtime wiring that needs to
    /// update status while long filesystem work is still running.
    pub fn scan_with_progress<P>(
        &self,
        request: ScannerScanRequest<'_>,
        mut on_progress: P,
    ) -> ScannerScanOutput
    where
        P: FnMut(&ScannerProgressEvent),
    {
        let span = info_span!(
            "scanner.scan",
            scan_id = request.scan_id,
            has_data_path = request.effective_data_path().is_some(),
            overview_problem_count = request.overview_problems.len(),
            enabled_module_count = request
                .enabled_modules
                .iter()
                .filter(|module| module.enabled)
                .count(),
            enabled_archive_count = request
                .enabled_archives
                .iter()
                .filter(|archive| archive.enabled)
                .count(),
        );
        let _guard = span.enter();
        info!(event = "scanner-scan-request", "Scanner scan requested");

        let mut diagnostics = ScannerScanDiagnostics::new(request.scan_id);
        let mut progress = Vec::new();
        record_scanner_progress(
            &mut progress,
            ScannerProgressEvent::new(
                request.scan_id,
                ScannerScanPhase::OverviewRefresh,
                PROGRESS_REFRESHING_OVERVIEW_TEXT,
                PROGRESS_AFTER_OVERVIEW_PERCENT,
            ),
            &mut on_progress,
        );
        debug!(
            event = "scanner-overview-refresh-phase",
            scan_id = request.scan_id,
            "Scanner accepted Overview problem handoff"
        );

        if !any_category_enabled(request.settings) {
            let status = ScannerScanStatus::new(
                ScannerScanStatusKind::NoEnabledCategories,
                "No scanner categories are enabled.",
            );
            record_scanner_progress(
                &mut progress,
                ScannerProgressEvent::new(
                    request.scan_id,
                    ScannerScanPhase::Complete,
                    "Scanner completed with no enabled categories.",
                    PROGRESS_COMPLETE_PERCENT,
                ),
                &mut on_progress,
            );
            return finalize_output(request.scan_id, Vec::new(), status, progress, diagnostics);
        }

        let data_scan_enabled = data_scan_enabled(request.settings);
        let mut results = Vec::new();
        let mut mod_index = ModFileIndex::default();

        if request.settings.race_subgraphs {
            self.scan_race_subgraphs(
                &request,
                &mut results,
                &mut diagnostics,
                &mut progress,
                &mut on_progress,
            );
        }

        if data_scan_enabled {
            let Some(data_path) = request.effective_data_path() else {
                self.record_missing_data(&request, &mut results, &mut diagnostics);
                self.append_overview_results(&request, None, &mut results, &mut diagnostics);
                let status = ScannerScanStatus::new(
                    ScannerScanStatusKind::MissingData,
                    DATA_FOLDER_NOT_FOUND_SUMMARY,
                );
                record_scanner_progress(
                    &mut progress,
                    ScannerProgressEvent::new(
                        request.scan_id,
                        ScannerScanPhase::Complete,
                        DATA_FOLDER_NOT_FOUND_SUMMARY,
                        PROGRESS_COMPLETE_PERCENT,
                    ),
                    &mut on_progress,
                );
                return finalize_output(request.scan_id, results, status, progress, diagnostics);
            };

            if !self.validate_data_path(data_path, &request, &mut results, &mut diagnostics) {
                self.append_overview_results(&request, None, &mut results, &mut diagnostics);
                let status = ScannerScanStatus::new(
                    ScannerScanStatusKind::MissingData,
                    DATA_FOLDER_NOT_FOUND_SUMMARY,
                );
                record_scanner_progress(
                    &mut progress,
                    ScannerProgressEvent::new(
                        request.scan_id,
                        ScannerScanPhase::Complete,
                        DATA_FOLDER_NOT_FOUND_SUMMARY,
                        PROGRESS_COMPLETE_PERCENT,
                    ),
                    &mut on_progress,
                );
                return finalize_output(request.scan_id, results, status, progress, diagnostics);
            }

            if let Some(manager) = request.mod_manager {
                match manager {
                    ModManagerContext::ModOrganizer(context) => {
                        record_scanner_progress(
                            &mut progress,
                            ScannerProgressEvent::new(
                                request.scan_id,
                                ScannerScanPhase::ModIndex,
                                PROGRESS_BUILDING_MOD_INDEX_TEXT,
                                PROGRESS_AFTER_OVERVIEW_PERCENT,
                            ),
                            &mut on_progress,
                        );
                        self.build_mod_file_index(
                            context,
                            request.settings,
                            &mut mod_index,
                            &mut results,
                            &mut diagnostics,
                        );
                    }
                    ModManagerContext::Vortex(_) => {
                        diagnostics.record_error(
                            ScannerScanPhase::ModIndex,
                            ScannerScanDiagnosticKind::Skipped,
                            None,
                            "Vortex staging folders are not parsed; Scanner will scan Data only.",
                        );
                        debug!(
                            event = "scanner-vortex-data-only",
                            scan_id = request.scan_id,
                            "Vortex context detected; staged attribution intentionally skipped"
                        );
                    }
                }
            }

            self.append_overview_results(
                &request,
                Some(&mod_index),
                &mut results,
                &mut diagnostics,
            );
            self.scan_data_tree(
                data_path,
                &request,
                &mod_index,
                &mut results,
                &mut diagnostics,
                &mut progress,
                &mut on_progress,
            );
        } else {
            diagnostics.record_error(
                ScannerScanPhase::DataTraversal,
                ScannerScanDiagnosticKind::Skipped,
                None,
                "Data traversal skipped because all Data scan categories are disabled.",
            );
            self.append_overview_results(&request, None, &mut results, &mut diagnostics);
        }

        let status = if diagnostics
            .errors
            .iter()
            .any(|error| !matches!(error.kind, ScannerScanDiagnosticKind::Skipped))
        {
            ScannerScanStatus::new(
                ScannerScanStatusKind::CompletedWithRecoverableIssues,
                "Scanner completed with recoverable issues.",
            )
        } else {
            ScannerScanStatus::new(
                ScannerScanStatusKind::Completed,
                format!("Scanner completed with {} results.", results.len()),
            )
        };
        record_scanner_progress(
            &mut progress,
            ScannerProgressEvent::new(
                request.scan_id,
                ScannerScanPhase::Complete,
                status.safe_message.clone(),
                PROGRESS_COMPLETE_PERCENT,
            ),
            &mut on_progress,
        );

        finalize_output(request.scan_id, results, status, progress, diagnostics)
    }

    fn append_overview_results(
        &self,
        request: &ScannerScanRequest<'_>,
        mod_index: Option<&ModFileIndex>,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        if !request.settings.overview_issues {
            return;
        }

        diagnostics.overview_problem_count = request.overview_problems.len();
        for problem in request.overview_problems {
            let mut result = scanner_result_from_overview_problem(problem);
            if let Some(index) = mod_index
                && result
                    .mod_attribution
                    .as_ref()
                    .is_none_or(|attribution| attribution.name == "OVERVIEW")
                && let Some(relative_path) = problem.relative_path.as_deref()
                && let Some(attribution) = index.file(relative_path)
            {
                result = result.with_mod_attribution(attribution.mod_name.clone());
                result.absolute_path = Some(attribution.full_path.clone());
            }
            results.push(result);
        }
    }

    fn scan_race_subgraphs<P>(
        &self,
        request: &ScannerScanRequest<'_>,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
        progress: &mut Vec<ScannerProgressEvent>,
        on_progress: &mut P,
    ) where
        P: FnMut(&ScannerProgressEvent),
    {
        record_scanner_progress(
            progress,
            ScannerProgressEvent::new(
                request.scan_id,
                ScannerScanPhase::RaceSubgraphs,
                "Race Subgraph Records",
                PROGRESS_AFTER_OVERVIEW_PERCENT,
            ),
            on_progress,
        );

        let mut entries = Vec::new();
        let mut total = 0usize;
        for module in request
            .enabled_modules
            .iter()
            .filter(|module| module.enabled)
        {
            match self.filesystem.read_bytes(&module.path) {
                Ok(bytes) => {
                    let count = count_subsequence(&bytes, SADD_BYTES);
                    if count > 0 {
                        total += count;
                        entries.push(ScannerFileListEntry::new(count, module.path.clone()));
                    }
                }
                Err(error) => {
                    warn!(
                        event = "scanner-race-module-read-failure",
                        scan_id = request.scan_id,
                        kind = ?error.kind,
                        target = %error.target,
                        "Enabled module could not be read for race-subgraph counting"
                    );
                    diagnostics.record_platform_error(
                        ScannerScanPhase::RaceSubgraphs,
                        Some(module.path.clone()),
                        &error,
                    );
                }
            }
        }

        diagnostics.race_subgraph_record_count = total;
        diagnostics.race_subgraph_module_count = entries.len();
        info!(
            event = "scanner-race-subgraph-counts",
            scan_id = request.scan_id,
            sadd_total = total,
            contributing_modules = entries.len(),
            threshold = RACE_SUBGRAPH_THRESHOLD,
            "Race subgraph counting completed"
        );

        if total > RACE_SUBGRAPH_THRESHOLD {
            results.push(
                ScannerResult::simple(
                    ScannerProblemType::RaceSubgraphRecordCount,
                    format!("{total} SADD Records from {} modules", entries.len()),
                    INFO_SCAN_RACE_SUBGRAPHS,
                    Some(RACE_SUBGRAPH_SOLUTION.to_owned()),
                )
                .with_file_list(ScannerFileList::race_subgraph_records(entries)),
            );
        }
    }

    fn validate_data_path(
        &self,
        data_path: &Path,
        request: &ScannerScanRequest<'_>,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) -> bool {
        match self.filesystem.metadata(data_path) {
            Ok(metadata) if metadata.is_dir() => true,
            Ok(_) => {
                diagnostics.record_error(
                    ScannerScanPhase::DataValidation,
                    ScannerScanDiagnosticKind::MissingData,
                    Some(data_path.to_path_buf()),
                    "Data path is not a directory.",
                );
                if request.settings.errors {
                    results.push(scanner_error_row(
                        ScannerProblemType::FileNotFound,
                        Some(data_path.to_path_buf()),
                        PathBuf::from("Data"),
                        "Data path is not a directory.",
                    ));
                }
                false
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    ScannerScanPhase::DataValidation,
                    Some(data_path.to_path_buf()),
                    &error,
                );
                if request.settings.errors {
                    results.push(scanner_error_row(
                        ScannerProblemType::FileNotFound,
                        Some(data_path.to_path_buf()),
                        PathBuf::from("Data"),
                        DATA_FOLDER_NOT_FOUND_SUMMARY,
                    ));
                }
                false
            }
        }
    }

    fn record_missing_data(
        &self,
        request: &ScannerScanRequest<'_>,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        diagnostics.record_error(
            ScannerScanPhase::DataValidation,
            ScannerScanDiagnosticKind::MissingData,
            None,
            DATA_FOLDER_NOT_FOUND_SUMMARY,
        );
        if request.settings.errors {
            results.push(scanner_error_row(
                ScannerProblemType::FileNotFound,
                None,
                PathBuf::from("Data"),
                DATA_FOLDER_NOT_FOUND_SUMMARY,
            ));
        }
    }

    fn build_mod_file_index(
        &self,
        context: &ModOrganizerContext,
        settings: &ScannerSettings,
        index: &mut ModFileIndex,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        debug!(
            event = "scanner-mo2-index-build-started",
            selected_profile = %context.selected_profile,
            "Building MO2 staged file index"
        );

        if path_is_empty(context.mod_directory())
            || path_is_empty(context.profiles_directory())
            || path_is_empty(context.overwrite_directory())
            || context.selected_profile.trim().is_empty()
        {
            diagnostics.record_error(
                ScannerScanPhase::ModIndex,
                ScannerScanDiagnosticKind::MissingMo2Prerequisite,
                None,
                "Missing MO2 settings",
            );
            if settings.errors {
                results.push(scanner_error_row(
                    ScannerProblemType::FileNotFound,
                    None,
                    PathBuf::from("Mod Organizer"),
                    "Missing MO2 settings",
                ));
            }
            return;
        }

        let modlist_path = context
            .profiles_directory()
            .join(&context.selected_profile)
            .join("modlist.txt");
        match self.filesystem.is_file(&modlist_path) {
            Ok(true) => {}
            Ok(false) => {
                self.record_missing_modlist(&modlist_path, settings, results, diagnostics);
                return;
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    ScannerScanPhase::ModIndex,
                    Some(modlist_path.clone()),
                    &error,
                );
                if settings.errors {
                    results.push(scanner_error_row(
                        ScannerProblemType::FileNotFound,
                        Some(modlist_path.clone()),
                        modlist_path.clone(),
                        error.user_message().to_owned(),
                    ));
                }
                return;
            }
        }

        let text = match self.filesystem.read_to_string(&modlist_path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.record_platform_error(
                    ScannerScanPhase::ModIndex,
                    Some(modlist_path.clone()),
                    &error,
                );
                if settings.errors {
                    results.push(scanner_error_row(
                        ScannerProblemType::FileNotFound,
                        Some(modlist_path.clone()),
                        modlist_path.clone(),
                        error.user_message().to_owned(),
                    ));
                }
                return;
            }
        };

        let rules = ScanSkipRules::for_manager(Some(context));
        let mut stage_paths = Vec::new();
        for line in text.lines().rev() {
            let Some(mod_name) = line.strip_prefix('+') else {
                continue;
            };
            if mod_name.is_empty() {
                continue;
            }
            let mod_path = context.mod_directory().join(mod_name);
            if self.lenient_is_dir(&mod_path, ScannerScanPhase::ModIndex, diagnostics) {
                stage_paths.push(mod_path);
            }
        }
        if self.lenient_is_dir(
            context.overwrite_directory(),
            ScannerScanPhase::ModIndex,
            diagnostics,
        ) {
            stage_paths.push(context.overwrite_directory().to_path_buf());
        }

        for mod_path in &stage_paths {
            let mod_name = display_file_name(mod_path);
            self.index_mod_directory(
                mod_path,
                mod_path,
                Path::new(""),
                &mod_name,
                &rules,
                index,
                diagnostics,
            );
        }

        diagnostics.indexed_mod_count = stage_paths.len();
        diagnostics.indexed_folder_count = index.folders.len();
        diagnostics.indexed_file_count = index.files.len();
        diagnostics.indexed_module_count = index.modules.len();
        diagnostics.indexed_archive_count = index.archives.len();
        info!(
            event = "scanner-mo2-index-build-completed",
            indexed_mods = diagnostics.indexed_mod_count,
            indexed_folders = diagnostics.indexed_folder_count,
            indexed_files = diagnostics.indexed_file_count,
            indexed_modules = diagnostics.indexed_module_count,
            indexed_archives = diagnostics.indexed_archive_count,
            "MO2 staged file index completed"
        );
    }

    fn record_missing_modlist(
        &self,
        modlist_path: &Path,
        settings: &ScannerSettings,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        let message = format!("File doesn't exist: {}", modlist_path.display());
        diagnostics.record_error(
            ScannerScanPhase::ModIndex,
            ScannerScanDiagnosticKind::MissingMo2Modlist,
            Some(modlist_path.to_path_buf()),
            message.clone(),
        );
        if settings.errors {
            results.push(scanner_error_row(
                ScannerProblemType::FileNotFound,
                Some(modlist_path.to_path_buf()),
                modlist_path.to_path_buf(),
                message,
            ));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn index_mod_directory(
        &self,
        mod_root: &Path,
        current_path: &Path,
        relative_path: &Path,
        mod_name: &str,
        rules: &ScanSkipRules,
        index: &mut ModFileIndex,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        if !relative_path.as_os_str().is_empty() {
            index.folders.insert(
                path_key(relative_path),
                ModIndexedPath::new(mod_name, current_path.to_path_buf()),
            );
        }

        let entries = match self.filesystem.read_dir(current_path) {
            Ok(entries) => sorted_entries(entries),
            Err(error) => {
                warn!(
                    event = "scanner-mo2-index-read-dir-failure",
                    kind = ?error.kind,
                    target = %error.target,
                    "MO2 staged directory could not be read while building scanner attribution index"
                );
                diagnostics.record_platform_error(
                    ScannerScanPhase::ModIndex,
                    Some(current_path.to_path_buf()),
                    &error,
                );
                return;
            }
        };

        let (folders, files) = partition_entries(entries);
        for file in files {
            let file_name = display_file_name(&file.path);
            let file_lower = file_name.to_ascii_lowercase();
            if rules.skip_file(&file_lower) {
                continue;
            }
            let file_relative = relative_path.join(&file_name);
            let indexed = ModIndexedPath::new(mod_name, file.path.clone());
            index
                .files
                .insert(path_key(&file_relative), indexed.clone());
            if relative_path.as_os_str().is_empty() {
                if has_extension_name(&file_name, &["esp", "esl", "esm"]) {
                    index.modules.insert(file_lower.clone(), indexed.clone());
                } else if has_extension_name(&file_name, &["ba2"]) {
                    index.archives.insert(file_lower, indexed);
                }
            }
        }

        for folder in folders {
            let folder_name = display_file_name(&folder.path);
            let folder_lower = folder_name.to_ascii_lowercase();
            if rules.skip_directory(&folder_lower) {
                continue;
            }
            let folder_relative = relative_path.join(&folder_name);
            self.index_mod_directory(
                mod_root,
                &folder.path,
                &folder_relative,
                mod_name,
                rules,
                index,
                diagnostics,
            );
        }

        debug_assert!(current_path.starts_with(mod_root));
    }

    fn scan_data_tree<P>(
        &self,
        data_path: &Path,
        request: &ScannerScanRequest<'_>,
        mod_index: &ModFileIndex,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
        progress: &mut Vec<ScannerProgressEvent>,
        on_progress: &mut P,
    ) where
        P: FnMut(&ScannerProgressEvent),
    {
        let rules = match request.mod_manager {
            Some(ModManagerContext::ModOrganizer(context)) => {
                ScanSkipRules::for_manager(Some(context))
            }
            _ => ScanSkipRules::for_manager(None),
        };
        let archive_suffixes = request
            .installation
            .map(|installation| ba2_suffixes(&installation.ini_files))
            .unwrap_or_else(default_ba2_suffixes);
        let enabled_archives = enabled_archive_keys(request.enabled_archives);

        let entries = match self.filesystem.read_dir(data_path) {
            Ok(entries) => sorted_entries(entries),
            Err(error) => {
                diagnostics.record_platform_error(
                    ScannerScanPhase::DataTraversal,
                    Some(data_path.to_path_buf()),
                    &error,
                );
                self.add_read_error_row(
                    request,
                    results,
                    data_path,
                    Path::new("Data"),
                    error.user_message(),
                );
                return;
            }
        };
        diagnostics.traversed_folder_count += 1;

        let (folders, files) = partition_entries(entries);
        let total_folders = folders.len();
        for file in files {
            self.scan_data_file(
                data_path,
                data_path,
                Path::new(""),
                "Data",
                file,
                request,
                mod_index,
                &archive_suffixes,
                &enabled_archives,
                &rules,
                results,
                diagnostics,
            );
        }

        for (index, folder) in folders.into_iter().enumerate() {
            let folder_name = display_file_name(&folder.path);
            record_scanner_progress(
                progress,
                ScannerProgressEvent::folder(
                    request.scan_id,
                    index + 1,
                    total_folders,
                    folder_name.clone(),
                ),
                on_progress,
            );
            debug!(
                event = "scanner-data-root-progress",
                scan_id = request.scan_id,
                folder = %folder_name,
                folder_index = index + 1,
                folder_total = total_folders,
                "Scanner advanced to top-level Data folder"
            );

            let folder_relative = PathBuf::from(&folder_name);
            let folder_lower = folder_name.to_ascii_lowercase();
            if rules.skip_directory(&folder_lower) {
                diagnostics.skipped_directory_count += 1;
                continue;
            }

            if request.settings.junk_files && folder_lower == "fomod" {
                results.push(path_result_with_solution_kind(
                    ScannerProblemType::JunkFile,
                    &folder.path,
                    &folder_relative,
                    mod_index.folder(&folder_relative),
                    JUNK_FOLDER_SUMMARY,
                    ScannerSolutionKind::DeleteOrIgnoreFolder,
                ));
                diagnostics.skipped_directory_count += 1;
                continue;
            }

            if !is_known_data_root(&folder_lower) {
                diagnostics.skipped_directory_count += 1;
                continue;
            }

            if request.settings.loose_previs && folder_lower == "vis" {
                results.push(path_result_with_solution_kind(
                    ScannerProblemType::LoosePrevis,
                    &folder.path,
                    &folder_relative,
                    mod_index.folder(&folder_relative),
                    LOOSE_PREVIS_SUMMARY,
                    ScannerSolutionKind::ArchiveFolder,
                ));
                diagnostics.skipped_directory_count += 1;
                continue;
            }

            self.scan_data_directory(
                data_path,
                &folder.path,
                &folder_relative,
                &folder_lower,
                request,
                mod_index,
                &archive_suffixes,
                &enabled_archives,
                &rules,
                results,
                diagnostics,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_data_directory(
        &self,
        data_path: &Path,
        current_path: &Path,
        relative_path: &Path,
        data_root_lower: &str,
        request: &ScannerScanRequest<'_>,
        mod_index: &ModFileIndex,
        archive_suffixes: &[String],
        enabled_archives: &BTreeSet<String>,
        rules: &ScanSkipRules,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        let entries = match self.filesystem.read_dir(current_path) {
            Ok(entries) => sorted_entries(entries),
            Err(error) => {
                diagnostics.record_platform_error(
                    ScannerScanPhase::DataTraversal,
                    Some(current_path.to_path_buf()),
                    &error,
                );
                self.add_read_error_row(
                    request,
                    results,
                    current_path,
                    relative_path,
                    error.user_message(),
                );
                return;
            }
        };
        diagnostics.traversed_folder_count += 1;

        let (folders, files) = partition_entries(entries);
        let mut recurse_folders = Vec::new();
        for folder in folders {
            let folder_name = display_file_name(&folder.path);
            let folder_lower = folder_name.to_ascii_lowercase();
            if rules.skip_directory(&folder_lower) {
                diagnostics.skipped_directory_count += 1;
                continue;
            }
            let folder_relative = relative_path.join(&folder_name);

            if data_root_lower == "meshes" {
                if request.settings.loose_previs && folder_lower == "precombined" {
                    results.push(path_result_with_solution_kind(
                        ScannerProblemType::LoosePrevis,
                        &folder.path,
                        &folder_relative,
                        mod_index.folder(&folder_relative),
                        LOOSE_PREVIS_SUMMARY,
                        ScannerSolutionKind::ArchiveFolder,
                    ));
                    diagnostics.skipped_directory_count += 1;
                    continue;
                }

                if request.settings.problem_overrides && folder_lower == "animtextdata" {
                    results.push(path_result_with_solution_kind(
                        ScannerProblemType::LooseAnimTextData,
                        &folder.path,
                        &folder_relative,
                        mod_index.folder(&folder_relative),
                        LOOSE_ANIM_TEXT_DATA_SUMMARY,
                        ScannerSolutionKind::ArchiveFolder,
                    ));
                    diagnostics.skipped_directory_count += 1;
                    continue;
                }
            }

            recurse_folders.push((folder.path, folder_relative));
        }

        for file in files {
            self.scan_data_file(
                data_path,
                current_path,
                relative_path,
                data_root_lower,
                file,
                request,
                mod_index,
                archive_suffixes,
                enabled_archives,
                rules,
                results,
                diagnostics,
            );
        }

        for (folder_path, folder_relative) in recurse_folders {
            self.scan_data_directory(
                data_path,
                &folder_path,
                &folder_relative,
                data_root_lower,
                request,
                mod_index,
                archive_suffixes,
                enabled_archives,
                rules,
                results,
                diagnostics,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_data_file(
        &self,
        data_path: &Path,
        current_path: &Path,
        relative_path: &Path,
        data_root_lower: &str,
        file: DirectoryEntry,
        request: &ScannerScanRequest<'_>,
        mod_index: &ModFileIndex,
        archive_suffixes: &[String],
        enabled_archives: &BTreeSet<String>,
        rules: &ScanSkipRules,
        results: &mut Vec<ScannerResult>,
        diagnostics: &mut ScannerScanDiagnostics,
    ) {
        if file.file_type != FileType::File {
            return;
        }
        let file_name = display_file_name(&file.path);
        let file_lower = file_name.to_ascii_lowercase();
        if rules.skip_file(&file_lower) {
            return;
        }
        diagnostics.traversed_file_count += 1;

        let file_relative = relative_path.join(&file_name);
        let attribution = mod_index.file(&file_relative);

        if request.settings.junk_files
            && (JUNK_FILES.contains(&file_lower.as_str())
                || JUNK_FILE_SUFFIXES
                    .iter()
                    .any(|suffix| file_lower.ends_with(suffix)))
        {
            results.push(path_result_with_solution_kind(
                ScannerProblemType::JunkFile,
                &file.path,
                &file_relative,
                attribution,
                JUNK_FILE_SUMMARY,
                ScannerSolutionKind::DeleteOrIgnoreFile,
            ));
            return;
        }

        if request.settings.problem_overrides
            && data_root_lower == "scripts"
            && current_path == data_path.join("Scripts")
            && attribution.is_some()
            && F4SE_SCRIPT_NAMES.contains(&file_lower.as_str())
        {
            results.push(path_result(
                ScannerProblemType::F4seScriptOverride,
                &file.path,
                &file_relative,
                attribution,
                F4SE_SCRIPT_OVERRIDE_SUMMARY,
                Some(F4SE_SCRIPT_OVERRIDE_SOLUTION.to_owned()),
            ));
            return;
        }

        let Some((stem_lower, file_ext)) = split_extension_lower(&file_lower) else {
            return;
        };

        if request.settings.wrong_format {
            if self.is_wrong_format(data_root_lower, &file_relative, &file_ext) {
                results.push(self.unexpected_format_result(
                    data_root_lower,
                    &file.path,
                    &file_relative,
                    attribution,
                    &file_ext,
                    diagnostics,
                ));
                return;
            }

            if file_ext == "ba2"
                && !ARCHIVE_NAME_WHITELIST.contains(&file_lower.as_str())
                && !enabled_archives.contains(&path_key(&file.path))
            {
                let (base_name, suffix_valid) =
                    archive_name_suffix_valid(&stem_lower, archive_suffixes);
                if !suffix_valid {
                    results.push(
                        path_result_with_solution_kind(
                            ScannerProblemType::InvalidArchiveName,
                            &file.path,
                            &file_relative,
                            attribution,
                            INVALID_ARCHIVE_NAME_SUMMARY,
                            ScannerSolutionKind::RenameArchive,
                        )
                        .with_extra_data(vec![
                            ScannerExtraData::text(format!(
                                "\nValid Suffixes: {}",
                                archive_suffixes.join(", ")
                            )),
                            ScannerExtraData::text(format!("Example: {base_name} - Main.ba2")),
                        ]),
                    );
                }
            }
        }
    }

    fn is_wrong_format(&self, data_root_lower: &str, relative_path: &Path, file_ext: &str) -> bool {
        let disallowed_by_whitelist = data_root_whitelist(data_root_lower)
            .and_then(|whitelist| whitelist)
            .is_some_and(|whitelist| !whitelist.contains(&file_ext));
        let misplaced_dll =
            file_ext == "dll" && !relative_starts_with(relative_path, "f4se/plugins");
        disallowed_by_whitelist || misplaced_dll
    }

    fn unexpected_format_result(
        &self,
        data_root_lower: &str,
        absolute_path: &Path,
        relative_path: &Path,
        attribution: Option<&ModIndexedPath>,
        file_ext: &str,
        diagnostics: &mut ScannerScanDiagnostics,
    ) -> ScannerResult {
        let (summary, solution) = if let Some(expected_formats) = proper_formats(file_ext) {
            let proper_found = expected_formats
                .iter()
                .filter_map(|extension| {
                    let candidate = absolute_path.with_extension(extension);
                    self.lenient_is_file(&candidate, ScannerScanPhase::DataTraversal, diagnostics)
                        .then(|| display_file_name(&candidate))
                })
                .collect::<Vec<_>>();
            if proper_found.is_empty() {
                (
                    format!(
                        "Format not in whitelist for {data_root_lower}.\nA file with the expected format was NOT found ({}).",
                        expected_formats.join(", ")
                    ),
                    ScannerSolutionKind::ConvertDeleteOrIgnoreFile,
                )
            } else {
                (
                    format!(
                        "Format not in whitelist for {data_root_lower}.\nA file with the expected format was found ({}).",
                        proper_found.join(", ")
                    ),
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )
            }
        } else {
            (
                format!(
                    "Format not in whitelist for {data_root_lower}.\nUnable to determine whether the game will use this file."
                ),
                ScannerSolutionKind::UnknownFormat,
            )
        };

        path_result_with_solution_kind(
            ScannerProblemType::UnexpectedFormat,
            absolute_path,
            relative_path,
            attribution,
            summary,
            solution,
        )
    }

    fn add_read_error_row(
        &self,
        request: &ScannerScanRequest<'_>,
        results: &mut Vec<ScannerResult>,
        path: &Path,
        relative_path: &Path,
        safe_message: &str,
    ) {
        if request.settings.errors {
            results.push(scanner_error_row(
                ScannerProblemType::FileNotFound,
                Some(path.to_path_buf()),
                relative_path.to_path_buf(),
                safe_message.to_owned(),
            ));
        }
    }

    fn lenient_is_dir(
        &self,
        path: &Path,
        phase: ScannerScanPhase,
        diagnostics: &mut ScannerScanDiagnostics,
    ) -> bool {
        match self.filesystem.is_dir(path) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.record_platform_error(phase, Some(path.to_path_buf()), &error);
                false
            }
        }
    }

    fn lenient_is_file(
        &self,
        path: &Path,
        phase: ScannerScanPhase,
        diagnostics: &mut ScannerScanDiagnostics,
    ) -> bool {
        match self.filesystem.is_file(path) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.record_platform_error(phase, Some(path.to_path_buf()), &error);
                false
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ModFileIndex {
    folders: BTreeMap<String, ModIndexedPath>,
    files: BTreeMap<String, ModIndexedPath>,
    modules: BTreeMap<String, ModIndexedPath>,
    archives: BTreeMap<String, ModIndexedPath>,
}

impl ModFileIndex {
    fn folder(&self, relative_path: &Path) -> Option<&ModIndexedPath> {
        self.folders.get(&path_key(relative_path))
    }

    fn file(&self, relative_path: &Path) -> Option<&ModIndexedPath> {
        self.files.get(&path_key(relative_path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModIndexedPath {
    mod_name: String,
    full_path: PathBuf,
}

impl ModIndexedPath {
    fn new(mod_name: impl Into<String>, full_path: PathBuf) -> Self {
        Self {
            mod_name: mod_name.into(),
            full_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanSkipRules {
    file_suffixes: Vec<String>,
    directories: BTreeSet<String>,
}

impl ScanSkipRules {
    fn for_manager(context: Option<&ModOrganizerContext>) -> Self {
        let mut file_suffixes = DEFAULT_SKIP_FILE_SUFFIXES
            .iter()
            .map(|suffix| (*suffix).to_owned())
            .collect::<Vec<_>>();
        let mut directories = IGNORE_FOLDERS
            .iter()
            .map(|directory| (*directory).to_owned())
            .collect::<BTreeSet<_>>();
        if let Some(context) = context {
            for suffix in &context.skip_rules.file_suffixes {
                let suffix = suffix.to_ascii_lowercase();
                if !file_suffixes.contains(&suffix) {
                    file_suffixes.push(suffix);
                }
            }
            directories.extend(
                context
                    .skip_rules
                    .directories
                    .iter()
                    .map(|dir| dir.to_ascii_lowercase()),
            );
        }
        Self {
            file_suffixes,
            directories,
        }
    }

    fn skip_file(&self, file_lower: &str) -> bool {
        self.file_suffixes
            .iter()
            .any(|suffix| file_lower.ends_with(suffix))
    }

    fn skip_directory(&self, folder_lower: &str) -> bool {
        self.directories.contains(folder_lower)
    }
}

fn record_scanner_progress<P>(
    progress: &mut Vec<ScannerProgressEvent>,
    event: ScannerProgressEvent,
    on_progress: &mut P,
) where
    P: FnMut(&ScannerProgressEvent),
{
    on_progress(&event);
    progress.push(event);
}

fn finalize_output(
    scan_id: u64,
    results: Vec<ScannerResult>,
    status: ScannerScanStatus,
    progress: Vec<ScannerProgressEvent>,
    mut diagnostics: ScannerScanDiagnostics,
) -> ScannerScanOutput {
    diagnostics.rows_by_problem_type = rows_by_problem_type(&results);
    let groups = group_scanner_results(&results);
    info!(
        event = "scanner-scan-completed",
        scan_id,
        result_count = results.len(),
        group_count = groups.len(),
        traversed_folders = diagnostics.traversed_folder_count,
        traversed_files = diagnostics.traversed_file_count,
        indexed_mods = diagnostics.indexed_mod_count,
        partial_read_failures = diagnostics.partial_read_failure_count,
        status = ?status.kind,
        "Scanner scan completed"
    );
    ScannerScanOutput {
        scan_id,
        results,
        groups,
        status,
        progress,
        diagnostics,
    }
}

fn rows_by_problem_type(results: &[ScannerResult]) -> BTreeMap<String, usize> {
    let mut rows = BTreeMap::new();
    for result in results {
        *rows
            .entry(result.problem_type.label().to_owned())
            .or_insert(0) += 1;
    }
    rows
}

fn path_result(
    problem_type: ScannerProblemType,
    absolute_path: &Path,
    relative_path: &Path,
    attribution: Option<&ModIndexedPath>,
    summary: impl Into<String>,
    solution: Option<String>,
) -> ScannerResult {
    let mut result = ScannerResult::with_path(
        problem_type,
        attribution
            .map(|attribution| attribution.full_path.clone())
            .unwrap_or_else(|| absolute_path.to_path_buf()),
        relative_path.to_path_buf(),
        summary,
        solution,
    );
    if let Some(attribution) = attribution {
        result = result.with_mod_attribution(attribution.mod_name.clone());
    }
    result
}

fn path_result_with_solution_kind(
    problem_type: ScannerProblemType,
    absolute_path: &Path,
    relative_path: &Path,
    attribution: Option<&ModIndexedPath>,
    summary: impl Into<String>,
    solution_kind: ScannerSolutionKind,
) -> ScannerResult {
    path_result(
        problem_type,
        absolute_path,
        relative_path,
        attribution,
        summary,
        None,
    )
    .with_solution_kind(solution_kind)
}

fn scanner_error_row(
    problem_type: ScannerProblemType,
    absolute_path: Option<PathBuf>,
    relative_path: PathBuf,
    summary: impl Into<String>,
) -> ScannerResult {
    match absolute_path {
        Some(path) => ScannerResult::with_path(problem_type, path, relative_path, summary, None),
        None => ScannerResult::simple(
            problem_type,
            relative_path.display().to_string(),
            summary,
            None,
        ),
    }
}

fn diagnostic_kind_from_platform_error(error: &PlatformError) -> ScannerScanDiagnosticKind {
    match error.kind {
        PlatformErrorKind::NotFound => ScannerScanDiagnosticKind::MissingData,
        PlatformErrorKind::PermissionDenied
        | PlatformErrorKind::CommandFailed
        | PlatformErrorKind::Io => match error.operation {
            crate::platform::PlatformOperation::ReadDirectory
            | crate::platform::PlatformOperation::WalkDirectory => {
                ScannerScanDiagnosticKind::UnreadableDirectory
            }
            _ => ScannerScanDiagnosticKind::UnreadableFile,
        },
        PlatformErrorKind::InvalidInput | PlatformErrorKind::ParseError => {
            ScannerScanDiagnosticKind::InvalidInput
        }
        PlatformErrorKind::UnsupportedPlatform => ScannerScanDiagnosticKind::Skipped,
    }
}

fn any_category_enabled(settings: &ScannerSettings) -> bool {
    settings.overview_issues
        || settings.errors
        || settings.wrong_format
        || settings.loose_previs
        || settings.junk_files
        || settings.problem_overrides
        || settings.race_subgraphs
}

fn data_scan_enabled(settings: &ScannerSettings) -> bool {
    settings.errors
        || settings.wrong_format
        || settings.loose_previs
        || settings.junk_files
        || settings.problem_overrides
}

fn sorted_entries(mut entries: Vec<DirectoryEntry>) -> Vec<DirectoryEntry> {
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn partition_entries(entries: Vec<DirectoryEntry>) -> (Vec<DirectoryEntry>, Vec<DirectoryEntry>) {
    let mut folders = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        match entry.file_type {
            FileType::Directory => folders.push(entry),
            FileType::File => files.push(entry),
            FileType::Symlink | FileType::Other => {}
        }
    }
    (folders, files)
}

fn data_root_whitelist(data_root_lower: &str) -> Option<Option<&'static [&'static str]>> {
    match data_root_lower {
        "f4se" => Some(None),
        "materials" => Some(Some(&["bgem", "bgsm", "txt"])),
        "meshes" => Some(Some(&[
            "bto",
            "btr",
            "hko",
            "hkx",
            "hkx_back",
            "hkx_backup",
            "lst",
            "max",
            "nif",
            "obj",
            "sclp",
            "ssf",
            "tri",
            "txt",
            "xml",
        ])),
        "music" => Some(Some(&["wav", "xwm"])),
        "textures" => Some(Some(&["dds"])),
        "scripts" => Some(Some(&["pex", "psc", "txt", "zip"])),
        "sound" => Some(Some(&["cdf", "fuz", "lip", "wav", "xwm"])),
        "vis" => Some(Some(&["uvd"])),
        _ => None,
    }
}

fn is_known_data_root(data_root_lower: &str) -> bool {
    data_root_whitelist(data_root_lower).is_some()
}

fn proper_formats(file_ext: &str) -> Option<&'static [&'static str]> {
    match file_ext {
        "bmp" | "jpeg" | "jpg" | "png" | "psd" | "tga" => Some(&TEXTURE_PROPER_FORMATS),
        "mp3" => Some(&SOUND_PROPER_FORMATS),
        _ => None,
    }
}

fn enabled_archive_keys(records: &[ArchiveRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| record.enabled)
        .map(|record| path_key(&record.path))
        .collect()
}

fn archive_name_suffix_valid(stem_lower: &str, suffixes: &[String]) -> (String, bool) {
    match stem_lower.rsplit_once(" - ") {
        Some((base, suffix)) => (
            base.to_owned(),
            suffixes
                .iter()
                .any(|valid| valid.eq_ignore_ascii_case(suffix)),
        ),
        None => (stem_lower.to_owned(), false),
    }
}

fn split_extension_lower(file_lower: &str) -> Option<(String, String)> {
    file_lower
        .rsplit_once('.')
        .map(|(stem, extension)| (stem.to_owned(), extension.to_owned()))
}

fn has_extension_name(file_name: &str, extensions: &[&str]) -> bool {
    split_extension_lower(&file_name.to_ascii_lowercase())
        .is_some_and(|(_, extension)| extensions.contains(&extension.as_str()))
}

fn relative_starts_with(relative_path: &Path, expected_slash_path: &str) -> bool {
    path_key(relative_path).starts_with(expected_slash_path)
}

fn count_subsequence(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn default_ba2_suffixes() -> Vec<String> {
    vec![
        "main".to_owned(),
        "textures".to_owned(),
        "voices_en".to_owned(),
    ]
}

fn ba2_suffixes(ini_files: &Fallout4IniFiles) -> Vec<String> {
    let language = ini_value(&ini_files.custom, "general", "slanguage")
        .or_else(|| ini_value(&ini_files.fallout4, "general", "slanguage"))
        .unwrap_or("en")
        .trim()
        .to_ascii_lowercase();
    let mut suffixes = default_ba2_suffixes();
    if !language.is_empty() && language != "en" {
        suffixes.push(format!("voices_{language}"));
    }
    suffixes
}

fn ini_value<'a>(document: &'a IniDocument, section: &str, key: &str) -> Option<&'a str> {
    document.get(&section.to_ascii_lowercase(), &key.to_ascii_lowercase())
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn path_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

#[cfg(test)]
mod scanner_scan_service {
    use std::{cell::RefCell, collections::BTreeMap};

    use crate::{
        domain::{
            autofix::AutoFixOperationKey,
            discovery::{
                ArchiveFormat, ArchiveVersion, Fallout4Installation, ModuleHeaderVersion,
                ModuleKind, SemanticVersion,
            },
            mod_manager::{
                DetectedModManager, ModManagerKind, ModOrganizerDirectories, ModOrganizerSkipRules,
                VortexContext,
            },
            overview::{OverviewProblem, OverviewProblemSource, OverviewProblemType},
        },
        platform::{PlatformError, PlatformOperation, PlatformResult, filesystem::FileMetadata},
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        UnreadableFile,
        UnreadableDirectory,
    }

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
        read_dirs: RefCell<Vec<PathBuf>>,
        read_files: RefCell<Vec<PathBuf>>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::Directory);
            self
        }

        fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::File(bytes.into()));
            self
        }

        fn with_text(self, path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
            self.with_file(path, text.into().into_bytes())
        }

        fn with_unreadable_file(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::UnreadableFile);
            self
        }

        fn with_unreadable_dir(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::UnreadableDirectory);
            self
        }

        fn ensure_parent_dirs(&mut self, path: &Path) {
            let mut parents = Vec::new();
            let mut current = path.parent();
            while let Some(parent) = current {
                if parent.as_os_str().is_empty() {
                    break;
                }
                parents.push(parent.to_path_buf());
                current = parent.parent();
            }
            for parent in parents.into_iter().rev() {
                self.nodes.entry(parent).or_insert(FakeNode::Directory);
            }
        }

        fn node(&self, path: &Path, operation: PlatformOperation) -> PlatformResult<&FakeNode> {
            self.nodes.get(path).ok_or_else(|| {
                PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )
            })
        }

        fn permission_denied(path: &Path, operation: PlatformOperation) -> PlatformError {
            PlatformError::new(
                operation,
                path.display().to_string(),
                PlatformErrorKind::PermissionDenied,
                format!(
                    "{} target could not be accessed because permission was denied.",
                    operation.label()
                ),
            )
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata {
                    file_type: FileType::File,
                    len: bytes.len() as u64,
                }),
                FakeNode::Directory | FakeNode::UnreadableDirectory => Ok(FileMetadata {
                    file_type: FileType::Directory,
                    len: 0,
                }),
                FakeNode::UnreadableFile => Ok(FileMetadata {
                    file_type: FileType::File,
                    len: 0,
                }),
            }
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            self.read_files.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.clone()),
                FakeNode::Directory | FakeNode::UnreadableDirectory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => {
                    Err(Self::permission_denied(path, PlatformOperation::ReadFile))
                }
            }
        }

        fn read_to_string(&self, path: &Path) -> PlatformResult<String> {
            String::from_utf8(self.read_bytes(path)?).map_err(|error| {
                PlatformError::parse_error(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    error.to_string(),
                )
            })
        }

        fn read_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.read_dirs.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadDirectory)? {
                FakeNode::Directory => {}
                FakeNode::UnreadableDirectory => {
                    return Err(Self::permission_denied(
                        path,
                        PlatformOperation::ReadDirectory,
                    ));
                }
                FakeNode::File(_) | FakeNode::UnreadableFile => {
                    return Err(PlatformError::new(
                        PlatformOperation::ReadDirectory,
                        path.display().to_string(),
                        PlatformErrorKind::InvalidInput,
                        "Directory read target is invalid.",
                    ));
                }
            }

            let mut entries = Vec::new();
            for (candidate, node) in &self.nodes {
                if candidate.parent() == Some(path) {
                    let file_type = match node {
                        FakeNode::File(_) | FakeNode::UnreadableFile => FileType::File,
                        FakeNode::Directory | FakeNode::UnreadableDirectory => FileType::Directory,
                    };
                    entries.push(DirectoryEntry::new(candidate.clone(), file_type));
                }
            }
            entries.sort_by(|left, right| left.path.cmp(&right.path));
            Ok(entries)
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.read_dir(path)
        }
    }

    fn data_path() -> PathBuf {
        PathBuf::from("Game/Data")
    }

    fn service_scan<'a>(
        fs: &'a FakeFilesystem,
        _settings: &'a ScannerSettings,
        request: ScannerScanRequest<'a>,
    ) -> ScannerScanOutput {
        let service = ScannerScanService::new(fs);
        let request = if request.data_path.is_some() {
            request
        } else {
            request.with_data_path(data_path())
        };
        service.scan(request)
    }

    fn mo2_context() -> ModManagerContext {
        let manager = DetectedModManager::mod_organizer(
            "MO2/ModOrganizer.exe",
            SemanticVersion::new(2, 5, 3),
        );
        let directories = ModOrganizerDirectories::new(
            "MO2",
            "MO2/webcache",
            "MO2/downloads",
            "MO2/mods",
            "MO2/overwrite",
            "MO2/profiles",
        );
        let context = ModOrganizerContext::new(manager, "Default", directories)
            .with_skip_rules(ModOrganizerSkipRules::new([".mohidden"], ["SkippedDir"]));
        ModManagerContext::ModOrganizer(Box::new(context))
    }

    fn vortex_context() -> ModManagerContext {
        ModManagerContext::Vortex(VortexContext::new(
            "Vortex/Vortex.exe",
            Some(SemanticVersion::new(1, 12, 0)),
        ))
    }

    fn settings_with_all_data_rules_off() -> ScannerSettings {
        ScannerSettings {
            overview_issues: false,
            errors: false,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        }
    }

    fn problem_labels(output: &ScannerScanOutput) -> Vec<&str> {
        output
            .results
            .iter()
            .map(|result| result.problem_type.label())
            .collect()
    }

    fn result_by_type(
        output: &ScannerScanOutput,
        problem_type: ScannerProblemType,
    ) -> &ScannerResult {
        output
            .results
            .iter()
            .find(|result| result.problem_type == problem_type)
            .unwrap_or_else(|| panic!("missing result type {}", problem_type.label()))
    }

    #[test]
    fn scanner_scan_service_mo2_attribution_uses_modlist_order_and_reports_f4se_overrides() {
        let fs = FakeFilesystem::default()
            .with_file("Game/Data/Scripts/actor.pex", b"script")
            .with_text(
                "MO2/profiles/Default/modlist.txt",
                "+HighPriority\n+LowPriority\n",
            )
            .with_file("MO2/mods/LowPriority/Scripts/actor.pex", b"low")
            .with_file("MO2/mods/HighPriority/Scripts/actor.pex", b"high")
            .with_file("MO2/mods/HighPriority/Scripts/ignored.mohidden", b"hidden")
            .with_dir("MO2/overwrite");
        let manager = mo2_context();
        let settings = ScannerSettings {
            overview_issues: false,
            errors: true,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: true,
            race_subgraphs: false,
        };
        let request = ScannerScanRequest::new(7, &settings).with_mod_manager(&manager);

        let output = service_scan(&fs, &settings, request);

        assert_eq!(output.status.kind, ScannerScanStatusKind::Completed);
        let result = result_by_type(&output, ScannerProblemType::F4seScriptOverride);
        assert_eq!(result.mod_display_name(), "HighPriority");
        assert_eq!(result.detail_path, "Scripts/actor.pex");
        assert_eq!(
            result.absolute_path.as_deref(),
            Some(Path::new("MO2/mods/HighPriority/Scripts/actor.pex"))
        );
        assert_eq!(output.diagnostics.indexed_mod_count, 3);
        assert_eq!(output.diagnostics.indexed_file_count, 1);
        assert!(
            fs.read_dirs
                .borrow()
                .iter()
                .any(|path| path == Path::new("Game/Data"))
        );
    }

    #[test]
    fn scanner_scan_service_vortex_scans_data_only_without_mod_attribution() {
        let fs = FakeFilesystem::default().with_dir("Game/Data/fomod");
        let manager = vortex_context();
        let settings = ScannerSettings {
            overview_issues: false,
            errors: true,
            wrong_format: false,
            loose_previs: false,
            junk_files: true,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let request = ScannerScanRequest::new(8, &settings).with_mod_manager(&manager);

        let output = service_scan(&fs, &settings, request);

        let result = result_by_type(&output, ScannerProblemType::JunkFile);
        assert_eq!(result.detail_path, "fomod");
        assert_eq!(result.mod_attribution, None);
        assert_eq!(output.diagnostics.indexed_mod_count, 0);
        assert!(output.diagnostics.errors.iter().any(|diagnostic| {
            diagnostic.kind == ScannerScanDiagnosticKind::Skipped
                && diagnostic.safe_message.contains("Vortex")
        }));
    }

    #[test]
    fn scanner_scan_service_detects_reference_rule_categories_and_stable_group_order() {
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data/fomod")
            .with_dir("Game/Data/Vis")
            .with_dir("Game/Data/Meshes/World/PreCombined")
            .with_dir("Game/Data/Meshes/AnimTextData")
            .with_file("Game/Data/Textures/sign.png", b"png")
            .with_file("Game/Data/Textures/sign.dds", b"dds")
            .with_file("Game/Data/Music/theme.mp3", b"mp3")
            .with_file("Game/Data/Sound/plugin.dll", b"dll")
            .with_file("Game/Data/desktop.ini", b"junk")
            .with_file("Game/Data/BadArchive.ba2", b"archive")
            .with_file("Game/Data/UnknownRoot/ignored.png", b"ignored");
        let settings = ScannerSettings {
            overview_issues: false,
            race_subgraphs: false,
            ..ScannerSettings::default()
        };
        let request = ScannerScanRequest::new(9, &settings);

        let output = service_scan(&fs, &settings, request);

        let labels = problem_labels(&output);
        assert!(labels.contains(&"Junk File"));
        assert!(labels.contains(&"Loose Previs"));
        assert!(labels.contains(&"Loose AnimTextData"));
        assert!(labels.contains(&"Unexpected Format"));
        assert!(labels.contains(&"Invalid Archive Name"));
        assert!(output.results.iter().any(|result| {
            result
                .summary
                .contains("expected format was found (sign.dds)")
        }));
        assert!(output.results.iter().any(|result| {
            result
                .summary
                .contains("expected format was NOT found (wav, xwm)")
        }));
        assert_eq!(
            output
                .groups
                .iter()
                .map(|group| group.label.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Junk File",
                "Unexpected Format",
                "Loose Previs",
                "Loose AnimTextData",
                "Invalid Archive Name",
            ]
        );
        assert!(
            !output
                .results
                .iter()
                .any(|result| result.detail_path.contains("UnknownRoot"))
        );
    }

    #[test]
    fn scanner_autofix_domain_scan_service_preserves_typed_keys_without_string_matching() {
        let fs = FakeFilesystem::default().with_file("Game/Data/desktop.ini", b"junk");
        let overview = vec![OverviewProblem::with_path(
            OverviewProblemSource::Scanner,
            "Game/Data/overview-desktop.ini",
            Some(PathBuf::from("overview-desktop.ini")),
            OverviewProblemType::Custom("Junk File".to_owned()),
            "Overview supplied display-only problem.",
            Some(
                ScannerSolutionKind::DeleteOrIgnoreFile
                    .as_reference_text()
                    .to_owned(),
            ),
        )];
        let settings = ScannerSettings {
            overview_issues: true,
            errors: false,
            wrong_format: false,
            loose_previs: false,
            junk_files: true,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let request = ScannerScanRequest::new(909, &settings).with_overview_problems(&overview);

        let output = service_scan(&fs, &settings, request);

        let scanner_owned = output
            .results
            .iter()
            .find(|result| result.summary == JUNK_FILE_SUMMARY)
            .expect("scanner-owned junk-file result should be present");
        assert_eq!(
            scanner_owned.solution_kind,
            Some(ScannerSolutionKind::DeleteOrIgnoreFile)
        );
        assert_eq!(
            scanner_owned.auto_fix_operation_key(),
            Some(AutoFixOperationKey::DeleteOrIgnoreFile)
        );

        let overview_owned = output
            .results
            .iter()
            .find(|result| result.summary == "Overview supplied display-only problem.")
            .expect("overview handoff result should be present");
        assert_eq!(overview_owned.solution_kind, None);
        assert_eq!(overview_owned.auto_fix_operation_key(), None);
        assert_eq!(
            overview_owned.solution.as_deref(),
            Some(ScannerSolutionKind::DeleteOrIgnoreFile.as_reference_text())
        );
    }

    #[test]
    fn scanner_scan_service_unreadable_child_directory_reports_error_and_continues_siblings() {
        let fs = FakeFilesystem::default()
            .with_unreadable_dir("Game/Data/Textures/Denied")
            .with_file("Game/Data/Textures/ok.png", b"png");
        let settings = ScannerSettings {
            overview_issues: false,
            race_subgraphs: false,
            ..ScannerSettings::default()
        };
        let request = ScannerScanRequest::new(10, &settings);

        let output = service_scan(&fs, &settings, request);

        assert!(
            output
                .results
                .iter()
                .any(|result| result.detail_path == "Textures/ok.png")
        );
        assert!(
            output
                .results
                .iter()
                .any(|result| result.detail_path == "Textures/Denied")
        );
        assert!(output.diagnostics.errors.iter().any(|diagnostic| {
            diagnostic.kind == ScannerScanDiagnosticKind::UnreadableDirectory
                && diagnostic.path.as_deref() == Some(Path::new("Game/Data/Textures/Denied"))
        }));
        assert!(output.diagnostics.partial_read_failure_count >= 1);
    }

    #[test]
    fn scanner_scan_service_missing_data_returns_safe_visible_row_and_no_traversal() {
        let fs = FakeFilesystem::default();
        let settings = ScannerSettings::default();
        let request = ScannerScanRequest::new(11, &settings);
        let service = ScannerScanService::new(&fs);

        let output = service.scan(request);

        assert_eq!(output.status.kind, ScannerScanStatusKind::MissingData);
        assert_eq!(output.status.safe_message, DATA_FOLDER_NOT_FOUND_SUMMARY);
        assert_eq!(output.diagnostics.traversed_folder_count, 0);
        assert_eq!(output.results.len(), 1);
        assert_eq!(
            output.results[0].problem_type,
            ScannerProblemType::FileNotFound
        );
        assert_eq!(output.results[0].detail_path, "Data");
        assert!(fs.read_dirs.borrow().is_empty());
    }

    #[test]
    fn scanner_scan_service_missing_mo2_modlist_is_error_row_and_data_scan_continues() {
        let fs = FakeFilesystem::default().with_file("Game/Data/Textures/missing.png", b"png");
        let manager = mo2_context();
        let settings = ScannerSettings {
            overview_issues: false,
            race_subgraphs: false,
            ..ScannerSettings::default()
        };
        let request = ScannerScanRequest::new(12, &settings).with_mod_manager(&manager);

        let output = service_scan(&fs, &settings, request);

        assert!(output.results.iter().any(|result| {
            result.problem_type == ScannerProblemType::FileNotFound
                && result.summary.contains("modlist.txt")
        }));
        assert!(
            output
                .results
                .iter()
                .any(|result| result.detail_path == "Textures/missing.png")
        );
        assert_eq!(
            output.status.kind,
            ScannerScanStatusKind::CompletedWithRecoverableIssues
        );
        assert!(
            output.diagnostics.errors.iter().any(|diagnostic| {
                diagnostic.kind == ScannerScanDiagnosticKind::MissingMo2Modlist
            })
        );
    }

    #[test]
    fn scanner_scan_service_race_subgraph_threshold_counts_enabled_readable_modules_only() {
        let mut bytes = Vec::new();
        for _ in 0..101 {
            bytes.extend_from_slice(SADD_BYTES);
        }
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_file("Game/Data/RaceHeavy.esp", bytes)
            .with_unreadable_file("Game/Data/Unreadable.esp");
        let modules = vec![
            ModuleRecord::new(
                "Game/Data/RaceHeavy.esp",
                ModuleKind::Full,
                ModuleHeaderVersion::Version100,
                true,
            ),
            ModuleRecord::new(
                "Game/Data/Disabled.esp",
                ModuleKind::Full,
                ModuleHeaderVersion::Version100,
                false,
            ),
            ModuleRecord::new(
                "Game/Data/Unreadable.esp",
                ModuleKind::Full,
                ModuleHeaderVersion::Version100,
                true,
            ),
        ];
        let settings = ScannerSettings {
            overview_issues: false,
            errors: true,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: true,
        };
        let request = ScannerScanRequest::new(13, &settings).with_enabled_modules(&modules);

        let output = service_scan(&fs, &settings, request);

        let result = result_by_type(&output, ScannerProblemType::RaceSubgraphRecordCount);
        assert_eq!(result.detail_path, "101 SADD Records from 1 modules");
        assert_eq!(output.diagnostics.race_subgraph_record_count, 101);
        assert_eq!(output.diagnostics.race_subgraph_module_count, 1);
        assert!(output.diagnostics.errors.iter().any(|diagnostic| {
            diagnostic.kind == ScannerScanDiagnosticKind::UnreadableFile
                && diagnostic.path.as_deref() == Some(Path::new("Game/Data/Unreadable.esp"))
        }));
    }

    #[test]
    fn scanner_scan_service_zero_results_complete_with_empty_groups() {
        let fs = FakeFilesystem::default().with_dir("Game/Data/Textures");
        let settings = ScannerSettings {
            overview_issues: false,
            race_subgraphs: false,
            ..ScannerSettings::default()
        };
        let request = ScannerScanRequest::new(14, &settings);

        let output = service_scan(&fs, &settings, request);

        assert_eq!(output.status.kind, ScannerScanStatusKind::Completed);
        assert!(output.results.is_empty());
        assert!(output.groups.is_empty());
        assert_eq!(output.diagnostics.rows_by_problem_type, BTreeMap::new());
    }

    #[test]
    fn scanner_scan_service_all_toggles_off_does_not_touch_filesystem() {
        let fs = FakeFilesystem::default().with_file("Game/Data/Textures/missing.png", b"png");
        let settings = settings_with_all_data_rules_off();
        let request = ScannerScanRequest::new(15, &settings).with_data_path(data_path());
        let service = ScannerScanService::new(&fs);

        let output = service.scan(request);

        assert_eq!(
            output.status.kind,
            ScannerScanStatusKind::NoEnabledCategories
        );
        assert!(output.results.is_empty());
        assert!(fs.read_dirs.borrow().is_empty());
        assert!(fs.read_files.borrow().is_empty());
    }

    #[test]
    fn scanner_scan_service_unexpected_format_replacement_absent_and_archive_enabled_cases() {
        let fs = FakeFilesystem::default()
            .with_file("Game/Data/Textures/logo.tga", b"tga")
            .with_file("Game/Data/LooseArchive.ba2", b"archive")
            .with_file("Game/Data/LoadedArchive.ba2", b"archive");
        let enabled_archives = vec![ArchiveRecord::new(
            "Game/Data/LoadedArchive.ba2",
            ArchiveFormat::General,
            ArchiveVersion::OldGen,
            true,
        )];
        let settings = ScannerSettings {
            overview_issues: false,
            errors: true,
            wrong_format: true,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let request =
            ScannerScanRequest::new(16, &settings).with_enabled_archives(&enabled_archives);

        let output = service_scan(&fs, &settings, request);

        assert!(output.results.iter().any(|result| {
            result.problem_type == ScannerProblemType::UnexpectedFormat
                && result
                    .summary
                    .contains("expected format was NOT found (dds)")
        }));
        assert!(output.results.iter().any(|result| {
            result.problem_type == ScannerProblemType::InvalidArchiveName
                && result.detail_path == "LooseArchive.ba2"
        }));
        assert!(!output.results.iter().any(|result| {
            result.problem_type == ScannerProblemType::InvalidArchiveName
                && result.detail_path == "LoadedArchive.ba2"
        }));
    }

    #[test]
    fn scanner_scan_service_malformed_modlist_surfaces_safe_error_without_panic() {
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_file("MO2/profiles/Default/modlist.txt", vec![0xff, 0xfe, 0xfd]);
        let manager = mo2_context();
        let settings = ScannerSettings {
            overview_issues: false,
            race_subgraphs: false,
            ..ScannerSettings::default()
        };
        let request = ScannerScanRequest::new(17, &settings).with_mod_manager(&manager);

        let output = service_scan(&fs, &settings, request);

        assert_eq!(
            output.status.kind,
            ScannerScanStatusKind::CompletedWithRecoverableIssues
        );
        assert!(output.results.iter().any(|result| {
            result.problem_type == ScannerProblemType::FileNotFound
                && result.summary == "File read returned data that could not be understood."
        }));
        assert!(
            output
                .diagnostics
                .errors
                .iter()
                .any(|diagnostic| { diagnostic.kind == ScannerScanDiagnosticKind::InvalidInput })
        );
    }

    #[test]
    fn scanner_scan_service_overview_handoff_can_use_mo2_file_index_for_overview_placeholder_mod() {
        let fs = FakeFilesystem::default()
            .with_file("Game/Data/Sound/problem.mp3", b"mp3")
            .with_text("MO2/profiles/Default/modlist.txt", "+AudioFix\n")
            .with_file("MO2/mods/AudioFix/Sound/problem.mp3", b"mp3")
            .with_dir("MO2/overwrite");
        let overview = vec![
            OverviewProblem::with_path(
                OverviewProblemSource::Scanner,
                "Game/Data/Sound/problem.mp3",
                Some(PathBuf::from("Sound/problem.mp3")),
                OverviewProblemType::Custom("Unexpected Format".to_owned()),
                "Overview supplied scanner problem.",
                Some("Overview solution.".to_owned()),
            )
            .with_mod_name(Some("OVERVIEW".to_owned())),
        ];
        let manager = mo2_context();
        let settings = ScannerSettings {
            errors: true,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
            overview_issues: true,
        };
        let request = ScannerScanRequest::new(18, &settings)
            .with_mod_manager(&manager)
            .with_overview_problems(&overview);

        let output = service_scan(&fs, &settings, request);

        let result = output
            .results
            .iter()
            .find(|result| result.summary == "Overview supplied scanner problem.")
            .expect("overview result should be present");
        assert_eq!(result.mod_display_name(), "AudioFix");
        assert_eq!(
            result.absolute_path.as_deref(),
            Some(Path::new("MO2/mods/AudioFix/Sound/problem.mp3"))
        );
    }

    #[test]
    fn scanner_scan_service_non_english_ini_suffix_allows_language_voice_archive() {
        let fs = FakeFilesystem::default().with_file("Game/Data/MyMod - Voices_fr.ba2", b"archive");
        let mut installation =
            Fallout4Installation::with_optional_paths("Game", Some("Game/Data"), None::<PathBuf>);
        installation
            .ini_files
            .fallout4
            .sections
            .entry("general".to_owned())
            .or_default()
            .insert("slanguage".to_owned(), "fr".to_owned());
        let settings = ScannerSettings {
            overview_issues: false,
            errors: true,
            wrong_format: true,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let request = ScannerScanRequest::new(19, &settings).with_installation(&installation);
        let service = ScannerScanService::new(&fs);

        let output = service.scan(request);

        assert!(
            !output
                .results
                .iter()
                .any(|result| { result.problem_type == ScannerProblemType::InvalidArchiveName })
        );
    }

    #[test]
    fn scanner_scan_service_errors_toggle_gates_scanner_generated_error_rows() {
        let fs = FakeFilesystem::default().with_file("Game/Data/Textures/logo.tga", b"tga");
        let manager = mo2_context();
        let settings = ScannerSettings {
            overview_issues: false,
            errors: false,
            wrong_format: true,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let request = ScannerScanRequest::new(20, &settings).with_mod_manager(&manager);

        let output = service_scan(&fs, &settings, request);

        assert!(
            output
                .results
                .iter()
                .any(|result| { result.problem_type == ScannerProblemType::UnexpectedFormat })
        );
        assert!(
            !output
                .results
                .iter()
                .any(|result| { result.problem_type == ScannerProblemType::FileNotFound })
        );
        assert!(
            output.diagnostics.errors.iter().any(|diagnostic| {
                diagnostic.kind == ScannerScanDiagnosticKind::MissingMo2Modlist
            })
        );
    }

    #[test]
    fn scanner_scan_service_public_context_accepts_mod_manager_kind_imports() {
        assert_eq!(ModManagerKind::ModOrganizer.display_name(), "Mod Organizer");
    }
}
