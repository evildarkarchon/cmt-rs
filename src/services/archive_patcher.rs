//! Read-only Archive Patcher candidate and preview-plan service.
//!
//! The Python reference selects BA2 files from Overview's enabled archive sets,
//! previews them in sorted order, and only mutates when the user presses
//! `Patch All`. This service preserves that split: it consumes already-collected
//! [`ArchiveRecord`] values, applies the target/filter rules without touching
//! Slint, reads only bounded BA2 header prefixes through [`Filesystem::read_prefix`],
//! and returns fail-closed plan rows for a later confirmed worker.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;
use tracing::{debug, info, info_span, warn};

use crate::{
    domain::{
        archive_patcher::{
            ARCHIVE_PATCHER_MANIFEST_SCHEMA_VERSION, ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE,
            ArchivePatcherArchiveFormat, ArchivePatcherCandidateRow,
            ArchivePatcherCandidateSnapshot, ArchivePatcherExecutionFileResult,
            ArchivePatcherExecutionOutcome, ArchivePatcherExecutionResult, ArchivePatcherHeader,
            ArchivePatcherLatestManifest, ArchivePatcherLogLevel, ArchivePatcherLogRow,
            ArchivePatcherPreviewPlan, ArchivePatcherPreviewPlanRow,
            ArchivePatcherRestoreManifestEntry, ArchivePatcherSummaryCounts, ArchivePatcherTarget,
            BA2_FORMAT_DIRECTX10, BA2_FORMAT_GENERAL, BA2_HEADER_PREFIX_LEN, BA2_MAGIC,
            BA2_VERSION_FIELD_OFFSET, DATA_ROOT_MISSING_FAILURE_MESSAGE,
            archive_changed_before_patching_message, ba2_header_prefix,
            failed_patching_file_not_found_message, failed_patching_permissions_message,
            failed_patching_unknown_os_message, failed_restoring_file_not_found_message,
            failed_restoring_permissions_message, failed_restoring_unknown_os_message,
            nothing_to_do_log_row, patched_to_target_log_row, patching_complete_message,
            restore_complete_message, restored_to_original_log_row, short_header_message,
            skipping_already_patched_message, skipping_restore_stale_message,
            unrecognized_format_message, unrecognized_version_message,
            unsupported_archive_format_message,
        },
        discovery::ArchiveRecord,
    },
    platform::{
        PlatformError, PlatformErrorKind,
        filesystem::{Filesystem, WritableFilesystem},
    },
};

/// Request input for selecting Archive Patcher candidate rows.
#[derive(Debug, Clone, Copy)]
pub struct ArchivePatcherCandidateRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Current Overview/discovery archive records.
    pub archives: &'a [ArchiveRecord],
    /// Desired target selected by the user.
    pub target: ArchivePatcherTarget,
    /// Optional basename filter from the modal entry.
    pub name_filter: Option<&'a str>,
}

impl<'a> ArchivePatcherCandidateRequest<'a> {
    /// Creates a candidate request from Overview archive records and modal state.
    pub const fn new(
        request_id: u64,
        archives: &'a [ArchiveRecord],
        target: ArchivePatcherTarget,
        name_filter: Option<&'a str>,
    ) -> Self {
        Self {
            request_id,
            archives,
            target,
            name_filter,
        }
    }
}

/// Request input for building a read-only Archive Patcher preview plan.
#[derive(Debug, Clone, Copy)]
pub struct ArchivePatcherPlanRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional validated Data directory used for path-containment checks.
    pub data_root: Option<&'a Path>,
    /// Current Overview/discovery archive records.
    pub archives: &'a [ArchiveRecord],
    /// Desired target selected by the user.
    pub target: ArchivePatcherTarget,
    /// Optional basename filter from the modal entry.
    pub name_filter: Option<&'a str>,
}

impl<'a> ArchivePatcherPlanRequest<'a> {
    /// Creates a preview-plan request from Overview archive records and modal state.
    pub const fn new(
        request_id: u64,
        data_root: Option<&'a Path>,
        archives: &'a [ArchiveRecord],
        target: ArchivePatcherTarget,
        name_filter: Option<&'a str>,
    ) -> Self {
        Self {
            request_id,
            data_root,
            archives,
            target,
            name_filter,
        }
    }
}

/// Request input for a confirmed Archive Patcher patch run.
#[derive(Debug, Clone, Copy)]
pub struct ArchivePatcherExecutionRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional validated Data directory used for path-containment checks.
    pub data_root: Option<&'a Path>,
    /// Current Overview/discovery archive records.
    pub archives: &'a [ArchiveRecord],
    /// Desired target selected by the user.
    pub target: ArchivePatcherTarget,
    /// Optional basename filter from the modal entry.
    pub name_filter: Option<&'a str>,
    /// Stable digest of the preview that the user reviewed before confirming.
    pub confirmed_plan_digest: Option<&'a str>,
    /// App-owned latest restore manifest path.
    pub manifest_path: &'a Path,
}

impl<'a> ArchivePatcherExecutionRequest<'a> {
    /// Creates a confirmed patch request from Overview archive records and modal state.
    pub const fn new(
        request_id: u64,
        data_root: Option<&'a Path>,
        archives: &'a [ArchiveRecord],
        target: ArchivePatcherTarget,
        name_filter: Option<&'a str>,
        manifest_path: &'a Path,
    ) -> Self {
        Self {
            request_id,
            data_root,
            archives,
            target,
            name_filter,
            confirmed_plan_digest: None,
            manifest_path,
        }
    }

    /// Binds execution to the exact read-only preview plan the user confirmed.
    pub const fn with_confirmed_plan_digest(mut self, digest: &'a str) -> Self {
        self.confirmed_plan_digest = Some(digest);
        self
    }
}

/// Request input for restoring the most recent Archive Patcher run.
#[derive(Debug, Clone, Copy)]
pub struct ArchivePatcherRestoreRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional validated Data directory used for path-containment checks.
    pub data_root: Option<&'a Path>,
    /// App-owned latest restore manifest path.
    pub manifest_path: &'a Path,
}

impl<'a> ArchivePatcherRestoreRequest<'a> {
    /// Creates a restore-last-run request.
    pub const fn new(
        request_id: u64,
        data_root: Option<&'a Path>,
        manifest_path: &'a Path,
    ) -> Self {
        Self {
            request_id,
            data_root,
            manifest_path,
        }
    }
}

/// Safe failure returned before a patch or restore result can be trusted.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ArchivePatcherExecutionError {
    /// A confirmed run observed preview state different from the user-confirmed digest.
    #[error("{safe_message}")]
    ConfirmedPlanChanged {
        /// User-facing safe failure text.
        safe_message: String,
        /// Stable digest captured when the user reviewed the plan.
        expected_digest: String,
        /// Stable digest built immediately before execution.
        actual_digest: String,
    },
    /// The latest restore manifest could not be serialized or written before mutation.
    #[error("{safe_message}")]
    ManifestWriteFailed {
        /// User-facing safe failure text.
        safe_message: String,
        /// Diagnostic detail for tracing/tests.
        diagnostic: String,
    },
    /// The latest restore manifest could not be read.
    #[error("{safe_message}")]
    ManifestReadFailed {
        /// User-facing safe failure text.
        safe_message: String,
        /// Diagnostic detail for tracing/tests.
        diagnostic: String,
    },
    /// The latest restore manifest could not be parsed or was for an unsupported schema.
    #[error("{safe_message}")]
    ManifestParseFailed {
        /// User-facing safe failure text.
        safe_message: String,
        /// Diagnostic detail for tracing/tests.
        diagnostic: String,
    },
}

impl ArchivePatcherExecutionError {
    /// Returns the safe text suitable for modal logs or disabled-state banners.
    pub fn user_message(&self) -> &str {
        match self {
            Self::ConfirmedPlanChanged { safe_message, .. }
            | Self::ManifestWriteFailed { safe_message, .. }
            | Self::ManifestReadFailed { safe_message, .. }
            | Self::ManifestParseFailed { safe_message, .. } => safe_message,
        }
    }

    /// Returns diagnostic detail suitable for tracing/tests.
    pub fn diagnostic(&self) -> Option<&str> {
        match self {
            Self::ConfirmedPlanChanged { .. } => None,
            Self::ManifestWriteFailed { diagnostic, .. }
            | Self::ManifestReadFailed { diagnostic, .. }
            | Self::ManifestParseFailed { diagnostic, .. } => Some(diagnostic),
        }
    }
}

