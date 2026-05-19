//! Adapter-backed F4SE DLL scan service.
//!
//! The reference F4SE tab scans only direct children of `Data/F4SE/Plugins`,
//! skips `msdia*` DLLs, and determines compatibility from exported F4SE symbols.
//! This module keeps that behavior off the UI thread by exposing a synchronous,
//! fakeable service that callers can run inside a worker. The production
//! inspector parses PE bytes locally with Pelite and never loads untrusted DLLs.

use std::path::{Path, PathBuf};

use pelite::{Error as PeError, PeFile};
use thiserror::Error;
use tracing::{debug, info, info_span, warn};

use crate::{
    domain::{
        discovery::Fallout4Installation,
        f4se::{F4seDllFacts, F4seGameTarget, F4seScanSnapshot, render_f4se_dll_rows},
    },
    platform::{
        PlatformError,
        filesystem::{DirectoryEntry, FileType, Filesystem},
    },
};

const F4SE_PLUGINS_READ_ERROR_MESSAGE: &str = "Data/F4SE/Plugins folder could not be read.";
const F4SE_DATA_READ_ERROR_MESSAGE: &str = "Data folder could not be read.";
const DLL_READ_SAFE_MESSAGE: &str =
    "Could not read DLL. Check file permissions or whether the file is still available.";
const DLL_INSPECTION_SAFE_MESSAGE: &str =
    "Could not inspect DLL. The file is not a valid PE DLL or its export table is malformed.";
const VERSION_DATA_WARNING_SAFE_MESSAGE: &str =
    "F4SEPlugin_Version compatibleVersions could not be read; NG/AE support is unknown.";

const EXPORT_LOAD: &str = "F4SEPlugin_Load";
const EXPORT_PRELOAD: &str = "F4SEPlugin_Preload";
const EXPORT_QUERY: &str = "F4SEPlugin_Query";
const EXPORT_VERSION: &str = "F4SEPlugin_Version";
const COMPATIBLE_VERSIONS_OFFSET: u32 = 528;
const COMPATIBLE_VERSION_COUNT: usize = 16;
const NEXT_GEN_COMPATIBLE_VERSIONS: [u32; 2] = [0x010A3D40, 0x010A3D80];
const ANNIVERSARY_MIN_EXCLUSIVE: u32 = 0x010B0890;

/// Request input for a single F4SE plugin scan.
#[derive(Debug, Clone, Copy)]
pub struct F4seScanRequest<'a> {
    /// Optional discovered Fallout 4 installation.
    pub installation: Option<&'a Fallout4Installation>,
    /// Current game target used for the rendered `Your Game` column.
    pub current_game: F4seGameTarget,
    /// Whether a mod manager was already detected by discovery.
    pub mod_manager_detected: bool,
}

impl<'a> F4seScanRequest<'a> {
    /// Creates a scan request from already-discovered installation facts.
    pub const fn new(
        installation: Option<&'a Fallout4Installation>,
        current_game: F4seGameTarget,
        mod_manager_detected: bool,
    ) -> Self {
        Self {
            installation,
            current_game,
            mod_manager_detected,
        }
    }
}

/// Complete scan result returned by [`F4seScanService`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F4seScanReport {
    /// UI-ready snapshot with safe messages and rendered rows.
    pub snapshot: F4seScanSnapshot,
    /// Structured scan counts and safe diagnostics for logs/tests/controllers.
    pub diagnostics: F4seScanDiagnostics,
}

impl F4seScanReport {
    fn new(snapshot: F4seScanSnapshot, diagnostics: F4seScanDiagnostics) -> Self {
        Self {
            snapshot,
            diagnostics,
        }
    }
}

/// Safe diagnostic category emitted during F4SE scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum F4seScanDiagnosticKind {
    /// No usable Fallout 4 `Data` folder was supplied or found.
    MissingDataFolder,
    /// No usable `Data/F4SE/Plugins` folder was supplied or found.
    MissingPluginsFolder,
    /// A required directory existed but could not be read.
    DirectoryReadFailed,
    /// A candidate DLL could not be read.
    FileReadFailed,
    /// A candidate DLL's PE/export data could not be parsed safely.
    DllInspectionFailed,
    /// `F4SEPlugin_Version` existed but compatible version data was unreadable.
    VersionDataUnreadable,
    /// A direct child was ignored because it is outside the F4SE DLL scan scope.
    SkippedEntry,
}

/// Safe diagnostic detail for a path or scan prerequisite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F4seScanDiagnostic {
    /// Category of the diagnostic.
    pub kind: F4seScanDiagnosticKind,
    /// Path involved in the diagnostic, if any.
    pub path: Option<PathBuf>,
    /// User-safe diagnostic summary.
    pub safe_message: String,
}

