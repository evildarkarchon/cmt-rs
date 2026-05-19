//! Slint-free Archive Patcher workflow contract.
//!
//! The reference Tkinter modal lives in `CMT/src/patcher/_archives.py` and
//! `CMT/src/patcher/_base.py`, with shared strings in `CMT/src/globals.py`.
//! This module freezes the user-visible labels, target semantics, log rows,
//! preview rows, progress state, summary counts, and JSON manifest payloads as
//! pure Rust data. It deliberately performs no filesystem, Slint, or OS work.

use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::discovery::{ArchiveFormat, ArchiveVersion};

/// Reference modal title passed to `PatcherBase`.
pub const ARCHIVE_PATCHER_MODAL_TITLE: &str = "Archive Patcher";
/// Reference modal width in logical pixels.
pub const ARCHIVE_PATCHER_MODAL_WIDTH: i32 = 700;
/// Reference modal height in logical pixels.
pub const ARCHIVE_PATCHER_MODAL_HEIGHT: i32 = 600;

/// Reference labelframe text for the desired-version radios.
pub const DESIRED_VERSION_GROUP_LABEL: &str = "Desired Version";
/// Reference desired-version radio label for patching to old-gen BA2 headers.
pub const TARGET_OLD_GEN_LABEL: &str = "v1 (OG)";
/// Reference desired-version radio label for patching to next-gen BA2 headers.
pub const TARGET_NEXT_GEN_LABEL: &str = "v8 (NG)";
/// Reference patch button label from the shared patcher base modal.
pub const PATCH_ALL_BUTTON_LABEL: &str = "Patch All";
/// Reference about button label from the shared patcher base modal.
pub const ABOUT_BUTTON_LABEL: &str = "About";
/// Reference name-filter label, preserving the trailing colon.
pub const NAME_FILTER_LABEL: &str = "Name Filter:";

/// Reference filter explainer shown when the desired target is `v8 (NG)`.
pub const PATCHER_FILTER_OLD_GEN: &str = "Showing all v1\n(Includes Base Game/DLC/CC)";
/// Reference filter explainer shown when the desired target is `v1 (OG)`.
pub const PATCHER_FILTER_NEXT_GEN: &str = "Showing all v7 & v8\n(Includes Base Game/DLC/CC)";

/// Reference title for the Archive Patcher About dialog.
pub const ABOUT_ARCHIVES_TITLE: &str = "Bethesda Archive (BA2) Formats & Versions";
/// Reference body copy for the Archive Patcher About dialog.
pub const ABOUT_ARCHIVES_BODY: &str = "There are 2 formats and 3 versions for Fallout 4 BA2 files:\n• General (GNRL)\n• Textures (DX10)\n\n• v1: Required by FO4 v1.10.163 and earlier. Works in all versions.\n• v7/8: Only supported in FO4 v1.10.980 and later.\n\nIt's suspected there are format differences between versions for XBox/PS, but for PC the only difference is the number itself.\nv7/8 are identical so this tool only patches to v1 & v8.\n\nWhy Patch Versions?\n\nPatching is only needed if you use tools that require it.\nMost tools check the version to ensure compatibility but v7/8 didn't exist when these tools were made, so they assume it's a different format and show errors.\nBecause they're actually identical, you can just patch the version number in the file header so the tools will allow reading them.";

/// BA2 header prefix length needed to validate magic, version, and format.
pub const BA2_HEADER_PREFIX_LEN: usize = 12;
/// Reference BA2 container magic.
pub const BA2_MAGIC: &[u8; 4] = b"BTDX";
/// Reference General archive format marker.
pub const BA2_FORMAT_GENERAL: &[u8; 4] = b"GNRL";
/// Reference DirectX texture archive format marker.
pub const BA2_FORMAT_DIRECTX10: &[u8; 4] = b"DX10";

/// Safe message used when a candidate path cannot be proven to live below Data.
pub const ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE: &str =
    "Archive path could not be validated under the Data folder.";
/// Safe message used when the Data root needed for containment checks is absent.
pub const DATA_ROOT_MISSING_FAILURE_MESSAGE: &str =
    "Data folder is unavailable; archive path containment could not be verified.";