const CONFIRMED_ARCHIVE_PLAN_CHANGED_MESSAGE: &str =
    "Archive Patcher files changed after preview. Refresh the plan and try again.";
const MANIFEST_WRITE_FAILED_MESSAGE: &str =
    "Restore manifest could not be written; patching was cancelled.";
const MANIFEST_READ_FAILED_MESSAGE: &str = "Restore manifest could not be read.";
const MANIFEST_PARSE_FAILED_MESSAGE: &str = "Restore manifest could not be understood.";

/// Read-only service that selects Archive Patcher candidates and validates preview rows.
#[derive(Debug, Clone, Copy)]
pub struct ArchivePatcherService<'a, F: Filesystem + ?Sized> {
    filesystem: &'a F,
}

impl<'a, F: Filesystem + ?Sized> ArchivePatcherService<'a, F> {
    /// Creates an Archive Patcher planning service over a read-only filesystem adapter.
    pub const fn new(filesystem: &'a F) -> Self {
        Self { filesystem }
    }

    /// Selects candidate rows from current Overview archive records without filesystem reads.
    pub fn candidate_snapshot(
        &self,
        request: ArchivePatcherCandidateRequest<'_>,
    ) -> ArchivePatcherCandidateSnapshot {
        let span = info_span!(
            "archive_patcher.candidate_snapshot",
            request_id = request.request_id,
            target = request.target.as_reference_str(),
            archive_count = request.archives.len(),
            has_filter = request.name_filter.is_some(),
        );
        let _guard = span.enter();
        info!(
            event = "archive-patcher-candidates-request",
            "Archive Patcher candidates requested"
        );

        let normalized_filter = normalize_filter(request.name_filter);
        let mut rows: Vec<_> = request
            .archives
            .iter()
            .filter(|record| archive_is_candidate(record, request.target))
            .filter(|record| basename_matches_filter(&record.path, normalized_filter.as_deref()))
            .map(|record| candidate_row(record, request.target))
            .collect();
        rows.sort_by(|left, right| compare_reference_paths(&left.path, &right.path));

        debug!(
            event = "archive-patcher-candidates-selected",
            request_id = request.request_id,
            candidate_count = rows.len(),
            "Archive Patcher candidates selected"
        );
        ArchivePatcherCandidateSnapshot::new(
            request.request_id,
            request.target,
            normalized_filter,
            rows,
        )
    }

    /// Builds a read-only preview plan by probing only BA2 header prefixes.
    pub fn preview_plan(
        &self,
        request: ArchivePatcherPlanRequest<'_>,
    ) -> ArchivePatcherPreviewPlan {
        let span = info_span!(
            "archive_patcher.preview_plan",
            request_id = request.request_id,
            target = request.target.as_reference_str(),
            archive_count = request.archives.len(),
            has_filter = request.name_filter.is_some(),
            has_data_root = request.data_root.is_some(),
        );
        let _guard = span.enter();
        info!(
            event = "archive-patcher-plan-request",
            "Archive Patcher preview plan requested"
        );

        let candidates = self.candidate_snapshot(ArchivePatcherCandidateRequest::new(
            request.request_id,
            request.archives,
            request.target,
            request.name_filter,
        ));
        let rows = candidates
            .rows
            .iter()
            .cloned()
            .map(|candidate| self.preview_row(request.data_root, candidate))
            .collect::<Vec<_>>();
        let plan = ArchivePatcherPreviewPlan::from_rows(
            request.request_id,
            request.target,
            candidates.name_filter.clone(),
            request.data_root.map(Path::to_path_buf),
            candidates,
            rows,
        );
        info!(
            event = "archive-patcher-plan-complete",
            request_id = plan.request_id,
            candidate_rows = plan.counts.candidate_rows,
            patchable_rows = plan.counts.patchable_rows,
            failed_rows = plan.counts.failed_rows,
            can_execute = plan.can_execute,
            "Archive Patcher preview plan built"
        );
        plan
    }

    fn preview_row(
        &self,
        data_root: Option<&Path>,
        candidate: ArchivePatcherCandidateRow,
    ) -> ArchivePatcherPreviewPlanRow {
        let display_name = candidate.display_name.clone();
        let relative_path = match contained_relative_path(data_root, &candidate.path) {
            Ok(relative_path) => relative_path,
            Err(message) => {
                warn!(
                    event = "archive-patcher-path-containment-failed",
                    path = %candidate.path.display(),
                    failure = message.as_str(),
                    "Archive Patcher candidate failed containment validation"
                );
                return ArchivePatcherPreviewPlanRow::failure(candidate, None, message);
            }
        };

        let header_bytes = match self
            .filesystem
            .read_prefix(&candidate.path, BA2_HEADER_PREFIX_LEN)
        {
            Ok(header) => header,
            Err(error) => {
                let message = read_error_message(&error, &display_name);
                warn!(
                    event = "archive-patcher-header-read-failed",
                    path = %candidate.path.display(),
                    failure_kind = ?error.kind,
                    "Archive Patcher header prefix read failed"
                );
                return ArchivePatcherPreviewPlanRow::failure(candidate, None, message);
            }
        };

        let header = match parse_ba2_header(&header_bytes, &display_name) {
            Ok(header) => header,
            Err(message) => {
                debug!(
                    event = "archive-patcher-header-invalid",
                    path = %candidate.path.display(),
                    failure = message.as_str(),
                    "Archive Patcher header prefix failed validation"
                );
                return ArchivePatcherPreviewPlanRow::failure(candidate, None, message);
            }
        };

        if candidate.target.is_target_header_version(header.version) {
            return ArchivePatcherPreviewPlanRow::failure(
                candidate,
                Some(header),
                skipping_already_patched_message(&display_name),
            );
        }
        if !candidate.target.accepts_header_transition(header.version) {
            return ArchivePatcherPreviewPlanRow::failure(
                candidate,
                Some(header),
                unrecognized_version_message(version_byte_hex(header.version), &display_name),
            );
        }

        let manifest_entry = ArchivePatcherRestoreManifestEntry::new(
            candidate.path.clone(),
            relative_path,
            display_name,
            header.format,
            header.version,
            candidate.target.target_header_value(),
        )
        .with_header_prefixes(
            header_bytes.clone(),
            ba2_header_prefix(candidate.target.target_header_value(), header.format),
        );
        ArchivePatcherPreviewPlanRow::patch(candidate, header, manifest_entry)
    }
}