/// Aggregate counts and safe diagnostics from a single F4SE scan.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct F4seScanDiagnostics {
    /// Number of direct entries returned by `Data/F4SE/Plugins` enumeration.
    pub enumerated_entry_count: usize,
    /// Number of direct child DLLs considered after extension and `msdia` filters.
    pub dll_candidate_count: usize,
    /// Number of DLLs that reached the inspector.
    pub inspected_dll_count: usize,
    /// Number of inspected DLLs classified as F4SE DLLs.
    pub f4se_dll_count: usize,
    /// Number of inspected DLLs classified as non-F4SE DLLs.
    pub non_f4se_dll_count: usize,
    /// Number of direct entries skipped by scope filters.
    pub skipped_entry_count: usize,
    /// Number of candidate DLLs that could not be read.
    pub unreadable_dll_count: usize,
    /// Number of candidate DLLs whose PE/export data failed closed.
    pub malformed_dll_count: usize,
    /// Number of F4SE version exports whose compatible version array was unreadable.
    pub version_data_warning_count: usize,
    /// Safe per-path details for missing folders, skipped entries, read failures, and parse failures.
    pub details: Vec<F4seScanDiagnostic>,
}

impl F4seScanDiagnostics {
    fn record(
        &mut self,
        kind: F4seScanDiagnosticKind,
        path: Option<PathBuf>,
        safe_message: impl Into<String>,
    ) {
        self.details.push(F4seScanDiagnostic {
            kind,
            path,
            safe_message: safe_message.into(),
        });
    }

    fn record_platform_error(
        &mut self,
        kind: F4seScanDiagnosticKind,
        path: Option<PathBuf>,
        error: &PlatformError,
    ) {
        self.record(kind, path, error.user_message().to_owned());
    }
}

/// Raw PE/export inspection facts before DLL names and UI rows are attached.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct F4seDllInspection {
    /// Whether `F4SEPlugin_Load` was found.
    pub exports_load: bool,
    /// Whether `F4SEPlugin_Preload` was found.
    pub exports_preload: bool,
    /// Whether `F4SEPlugin_Query` was found.
    pub exports_query: bool,
    /// Whether `F4SEPlugin_Version` was found.
    pub exports_version: bool,
    /// NG support proven from `compatibleVersions`, or unknown if not proven/readable.
    pub supports_ng: Option<bool>,
    /// AE support proven from `compatibleVersions`, or unknown if not proven/readable.
    pub supports_ae: Option<bool>,
    /// Safe non-fatal warning when version data exists but cannot be read.
    pub version_data_warning: Option<String>,
}

impl F4seDllInspection {
    /// Creates an inspection result for a DLL with no F4SE exports.
    pub const fn non_f4se() -> Self {
        Self {
            exports_load: false,
            exports_preload: false,
            exports_query: false,
            exports_version: false,
            supports_ng: None,
            supports_ae: None,
            version_data_warning: None,
        }
    }

    /// Creates an inspection result for a DLL with F4SE-related exports.
    pub const fn f4se(
        exports_load: bool,
        exports_preload: bool,
        exports_query: bool,
        exports_version: bool,
        supports_ng: Option<bool>,
        supports_ae: Option<bool>,
    ) -> Self {
        Self {
            exports_load,
            exports_preload,
            exports_query,
            exports_version,
            supports_ng,
            supports_ae,
            version_data_warning: None,
        }
    }

    /// Returns whether the reference scanner would classify this as an F4SE DLL.
    pub const fn is_f4se(&self) -> bool {
        self.exports_load || self.exports_preload
    }

    fn with_version_data_warning(mut self, diagnostic: impl Into<String>) -> Self {
        self.version_data_warning = Some(diagnostic.into());
        self
    }
}

/// Typed inspection failure returned by [`F4seDllInspector`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum F4seDllInspectionError {
    /// The bytes could not be parsed as a PE file.
    #[error("DLL bytes could not be parsed as a PE file: {diagnostic}")]
    ParseFailure {
        /// Non-user-facing parser detail.
        diagnostic: String,
    },
    /// The PE export table was present but malformed.
    #[error("DLL export table could not be inspected: {diagnostic}")]
    ExportTableFailure {
        /// Non-user-facing parser detail.
        diagnostic: String,
    },
    /// `F4SEPlugin_Version` data could not be read at the expected struct offsets.
    #[error("F4SEPlugin_Version data could not be inspected: {diagnostic}")]
    VersionDataFailure {
        /// Non-user-facing parser detail.
        diagnostic: String,
    },
}

impl F4seDllInspectionError {
    fn parse_failure(error: PeError) -> Self {
        Self::ParseFailure {
            diagnostic: error.to_string(),
        }
    }

    fn export_table_failure(error: PeError) -> Self {
        Self::ExportTableFailure {
            diagnostic: error.to_string(),
        }
    }

    fn version_data_failure(diagnostic: impl Into<String>) -> Self {
        Self::VersionDataFailure {
            diagnostic: diagnostic.into(),
        }
    }

    fn safe_message(&self) -> &'static str {
        match self {
            Self::ParseFailure { .. }
            | Self::ExportTableFailure { .. }
            | Self::VersionDataFailure { .. } => DLL_INSPECTION_SAFE_MESSAGE,
        }
    }
}