/// Default Archive Patcher target, matching `IntVar(value=ArchiveVersion.OG)`.
pub const DEFAULT_ARCHIVE_PATCHER_TARGET: ArchivePatcherTarget = ArchivePatcherTarget::OldGen;
/// JSON manifest schema version for latest Archive Patcher restore metadata.
pub const ARCHIVE_PATCHER_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Desired BA2 version selected by the Archive Patcher radio buttons.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivePatcherTarget {
    /// Patch eligible next-gen BA2 headers to v1 for old-gen tooling.
    #[default]
    OldGen,
    /// Patch eligible old-gen BA2 headers to v8 for next-gen tooling.
    NextGen,
}

impl ArchivePatcherTarget {
    /// Returns the exact reference radio label for this target.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::OldGen => TARGET_OLD_GEN_LABEL,
            Self::NextGen => TARGET_NEXT_GEN_LABEL,
        }
    }

    /// Returns the reference filter explainer for this target.
    pub const fn filter_text(self) -> &'static str {
        match self {
            Self::OldGen => PATCHER_FILTER_NEXT_GEN,
            Self::NextGen => PATCHER_FILTER_OLD_GEN,
        }
    }

    /// Returns the BA2 header version that would be written after confirmation.
    pub const fn target_header_value(self) -> u32 {
        match self {
            Self::OldGen => 1,
            Self::NextGen => 8,
        }
    }

    /// Returns the typed ArchiveVersion corresponding to the target header byte.
    pub const fn target_archive_version(self) -> ArchiveVersion {
        match self {
            Self::OldGen => ArchiveVersion::OldGen,
            Self::NextGen => ArchiveVersion::NextGen8,
        }
    }

    /// Returns whether an Overview record with this version is a candidate source.
    pub const fn selects_overview_version(self, version: ArchiveVersion) -> bool {
        match self {
            Self::OldGen => matches!(version, ArchiveVersion::NextGen7 | ArchiveVersion::NextGen8),
            Self::NextGen => matches!(version, ArchiveVersion::OldGen),
        }
    }

    /// Returns whether a freshly read BA2 header version can transition to this target.
    pub const fn accepts_header_transition(self, current_version: u32) -> bool {
        match self {
            Self::OldGen => matches!(current_version, 7 | 8),
            Self::NextGen => current_version == 1,
        }
    }

    /// Returns true when the freshly read BA2 header is already at this target.
    pub const fn is_target_header_version(self, current_version: u32) -> bool {
        current_version == self.target_header_value()
    }
}

impl fmt::Display for ArchivePatcherTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_reference_str())
    }
}

/// Known BA2 archive formats accepted by the Archive Patcher preview plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivePatcherArchiveFormat {
    /// General BA2 archive (`GNRL`).
    General,
    /// DirectX 10 texture BA2 archive (`DX10`).
    DirectX10,
}

impl ArchivePatcherArchiveFormat {
    /// Returns the exact four-character BA2 format marker.
    pub const fn as_reference_magic(self) -> &'static str {
        match self {
            Self::General => "GNRL",
            Self::DirectX10 => "DX10",
        }
    }

    /// Converts an Overview archive format into the patcher-specific known format.
    pub fn from_overview_format(format: &ArchiveFormat) -> Option<Self> {
        match format {
            ArchiveFormat::General => Some(Self::General),
            ArchiveFormat::DirectX10 => Some(Self::DirectX10),
            ArchiveFormat::Unknown(_) => None,
        }
    }
}

impl fmt::Display for ArchivePatcherArchiveFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_reference_magic())
    }
}

/// BA2 header facts parsed from the bounded prefix read during preview planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArchivePatcherHeader {
    /// Numeric BA2 header version read as little-endian u32.
    pub version: u32,
    /// Known BA2 format marker.
    pub format: ArchivePatcherArchiveFormat,
}

impl ArchivePatcherHeader {
    /// Creates parsed BA2 header facts.
    pub const fn new(version: u32, format: ArchivePatcherArchiveFormat) -> Self {
        Self { version, format }
    }