impl<'a, F: Filesystem + WritableFilesystem + ?Sized> ArchivePatcherService<'a, F> {
    /// Executes a freshly revalidated Archive Patcher plan after explicit confirmation.
    ///
    /// The latest restore manifest is written before any BA2 header byte is mutated. Each archive
    /// is then revalidated and processed independently so one bad file does not hide later results.
    pub fn execute_confirmed(
        &self,
        request: ArchivePatcherExecutionRequest<'_>,
    ) -> Result<ArchivePatcherExecutionResult, ArchivePatcherExecutionError> {
        let span = info_span!(
            "archive_patcher.execute_confirmed",
            request_id = request.request_id,
            target = request.target.as_reference_str(),
            archive_count = request.archives.len(),
            has_filter = request.name_filter.is_some(),
            has_data_root = request.data_root.is_some(),
            manifest_path = %request.manifest_path.display(),
        );
        let _guard = span.enter();
        info!(
            event = "archive-patcher-execute-request",
            "Archive Patcher confirmed execution requested"
        );

        let plan = self.preview_plan(ArchivePatcherPlanRequest::new(
            request.request_id,
            request.data_root,
            request.archives,
            request.target,
            request.name_filter,
        ));
        let plan_digest = plan.stable_digest();
        if let Some(expected_digest) = request.confirmed_plan_digest {
            if plan_digest != expected_digest {
                warn!(
                    event = "archive-patcher-confirmed-plan-changed",
                    request_id = request.request_id,
                    expected_digest,
                    actual_digest = plan_digest.as_str(),
                    "Archive Patcher confirmed run aborted because the preview plan changed"
                );
                return Err(ArchivePatcherExecutionError::ConfirmedPlanChanged {
                    safe_message: CONFIRMED_ARCHIVE_PLAN_CHANGED_MESSAGE.to_owned(),
                    expected_digest: expected_digest.to_owned(),
                    actual_digest: plan_digest,
                });
            }
        }

        let manifest_entries = plan
            .rows
            .iter()
            .filter_map(|row| row.restore_manifest_entry.clone())
            .collect::<Vec<_>>();
        if !manifest_entries.is_empty() {
            let manifest = ArchivePatcherLatestManifest::new(
                plan_digest.clone(),
                request.target,
                manifest_entries,
            );
            self.write_latest_manifest(request.manifest_path, &manifest)?;
            info!(
                event = "archive-patcher-manifest-written",
                request_id = request.request_id,
                entry_count = manifest.entries.len(),
                manifest_path = %request.manifest_path.display(),
                "Archive Patcher latest restore manifest written before mutation"
            );
        }

        let mut result = ArchivePatcherExecutionResult {
            request_id: request.request_id,
            target: request.target,
            manifest_path: request.manifest_path.to_path_buf(),
            plan_digest,
            rows: Vec::with_capacity(plan.rows.len()),
            log_rows: Vec::with_capacity(plan.rows.len().saturating_add(1)),
            counts: ArchivePatcherSummaryCounts::default(),
            diagnostics: Vec::new(),
        };

        if plan.rows.is_empty() {
            let log_row = nothing_to_do_log_row();
            result.log_rows.push(log_row);
            return Ok(result);
        }

        for row in &plan.rows {
            let file_result = self.execute_patch_row(request.data_root, row);
            debug!(
                event = "archive-patcher-execute-row-complete",
                request_id = request.request_id,
                path = %file_result.archive_path.display(),
                outcome = ?file_result.outcome,
                diagnostic_count = file_result.diagnostics.len(),
                "Archive Patcher execution row completed"
            );
            result.log_rows.push(file_result.log_row.clone());
            result.diagnostics.extend(file_result.diagnostics.clone());
            result.rows.push(file_result);
        }

        let patched = result
            .rows
            .iter()
            .filter(|row| row.outcome == ArchivePatcherExecutionOutcome::Patched)
            .count();
        let failed = result
            .rows
            .iter()
            .filter(|row| row.outcome == ArchivePatcherExecutionOutcome::Failed)
            .count();
        result.counts = ArchivePatcherSummaryCounts::patch(patched, failed);
        result.log_rows.push(ArchivePatcherLogRow::new(
            ArchivePatcherLogLevel::Info,
            patching_complete_message(patched, failed),
        ));
        info!(
            event = "archive-patcher-execute-complete",
            request_id = result.request_id,
            patched,
            failed,
            diagnostic_count = result.diagnostics.len(),
            "Archive Patcher confirmed execution completed"
        );
        Ok(result)
    }

    /// Restores the most recent Archive Patcher run from the app-owned latest manifest.
    pub fn restore_last_run(
        &self,
        request: ArchivePatcherRestoreRequest<'_>,
    ) -> Result<ArchivePatcherExecutionResult, ArchivePatcherExecutionError> {
        let span = info_span!(
            "archive_patcher.restore_last_run",
            request_id = request.request_id,
            has_data_root = request.data_root.is_some(),
            manifest_path = %request.manifest_path.display(),
        );
        let _guard = span.enter();
        info!(
            event = "archive-patcher-restore-request",
            "Archive Patcher restore-last-run requested"
        );

        let manifest = self.read_latest_manifest(request.manifest_path)?;
        let mut result = ArchivePatcherExecutionResult {
            request_id: request.request_id,
            target: manifest.target,
            manifest_path: request.manifest_path.to_path_buf(),
            plan_digest: manifest.plan_digest.clone(),
            rows: Vec::with_capacity(manifest.entries.len()),
            log_rows: Vec::with_capacity(manifest.entries.len().saturating_add(1)),
            counts: ArchivePatcherSummaryCounts::default(),
            diagnostics: Vec::new(),
        };

        for entry in &manifest.entries {
            let file_result = self.restore_manifest_entry(request.data_root, entry);
            debug!(
                event = "archive-patcher-restore-row-complete",
                request_id = request.request_id,
                path = %file_result.archive_path.display(),
                outcome = ?file_result.outcome,
                diagnostic_count = file_result.diagnostics.len(),
                "Archive Patcher restore row completed"
            );
            result.log_rows.push(file_result.log_row.clone());
            result.diagnostics.extend(file_result.diagnostics.clone());
            result.rows.push(file_result);
        }

        let restored = result
            .rows
            .iter()
            .filter(|row| row.outcome == ArchivePatcherExecutionOutcome::Restored)
            .count();
        let skipped = result
            .rows
            .iter()
            .filter(|row| row.outcome == ArchivePatcherExecutionOutcome::Skipped)
            .count();
        let failed = result
            .rows
            .iter()
            .filter(|row| row.outcome == ArchivePatcherExecutionOutcome::Failed)
            .count();
        result.counts = ArchivePatcherSummaryCounts::restore(restored, skipped, failed);
        result.log_rows.push(ArchivePatcherLogRow::new(
            ArchivePatcherLogLevel::Info,
            restore_complete_message(restored, skipped, failed),
        ));
        info!(
            event = "archive-patcher-restore-complete",
            request_id = result.request_id,
            restored,
            skipped,
            failed,
            diagnostic_count = result.diagnostics.len(),
            "Archive Patcher restore-last-run completed"
        );
        Ok(result)
    }