/// Fakeable DLL inspector boundary.
pub trait F4seDllInspector {
    /// Inspects PE bytes without loading or executing the DLL.
    fn inspect(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<F4seDllInspection, F4seDllInspectionError>;
}

/// Production DLL inspector backed by Pelite PE parsing.
#[derive(Debug, Default, Clone, Copy)]
pub struct PeliteF4seDllInspector;

impl PeliteF4seDllInspector {
    /// Creates a production inspector that parses PE bytes locally.
    pub const fn new() -> Self {
        Self
    }
}

impl F4seDllInspector for PeliteF4seDllInspector {
    fn inspect(
        &self,
        _path: &Path,
        bytes: &[u8],
    ) -> Result<F4seDllInspection, F4seDllInspectionError> {
        let pe = PeFile::from_bytes(bytes).map_err(F4seDllInspectionError::parse_failure)?;
        let exports = match pe.exports() {
            Ok(exports) => exports,
            Err(PeError::Null) => return Ok(F4seDllInspection::non_f4se()),
            Err(error) => return Err(F4seDllInspectionError::export_table_failure(error)),
        };
        let by = exports
            .by()
            .map_err(F4seDllInspectionError::export_table_failure)?;

        let find_export = |name: &str| match by.name_linear(name) {
            Ok(export) => Ok(Some(export)),
            Err(PeError::Null) => Ok(None),
            Err(error) => Err(F4seDllInspectionError::export_table_failure(error)),
        };

        let exports_load = find_export(EXPORT_LOAD)?.is_some();
        let exports_preload = find_export(EXPORT_PRELOAD)?.is_some();
        let exports_query = find_export(EXPORT_QUERY)?.is_some();
        let version_export = find_export(EXPORT_VERSION)?;
        let exports_version = version_export.is_some();

        let mut inspection = F4seDllInspection::f4se(
            exports_load,
            exports_preload,
            exports_query,
            exports_version,
            None,
            None,
        );

        if let Some(version_export) = version_export {
            if let Some(version_rva) = version_export.symbol() {
                match read_compatible_versions(&pe, version_rva) {
                    Ok(versions) => {
                        let (supports_ng, supports_ae) = compatible_version_support(versions);
                        inspection.supports_ng = supports_ng;
                        inspection.supports_ae = supports_ae;
                    }
                    Err(error) => {
                        inspection = inspection.with_version_data_warning(error.to_string());
                    }
                }
            } else {
                inspection = inspection.with_version_data_warning(
                    "F4SEPlugin_Version was forwarded instead of exported as data.",
                );
            }
        }

        Ok(inspection)
    }
}

/// F4SE scan service over injected filesystem and DLL inspector adapters.
#[derive(Debug)]
pub struct F4seScanService<'a, F: Filesystem + ?Sized, I: F4seDllInspector + ?Sized> {
    filesystem: &'a F,
    inspector: &'a I,
}

impl<'a, F: Filesystem + ?Sized, I: F4seDllInspector + ?Sized> F4seScanService<'a, F, I> {
    /// Creates an F4SE scan service without touching the filesystem.
    pub const fn new(filesystem: &'a F, inspector: &'a I) -> Self {
        Self {
            filesystem,
            inspector,
        }
    }

    /// Scans direct DLL children of `Data/F4SE/Plugins` and renders F4SE rows.
    ///
    /// This method is intentionally infallible: missing prerequisites become
    /// error snapshots, and individual unreadable/malformed DLLs become visible
    /// warning rows while remaining DLLs continue to scan.
    pub fn scan(&self, request: F4seScanRequest<'_>) -> F4seScanReport {
        let span = info_span!(
            "f4se_scan_service.scan",
            current_game = ?request.current_game,
            mod_manager_detected = request.mod_manager_detected
        );
        let _guard = span.enter();
        info!(event = "f4se-scan-started", "F4SE DLL scan started");

        let mut diagnostics = F4seScanDiagnostics::default();
        let Some(installation) = request.installation else {
            diagnostics.record(
                F4seScanDiagnosticKind::MissingDataFolder,
                None,
                crate::domain::f4se::F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE,
            );
            warn!(
                event = "f4se-scan-missing-data",
                reason = "no_installation",
                "F4SE scan cannot start because no installation was supplied"
            );
            return F4seScanReport::new(F4seScanSnapshot::missing_data_folder(), diagnostics);
        };

        let Some(data_path) = installation.data_path.as_deref() else {
            diagnostics.record(
                F4seScanDiagnosticKind::MissingDataFolder,
                None,
                crate::domain::f4se::F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE,
            );
            warn!(
                event = "f4se-scan-missing-data",
                reason = "no_data_path",
                game_path = %installation.game_path.display(),
                "F4SE scan cannot start because discovery did not supply Data"
            );
            return F4seScanReport::new(F4seScanSnapshot::missing_data_folder(), diagnostics);
        };

        if let Some(report) = self.ensure_data_directory(data_path, &mut diagnostics) {
            return report;
        }

        let Some(plugins_path) = installation.f4se_plugins_path.as_deref() else {
            diagnostics.record(
                F4seScanDiagnosticKind::MissingPluginsFolder,
                None,
                crate::domain::f4se::f4se_missing_plugins_message(request.mod_manager_detected),
            );
            warn!(
                event = "f4se-scan-missing-plugins",
                reason = "no_plugins_path",
                data_path = %data_path.display(),
                "F4SE scan cannot start because discovery did not supply Data/F4SE/Plugins"
            );
            return F4seScanReport::new(
                F4seScanSnapshot::missing_plugins_folder(request.mod_manager_detected),
                diagnostics,
            );
        };

        if let Some(report) = self.ensure_plugins_directory(
            plugins_path,
            request.mod_manager_detected,
            &mut diagnostics,
        ) {
            return report;
        }

        let entries = match self.filesystem.read_dir(plugins_path) {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.record_platform_error(
                    F4seScanDiagnosticKind::DirectoryReadFailed,
                    Some(plugins_path.to_path_buf()),
                    &error,
                );
                warn!(
                    event = "f4se-scan-plugins-read-failed",
                    plugins_path = %plugins_path.display(),
                    safe_message = error.user_message(),
                    diagnostic = error.diagnostic().unwrap_or(""),
                    "F4SE plugin directory could not be enumerated"
                );
                return F4seScanReport::new(
                    F4seScanSnapshot::error(F4SE_PLUGINS_READ_ERROR_MESSAGE),
                    diagnostics,
                );
            }
        };