    /// Returns the display string used in diagnostics and digests.
    pub fn version_label(self) -> String {
        format!("v{}", self.version)
    }
}

/// Log row levels matching `CMT/src/enums.py::LogType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivePatcherLogLevel {
    /// Reference informational log row.
    Info,
    /// Reference successful patch row.
    Good,
    /// Reference failed patch row.
    Bad,
}

impl ArchivePatcherLogLevel {
    /// Returns the exact string value used by the reference `LogType` enum.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Good => "good",
            Self::Bad => "bad",
        }
    }
}

/// User-visible Archive Patcher log row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePatcherLogRow {
    /// Reference log level/color intent.
    pub level: ArchivePatcherLogLevel,
    /// User-visible message text.
    pub message: String,
    /// True for transient modal messages the Python reference does not forward to file logging.
    pub skip_file_logging: bool,
}

impl ArchivePatcherLogRow {
    /// Creates a log row that may also be forwarded to the application log.
    pub fn new(level: ArchivePatcherLogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            skip_file_logging: false,
        }
    }

    /// Creates a transient informational row matching `skip_logging=True` in the reference.
    pub fn transient_info(message: impl Into<String>) -> Self {
        Self {
            level: ArchivePatcherLogLevel::Info,
            message: message.into(),
            skip_file_logging: true,
        }
    }
}

/// Render-ready candidate row selected from Overview archive records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherCandidateRow {
    /// Archive path as carried by the Overview/discovery record.
    pub path: PathBuf,
    /// Basename displayed in the reference tree.
    pub display_name: String,
    /// Overview's parsed archive format.
    pub overview_format: ArchiveFormat,
    /// Overview's parsed archive version.
    pub overview_version: ArchiveVersion,
    /// Desired target selected when this row was built.
    pub target: ArchivePatcherTarget,
}

impl ArchivePatcherCandidateRow {
    /// Creates a render-ready candidate row.
    pub fn new(
        path: impl Into<PathBuf>,
        display_name: impl Into<String>,
        overview_format: ArchiveFormat,
        overview_version: ArchiveVersion,
        target: ArchivePatcherTarget,
    ) -> Self {
        Self {
            path: path.into(),
            display_name: display_name.into(),
            overview_format,
            overview_version,
            target,
        }
    }
}

/// Candidate snapshot returned when the tree contents are refreshed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherCandidateSnapshot {
    /// Request id copied from the caller for stale-event rejection by later controllers.
    pub request_id: u64,
    /// Desired target used for this candidate set.
    pub target: ArchivePatcherTarget,
    /// Name filter exactly as normalized by the service, if any.
    pub name_filter: Option<String>,
    /// Candidate rows in deterministic path order.
    pub rows: Vec<ArchivePatcherCandidateRow>,
    /// Reference log row shown after the tree is populated.
    pub log_row: ArchivePatcherLogRow,
}

impl ArchivePatcherCandidateSnapshot {
    /// Creates a candidate snapshot and its reference `Showing N files...` row.
    pub fn new(
        request_id: u64,
        target: ArchivePatcherTarget,
        name_filter: Option<String>,
        rows: Vec<ArchivePatcherCandidateRow>,
    ) -> Self {
        let count = rows.len();
        Self {
            request_id,
            target,
            name_filter,
            rows,
            log_row: showing_files_log_row(count),
        }
    }
}

/// Read-only preview action for one Archive Patcher row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivePatcherPlanAction {
    /// This row can patch the BA2 version byte after confirmation.
    PatchVersionByte,
    /// This row cannot be mutated safely and carries a failure message.
    PlanFailure,
}

impl ArchivePatcherPlanAction {
    /// Returns a stable lowercase label for logs, digests, and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PatchVersionByte => "patch_version_byte",
            Self::PlanFailure => "plan_failure",
        }
    }
}