    fn write_latest_manifest(
        &self,
        manifest_path: &Path,
        manifest: &ArchivePatcherLatestManifest,
    ) -> Result<(), ArchivePatcherExecutionError> {
        let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
            ArchivePatcherExecutionError::ManifestWriteFailed {
                safe_message: MANIFEST_WRITE_FAILED_MESSAGE.to_owned(),
                diagnostic: error.to_string(),
            }
        })?;
        self.filesystem
            .write_bytes(manifest_path, &bytes)
            .map_err(|error| {
                warn!(
                    event = "archive-patcher-manifest-write-failed",
                    manifest_path = %manifest_path.display(),
                    failure_kind = ?error.kind,
                    "Archive Patcher latest restore manifest write failed before mutation"
                );
                ArchivePatcherExecutionError::ManifestWriteFailed {
                    safe_message: MANIFEST_WRITE_FAILED_MESSAGE.to_owned(),
                    diagnostic: error.user_message().to_owned(),
                }
            })
    }

    fn read_latest_manifest(
        &self,
        manifest_path: &Path,
    ) -> Result<ArchivePatcherLatestManifest, ArchivePatcherExecutionError> {
        let json = self
            .filesystem
            .read_to_string(manifest_path)
            .map_err(|error| ArchivePatcherExecutionError::ManifestReadFailed {
                safe_message: MANIFEST_READ_FAILED_MESSAGE.to_owned(),
                diagnostic: error.user_message().to_owned(),
            })?;
        let manifest: ArchivePatcherLatestManifest =
            serde_json::from_str(&json).map_err(|error| {
                ArchivePatcherExecutionError::ManifestParseFailed {
                    safe_message: MANIFEST_PARSE_FAILED_MESSAGE.to_owned(),
                    diagnostic: error.to_string(),
                }
            })?;
        if manifest.schema_version != ARCHIVE_PATCHER_MANIFEST_SCHEMA_VERSION {
            return Err(ArchivePatcherExecutionError::ManifestParseFailed {
                safe_message: MANIFEST_PARSE_FAILED_MESSAGE.to_owned(),
                diagnostic: format!(
                    "unsupported Archive Patcher manifest schema {}",
                    manifest.schema_version
                ),
            });
        }
        Ok(manifest)
    }

    fn execute_patch_row(
        &self,
        data_root: Option<&Path>,
        row: &ArchivePatcherPreviewPlanRow,
    ) -> ArchivePatcherExecutionFileResult {
        let display_name = row.candidate.display_name.clone();
        let archive_path = row.candidate.path.clone();
        let Some(entry) = row.restore_manifest_entry.as_ref() else {
            let message = row
                .failure
                .clone()
                .unwrap_or_else(|| failed_patching_unknown_os_message(&display_name));
            return execution_file_result(
                archive_path,
                display_name,
                ArchivePatcherExecutionOutcome::Failed,
                ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Bad, message),
                Vec::new(),
            );
        };

        if let Err(message) = self.ensure_archive_file_within_data_root(
            data_root,
            &archive_path,
            &display_name,
            patch_platform_error_message,
        ) {
            return failed_execution_result(archive_path, display_name, message);
        }

        let header = match self.read_current_header(
            &archive_path,
            &display_name,
            patch_platform_error_message,
        ) {
            Ok(header) => header,
            Err(message) => return failed_execution_result(archive_path, display_name, message),
        };

        if row
            .candidate
            .target
            .is_target_header_version(header.version)
        {
            return failed_execution_result(
                archive_path,
                display_name.clone(),
                skipping_already_patched_message(&display_name),
            );
        }
        if !row
            .candidate
            .target
            .accepts_header_transition(header.version)
        {
            return failed_execution_result(
                archive_path,
                display_name.clone(),
                unrecognized_version_message(version_byte_hex(header.version), &display_name),
            );
        }
        if header.format != entry.format
            || header.version != entry.original_version
            || entry.patched_version != row.target_version
        {
            return failed_execution_result(
                archive_path,
                display_name.clone(),
                archive_changed_before_patching_message(&display_name),
            );
        }

        let patched_version_bytes = entry.patched_version_bytes();
        if let Err(error) = self.filesystem.write_byte_range(
            &archive_path,
            BA2_VERSION_FIELD_OFFSET,
            &patched_version_bytes,
        ) {
            return failed_execution_result(
                archive_path,
                display_name.clone(),
                patch_platform_error_message(&error, &display_name),
            );
        }

        match self.read_current_header(&archive_path, &display_name, patch_platform_error_message) {
            Ok(post_header)
                if post_header.version == entry.patched_version
                    && post_header.format == entry.format =>
            {
                execution_file_result(
                    archive_path,
                    display_name.clone(),
                    ArchivePatcherExecutionOutcome::Patched,
                    patched_to_target_log_row(row.candidate.target, &display_name),
                    Vec::new(),
                )
            }
            Ok(_) => failed_execution_result(
                archive_path,
                display_name.clone(),
                failed_patching_unknown_os_message(&display_name),
            ),
            Err(message) => failed_execution_result(archive_path, display_name, message),
        }
    }

    fn restore_manifest_entry(
        &self,
        data_root: Option<&Path>,
        entry: &ArchivePatcherRestoreManifestEntry,
    ) -> ArchivePatcherExecutionFileResult {
        let display_name = entry.file_name.clone();
        let archive_path = match restore_archive_path(data_root, entry) {
            Ok(path) => path,
            Err(message) => {
                return execution_file_result(
                    entry.archive_path.clone(),
                    display_name,
                    ArchivePatcherExecutionOutcome::Failed,
                    ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Bad, message),
                    Vec::new(),
                );
            }
        };

        if let Err(message) = self.ensure_archive_file_within_data_root(
            data_root,
            &archive_path,
            &display_name,
            restore_platform_error_message,
        ) {
            return restore_failed_result(archive_path, display_name, message);
        }

        let header = match self.read_current_header(
            &archive_path,
            &display_name,
            restore_platform_error_message,
        ) {
            Ok(header) => header,
            Err(message) => return restore_failed_result(archive_path, display_name, message),
        };
        if header.format != entry.format || header.version != entry.patched_version {
            let message = skipping_restore_stale_message(&display_name);
            return execution_file_result(
                archive_path,
                display_name,
                ArchivePatcherExecutionOutcome::Skipped,
                ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Info, message),
                Vec::new(),
            );
        }

        let original_version_bytes = entry.original_version_bytes();
        if let Err(error) = self.filesystem.write_byte_range(
            &archive_path,
            BA2_VERSION_FIELD_OFFSET,
            &original_version_bytes,
        ) {
            return restore_failed_result(
                archive_path,
                display_name.clone(),
                restore_platform_error_message(&error, &display_name),
            );
        }

        match self.read_current_header(&archive_path, &display_name, restore_platform_error_message)
        {
            Ok(post_header)
                if post_header.version == entry.original_version
                    && post_header.format == entry.format =>
            {
                execution_file_result(
                    archive_path,
                    display_name.clone(),
                    ArchivePatcherExecutionOutcome::Restored,
                    restored_to_original_log_row(entry.original_version, &display_name),
                    Vec::new(),
                )
            }
            Ok(_) => restore_failed_result(
                archive_path,
                display_name.clone(),
                failed_restoring_unknown_os_message(&display_name),
            ),
            Err(message) => restore_failed_result(archive_path, display_name, message),
        }
    }

    fn read_current_header(
        &self,
        archive_path: &Path,
        display_name: &str,
        platform_error_message: fn(&PlatformError, &str) -> String,
    ) -> Result<ArchivePatcherHeader, String> {
        let bytes = self
            .filesystem
            .read_prefix(archive_path, BA2_HEADER_PREFIX_LEN)
            .map_err(|error| platform_error_message(&error, display_name))?;
        parse_ba2_header(&bytes, display_name)
    }

    fn ensure_archive_file_within_data_root(
        &self,
        data_root: Option<&Path>,
        archive_path: &Path,
        display_name: &str,
        platform_error_message: fn(&PlatformError, &str) -> String,
    ) -> Result<(), String> {
        let data_root = data_root.ok_or_else(|| DATA_ROOT_MISSING_FAILURE_MESSAGE.to_owned())?;
        if data_root.as_os_str().is_empty()
            || archive_path.as_os_str().is_empty()
            || contains_unsafe_components(data_root)
            || contains_unsafe_components(archive_path)
        {
            return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
        }

        let root_metadata = self
            .filesystem
            .symlink_metadata(data_root)
            .map_err(|error| {
                if error.kind == PlatformErrorKind::NotFound {
                    DATA_ROOT_MISSING_FAILURE_MESSAGE.to_owned()
                } else {
                    platform_error_message(&error, display_name)
                }
            })?;
        if !root_metadata.is_dir() || root_metadata.is_symlink_or_reparse_point() {
            return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
        }

        let archive_metadata = self
            .filesystem
            .symlink_metadata(archive_path)
            .map_err(|error| platform_error_message(&error, display_name))?;
        if !archive_metadata.is_file() {
            return Err(platform_error_message(
                &PlatformError::new(
                    crate::platform::PlatformOperation::ReadMetadata,
                    archive_path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "Archive path is not a file.",
                ),
                display_name,
            ));
        }
        if archive_metadata.is_symlink_or_reparse_point() {
            return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
        }

        let canonical_root = self
            .filesystem
            .canonicalize_path(data_root)
            .map_err(|error| platform_error_message(&error, display_name))?;
        let canonical_archive = self
            .filesystem
            .canonicalize_path(archive_path)
            .map_err(|error| platform_error_message(&error, display_name))?;
        if !canonical_archive.starts_with(canonical_root) {
            return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
        }
        Ok(())
    }
}

fn archive_is_candidate(record: &ArchiveRecord, target: ArchivePatcherTarget) -> bool {
    record.enabled && target.selects_overview_version(record.version)
}

fn candidate_row(
    record: &ArchiveRecord,
    target: ArchivePatcherTarget,
) -> ArchivePatcherCandidateRow {
    ArchivePatcherCandidateRow::new(
        record.path.clone(),
        display_name_for_path(&record.path),
        record.format.clone(),
        record.version,
        target,
    )
}

fn compare_reference_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    let left_key = left.to_string_lossy().to_lowercase();
    let right_key = right.to_string_lossy().to_lowercase();
    left_key.cmp(&right_key).then_with(|| left.cmp(right))
}

fn display_name_for_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn normalize_filter(name_filter: Option<&str>) -> Option<String> {
    name_filter
        .filter(|filter| !filter.is_empty())
        .map(|filter| filter.to_lowercase())
}

fn basename_matches_filter(path: &Path, normalized_filter: Option<&str>) -> bool {
    let Some(filter) = normalized_filter else {
        return true;
    };
    display_name_for_path(path).to_lowercase().contains(filter)
}

fn contained_relative_path(
    data_root: Option<&Path>,
    archive_path: &Path,
) -> Result<PathBuf, String> {
    let Some(data_root) = data_root else {
        return Err(DATA_ROOT_MISSING_FAILURE_MESSAGE.to_owned());
    };
    if data_root.as_os_str().is_empty() || contains_unsafe_components(data_root) {
        return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
    }
    if archive_path.as_os_str().is_empty() || contains_unsafe_components(archive_path) {
        return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
    }
    if !archive_path.starts_with(data_root) {
        return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
    }
    let relative_path = archive_path
        .strip_prefix(data_root)
        .map_err(|_| ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned())?;
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || contains_unsafe_components(relative_path)
    {
        return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
    }
    Ok(relative_path.to_path_buf())
}