        diagnostics.enumerated_entry_count = entries.len();
        debug!(
            event = "f4se-scan-plugins-enumerated",
            plugins_path = %plugins_path.display(),
            entries = entries.len(),
            "F4SE plugin directory enumerated"
        );

        let mut dll_entries = self.dll_entries(entries, &mut diagnostics);
        diagnostics.dll_candidate_count = dll_entries.len();
        dll_entries.sort_by(compare_entry_names);

        let mut facts = Vec::with_capacity(dll_entries.len());
        for entry in dll_entries {
            let dll_name = display_file_name(&entry.path);
            debug!(
                event = "f4se-scan-dll-started",
                dll_name = dll_name.as_str(),
                path = %entry.path.display(),
                "Inspecting F4SE candidate DLL"
            );

            match self.inspect_entry(&entry.path, &dll_name, &mut diagnostics) {
                Some(fact) => facts.push(fact),
                None => facts.push(F4seDllFacts::inspection_failed(
                    dll_name,
                    DLL_READ_SAFE_MESSAGE,
                )),
            }
        }

        let rows = render_f4se_dll_rows(&facts, request.current_game);
        info!(
            event = "f4se-scan-completed",
            rows = rows.len(),
            candidates = diagnostics.dll_candidate_count,
            inspected = diagnostics.inspected_dll_count,
            f4se = diagnostics.f4se_dll_count,
            non_f4se = diagnostics.non_f4se_dll_count,
            unreadable = diagnostics.unreadable_dll_count,
            malformed = diagnostics.malformed_dll_count,
            skipped = diagnostics.skipped_entry_count,
            version_warnings = diagnostics.version_data_warning_count,
            "F4SE DLL scan completed"
        );