/// JSON-serializable restore metadata for one archive patched by a future worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePatcherRestoreManifestEntry {
    /// Original archive path captured at preview time for diagnostics and display.
    pub archive_path: PathBuf,
    /// Archive path relative to the validated Data folder.
    pub data_relative_path: PathBuf,
    /// Basename displayed in modal logs.
    pub file_name: String,
    /// Known BA2 format marker.
    pub format: ArchivePatcherArchiveFormat,
    /// Header version observed before patching.
    pub original_version: u32,
    /// Header version that would be written after confirmation.
    pub patched_version: u32,
}

impl ArchivePatcherRestoreManifestEntry {
    /// Creates a manifest entry from validated preview facts.
    pub fn new(
        archive_path: impl Into<PathBuf>,
        data_relative_path: impl Into<PathBuf>,
        file_name: impl Into<String>,
        format: ArchivePatcherArchiveFormat,
        original_version: u32,
        patched_version: u32,
    ) -> Self {
        Self {
            archive_path: archive_path.into(),
            data_relative_path: data_relative_path.into(),
            file_name: file_name.into(),
            format,
            original_version,
            patched_version,
        }
    }
}

/// One read-only preview row built from a candidate and bounded header probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherPreviewPlanRow {
    /// Candidate row selected from Overview archive data.
    pub candidate: ArchivePatcherCandidateRow,
    /// Header facts read from the current file, when parsing reached that point.
    pub header: Option<ArchivePatcherHeader>,
    /// Planned action for this row.
    pub action: ArchivePatcherPlanAction,
    /// Header version that would be written after confirmation.
    pub target_version: u32,
    /// Manifest metadata needed to restore this row later, only present for writable rows.
    pub restore_manifest_entry: Option<ArchivePatcherRestoreManifestEntry>,
    /// Safe user-visible failure text for rows that cannot be written.
    pub failure: Option<String>,
}

impl ArchivePatcherPreviewPlanRow {
    /// Creates a writable preview row with restore metadata.
    pub fn patch(
        candidate: ArchivePatcherCandidateRow,
        header: ArchivePatcherHeader,
        manifest_entry: ArchivePatcherRestoreManifestEntry,
    ) -> Self {
        let target_version = candidate.target.target_header_value();
        Self {
            candidate,
            header: Some(header),
            action: ArchivePatcherPlanAction::PatchVersionByte,
            target_version,
            restore_manifest_entry: Some(manifest_entry),
            failure: None,
        }
    }

    /// Creates a fail-closed preview row that must never be handed to a writer.
    pub fn failure(
        candidate: ArchivePatcherCandidateRow,
        header: Option<ArchivePatcherHeader>,
        failure: impl Into<String>,
    ) -> Self {
        let target_version = candidate.target.target_header_value();
        Self {
            candidate,
            header,
            action: ArchivePatcherPlanAction::PlanFailure,
            target_version,
            restore_manifest_entry: None,
            failure: Some(failure.into()),
        }
    }

    /// Returns whether a later confirmed worker may mutate this row.
    pub fn can_write(&self) -> bool {
        self.action == ArchivePatcherPlanAction::PatchVersionByte
            && self.failure.is_none()
            && self.restore_manifest_entry.is_some()
    }

    /// Returns a log row describing this row's preview failure, if present.
    pub fn failure_log_row(&self) -> Option<ArchivePatcherLogRow> {
        self.failure
            .as_ref()
            .map(|message| ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Bad, message.clone()))
    }
}

/// Aggregate counts from an Archive Patcher preview plan.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePatcherPreviewPlanCounts {
    /// Number of Overview-selected candidate rows.
    pub candidate_rows: usize,
    /// Number of rows safe to patch after confirmation.
    pub patchable_rows: usize,
    /// Number of rows that failed read-only validation.
    pub failed_rows: usize,
}

/// Complete read-only Archive Patcher preview plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherPreviewPlan {
    /// Request id copied from the plan request.
    pub request_id: u64,
    /// Desired target selected by the user.
    pub target: ArchivePatcherTarget,
    /// Name filter exactly as normalized by the service, if any.
    pub name_filter: Option<String>,
    /// Data root used for path-containment checks, when supplied.
    pub data_root: Option<PathBuf>,
    /// Candidate tree rows and `Showing N files...` log message.
    pub candidates: ArchivePatcherCandidateSnapshot,
    /// Per-candidate preview rows in deterministic path order.
    pub rows: Vec<ArchivePatcherPreviewPlanRow>,
    /// Aggregate row counts for UI summaries and diagnostics.
    pub counts: ArchivePatcherPreviewPlanCounts,
    /// True when at least one row can be handed to a later confirmed writer.
    pub can_execute: bool,
    /// Summary log row matching the Patch All no-op message when there are no candidates.
    pub summary_log_row: ArchivePatcherLogRow,
}

