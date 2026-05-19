//! Read-only Archive Patcher candidate and preview-plan service.
//!
//! The Python reference selects BA2 files from Overview's enabled archive sets,
//! previews them in sorted order, and only mutates when the user presses
//! `Patch All`. This service preserves that split: it consumes already-collected
//! [`ArchiveRecord`] values, applies the target/filter rules without touching
//! Slint, reads only bounded BA2 header prefixes through [`Filesystem::read_prefix`],
//! and returns fail-closed plan rows for a later confirmed worker.

use std::path::{Component, Path, PathBuf};

use tracing::{debug, info, info_span, warn};

use crate::{
    domain::{
        archive_patcher::{
            ARCHIVE_PATH_CONTAINMENT_FAILURE_MESSAGE, ArchivePatcherArchiveFormat,
            ArchivePatcherCandidateRow, ArchivePatcherCandidateSnapshot, ArchivePatcherHeader,
            ArchivePatcherPreviewPlan, ArchivePatcherPreviewPlanRow,
            ArchivePatcherRestoreManifestEntry, ArchivePatcherTarget, BA2_FORMAT_DIRECTX10,
            BA2_FORMAT_GENERAL, BA2_HEADER_PREFIX_LEN, BA2_MAGIC,
            DATA_ROOT_MISSING_FAILURE_MESSAGE, failed_patching_file_not_found_message,
            failed_patching_permissions_message, failed_patching_unknown_os_message,
            short_header_message, skipping_already_patched_message, unrecognized_format_message,
            unrecognized_version_message, unsupported_archive_format_message,
        },
        discovery::ArchiveRecord,
    },
    platform::{PlatformError, PlatformErrorKind, filesystem::Filesystem},
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

        let header = match self
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

        let header = match parse_ba2_header(&header, &display_name) {
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
        );
        ArchivePatcherPreviewPlanRow::patch(candidate, header, manifest_entry)
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