        F4seScanReport::new(F4seScanSnapshot::ready(rows), diagnostics)
    }

    fn ensure_data_directory(
        &self,
        data_path: &Path,
        diagnostics: &mut F4seScanDiagnostics,
    ) -> Option<F4seScanReport> {
        match self.filesystem.is_dir(data_path) {
            Ok(true) => None,
            Ok(false) => {
                diagnostics.record(
                    F4seScanDiagnosticKind::MissingDataFolder,
                    Some(data_path.to_path_buf()),
                    crate::domain::f4se::F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE,
                );
                warn!(
                    event = "f4se-scan-missing-data",
                    data_path = %data_path.display(),
                    "F4SE scan cannot start because Data is not a directory"
                );
                Some(F4seScanReport::new(
                    F4seScanSnapshot::missing_data_folder(),
                    diagnostics.clone(),
                ))
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    F4seScanDiagnosticKind::DirectoryReadFailed,
                    Some(data_path.to_path_buf()),
                    &error,
                );
                warn!(
                    event = "f4se-scan-data-read-failed",
                    data_path = %data_path.display(),
                    safe_message = error.user_message(),
                    diagnostic = error.diagnostic().unwrap_or(""),
                    "F4SE Data directory could not be checked"
                );
                Some(F4seScanReport::new(
                    F4seScanSnapshot::error(F4SE_DATA_READ_ERROR_MESSAGE),
                    diagnostics.clone(),
                ))
            }
        }
    }

    fn ensure_plugins_directory(
        &self,
        plugins_path: &Path,
        mod_manager_detected: bool,
        diagnostics: &mut F4seScanDiagnostics,
    ) -> Option<F4seScanReport> {
        match self.filesystem.is_dir(plugins_path) {
            Ok(true) => None,
            Ok(false) => {
                let safe_message =
                    crate::domain::f4se::f4se_missing_plugins_message(mod_manager_detected);
                diagnostics.record(
                    F4seScanDiagnosticKind::MissingPluginsFolder,
                    Some(plugins_path.to_path_buf()),
                    safe_message,
                );
                warn!(
                    event = "f4se-scan-missing-plugins",
                    plugins_path = %plugins_path.display(),
                    "F4SE scan cannot start because Data/F4SE/Plugins is not a directory"
                );
                Some(F4seScanReport::new(
                    F4seScanSnapshot::missing_plugins_folder(mod_manager_detected),
                    diagnostics.clone(),
                ))
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    F4seScanDiagnosticKind::DirectoryReadFailed,
                    Some(plugins_path.to_path_buf()),
                    &error,
                );
                warn!(
                    event = "f4se-scan-plugins-metadata-failed",
                    plugins_path = %plugins_path.display(),
                    safe_message = error.user_message(),
                    diagnostic = error.diagnostic().unwrap_or(""),
                    "F4SE plugin directory could not be checked"
                );
                Some(F4seScanReport::new(
                    F4seScanSnapshot::error(F4SE_PLUGINS_READ_ERROR_MESSAGE),
                    diagnostics.clone(),
                ))
            }
        }
    }

    fn dll_entries(
        &self,
        entries: Vec<DirectoryEntry>,
        diagnostics: &mut F4seScanDiagnostics,
    ) -> Vec<DirectoryEntry> {
        let mut dll_entries = Vec::new();
        for entry in entries {
            if entry.file_type != FileType::File || !is_dll_path(&entry.path) {
                diagnostics.skipped_entry_count += 1;
                diagnostics.record(
                    F4seScanDiagnosticKind::SkippedEntry,
                    Some(entry.path),
                    "Entry is not a direct DLL file and was skipped.",
                );
                continue;
            }

            let dll_name = display_file_name(&entry.path);
            if dll_name.starts_with("msdia") {
                diagnostics.skipped_entry_count += 1;
                diagnostics.record(
                    F4seScanDiagnosticKind::SkippedEntry,
                    Some(entry.path),
                    "msdia helper DLL was skipped to match the reference scanner.",
                );
                continue;
            }

            dll_entries.push(entry);
        }
        dll_entries
    }

    fn inspect_entry(
        &self,
        path: &Path,
        dll_name: &str,
        diagnostics: &mut F4seScanDiagnostics,
    ) -> Option<F4seDllFacts> {
        let bytes = match self.filesystem.read_bytes(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.unreadable_dll_count += 1;
                diagnostics.record_platform_error(
                    F4seScanDiagnosticKind::FileReadFailed,
                    Some(path.to_path_buf()),
                    &error,
                );
                warn!(
                    event = "f4se-scan-dll-read-failed",
                    dll_name,
                    path = %path.display(),
                    safe_message = error.user_message(),
                    diagnostic = error.diagnostic().unwrap_or(""),
                    "F4SE candidate DLL could not be read"
                );
                return None;
            }
        };

        match self.inspector.inspect(path, &bytes) {
            Ok(inspection) => {
                diagnostics.inspected_dll_count += 1;
                if inspection.is_f4se() {
                    diagnostics.f4se_dll_count += 1;
                } else {
                    diagnostics.non_f4se_dll_count += 1;
                }

                if let Some(diagnostic) = inspection.version_data_warning.as_deref() {
                    diagnostics.version_data_warning_count += 1;
                    diagnostics.record(
                        F4seScanDiagnosticKind::VersionDataUnreadable,
                        Some(path.to_path_buf()),
                        VERSION_DATA_WARNING_SAFE_MESSAGE,
                    );
                    warn!(
                        event = "f4se-scan-version-data-unreadable",
                        dll_name,
                        path = %path.display(),
                        diagnostic,
                        "F4SEPlugin_Version compatibleVersions could not be read"
                    );
                }

                Some(facts_from_inspection(dll_name, inspection))
            }
            Err(error) => {
                diagnostics.malformed_dll_count += 1;
                diagnostics.record(
                    F4seScanDiagnosticKind::DllInspectionFailed,
                    Some(path.to_path_buf()),
                    error.safe_message(),
                );
                warn!(
                    event = "f4se-scan-dll-inspection-failed",
                    dll_name,
                    path = %path.display(),
                    error = %error,
                    "F4SE candidate DLL failed PE/export inspection safely"
                );
                Some(F4seDllFacts::inspection_failed(
                    dll_name.to_owned(),
                    error.safe_message(),
                ))
            }
        }
    }
}

fn facts_from_inspection(dll_name: &str, inspection: F4seDllInspection) -> F4seDllFacts {
    if inspection.is_f4se() {
        F4seDllFacts::f4se(
            dll_name,
            inspection.exports_query,
            inspection.exports_version,
            inspection.supports_ng,
            inspection.supports_ae,
        )
    } else {
        F4seDllFacts::non_f4se(dll_name)
    }
}

fn read_compatible_versions<'a>(
    pe: &pelite::PeFile<'a>,
    version_rva: u32,
) -> Result<&'a [u32], F4seDllInspectionError> {
    let versions_rva = version_rva
        .checked_add(COMPATIBLE_VERSIONS_OFFSET)
        .ok_or_else(|| {
            F4seDllInspectionError::version_data_failure("compatibleVersions RVA overflowed")
        })?;
    pe.derva_slice::<u32>(versions_rva, COMPATIBLE_VERSION_COUNT)
        .map_err(|error| F4seDllInspectionError::version_data_failure(error.to_string()))
}

fn compatible_version_support(versions: &[u32]) -> (Option<bool>, Option<bool>) {
    let supports_ng = versions
        .iter()
        .any(|version| NEXT_GEN_COMPATIBLE_VERSIONS.contains(version))
        .then_some(true);
    let supports_ae = versions
        .iter()
        .any(|version| *version > ANNIVERSARY_MIN_EXCLUSIVE)
        .then_some(true);
    (supports_ng, supports_ae)
}

fn is_dll_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
}