impl ArchivePatcherPreviewPlan {
    /// Creates a preview plan and computes aggregate counts.
    pub fn from_rows(
        request_id: u64,
        target: ArchivePatcherTarget,
        name_filter: Option<String>,
        data_root: Option<PathBuf>,
        candidates: ArchivePatcherCandidateSnapshot,
        rows: Vec<ArchivePatcherPreviewPlanRow>,
    ) -> Self {
        let patchable_rows = rows.iter().filter(|row| row.can_write()).count();
        let failed_rows = rows.iter().filter(|row| row.failure.is_some()).count();
        let counts = ArchivePatcherPreviewPlanCounts {
            candidate_rows: candidates.rows.len(),
            patchable_rows,
            failed_rows,
        };
        let summary_log_row = if counts.candidate_rows == 0 {
            nothing_to_do_log_row()
        } else {
            showing_files_log_row(counts.candidate_rows)
        };
        Self {
            request_id,
            target,
            name_filter,
            data_root,
            candidates,
            rows,
            counts,
            can_execute: patchable_rows > 0,
            summary_log_row,
        }
    }

    /// Returns a stable digest for target, filter, candidate, header, and manifest state.
    ///
    /// Request ids are intentionally excluded so a later confirmation can reject changed
    /// candidates or headers even when the controller assigns a fresh request id.
    pub fn stable_digest(&self) -> String {
        let mut digest = Sha256::new();
        update_digest_value(&mut digest, "cmt-rs-archive-patcher-preview-plan-v1");
        update_digest_value(&mut digest, self.target.as_reference_str());
        update_digest_value(&mut digest, self.name_filter.as_deref().unwrap_or(""));
        update_digest_value(
            &mut digest,
            self.data_root
                .as_ref()
                .map(|path| path.to_string_lossy())
                .as_deref()
                .unwrap_or(""),
        );
        update_digest_value(&mut digest, bool_digest_value(self.can_execute));
        update_digest_value(&mut digest, self.counts.candidate_rows.to_string());
        update_digest_value(&mut digest, self.counts.patchable_rows.to_string());
        update_digest_value(&mut digest, self.counts.failed_rows.to_string());

        for row in &self.rows {
            update_digest_value(&mut digest, row.candidate.path.to_string_lossy().as_ref());
            update_digest_value(&mut digest, &row.candidate.display_name);
            update_digest_value(
                &mut digest,
                row.candidate.overview_version.as_header_value().to_string(),
            );
            update_digest_value(
                &mut digest,
                row.candidate
                    .overview_format
                    .as_reference_magic()
                    .unwrap_or("unknown"),
            );
            update_digest_value(&mut digest, row.action.as_str());
            update_digest_value(&mut digest, row.target_version.to_string());
            if let Some(header) = row.header {
                update_digest_value(&mut digest, "header:some");
                update_digest_value(&mut digest, header.version.to_string());
                update_digest_value(&mut digest, header.format.as_reference_magic());
            } else {
                update_digest_value(&mut digest, "header:none");
            }
            update_digest_value(&mut digest, row.failure.as_deref().unwrap_or(""));
            if let Some(entry) = &row.restore_manifest_entry {
                update_digest_value(&mut digest, "manifest:some");
                update_digest_value(&mut digest, entry.archive_path.to_string_lossy().as_ref());
                update_digest_value(
                    &mut digest,
                    entry.data_relative_path.to_string_lossy().as_ref(),
                );
                update_digest_value(&mut digest, &entry.file_name);
                update_digest_value(&mut digest, entry.format.as_reference_magic());
                update_digest_value(&mut digest, entry.original_version.to_string());
                update_digest_value(&mut digest, entry.patched_version.to_string());
            } else {
                update_digest_value(&mut digest, "manifest:none");
            }
        }

        format!("{:x}", digest.finalize())
    }
}