fn contains_unsafe_components(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
}

fn read_error_message(error: &PlatformError, display_name: &str) -> String {
    patch_platform_error_message(error, display_name)
}

fn patch_platform_error_message(error: &PlatformError, display_name: &str) -> String {
    match error.kind {
        PlatformErrorKind::NotFound => failed_patching_file_not_found_message(display_name),
        PlatformErrorKind::PermissionDenied => failed_patching_permissions_message(display_name),
        PlatformErrorKind::UnsupportedPlatform
        | PlatformErrorKind::InvalidInput
        | PlatformErrorKind::CommandFailed
        | PlatformErrorKind::ParseError
        | PlatformErrorKind::Io => failed_patching_unknown_os_message(display_name),
    }
}

fn restore_platform_error_message(error: &PlatformError, display_name: &str) -> String {
    match error.kind {
        PlatformErrorKind::NotFound => failed_restoring_file_not_found_message(display_name),
        PlatformErrorKind::PermissionDenied => failed_restoring_permissions_message(display_name),
        PlatformErrorKind::UnsupportedPlatform
        | PlatformErrorKind::InvalidInput
        | PlatformErrorKind::CommandFailed
        | PlatformErrorKind::ParseError
        | PlatformErrorKind::Io => failed_restoring_unknown_os_message(display_name),
    }
}

fn restore_archive_path(
    data_root: Option<&Path>,
    entry: &ArchivePatcherRestoreManifestEntry,
) -> Result<PathBuf, String> {
    let data_root = data_root.ok_or_else(|| DATA_ROOT_MISSING_FAILURE_MESSAGE.to_owned())?;
    let relative_path = &entry.data_relative_path;
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || contains_unsafe_components(relative_path)
    {
        return Err(ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE.to_owned());
    }
    Ok(data_root.join(relative_path))
}

fn execution_file_result(
    archive_path: PathBuf,
    file_name: String,
    outcome: ArchivePatcherExecutionOutcome,
    log_row: ArchivePatcherLogRow,
    diagnostics: Vec<String>,
) -> ArchivePatcherExecutionFileResult {
    ArchivePatcherExecutionFileResult {
        archive_path,
        file_name,
        outcome,
        log_row,
        diagnostics,
    }
}

fn failed_execution_result(
    archive_path: PathBuf,
    file_name: String,
    message: String,
) -> ArchivePatcherExecutionFileResult {
    execution_file_result(
        archive_path,
        file_name,
        ArchivePatcherExecutionOutcome::Failed,
        ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Bad, message),
        Vec::new(),
    )
}

fn restore_failed_result(
    archive_path: PathBuf,
    file_name: String,
    message: String,
) -> ArchivePatcherExecutionFileResult {
    execution_file_result(
        archive_path,
        file_name,
        ArchivePatcherExecutionOutcome::Failed,
        ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Bad, message),
        Vec::new(),
    )
}

fn parse_ba2_header(bytes: &[u8], display_name: &str) -> Result<ArchivePatcherHeader, String> {
    if bytes.len() < BA2_HEADER_PREFIX_LEN {
        return Err(short_header_message(display_name));
    }
    if bytes.get(0..4) != Some(BA2_MAGIC.as_slice()) {
        return Err(unrecognized_format_message(display_name));
    }

    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if !matches!(version, 1 | 7 | 8) {
        return Err(unrecognized_version_message(
            version_byte_hex(version),
            display_name,
        ));
    }

    let format = match &bytes[8..12] {
        marker if marker == BA2_FORMAT_GENERAL.as_slice() => ArchivePatcherArchiveFormat::General,
        marker if marker == BA2_FORMAT_DIRECTX10.as_slice() => {
            ArchivePatcherArchiveFormat::DirectX10
        }
        other => {
            return Err(unsupported_archive_format_message(
                format_marker_display(other),
                display_name,
            ));
        }
    };

    Ok(ArchivePatcherHeader::new(version, format))
}

fn version_byte_hex(version: u32) -> String {
    format!("{:02x}", version.to_le_bytes()[0])
}

fn format_marker_display(bytes: &[u8]) -> String {
    if bytes
        .iter()
        .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
    {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
    }
}

#[cfg(test)]
mod archive_patcher_service_plan {
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use crate::{
        domain::{
            archive_patcher::{
                ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE, ArchivePatcherLogLevel,
                ArchivePatcherPlanAction,
            },
            discovery::{ArchiveFormat, ArchiveVersion},
        },
        platform::{
            PlatformError, PlatformErrorKind, PlatformOperation, PlatformResult,
            filesystem::{DirectoryEntry, FileMetadata, FileType},
        },
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        UnreadableFile,
    }