fn compare_entry_names(left: &DirectoryEntry, right: &DirectoryEntry) -> std::cmp::Ordering {
    let left_name = display_file_name(&left.path);
    let right_name = display_file_name(&right.path);
    left_name
        .to_ascii_lowercase()
        .cmp(&right_name.to_ascii_lowercase())
        .then_with(|| left_name.cmp(&right_name))
        .then_with(|| left.path.cmp(&right.path))
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use crate::{
        domain::f4se::{
            F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE, F4SE_MOD_MANAGER_HINT,
            F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE, F4seCompatibilityIcon, F4seRowSeverity,
            F4seScanStatus,
        },
        platform::{
            PlatformErrorKind, PlatformOperation, PlatformResult, filesystem::FileMetadata,
        },
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
        full_reads: RefCell<Vec<PathBuf>>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::Directory);
            self
        }

        fn with_unreadable_dir(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::UnreadableDirectory);
            self
        }

        fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::File(bytes.into()));
            self
        }

        fn with_unreadable_file(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::UnreadableFile);
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
            match self.nodes.get(path) {
                Some(node) => Ok(node),
                None => Err(PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )),
            }
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

        fn full_reads(&self) -> Vec<PathBuf> {
            self.full_reads.borrow().clone()
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata::new(FileType::File, bytes.len() as u64)),
                FakeNode::Directory | FakeNode::UnreadableDirectory => {
                    Ok(FileMetadata::new(FileType::Directory, 0))
                }
                FakeNode::UnreadableFile => Ok(FileMetadata::new(FileType::File, 0)),
            }
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            self.full_reads.borrow_mut().push(path.to_path_buf());
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
            match self.node(path, PlatformOperation::ReadDirectory)? {
                FakeNode::Directory => Ok(self
                    .nodes
                    .iter()
                    .filter(|(candidate, _)| candidate.parent() == Some(path))
                    .map(|(candidate, node)| {
                        DirectoryEntry::new(candidate.clone(), node.file_type())
                    })
                    .collect()),
                FakeNode::UnreadableDirectory => Err(Self::permission_denied(
                    path,
                    PlatformOperation::ReadDirectory,
                )),
                FakeNode::File(_) | FakeNode::UnreadableFile => Err(PlatformError::new(
                    PlatformOperation::ReadDirectory,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "Directory read target is invalid.",
                )),
            }
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(self
                .nodes
                .iter()
                .filter(|(candidate, _)| candidate == &path || candidate.starts_with(path))
                .map(|(candidate, node)| DirectoryEntry::new(candidate.clone(), node.file_type()))
                .collect())
        }
    }

    impl FakeNode {
        fn file_type(&self) -> FileType {
            match self {
                Self::File(_) | Self::UnreadableFile => FileType::File,
                Self::Directory | Self::UnreadableDirectory => FileType::Directory,
            }
        }
    }

    #[derive(Debug, Default)]
    struct FakeInspector {
        results: BTreeMap<PathBuf, Result<F4seDllInspection, F4seDllInspectionError>>,
        calls: RefCell<Vec<PathBuf>>,
    }

    impl FakeInspector {
        fn with_result(
            mut self,
            path: impl Into<PathBuf>,
            result: Result<F4seDllInspection, F4seDllInspectionError>,
        ) -> Self {
            self.results.insert(path.into(), result);
            self
        }

        fn calls(&self) -> Vec<PathBuf> {
            self.calls.borrow().clone()
        }
    }

    impl F4seDllInspector for FakeInspector {
        fn inspect(
            &self,
            path: &Path,
            _bytes: &[u8],
        ) -> Result<F4seDllInspection, F4seDllInspectionError> {
            self.calls.borrow_mut().push(path.to_path_buf());
            self.results
                .get(path)
                .cloned()
                .unwrap_or_else(|| Ok(F4seDllInspection::non_f4se()))
        }
    }

    fn installation() -> Fallout4Installation {
        Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            Some("C:/Games/Fallout 4/Data/F4SE/Plugins"),
        )
    }

    fn installation_without_data() -> Fallout4Installation {
        Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            None::<PathBuf>,
            None::<PathBuf>,
        )
    }

    fn installation_without_plugins() -> Fallout4Installation {
        Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            None::<PathBuf>,
        )
    }

    fn base_fs() -> FakeFilesystem {
        FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4/Data")
            .with_dir("C:/Games/Fallout 4/Data/F4SE/Plugins")
    }

    fn scan(
        fs: &FakeFilesystem,
        inspector: &FakeInspector,
        installation: Option<&Fallout4Installation>,
        current_game: F4seGameTarget,
        mod_manager_detected: bool,
    ) -> F4seScanReport {
        F4seScanService::new(fs, inspector).scan(F4seScanRequest::new(
            installation,
            current_game,
            mod_manager_detected,
        ))
    }

    fn path(name: &str) -> PathBuf {
        PathBuf::from("C:/Games/Fallout 4/Data/F4SE/Plugins").join(name)
    }

    fn f4se_modern() -> F4seDllInspection {
        F4seDllInspection::f4se(true, false, true, true, Some(true), None)
    }

    #[test]
    fn f4se_scan_service_missing_data_for_absent_installation_or_data_path() {
        let fs = FakeFilesystem::default();
        let inspector = FakeInspector::default();
        let missing_installation = scan(&fs, &inspector, None, F4seGameTarget::OldGen, false);
        assert_eq!(missing_installation.snapshot.status, F4seScanStatus::Error);
        assert_eq!(
            missing_installation.snapshot.status_message,
            F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE
        );
        assert_eq!(
            missing_installation.diagnostics.details[0].kind,
            F4seScanDiagnosticKind::MissingDataFolder
        );

        let installation = installation_without_data();
        let missing_data = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::OldGen,
            false,
        );
        assert_eq!(missing_data.snapshot.status, F4seScanStatus::Error);
        assert_eq!(
            missing_data.snapshot.status_message,
            F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE
        );
    }

    #[test]
    fn f4se_scan_service_missing_plugins_message_respects_manager_hint() {
        let fs = FakeFilesystem::default().with_dir("C:/Games/Fallout 4/Data");
        let inspector = FakeInspector::default();
        let installation = installation_without_plugins();

        let unmanaged = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::OldGen,
            false,
        );
        assert_eq!(unmanaged.snapshot.status, F4seScanStatus::Error);
        assert_eq!(
            unmanaged.snapshot.status_message,
            format!("{F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE}\n{F4SE_MOD_MANAGER_HINT}")
        );

        let managed = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::OldGen,
            true,
        );
        assert_eq!(managed.snapshot.status, F4seScanStatus::Error);
        assert_eq!(
            managed.snapshot.status_message,
            F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE
        );
    }

    #[test]
    fn f4se_scan_service_empty_plugin_folder_is_ready_with_zero_rows() {
        let fs = base_fs();
        let inspector = FakeInspector::default();
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::OldGen,
            true,
        );

        assert_eq!(report.snapshot.status, F4seScanStatus::Ready);
        assert!(report.snapshot.rows.is_empty());
        assert_eq!(report.diagnostics.enumerated_entry_count, 0);
        assert_eq!(report.diagnostics.dll_candidate_count, 0);
    }

    #[test]
    fn f4se_scan_service_direct_child_only_enumeration_and_deterministic_order() {
        let alpha = path("alpha.dll");
        let bravo = path("Bravo.DLL");
        let nested = path("Nested/beta.dll");
        let fs = base_fs()
            .with_file(&bravo, b"bravo".to_vec())
            .with_file(&alpha, b"alpha".to_vec())
            .with_file(&nested, b"nested".to_vec())
            .with_file(path("notes.txt"), b"notes".to_vec());
        let inspector = FakeInspector::default()
            .with_result(&alpha, Ok(f4se_modern()))
            .with_result(&bravo, Ok(F4seDllInspection::non_f4se()));
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::NextGen,
            true,
        );

        assert_eq!(
            report
                .snapshot
                .rows
                .iter()
                .map(|row| row.dll_name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.dll", "Bravo.DLL"]
        );
        assert_eq!(inspector.calls(), vec![alpha.clone(), bravo.clone()]);
        assert_eq!(fs.full_reads(), vec![alpha, bravo]);
        assert_eq!(report.diagnostics.dll_candidate_count, 2);
        assert_eq!(report.diagnostics.skipped_entry_count, 2);
    }

    #[test]
    fn f4se_scan_service_msdia_ignore_skips_reference_helper_dlls() {
        let keep = path("real.dll");
        let skip = path("msdia140.dll");
        let fs = base_fs()
            .with_file(&skip, b"msdia".to_vec())
            .with_file(&keep, b"keep".to_vec());
        let inspector = FakeInspector::default().with_result(&keep, Ok(f4se_modern()));
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::NextGen,
            true,
        );

        assert_eq!(report.snapshot.rows.len(), 1);
        assert_eq!(report.snapshot.rows[0].dll_name, "real.dll");
        assert_eq!(inspector.calls(), vec![keep]);
        assert_eq!(report.diagnostics.skipped_entry_count, 1);
    }

    #[test]
    fn f4se_scan_service_unreadable_file_stays_visible_as_warning_row() {
        let unreadable = path("locked.dll");
        let fs = base_fs().with_unreadable_file(&unreadable);
        let inspector = FakeInspector::default();
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::Anniversary,
            true,
        );

        assert_eq!(report.snapshot.status, F4seScanStatus::Ready);
        assert_eq!(report.snapshot.rows.len(), 1);
        let row = &report.snapshot.rows[0];
        assert_eq!(row.dll_name, "locked.dll");
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Warning);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("Could not read DLL"))
        );
        assert_eq!(report.diagnostics.unreadable_dll_count, 1);
        assert!(inspector.calls().is_empty());
    }

    #[test]
    fn f4se_scan_service_malformed_parser_failure_stays_visible_and_continues() {
        let broken = path("broken.dll");
        let good = path("good.dll");
        let fs = base_fs()
            .with_file(&broken, b"bad".to_vec())
            .with_file(&good, b"good".to_vec());
        let inspector = FakeInspector::default()
            .with_result(
                &broken,
                Err(F4seDllInspectionError::ParseFailure {
                    diagnostic: "unknown magic number".to_owned(),
                }),
            )
            .with_result(&good, Ok(f4se_modern()));
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::NextGen,
            true,
        );

        assert_eq!(report.snapshot.status, F4seScanStatus::Ready);
        assert_eq!(report.snapshot.rows.len(), 2);
        assert_eq!(report.snapshot.rows[0].dll_name, "broken.dll");
        assert_eq!(
            report.snapshot.rows[0].your_game.icon,
            F4seCompatibilityIcon::Warning
        );
        assert_eq!(report.snapshot.rows[1].dll_name, "good.dll");
        assert_eq!(
            report.snapshot.rows[1].your_game.icon,
            F4seCompatibilityIcon::Supported
        );
        assert_eq!(report.diagnostics.malformed_dll_count, 1);
        assert_eq!(report.diagnostics.inspected_dll_count, 1);
    }

    #[test]
    fn f4se_scan_service_classification_rows_use_export_facts() {
        let legacy = path("legacy.dll");
        let modern = path("modern.dll");
        let helper = path("helper.dll");
        let fs = base_fs()
            .with_file(&legacy, b"legacy".to_vec())
            .with_file(&modern, b"modern".to_vec())
            .with_file(&helper, b"helper".to_vec());
        let inspector = FakeInspector::default()
            .with_result(
                &legacy,
                Ok(F4seDllInspection::f4se(
                    true, false, true, false, None, None,
                )),
            )
            .with_result(&modern, Ok(f4se_modern()))
            .with_result(&helper, Ok(F4seDllInspection::non_f4se()));
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::NextGen,
            true,
        );

        let helper_row = report
            .snapshot
            .rows
            .iter()
            .find(|row| row.dll_name == "helper.dll")
            .expect("helper row should be present");
        assert_eq!(helper_row.og.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(helper_row.severity, F4seRowSeverity::Neutral);

        let legacy_row = report
            .snapshot
            .rows
            .iter()
            .find(|row| row.dll_name == "legacy.dll")
            .expect("legacy row should be present");
        assert_eq!(legacy_row.og.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(
            legacy_row.ng.icon,
            F4seCompatibilityIcon::UnsupportedReferenceColumn
        );
        assert_eq!(
            legacy_row.your_game.icon,
            F4seCompatibilityIcon::UnsupportedCurrentGame
        );
        assert_eq!(legacy_row.severity, F4seRowSeverity::Incompatible);

        let modern_row = report
            .snapshot
            .rows
            .iter()
            .find(|row| row.dll_name == "modern.dll")
            .expect("modern row should be present");
        assert_eq!(modern_row.ng.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(modern_row.ae.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(modern_row.your_game.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(modern_row.severity, F4seRowSeverity::Compatible);
        assert_eq!(report.diagnostics.f4se_dll_count, 2);
        assert_eq!(report.diagnostics.non_f4se_dll_count, 1);
    }

    #[test]
    fn f4se_scan_service_unknown_game_target_keeps_your_game_warning() {
        let dll = path("known.dll");
        let fs = base_fs().with_file(&dll, b"known".to_vec());
        let inspector = FakeInspector::default().with_result(
            &dll,
            Ok(F4seDllInspection::f4se(
                true,
                false,
                true,
                true,
                Some(true),
                Some(true),
            )),
        );
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::Unknown,
            true,
        );

        let row = &report.snapshot.rows[0];
        assert_eq!(row.ng.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(row.ae.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.severity, F4seRowSeverity::Warning);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("could not be classified"))
        );
    }

    #[test]
    fn f4se_scan_service_plugins_read_error_returns_safe_scan_error() {
        let fs = FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4/Data")
            .with_unreadable_dir("C:/Games/Fallout 4/Data/F4SE/Plugins");
        let inspector = FakeInspector::default();
        let installation = installation();

        let report = scan(
            &fs,
            &inspector,
            Some(&installation),
            F4seGameTarget::OldGen,
            true,
        );

        assert_eq!(report.snapshot.status, F4seScanStatus::Error);
        assert_eq!(
            report.snapshot.status_message,
            F4SE_PLUGINS_READ_ERROR_MESSAGE
        );
        assert_eq!(
            report.diagnostics.details[0].kind,
            F4seScanDiagnosticKind::DirectoryReadFailed
        );
        assert!(inspector.calls().is_empty());
    }

    #[test]
    fn f4se_dll_inspector_malformed_bytes_return_typed_parse_failure() {
        let inspector = PeliteF4seDllInspector::new();
        let error = inspector
            .inspect(Path::new("broken.dll"), b"not a portable executable")
            .expect_err("malformed bytes should fail closed");

        assert!(matches!(error, F4seDllInspectionError::ParseFailure { .. }));
    }

    #[test]
    fn f4se_dll_inspector_compatible_version_mapping_only_proves_known_runtime_support() {
        assert_eq!(compatible_version_support(&[]), (None, None));
        assert_eq!(
            compatible_version_support(&[0x010A3D40]),
            (Some(true), None)
        );
        assert_eq!(
            compatible_version_support(&[0x010A3D80]),
            (Some(true), None)
        );
        assert_eq!(compatible_version_support(&[0x010B0890]), (None, None));
        assert_eq!(
            compatible_version_support(&[0x010B0891]),
            (None, Some(true))
        );
        assert_eq!(
            compatible_version_support(&[0x010A3D40, 0x010B0891]),
            (Some(true), Some(true))
        );
    }
}