/// JSON-serializable latest restore manifest for a future confirmed patch run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePatcherLatestManifest {
    /// Manifest schema version for forwards-compatible parsing.
    pub schema_version: u32,
    /// Stable digest of the preview plan that produced this manifest.
    pub plan_digest: String,
    /// Target selected for the patch run that produced this manifest.
    pub target: ArchivePatcherTarget,
    /// Per-archive restore entries.
    pub entries: Vec<ArchivePatcherRestoreManifestEntry>,
}

impl ArchivePatcherLatestManifest {
    /// Creates a latest-manifest payload from a preview plan and its writable entries.
    pub fn new(
        plan_digest: impl Into<String>,
        target: ArchivePatcherTarget,
        entries: Vec<ArchivePatcherRestoreManifestEntry>,
    ) -> Self {
        Self {
            schema_version: ARCHIVE_PATCHER_MANIFEST_SCHEMA_VERSION,
            plan_digest: plan_digest.into(),
            target,
            entries,
        }
    }
}

/// Archive Patcher progress state for later worker/controller layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchivePatcherProgress {
    /// User-visible progress text.
    pub text: String,
    /// Progress percentage clamped to `0.0..=100.0`.
    pub percent: f32,
}

impl ArchivePatcherProgress {
    /// Creates a clamped progress value.
    pub fn new(text: impl Into<String>, percent: f32) -> Self {
        Self {
            text: text.into(),
            percent: percent.clamp(0.0, 100.0),
        }
    }

    /// Creates the idle progress state.
    pub fn idle() -> Self {
        Self::new("Ready", 0.0)
    }

    /// Creates a complete progress state.
    pub fn complete(text: impl Into<String>) -> Self {
        Self::new(text, 100.0)
    }
}

/// Summary counts emitted after a future patch or restore run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePatcherSummaryCounts {
    /// Number of archives successfully patched to the desired version.
    pub patched: usize,
    /// Number of archives successfully restored from manifest metadata.
    pub restored: usize,
    /// Number of archives skipped without mutation.
    pub skipped: usize,
    /// Number of archives that failed safely.
    pub failed: usize,
}

impl ArchivePatcherSummaryCounts {
    /// Creates patch summary counts.
    pub const fn patch(patched: usize, failed: usize) -> Self {
        Self {
            patched,
            restored: 0,
            skipped: 0,
            failed,
        }
    }

    /// Returns the reference patching-complete message.
    pub fn patching_complete_message(self) -> String {
        patching_complete_message(self.patched, self.failed)
    }
}

/// Builds the reference tree-population log message.
pub fn showing_files_message(count: usize) -> String {
    format!("Showing {count} files to be patched.")
}

/// Builds the transient reference tree-population log row.
pub fn showing_files_log_row(count: usize) -> ArchivePatcherLogRow {
    ArchivePatcherLogRow::transient_info(showing_files_message(count))
}

/// Builds the transient no-op log row emitted by `Patch All` with no candidates.
pub fn nothing_to_do_log_row() -> ArchivePatcherLogRow {
    ArchivePatcherLogRow::transient_info("Nothing to do!")
}

/// Builds the reference successful patch log row.
pub fn patched_to_target_log_row(
    target: ArchivePatcherTarget,
    file_name: impl AsRef<str>,
) -> ArchivePatcherLogRow {
    ArchivePatcherLogRow::new(
        ArchivePatcherLogLevel::Good,
        format!(
            "Patched to v{}: {}",
            target.target_header_value(),
            file_name.as_ref()
        ),
    )
}

/// Builds the reference unrecognized-magic failure message.
pub fn unrecognized_format_message(file_name: impl AsRef<str>) -> String {
    format!("Unrecognized format: {}", file_name.as_ref())
}

/// Builds the explicit short-header preview failure message.
pub fn short_header_message(file_name: impl AsRef<str>) -> String {
    format!(
        "Archive header is shorter than the BA2 header length: {}",
        file_name.as_ref()
    )
}