    #[derive(Debug, Default, Clone)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
        prefix_reads: RefCell<Vec<(PathBuf, usize)>>,
        full_reads: RefCell<Vec<PathBuf>>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            self.nodes.insert(path.into(), FakeNode::Directory);
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
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata::new(FileType::File, bytes.len() as u64)),
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
                FakeNode::UnreadableFile => Ok(FileMetadata::new(FileType::File, 0)),
            }
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            self.full_reads.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.clone()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File read target could not be accessed because permission was denied.",
                )),
            }
        }

        fn read_prefix(&self, path: &Path, max_len: usize) -> PlatformResult<Vec<u8>> {
            self.prefix_reads
                .borrow_mut()
                .push((path.to_path_buf(), max_len));
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.iter().copied().take(max_len).collect()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File read target could not be accessed because permission was denied.",
                )),
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
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(Vec::new())
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(Vec::new())
        }
    }

    fn archive(
        path: impl Into<PathBuf>,
        format: ArchiveFormat,
        version: ArchiveVersion,
        enabled: bool,
    ) -> ArchiveRecord {
        ArchiveRecord::new(path, format, version, enabled)
    }

    fn header(version: u32, format: &[u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BTDX");
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(format);
        bytes.extend_from_slice(b"ignored body that must not be read");
        bytes
    }

    fn data_root() -> PathBuf {
        PathBuf::from("Game/Data")
    }

    #[test]
    fn archive_patcher_service_plan_selects_target_inversion_filters_and_sorted_candidates() {
        let records = vec![
            archive(
                "Game/Data/Zeta.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/alpha.ba2",
                ArchiveFormat::DirectX10,
                ArchiveVersion::NextGen7,
                true,
            ),
            archive(
                "Game/Data/disabled.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                false,
            ),
            archive(
                "Game/Data/old.ba2",
                ArchiveFormat::General,
                ArchiveVersion::OldGen,
                true,
            ),
        ];
        let fs = FakeFilesystem::default();
        let service = ArchivePatcherService::new(&fs);

        let all = service.candidate_snapshot(ArchivePatcherCandidateRequest::new(
            1,
            &records,
            ArchivePatcherTarget::OldGen,
            Some(""),
        ));
        assert_eq!(all.request_id, 1);
        assert_eq!(all.log_row.message, "Showing 2 files to be patched.");
        assert!(all.log_row.skip_file_logging);
        assert_eq!(
            all.rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.ba2", "Zeta.ba2"]
        );

        let filtered = service.candidate_snapshot(ArchivePatcherCandidateRequest::new(
            2,
            &records,
            ArchivePatcherTarget::OldGen,
            Some("ALPHA"),
        ));
        assert_eq!(filtered.name_filter.as_deref(), Some("alpha"));
        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0].display_name, "alpha.ba2");

        let next_gen = service.candidate_snapshot(ArchivePatcherCandidateRequest::new(
            3,
            &records,
            ArchivePatcherTarget::NextGen,
            None,
        ));
        assert_eq!(next_gen.rows.len(), 1);
        assert_eq!(next_gen.rows[0].display_name, "old.ba2");
    }

    #[test]
    fn archive_patcher_service_plan_empty_overview_data_is_safe_noop() {
        let fs = FakeFilesystem::default();
        let service = ArchivePatcherService::new(&fs);
        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            4,
            Some(&data_root()),
            &[],
            ArchivePatcherTarget::OldGen,
            None,
        ));

        assert_eq!(plan.request_id, 4);
        assert_eq!(plan.counts.candidate_rows, 0);
        assert_eq!(plan.counts.patchable_rows, 0);
        assert_eq!(plan.counts.failed_rows, 0);
        assert!(!plan.can_execute);
        assert_eq!(
            plan.candidates.log_row.message,
            "Showing 0 files to be patched."
        );
        assert_eq!(plan.summary_log_row.message, "Nothing to do!");
        assert!(plan.rows.is_empty());
        assert!(fs.prefix_reads.borrow().is_empty());
    }

    #[test]
    fn archive_patcher_service_plan_reads_bounded_header_and_builds_manifest_entry() {
        let records = vec![archive(
            "Game/Data/Fallout4 - Textures.ba2",
            ArchiveFormat::DirectX10,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_file("Game/Data/Fallout4 - Textures.ba2", header(8, b"DX10"));
        let service = ArchivePatcherService::new(&fs);

        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            5,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));

        assert!(plan.can_execute);
        assert_eq!(plan.counts.patchable_rows, 1);
        assert_eq!(plan.counts.failed_rows, 0);
        assert_eq!(
            fs.prefix_reads.borrow().as_slice(),
            &[(
                PathBuf::from("Game/Data/Fallout4 - Textures.ba2"),
                BA2_HEADER_PREFIX_LEN
            )]
        );
        assert!(
            fs.full_reads.borrow().is_empty(),
            "planner must not read full BA2 files"
        );
        let row = &plan.rows[0];
        assert_eq!(row.action, ArchivePatcherPlanAction::PatchVersionByte);
        assert_eq!(row.header.expect("header").version, 8);
        let entry = row
            .restore_manifest_entry
            .as_ref()
            .expect("manifest entry should exist");
        assert_eq!(
            entry.data_relative_path,
            PathBuf::from("Fallout4 - Textures.ba2")
        );
        assert_eq!(entry.original_version, 8);
        assert_eq!(entry.patched_version, 1);
        assert_eq!(entry.format, ArchivePatcherArchiveFormat::DirectX10);
    }

    #[test]
    fn archive_patcher_service_plan_records_bad_magic_version_format_and_short_headers() {
        let records = vec![
            archive(
                "Game/Data/Short.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Magic.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Version.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Format.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
        ];
        let fs = FakeFilesystem::default()
            .with_file("Game/Data/Short.ba2", b"BTDX")
            .with_file("Game/Data/Magic.ba2", b"XXXX\x08\0\0\0GNRL")
            .with_file("Game/Data/Version.ba2", header(9, b"GNRL"))
            .with_file("Game/Data/Format.ba2", header(8, b"XXXX"));
        let service = ArchivePatcherService::new(&fs);

        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            6,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));

        assert!(!plan.can_execute);
        assert_eq!(plan.counts.failed_rows, 4);
        let failures = plan
            .rows
            .iter()
            .map(|row| row.failure.as_deref().expect("failure"))
            .collect::<Vec<_>>();
        assert!(
            failures.contains(&"Archive header is shorter than the BA2 header length: Short.ba2")
        );
        assert!(failures.contains(&"Unrecognized format: Magic.ba2"));
        assert!(failures.contains(&"Unrecognized version [09]: Version.ba2"));
        assert!(failures.contains(&"Unrecognized archive format [XXXX]: Format.ba2"));
        assert!(plan.rows.iter().all(|row| !row.can_write()));
    }

    #[test]
    fn archive_patcher_service_plan_rejects_unreadable_uncontained_and_already_target_rows() {
        let records = vec![
            archive(
                "Game/Data/Locked.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Outside/Other.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Already.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
        ];
        let fs = FakeFilesystem::default()
            .with_unreadable_file("Game/Data/Locked.ba2")
            .with_file("Outside/Other.ba2", header(8, b"GNRL"))
            .with_file("Game/Data/Already.ba2", header(1, b"GNRL"));
        let service = ArchivePatcherService::new(&fs);

        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            7,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));

        assert_eq!(plan.counts.failed_rows, 3);
        let failures = plan
            .rows
            .iter()
            .map(|row| row.failure.as_deref().expect("failure"))
            .collect::<Vec<_>>();
        assert!(failures.contains(&"Failed patching (Permissions/In-Use): Locked.ba2"));
        assert!(failures.contains(&ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE));
        assert!(failures.contains(&"Skipping already-patched archive: Already.ba2"));
        assert!(
            !fs.prefix_reads
                .borrow()
                .iter()
                .any(|(path, _)| path == &PathBuf::from("Outside/Other.ba2")),
            "uncontained paths must fail before header reads"
        );
    }

    #[test]
    fn archive_patcher_service_plan_marks_stale_target_transition_as_failure() {
        let records = vec![archive(
            "Game/Data/Stale.ba2",
            ArchiveFormat::General,
            ArchiveVersion::OldGen,
            true,
        )];
        let fs = FakeFilesystem::default().with_file("Game/Data/Stale.ba2", header(7, b"GNRL"));
        let service = ArchivePatcherService::new(&fs);

        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            8,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::NextGen,
            None,
        ));

        assert_eq!(plan.rows.len(), 1);
        assert_eq!(
            plan.rows[0].failure.as_deref(),
            Some("Unrecognized version [07]: Stale.ba2")
        );
        assert!(!plan.rows[0].can_write());
    }

    #[test]
    fn archive_patcher_service_plan_digest_changes_with_path_version_and_target() {
        let records = vec![archive(
            "Game/Data/A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default().with_file("Game/Data/A.ba2", header(8, b"GNRL"));
        let service = ArchivePatcherService::new(&fs);
        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            9,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));
        let same = service.preview_plan(ArchivePatcherPlanRequest::new(
            10,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));
        assert_eq!(
            plan.stable_digest(),
            same.stable_digest(),
            "request ids are excluded"
        );

        let path_records = vec![archive(
            "Game/Data/B.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let path_fs = FakeFilesystem::default().with_file("Game/Data/B.ba2", header(8, b"GNRL"));
        let path_plan =
            ArchivePatcherService::new(&path_fs).preview_plan(ArchivePatcherPlanRequest::new(
                11,
                Some(&data_root()),
                &path_records,
                ArchivePatcherTarget::OldGen,
                None,
            ));
        assert_ne!(plan.stable_digest(), path_plan.stable_digest());

        let version_fs = FakeFilesystem::default().with_file("Game/Data/A.ba2", header(7, b"GNRL"));
        let version_plan =
            ArchivePatcherService::new(&version_fs).preview_plan(ArchivePatcherPlanRequest::new(
                12,
                Some(&data_root()),
                &records,
                ArchivePatcherTarget::OldGen,
                None,
            ));
        assert_ne!(plan.stable_digest(), version_plan.stable_digest());

        let target_records = vec![archive(
            "Game/Data/A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::OldGen,
            true,
        )];
        let target_fs = FakeFilesystem::default().with_file("Game/Data/A.ba2", header(1, b"GNRL"));
        let target_plan =
            ArchivePatcherService::new(&target_fs).preview_plan(ArchivePatcherPlanRequest::new(
                13,
                Some(&data_root()),
                &target_records,
                ArchivePatcherTarget::NextGen,
                None,
            ));
        assert_ne!(plan.stable_digest(), target_plan.stable_digest());
    }

    #[test]
    fn archive_patcher_service_plan_exposes_failure_log_rows() {
        let records = vec![archive(
            "Game/Data/Missing.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default().with_dir("Game/Data");
        let service = ArchivePatcherService::new(&fs);

        let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
            14,
            Some(&data_root()),
            &records,
            ArchivePatcherTarget::OldGen,
            None,
        ));

        let log = plan.rows[0].failure_log_row().expect("failure log row");
        assert_eq!(log.level, ArchivePatcherLogLevel::Bad);
        assert_eq!(log.message, "Failed patching (File Not Found): Missing.ba2");
    }
}

#[cfg(test)]
mod archive_patcher_executor {
    use std::{
        cell::RefCell,
        collections::{BTreeMap, BTreeSet},
        path::{Path, PathBuf},
    };

    use crate::{
        domain::{
            archive_patcher::{
                ArchivePatcherExecutionOutcome, ArchivePatcherLatestManifest,
                ArchivePatcherRestoreManifestEntry, BA2_VERSION_FIELD_OFFSET,
            },
            discovery::{ArchiveFormat, ArchiveVersion},
        },
        platform::{
            PlatformError, PlatformErrorKind, PlatformOperation, PlatformResult,
            filesystem::{DirectoryEntry, FileMetadata, FileType},
        },
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        ReadDenied(Vec<u8>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeWriteOp {
        WriteBytes(PathBuf),
        WriteByteRange(PathBuf, u64, Vec<u8>),
    }

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        nodes: RefCell<BTreeMap<PathBuf, FakeNode>>,
        write_failures: RefCell<BTreeSet<PathBuf>>,
        writes: RefCell<Vec<FakeWriteOp>>,
        full_reads: RefCell<Vec<PathBuf>>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            self.nodes
                .get_mut()
                .insert(path.into(), FakeNode::Directory);
            self
        }

        fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes
                .get_mut()
                .insert(path, FakeNode::File(bytes.into()));
            self
        }

        fn with_read_denied_file(
            mut self,
            path: impl Into<PathBuf>,
            bytes: impl Into<Vec<u8>>,
        ) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes
                .get_mut()
                .insert(path, FakeNode::ReadDenied(bytes.into()));
            self
        }

        fn with_write_failure(self, path: impl Into<PathBuf>) -> Self {
            self.write_failures.borrow_mut().insert(path.into());
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
            let nodes = self.nodes.get_mut();
            for parent in parents.into_iter().rev() {
                nodes.entry(parent).or_insert(FakeNode::Directory);
            }
        }

        fn node(&self, path: &Path, operation: PlatformOperation) -> PlatformResult<FakeNode> {
            self.nodes.borrow().get(path).cloned().ok_or_else(|| {
                PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )
            })
        }

        fn bytes(&self, path: impl AsRef<Path>) -> Vec<u8> {
            match self
                .nodes
                .borrow()
                .get(path.as_ref())
                .expect("test file should exist")
            {
                FakeNode::File(bytes) | FakeNode::ReadDenied(bytes) => bytes.clone(),
                FakeNode::Directory => panic!("test path should be a file"),
            }
        }

        fn version(&self, path: impl AsRef<Path>) -> u32 {
            let bytes = self.bytes(path);
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
        }

        fn write_ops(&self) -> Vec<FakeWriteOp> {
            self.writes.borrow().clone()
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) | FakeNode::ReadDenied(bytes) => {
                    Ok(FileMetadata::new(FileType::File, bytes.len() as u64))
                }
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
            }
        }

        fn symlink_metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            self.metadata(path)
        }

        fn canonicalize_path(&self, path: &Path) -> PlatformResult<PathBuf> {
            self.metadata(path)?;
            Ok(path.to_path_buf())
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            self.full_reads.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes),
                FakeNode::ReadDenied(_) => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File read target could not be accessed because permission was denied.",
                )),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
            }
        }

        fn read_prefix(&self, path: &Path, max_len: usize) -> PlatformResult<Vec<u8>> {
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.iter().copied().take(max_len).collect()),
                FakeNode::ReadDenied(_) => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File read target could not be accessed because permission was denied.",
                )),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
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
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(Vec::new())
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(Vec::new())
        }
    }

    impl WritableFilesystem for FakeFilesystem {
        fn write_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
            if self.write_failures.borrow().contains(path) {
                return Err(PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File write target could not be accessed because permission was denied.",
                ));
            }
            self.nodes
                .borrow_mut()
                .insert(path.to_path_buf(), FakeNode::File(bytes.to_vec()));
            self.writes
                .borrow_mut()
                .push(FakeWriteOp::WriteBytes(path.to_path_buf()));
            Ok(())
        }

        fn write_byte_range(&self, path: &Path, offset: u64, bytes: &[u8]) -> PlatformResult<()> {
            if self.write_failures.borrow().contains(path) {
                return Err(PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File write target could not be accessed because permission was denied.",
                ));
            }
            let mut nodes = self.nodes.borrow_mut();
            let node = nodes.get_mut(path).ok_or_else(|| {
                PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File write target was not found.",
                )
            })?;
            match node {
                FakeNode::File(existing) => {
                    let start = usize::try_from(offset).map_err(|_| {
                        PlatformError::new(
                            PlatformOperation::WriteFile,
                            path.display().to_string(),
                            PlatformErrorKind::InvalidInput,
                            "File write target is invalid.",
                        )
                    })?;
                    let end = start.checked_add(bytes.len()).ok_or_else(|| {
                        PlatformError::new(
                            PlatformOperation::WriteFile,
                            path.display().to_string(),
                            PlatformErrorKind::InvalidInput,
                            "File write target is invalid.",
                        )
                    })?;
                    if end > existing.len() {
                        return Err(PlatformError::new(
                            PlatformOperation::WriteFile,
                            path.display().to_string(),
                            PlatformErrorKind::InvalidInput,
                            "File write target is invalid.",
                        ));
                    }
                    existing[start..end].copy_from_slice(bytes);
                    self.writes.borrow_mut().push(FakeWriteOp::WriteByteRange(
                        path.to_path_buf(),
                        offset,
                        bytes.to_vec(),
                    ));
                    Ok(())
                }
                FakeNode::ReadDenied(_) => Err(PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    "File write target could not be accessed because permission was denied.",
                )),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File write target is invalid.",
                )),
            }
        }

        fn replace_file_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
            self.write_bytes(path, bytes)
        }

        fn copy_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
            let bytes = self.read_bytes(from)?;
            self.write_bytes(to, &bytes)
        }

        fn rename_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
            let node = self.nodes.borrow_mut().remove(from).ok_or_else(|| {
                PlatformError::new(
                    PlatformOperation::RenameFile,
                    from.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File rename target was not found.",
                )
            })?;
            self.nodes.borrow_mut().insert(to.to_path_buf(), node);
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> PlatformResult<()> {
            self.nodes.borrow_mut().remove(path).ok_or_else(|| {
                PlatformError::new(
                    PlatformOperation::RemoveFile,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File removal target was not found.",
                )
            })?;
            Ok(())
        }
    }

    fn archive(
        path: impl Into<PathBuf>,
        format: ArchiveFormat,
        version: ArchiveVersion,
        enabled: bool,
    ) -> ArchiveRecord {
        ArchiveRecord::new(path, format, version, enabled)
    }

    fn header(version: u32, format: &[u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BTDX");
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(format);
        bytes.extend_from_slice(b"body bytes must survive");
        bytes
    }

    fn bad_magic_header() -> Vec<u8> {
        let mut bytes = header(8, b"GNRL");
        bytes[0..4].copy_from_slice(b"XXXX");
        bytes
    }

    fn data_root() -> PathBuf {
        PathBuf::from("Game/Data")
    }

    fn manifest_path() -> PathBuf {
        PathBuf::from("App/archive-patcher-latest.json")
    }

    fn patch_digest(
        fs: &FakeFilesystem,
        records: &[ArchiveRecord],
        target: ArchivePatcherTarget,
    ) -> String {
        ArchivePatcherService::new(fs)
            .preview_plan(ArchivePatcherPlanRequest::new(
                1,
                Some(&data_root()),
                records,
                target,
                None,
            ))
            .stable_digest()
    }

    #[test]
    fn archive_patcher_executor_patches_v7_and_v8_to_v1_after_manifest_write() {
        let records = vec![
            archive(
                "Game/Data/A.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen7,
                true,
            ),
            archive(
                "Game/Data/B.ba2",
                ArchiveFormat::DirectX10,
                ArchiveVersion::NextGen8,
                true,
            ),
        ];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/A.ba2", header(7, b"GNRL"))
            .with_file("Game/Data/B.ba2", header(8, b"DX10"));
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::OldGen);

        let result = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    2,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect("confirmed patch should succeed");

        assert_eq!(result.counts.patched, 2);
        assert_eq!(result.counts.failed, 0);
        assert_eq!(fs.version("Game/Data/A.ba2"), 1);
        assert_eq!(fs.version("Game/Data/B.ba2"), 1);
        assert_eq!(&fs.bytes("Game/Data/A.ba2")[8..12], b"GNRL");
        assert_eq!(&fs.bytes("Game/Data/B.ba2")[8..12], b"DX10");
        assert_eq!(
            fs.write_ops(),
            vec![
                FakeWriteOp::WriteBytes(manifest_path()),
                FakeWriteOp::WriteByteRange(
                    PathBuf::from("Game/Data/A.ba2"),
                    BA2_VERSION_FIELD_OFFSET,
                    1_u32.to_le_bytes().to_vec(),
                ),
                FakeWriteOp::WriteByteRange(
                    PathBuf::from("Game/Data/B.ba2"),
                    BA2_VERSION_FIELD_OFFSET,
                    1_u32.to_le_bytes().to_vec(),
                ),
            ],
            "latest manifest must be written before archive headers"
        );
        assert_eq!(
            result.log_rows.last().map(|row| row.message.as_str()),
            Some("Patching complete. 2 Successful, 0 Failed.")
        );
    }

    #[test]
    fn archive_patcher_executor_patches_v1_to_v8() {
        let records = vec![archive(
            "Game/Data/Old.ba2",
            ArchiveFormat::General,
            ArchiveVersion::OldGen,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/Old.ba2", header(1, b"GNRL"));
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::NextGen);

        let result = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    3,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::NextGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect("confirmed patch should succeed");

        assert_eq!(result.counts.patched, 1);
        assert_eq!(fs.version("Game/Data/Old.ba2"), 8);
        assert_eq!(result.rows[0].log_row.message, "Patched to v8: Old.ba2");
    }

    #[test]
    fn archive_patcher_executor_logs_validation_failures_and_partial_success() {
        let records = vec![
            archive(
                "Game/Data/Already.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/BadMagic.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Missing.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Unknown.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
            archive(
                "Game/Data/Valid.ba2",
                ArchiveFormat::General,
                ArchiveVersion::NextGen8,
                true,
            ),
        ];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/Already.ba2", header(1, b"GNRL"))
            .with_file("Game/Data/BadMagic.ba2", bad_magic_header())
            .with_file("Game/Data/Unknown.ba2", header(9, b"GNRL"))
            .with_file("Game/Data/Valid.ba2", header(8, b"GNRL"));
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::OldGen);

        let result = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    4,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect("partial patch should still return a result");

        let messages = result
            .log_rows
            .iter()
            .map(|row| row.message.as_str())
            .collect::<Vec<_>>();
        assert!(messages.contains(&"Skipping already-patched archive: Already.ba2"));
        assert!(messages.contains(&"Unrecognized format: BadMagic.ba2"));
        assert!(messages.contains(&"Failed patching (File Not Found): Missing.ba2"));
        assert!(messages.contains(&"Unrecognized version [09]: Unknown.ba2"));
        assert!(messages.contains(&"Patched to v1: Valid.ba2"));
        assert!(messages.contains(&"Patching complete. 1 Successful, 4 Failed."));
        assert_eq!(result.counts.patched, 1);
        assert_eq!(result.counts.failed, 4);
        assert_eq!(fs.version("Game/Data/Valid.ba2"), 1);
    }

    #[test]
    fn archive_patcher_executor_maps_permission_write_failure_per_file() {
        let records = vec![archive(
            "Game/Data/Locked.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/Locked.ba2", header(8, b"GNRL"))
            .with_write_failure("Game/Data/Locked.ba2");
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::OldGen);

        let result = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    5,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect("write failure should be per-file");

        assert_eq!(result.counts.patched, 0);
        assert_eq!(result.counts.failed, 1);
        assert_eq!(fs.version("Game/Data/Locked.ba2"), 8);
        assert_eq!(
            result.rows[0].log_row.message,
            "Failed patching (Permissions/In-Use): Locked.ba2"
        );
    }

    #[test]
    fn archive_patcher_executor_aborts_before_mutation_when_manifest_write_fails() {
        let records = vec![archive(
            "Game/Data/A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/A.ba2", header(8, b"GNRL"))
            .with_write_failure(manifest_path());
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::OldGen);

        let error = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    6,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect_err("manifest failure should abort the run");

        assert!(matches!(
            error,
            ArchivePatcherExecutionError::ManifestWriteFailed { .. }
        ));
        assert_eq!(fs.version("Game/Data/A.ba2"), 8);
        assert!(
            !fs.write_ops()
                .iter()
                .any(|op| matches!(op, FakeWriteOp::WriteByteRange(_, _, _))),
            "archive bytes must not be touched when manifest write fails"
        );
    }

    #[test]
    fn archive_patcher_executor_aborts_before_manifest_when_digest_mismatches() {
        let records = vec![archive(
            "Game/Data/A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/A.ba2", header(8, b"GNRL"));

        let error = ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    7,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest("stale-digest"),
            )
            .expect_err("digest mismatch should abort the run");

        assert!(matches!(
            error,
            ArchivePatcherExecutionError::ConfirmedPlanChanged { .. }
        ));
        assert_eq!(fs.version("Game/Data/A.ba2"), 8);
        assert!(fs.write_ops().is_empty());
    }

    #[test]
    fn archive_patcher_executor_restores_last_run_from_manifest() {
        let records = vec![archive(
            "Game/Data/A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/A.ba2", header(8, b"GNRL"));
        let digest = patch_digest(&fs, &records, ArchivePatcherTarget::OldGen);
        ArchivePatcherService::new(&fs)
            .execute_confirmed(
                ArchivePatcherExecutionRequest::new(
                    8,
                    Some(&data_root()),
                    &records,
                    ArchivePatcherTarget::OldGen,
                    None,
                    &manifest_path(),
                )
                .with_confirmed_plan_digest(&digest),
            )
            .expect("patch should write manifest");
        assert_eq!(fs.version("Game/Data/A.ba2"), 1);

        let result = ArchivePatcherService::new(&fs)
            .restore_last_run(ArchivePatcherRestoreRequest::new(
                9,
                Some(&data_root()),
                &manifest_path(),
            ))
            .expect("restore should succeed");

        assert_eq!(result.counts.restored, 1);
        assert_eq!(result.counts.skipped, 0);
        assert_eq!(result.counts.failed, 0);
        assert_eq!(fs.version("Game/Data/A.ba2"), 8);
        assert_eq!(
            result.rows[0].outcome,
            ArchivePatcherExecutionOutcome::Restored
        );
        assert_eq!(result.rows[0].log_row.message, "Restored to v8: A.ba2");
    }

    #[test]
    fn archive_patcher_executor_restore_skips_stale_files_without_writing() {
        let entry = ArchivePatcherRestoreManifestEntry::new(
            "Game/Data/Stale.ba2",
            "Stale.ba2",
            "Stale.ba2",
            ArchivePatcherArchiveFormat::General,
            8,
            1,
        );
        let manifest =
            ArchivePatcherLatestManifest::new("digest", ArchivePatcherTarget::OldGen, vec![entry]);
        let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest should serialize");
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_dir("App")
            .with_file("Game/Data/Stale.ba2", header(7, b"GNRL"))
            .with_file(manifest_path(), manifest_bytes);

        let result = ArchivePatcherService::new(&fs)
            .restore_last_run(ArchivePatcherRestoreRequest::new(
                10,
                Some(&data_root()),
                &manifest_path(),
            ))
            .expect("stale restore should return a safe result");

        assert_eq!(result.counts.restored, 0);
        assert_eq!(result.counts.skipped, 1);
        assert_eq!(result.counts.failed, 0);
        assert_eq!(fs.version("Game/Data/Stale.ba2"), 7);
        assert_eq!(
            result.rows[0].outcome,
            ArchivePatcherExecutionOutcome::Skipped
        );
        assert_eq!(
            result.rows[0].log_row.message,
            "Skipping restore (Archive changed): Stale.ba2"
        );
        assert!(
            !fs.write_ops()
                .iter()
                .any(|op| matches!(op, FakeWriteOp::WriteByteRange(_, _, _))),
            "stale restore must not write archive bytes"
        );
    }

    #[test]
    fn archive_patcher_executor_read_permission_failure_is_visible() {
        let records = vec![archive(
            "Game/Data/Private.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            true,
        )];
        let fs = FakeFilesystem::default()
            .with_dir("Game/Data")
            .with_read_denied_file("Game/Data/Private.ba2", header(8, b"GNRL"));

        let result = ArchivePatcherService::new(&fs)
            .execute_confirmed(ArchivePatcherExecutionRequest::new(
                11,
                Some(&data_root()),
                &records,
                ArchivePatcherTarget::OldGen,
                None,
                &manifest_path(),
            ))
            .expect("read permission failure should be per-file");

        assert_eq!(result.counts.patched, 0);
        assert_eq!(result.counts.failed, 1);
        assert_eq!(
            result.rows[0].log_row.message,
            "Failed patching (Permissions/In-Use): Private.ba2"
        );
        assert!(fs.write_ops().is_empty());
    }
}