/// Builds the explicit unsupported-format preview failure message.
pub fn unsupported_archive_format_message(
    format: impl AsRef<str>,
    file_name: impl AsRef<str>,
) -> String {
    format!(
        "Unrecognized archive format [{}]: {}",
        format.as_ref(),
        file_name.as_ref()
    )
}

/// Builds the reference already-target failure message.
pub fn skipping_already_patched_message(file_name: impl AsRef<str>) -> String {
    format!("Skipping already-patched archive: {}", file_name.as_ref())
}

/// Builds the reference unrecognized-version failure message.
pub fn unrecognized_version_message(
    version_hex: impl AsRef<str>,
    file_name: impl AsRef<str>,
) -> String {
    format!(
        "Unrecognized version [{}]: {}",
        version_hex.as_ref(),
        file_name.as_ref()
    )
}

/// Builds the reference file-not-found patch failure message.
pub fn failed_patching_file_not_found_message(file_name: impl AsRef<str>) -> String {
    format!("Failed patching (File Not Found): {}", file_name.as_ref())
}

/// Builds the reference permissions/in-use patch failure message.
pub fn failed_patching_permissions_message(file_name: impl AsRef<str>) -> String {
    format!(
        "Failed patching (Permissions/In-Use): {}",
        file_name.as_ref()
    )
}

/// Builds the reference unknown-OS-error patch failure message.
pub fn failed_patching_unknown_os_message(file_name: impl AsRef<str>) -> String {
    format!("Failed patching (Unknown OS Error): {}", file_name.as_ref())
}

/// Builds the reference final patching summary message.
pub fn patching_complete_message(patched: usize, failed: usize) -> String {
    format!("Patching complete. {patched} Successful, {failed} Failed.")
}

fn bool_digest_value(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn update_digest_value(digest: &mut Sha256, value: impl AsRef<str>) {
    let value = value.as_ref().as_bytes();
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value);
}

#[cfg(test)]
mod archive_patcher_domain {
    use super::*;

    #[test]
    fn archive_patcher_domain_reference_strings_match_python_modal() {
        assert_eq!(ARCHIVE_PATCHER_MODAL_TITLE, "Archive Patcher");
        assert_eq!(ARCHIVE_PATCHER_MODAL_WIDTH, 700);
        assert_eq!(ARCHIVE_PATCHER_MODAL_HEIGHT, 600);
        assert_eq!(DESIRED_VERSION_GROUP_LABEL, "Desired Version");
        assert_eq!(ArchivePatcherTarget::OldGen.as_reference_str(), "v1 (OG)");
        assert_eq!(ArchivePatcherTarget::NextGen.as_reference_str(), "v8 (NG)");
        assert_eq!(DEFAULT_ARCHIVE_PATCHER_TARGET, ArchivePatcherTarget::OldGen);
        assert_eq!(PATCH_ALL_BUTTON_LABEL, "Patch All");
        assert_eq!(ABOUT_BUTTON_LABEL, "About");
        assert_eq!(NAME_FILTER_LABEL, "Name Filter:");
        assert_eq!(
            ArchivePatcherTarget::OldGen.filter_text(),
            PATCHER_FILTER_NEXT_GEN
        );
        assert_eq!(
            ArchivePatcherTarget::NextGen.filter_text(),
            PATCHER_FILTER_OLD_GEN
        );
        assert_eq!(
            ABOUT_ARCHIVES_TITLE,
            "Bethesda Archive (BA2) Formats & Versions"
        );
        assert!(ABOUT_ARCHIVES_BODY.contains("v7/8 are identical"));
    }

    #[test]
    fn archive_patcher_domain_targets_invert_source_versions() {
        assert!(ArchivePatcherTarget::OldGen.selects_overview_version(ArchiveVersion::NextGen7));
        assert!(ArchivePatcherTarget::OldGen.selects_overview_version(ArchiveVersion::NextGen8));
        assert!(!ArchivePatcherTarget::OldGen.selects_overview_version(ArchiveVersion::OldGen));
        assert!(ArchivePatcherTarget::NextGen.selects_overview_version(ArchiveVersion::OldGen));
        assert!(!ArchivePatcherTarget::NextGen.selects_overview_version(ArchiveVersion::NextGen7));
        assert!(!ArchivePatcherTarget::NextGen.selects_overview_version(ArchiveVersion::NextGen8));
        assert_eq!(ArchivePatcherTarget::OldGen.target_header_value(), 1);
        assert_eq!(ArchivePatcherTarget::NextGen.target_header_value(), 8);
    }

    #[test]
    fn archive_patcher_domain_log_messages_match_reference_vocabulary() {
        assert_eq!(showing_files_message(3), "Showing 3 files to be patched.");
        assert_eq!(
            showing_files_log_row(0).message,
            "Showing 0 files to be patched."
        );
        assert!(showing_files_log_row(0).skip_file_logging);
        assert_eq!(nothing_to_do_log_row().message, "Nothing to do!");
        assert_eq!(
            patched_to_target_log_row(ArchivePatcherTarget::OldGen, "A.ba2").message,
            "Patched to v1: A.ba2"
        );
        assert_eq!(
            unrecognized_format_message("Bad.ba2"),
            "Unrecognized format: Bad.ba2"
        );
        assert_eq!(
            short_header_message("Short.ba2"),
            "Archive header is shorter than the BA2 header length: Short.ba2"
        );
        assert_eq!(
            unsupported_archive_format_message("XXXX", "Bad.ba2"),
            "Unrecognized archive format [XXXX]: Bad.ba2"
        );
        assert_eq!(
            skipping_already_patched_message("Old.ba2"),
            "Skipping already-patched archive: Old.ba2"
        );
        assert_eq!(
            unrecognized_version_message("09", "Odd.ba2"),
            "Unrecognized version [09]: Odd.ba2"
        );
        assert_eq!(
            failed_patching_file_not_found_message("Gone.ba2"),
            "Failed patching (File Not Found): Gone.ba2"
        );
        assert_eq!(
            failed_patching_permissions_message("Locked.ba2"),
            "Failed patching (Permissions/In-Use): Locked.ba2"
        );
        assert_eq!(
            failed_patching_unknown_os_message("Odd.ba2"),
            "Failed patching (Unknown OS Error): Odd.ba2"
        );
        assert_eq!(
            patching_complete_message(2, 1),
            "Patching complete. 2 Successful, 1 Failed."
        );
        assert_eq!(ArchivePatcherLogLevel::Info.as_reference_str(), "info");
        assert_eq!(ArchivePatcherLogLevel::Good.as_reference_str(), "good");
        assert_eq!(ArchivePatcherLogLevel::Bad.as_reference_str(), "bad");
    }

    #[test]
    fn archive_patcher_domain_progress_counts_and_manifest_are_typed() {
        assert_eq!(ArchivePatcherProgress::idle().percent, 0.0);
        assert_eq!(ArchivePatcherProgress::new("Half", 42.5).percent, 42.5);
        assert_eq!(ArchivePatcherProgress::new("Too low", -1.0).percent, 0.0);
        assert_eq!(ArchivePatcherProgress::complete("Done").percent, 100.0);
        assert_eq!(
            ArchivePatcherSummaryCounts::patch(4, 2).patching_complete_message(),
            "Patching complete. 4 Successful, 2 Failed."
        );

        let entry = ArchivePatcherRestoreManifestEntry::new(
            "Game/Data/Fallout4 - Main.ba2",
            "Fallout4 - Main.ba2",
            "Fallout4 - Main.ba2",
            ArchivePatcherArchiveFormat::General,
            8,
            1,
        );
        let manifest = ArchivePatcherLatestManifest::new(
            "digest",
            ArchivePatcherTarget::OldGen,
            vec![entry.clone()],
        );
        let json = serde_json::to_string(&manifest).expect("manifest should serialize");
        assert!(json.contains("schema_version"));
        assert!(json.contains("old_gen"));
        let round_trip: ArchivePatcherLatestManifest =
            serde_json::from_str(&json).expect("manifest should deserialize");
        assert_eq!(round_trip.entries, vec![entry]);
    }
}
