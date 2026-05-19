//! Read-only Downgrader status and preview-plan service.
//!
//! The Python reference computes file CRCs when the modal opens and performs all
//! destructive backup/restore/xdelta work only after the `Patch All` action is
//! confirmed. This service preserves that split: it depends only on the
//! read-only [`Filesystem`] trait, validates every managed path under the
//! discovered Fallout 4 root, and returns Slint-free status/plan payloads that a
//! later worker can execute through separate mutation/download/apply seams.

use std::{
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, info, info_span, warn};

use crate::{
    domain::{
        discovery::{FALLOUT4_NOT_FOUND_MESSAGE, Fallout4Installation},
        downgrader::{
            DOWNGRADER_FILE_DEFINITIONS, DowngraderExecutionLogRow, DowngraderFileDefinition,
            DowngraderFileGroup, DowngraderInstallStatus, DowngraderOptionsSnapshot,
            DowngraderPlanAction, DowngraderPlanRow, DowngraderPlanStep, DowngraderPlanStepKind,
            DowngraderProgress, DowngraderStatusRow, DowngraderTarget,
            accepted_source_crcs_for_target, crcs_for_status, failed_patching_log_row,
            patched_log_row, skipped_already_message, skipped_not_found_message,
            skipped_unsupported_message,
        },
    },
    platform::{
        PlatformErrorKind,
        filesystem::{FileMetadata, Filesystem, WritableFilesystem},
    },
};

const UNSAFE_ROOT_MESSAGE: &str = "Fallout 4 installation path is unsafe.";
const UNSAFE_MANAGED_PATH_MESSAGE: &str = "Downgrader managed file path is unsafe.";
const ROOT_NOT_DIRECTORY_MESSAGE: &str = "Fallout 4 installation path is not a folder.";
const CURRENT_FILE_READ_FAILURE_MESSAGE: &str =
    "Current file could not be read; patching is disabled for this file.";
const BACKUP_READ_FAILURE_MESSAGE: &str =
    "Backup file could not be read; patching is disabled for this file.";
const DOWNLOAD_FAILURE_MESSAGE: &str = "Delta patch could not be downloaded.";
const APPLY_FAILURE_MESSAGE: &str = "Delta patch could not be applied.";
const PATCH_INTEGRITY_FAILURE_MESSAGE: &str = "Delta patch failed integrity verification.";
const OUTPUT_INTEGRITY_FAILURE_MESSAGE: &str = "Patched output failed integrity verification.";
const MAX_DELTA_PATCH_BYTES: u64 = 128 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_CHUNK_BYTES: usize = 64 * 1024;
const MIN_PROGRESS_DELTA_PERCENT: f32 = 1.0;
/// Safe message shown when the confirmed filesystem state no longer matches the reviewed plan.
pub const CONFIRMED_PLAN_CHANGED_MESSAGE: &str =
    "Downgrader files changed after preview. Refresh the plan and try again.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DowngraderPatchIntegrity {
    patch_name: &'static str,
    expected_patch_bytes: u64,
    sha256_hex: &'static str,
    expected_output_bytes: u64,
}

// SHA-256 pins were computed from the public CMT `delta-patches` GitHub release
// assets. Output sizes are parsed from the VCDIFF target-window sizes and are
// used to bound decompression before any active file is replaced.
const PATCH_INTEGRITY_MANIFEST: [DowngraderPatchIntegrity; 12] = [
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-Archive2.exe.xdelta",
        expected_patch_bytes: 23_825,
        sha256_hex: "59e6598f4603c48103aa051eda156914ecd640263cd0f6c01f8bf4a284ba61db",
        expected_output_bytes: 63_488,
    },
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-Archive2Interop.dll.xdelta",
        expected_patch_bytes: 168_265,
        sha256_hex: "43c10dea8dc87985da20a93ac5088620245fa085415ddf87f3b1e9f5ed55aed7",
        expected_output_bytes: 513_024,
    },
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-CreationKit.exe.xdelta",
        expected_patch_bytes: 65_857_566,
        sha256_hex: "0a873f7f07a86de955994343f7dc6455c7392c83612cc01674836de8c9bdf65e",
        expected_output_bytes: 80_361_352,
    },
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-Fallout4.exe.xdelta",
        expected_patch_bytes: 53_072_487,
        sha256_hex: "ede24453c08e2ffa0dc0f73f7f7fae3b5434a02befeb3edc692f958ec2b6beba",
        expected_output_bytes: 65_503_104,
    },
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-Fallout4Launcher.exe.xdelta",
        expected_patch_bytes: 94_023,
        sha256_hex: "e5289695f96b4ada4f30aef7d706d14ddc91ae897181b0f4b46e3674179c1118",
        expected_output_bytes: 4_522_496,
    },
    DowngraderPatchIntegrity {
        patch_name: "NG-to-OG-steam_api64.dll.xdelta",
        expected_patch_bytes: 99_130,
        sha256_hex: "01de64945b9263c69bc0a82dda0dad153e39c27954d965d58ff92a9b09c65e91",
        expected_output_bytes: 206_760,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-Archive2.exe.xdelta",
        expected_patch_bytes: 23_679,
        sha256_hex: "9f1d74aefbec81fbda47116c58408e58c122ca58559b41a3a883364eda8695b6",
        expected_output_bytes: 62_976,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-Archive2Interop.dll.xdelta",
        expected_patch_bytes: 158_699,
        sha256_hex: "578930ad7a470e05ec55e98c88286b8bfdb22ba1a246f9f703fc01150f75279c",
        expected_output_bytes: 470_016,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-CreationKit.exe.xdelta",
        expected_patch_bytes: 27_441_440,
        sha256_hex: "acc49129151df43ae556b655b1f8eebf633ed34565d53f4dd25bda1afa8f2103",
        expected_output_bytes: 68_193_792,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-Fallout4.exe.xdelta",
        expected_patch_bytes: 42_028_246,
        sha256_hex: "224687af80a61e987798258cd37ac0bffbbf3935dc9b0a0ef2626eaa7eeac29c",
        expected_output_bytes: 52_552_472,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-Fallout4Launcher.exe.xdelta",
        expected_patch_bytes: 81_597,
        sha256_hex: "56b00b2e40a5051c71d0851484c130a889d3af99b3a2316b627257296688755e",
        expected_output_bytes: 4_520_448,
    },
    DowngraderPatchIntegrity {
        patch_name: "OG-to-NG-steam_api64.dll.xdelta",
        expected_patch_bytes: 154_573,
        sha256_hex: "533ad4d6894a551a5997d4a9674195baf2a8c88619dbcb1fe80014aa068a380c",
        expected_output_bytes: 298_384,
    },
];

/// Request input for a read-only Downgrader status snapshot.
#[derive(Debug, Clone, Copy)]
pub struct DowngraderStatusRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional discovered Fallout 4 installation.
    pub installation: Option<&'a Fallout4Installation>,
}

impl<'a> DowngraderStatusRequest<'a> {
    /// Creates a status request from already-discovered installation facts.
    pub const fn new(request_id: u64, installation: Option<&'a Fallout4Installation>) -> Self {
        Self {
            request_id,
            installation,
        }
    }
}

/// Request input for a read-only Downgrader inline preview plan.
#[derive(Debug, Clone, Copy)]
pub struct DowngraderPlanRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional discovered Fallout 4 installation.
    pub installation: Option<&'a Fallout4Installation>,
    /// User-selected target and cleanup preferences.
    pub options: DowngraderOptionsSnapshot,
}

impl<'a> DowngraderPlanRequest<'a> {
    /// Creates a plan request from already-discovered installation facts and options.
    pub const fn new(
        request_id: u64,
        installation: Option<&'a Fallout4Installation>,
        options: DowngraderOptionsSnapshot,
    ) -> Self {
        Self {
            request_id,
            installation,
            options,
        }
    }
}

/// Safe failure returned before any status or plan output can be trusted.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DowngraderServiceError {
    /// No discovered game root was supplied or the supplied root is absent.
    #[error("{safe_message}")]
    MissingGameRoot {
        /// User-facing safe failure text.
        safe_message: String,
    },
    /// The discovered game root was malformed, inaccessible, or not a directory.
    #[error("{safe_message}")]
    InvalidGameRoot {
        /// Rejected root path retained for diagnostics/tests.
        root: PathBuf,
        /// User-facing safe failure text.
        safe_message: String,
    },
    /// A confirmed run observed file or backup state that no longer matches the preview plan.
    #[error("{safe_message}")]
    ConfirmedPlanChanged {
        /// User-facing safe failure text.
        safe_message: String,
        /// Stable digest captured when the user reviewed the plan.
        expected_digest: String,
        /// Stable digest built immediately before execution.
        actual_digest: String,
    },
    /// A managed relative path was malformed or escaped the game root.
    #[error("{safe_message}")]
    UnsafeManagedPath {
        /// Malformed relative path retained for diagnostics/tests.
        relative_path: String,
        /// User-facing safe failure text.
        safe_message: String,
    },
}

impl DowngraderServiceError {
    /// Returns the safe text suitable for modal logs or disabled-state banners.
    pub fn user_message(&self) -> &str {
        match self {
            Self::MissingGameRoot { safe_message }
            | Self::InvalidGameRoot { safe_message, .. }
            | Self::ConfirmedPlanChanged { safe_message, .. }
            | Self::UnsafeManagedPath { safe_message, .. } => safe_message,
        }
    }
}

/// Safe failure returned by a delta downloader before a patch can be applied.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DowngraderDownloadError {
    /// HTTP client setup, request, stream, or status failure.
    #[error("{safe_message}")]
    Request {
        /// User-safe failure text.
        safe_message: String,
        /// Diagnostic detail for tracing/tests.
        diagnostic: String,
    },
    /// The response advertised or exceeded the configured patch byte cap.
    #[error("{safe_message}")]
    TooLarge {
        /// User-safe failure text.
        safe_message: String,
        /// Maximum bytes accepted by the production downloader.
        limit_bytes: u64,
    },
}

impl DowngraderDownloadError {
    /// Creates a safe request/download failure with diagnostic detail separated.
    pub fn request(diagnostic: impl Into<String>) -> Self {
        Self::Request {
            safe_message: DOWNLOAD_FAILURE_MESSAGE.to_owned(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Creates a safe patch-size failure.
    pub fn too_large(limit_bytes: u64) -> Self {
        Self::TooLarge {
            safe_message: DOWNLOAD_FAILURE_MESSAGE.to_owned(),
            limit_bytes,
        }
    }

    /// Returns safe text suitable for the modal log fallback.
    pub fn user_message(&self) -> &str {
        match self {
            Self::Request { safe_message, .. } | Self::TooLarge { safe_message, .. } => {
                safe_message
            }
        }
    }

    /// Returns diagnostic detail suitable for logs/tests.
    pub fn diagnostic(&self) -> Option<&str> {
        match self {
            Self::Request { diagnostic, .. } => Some(diagnostic),
            Self::TooLarge { .. } => Some("delta patch exceeds configured size limit"),
        }
    }
}

/// Fakeable seam for fetching one xdelta patch asset.
pub trait DeltaDownloader {
    /// Downloads a delta patch and reports bounded progress percentages.
    fn download_delta(
        &self,
        url: &str,
        progress: &mut dyn FnMut(DowngraderProgress),
    ) -> Result<Vec<u8>, DowngraderDownloadError>;
}

/// Reqwest-backed delta downloader intended to run on a background worker thread.
#[derive(Clone)]
pub struct ReqwestDeltaDownloader {
    client: reqwest::blocking::Client,
    max_bytes: u64,
}

impl ReqwestDeltaDownloader {
    /// Creates a downloader with the reference 10 second timeout and a bounded patch size.
    pub fn new() -> Result<Self, DowngraderDownloadError> {
        Self::with_limits(DOWNLOAD_TIMEOUT, MAX_DELTA_PATCH_BYTES)
    }

    /// Creates a downloader with explicit timeout and byte cap for tests or alternate workers.
    pub fn with_limits(timeout: Duration, max_bytes: u64) -> Result<Self, DowngraderDownloadError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| DowngraderDownloadError::request(error.to_string()))?;
        Ok(Self { client, max_bytes })
    }
}

impl DeltaDownloader for ReqwestDeltaDownloader {
    fn download_delta(
        &self,
        url: &str,
        progress: &mut dyn FnMut(DowngraderProgress),
    ) -> Result<Vec<u8>, DowngraderDownloadError> {
        progress(DowngraderProgress::idle());
        let mut response = self
            .client
            .get(url)
            .send()
            .map_err(|error| DowngraderDownloadError::request(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(DowngraderDownloadError::request(format!(
                "HTTP status {status}"
            )));
        }

        let content_length = response.content_length();
        if content_length.is_some_and(|length| length > self.max_bytes) {
            return Err(DowngraderDownloadError::too_large(self.max_bytes));
        }

        let initial_capacity = content_length
            .unwrap_or(0)
            .min(self.max_bytes)
            .min(usize::MAX as u64) as usize;
        let mut bytes = Vec::with_capacity(initial_capacity);
        let mut buffer = vec![0_u8; DOWNLOAD_CHUNK_BYTES];
        let mut downloaded = 0_u64;
        let mut last_progress = 0.0_f32;

        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| DowngraderDownloadError::request(error.to_string()))?;
            if read == 0 {
                break;
            }
            downloaded += read as u64;
            if downloaded > self.max_bytes {
                return Err(DowngraderDownloadError::too_large(self.max_bytes));
            }
            bytes.extend_from_slice(&buffer[..read]);

            if let Some(total) = content_length.filter(|total| *total > 0) {
                let percent = (downloaded as f32 / total as f32) * 100.0;
                if percent >= 100.0 || percent - last_progress >= MIN_PROGRESS_DELTA_PERCENT {
                    let progress_value = DowngraderProgress::new(percent);
                    last_progress = progress_value.percent;
                    progress(progress_value);
                }
            }
        }

        progress(DowngraderProgress::complete());
        Ok(bytes)
    }
}

/// Safe failure returned when a delta patch cannot transform source bytes.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DowngraderDeltaApplyError {
    /// The patch decoder rejected the payload or failed to produce output.
    #[error("{safe_message}")]
    Failed {
        /// User-safe failure text.
        safe_message: String,
        /// Diagnostic detail for tracing/tests.
        diagnostic: String,
    },
}

impl DowngraderDeltaApplyError {
    /// Creates a safe apply failure with diagnostic detail separated.
    pub fn failed(diagnostic: impl Into<String>) -> Self {
        Self::Failed {
            safe_message: APPLY_FAILURE_MESSAGE.to_owned(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Returns safe text suitable for modal fallback logs.
    pub fn user_message(&self) -> &str {
        match self {
            Self::Failed { safe_message, .. } => safe_message,
        }
    }

    /// Returns diagnostic detail suitable for logs/tests.
    pub fn diagnostic(&self) -> &str {
        match self {
            Self::Failed { diagnostic, .. } => diagnostic,
        }
    }
}

/// Fakeable seam for applying an xdelta/VCDIFF patch to source bytes.
pub trait DeltaApplier {
    /// Applies `patch_bytes` to `source_bytes` and returns the desired-version bytes.
    fn apply_delta(
        &self,
        source_bytes: &[u8],
        patch_bytes: &[u8],
        expected_output_bytes: u64,
    ) -> Result<Vec<u8>, DowngraderDeltaApplyError>;
}

/// Pure-Rust VCDIFF applier for xdelta-compatible patch payloads.
#[derive(Debug, Default, Clone, Copy)]
pub struct VcdiffDeltaApplier;

impl DeltaApplier for VcdiffDeltaApplier {
    fn apply_delta(
        &self,
        source_bytes: &[u8],
        patch_bytes: &[u8],
        expected_output_bytes: u64,
    ) -> Result<Vec<u8>, DowngraderDeltaApplyError> {
        let max_output_bytes = usize::try_from(expected_output_bytes).map_err(|_| {
            DowngraderDeltaApplyError::failed("expected output size does not fit this platform")
        })?;
        let mut source = Cursor::new(source_bytes.to_vec());
        let mut patch = Cursor::new(patch_bytes.to_vec());
        let mut output = BoundedVecWriter::new(max_output_bytes);
        vcdiff_decoder::apply_patch(&mut patch, Some(&mut source), &mut output)
            .map_err(|error| DowngraderDeltaApplyError::failed(error.to_string()))?;
        Ok(output.into_inner())
    }
}

struct BoundedVecWriter {
    bytes: Vec<u8>,
    max_len: usize,
}

impl BoundedVecWriter {
    fn new(max_len: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(max_len.min(1024 * 1024)),
            max_len,
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedVecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.bytes.len().saturating_add(buf.len()) > self.max_len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VCDIFF output exceeded expected target size",
            ));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Request input for a confirmed Downgrader run after inline user confirmation.
#[derive(Debug, Clone)]
pub struct DowngraderExecutionRequest<'a> {
    /// Monotonic request id assigned by the caller for stale-event rejection and tracing.
    pub request_id: u64,
    /// Optional discovered Fallout 4 installation.
    pub installation: Option<&'a Fallout4Installation>,
    /// Confirmed target and cleanup preferences.
    pub options: DowngraderOptionsSnapshot,
    /// Stable digest of the inline preview that the user reviewed before confirming.
    pub confirmed_plan_digest: Option<String>,
}

impl<'a> DowngraderExecutionRequest<'a> {
    /// Creates a confirmed execution request from discovery facts and options.
    pub const fn new(
        request_id: u64,
        installation: Option<&'a Fallout4Installation>,
        options: DowngraderOptionsSnapshot,
    ) -> Self {
        Self {
            request_id,
            installation,
            options,
            confirmed_plan_digest: None,
        }
    }

    /// Binds execution to the exact read-only preview plan the user confirmed.
    pub fn with_confirmed_plan_digest(mut self, digest: impl Into<String>) -> Self {
        self.confirmed_plan_digest = Some(digest.into());
        self
    }
}

/// Final per-file outcome for a confirmed Downgrader run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderExecutionOutcome {
    /// File was intentionally skipped with a reference informational row.
    Skipped,
    /// File was restored or patched successfully.
    Patched,
    /// File failed safely and execution continued with later files.
    Failed,
}

/// Bounded progress event emitted while downloading a patch for one file.
#[derive(Debug, Clone, PartialEq)]
pub struct DowngraderExecutionProgressEvent {
    /// Request id copied from the execution request.
    pub request_id: u64,
    /// Managed relative path whose patch is being downloaded.
    pub relative_path: &'static str,
    /// Patch asset name being downloaded.
    pub patch_name: String,
    /// Clamped reference progress value.
    pub progress: DowngraderProgress,
}

/// Per-file execution result and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderExecutionFileResult {
    /// Managed relative path from the Fallout 4 root.
    pub relative_path: &'static str,
    /// Basename displayed in modal logs.
    pub display_name: &'static str,
    /// Final outcome for this file.
    pub outcome: DowngraderExecutionOutcome,
    /// User-visible log row emitted for this file.
    pub log_row: DowngraderExecutionLogRow,
    /// Safe diagnostics for tracing/tests, not modal text.
    pub diagnostics: Vec<String>,
}

/// Complete confirmed execution result.
#[derive(Debug, Clone, PartialEq)]
pub struct DowngraderExecutionResult {
    /// Request id copied from the execution request.
    pub request_id: u64,
    /// Validated game root used for all managed files.
    pub game_root: PathBuf,
    /// Confirmed options used for execution.
    pub options: DowngraderOptionsSnapshot,
    /// Six per-file results in reference patch order.
    pub rows: Vec<DowngraderExecutionFileResult>,
    /// User-visible modal log rows in emission order.
    pub log_rows: Vec<DowngraderExecutionLogRow>,
    /// Bounded download progress events emitted during the run.
    pub progress_events: Vec<DowngraderExecutionProgressEvent>,
    /// Aggregate safe diagnostics for tracing/tests.
    pub diagnostics: Vec<String>,
}

/// A per-file read diagnostic captured while building a status snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderStatusDiagnostic {
    /// Managed relative path involved in the diagnostic.
    pub relative_path: &'static str,
    /// Safe summary suitable for logs/tests.
    pub safe_message: String,
}

/// Render-ready status row plus raw CRC facts from one managed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderStatusFile {
    /// Managed relative path from the Fallout 4 root.
    pub relative_path: &'static str,
    /// Basename displayed in the modal.
    pub display_name: &'static str,
    /// Reference modal group for this row.
    pub group: DowngraderFileGroup,
    /// Actual CRC classification before any display-only translation.
    pub detected_status: DowngraderInstallStatus,
    /// Status label to display after the `steam_api64.dll` NG/AE rule is applied.
    pub display_status: DowngraderInstallStatus,
    /// Uppercase eight-character CRC32 when the file was readable.
    pub crc32: Option<String>,
    /// Absolute or caller-relative resolved path under the game root.
    pub resolved_path: PathBuf,
    /// Safe read/metadata diagnostic when the row had to fall back to `Unknown`.
    pub read_error: Option<String>,
}

impl DowngraderStatusFile {
    /// Returns a compatibility row using the display status expected by the modal.
    pub fn display_row(&self) -> DowngraderStatusRow {
        DowngraderStatusRow {
            relative_path: self.relative_path,
            display_name: self.display_name,
            group: self.group,
            status: self.display_status,
        }
    }

    /// Returns a row preserving the raw CRC classification before display translation.
    pub fn detected_row(&self) -> DowngraderStatusRow {
        DowngraderStatusRow {
            relative_path: self.relative_path,
            display_name: self.display_name,
            group: self.group,
            status: self.detected_status,
        }
    }
}

/// Complete read-only status snapshot for the Downgrader modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderStatusSnapshot {
    /// Request id copied from the status request.
    pub request_id: u64,
    /// Validated game root used to resolve all managed files.
    pub game_root: PathBuf,
    /// Six managed status rows in reference display/patch order.
    pub rows: Vec<DowngraderStatusFile>,
    /// Reference default radio target derived from `Fallout4.exe`.
    pub default_target: DowngraderTarget,
    /// Whether any game-group file is `Unknown` or `Not Found`.
    pub unknown_game: bool,
    /// Whether any Creation Kit-group file is `Unknown` or `Not Found`.
    pub unknown_creation_kit: bool,
    /// Safe non-fatal diagnostics captured while reading rows.
    pub diagnostics: Vec<DowngraderStatusDiagnostic>,
}

impl DowngraderStatusSnapshot {
    /// Returns the status for a managed file by reference display name or path.
    pub fn status_for(&self, file_name_or_path: &str) -> Option<&DowngraderStatusFile> {
        self.rows.iter().find(|row| {
            row.relative_path.eq_ignore_ascii_case(file_name_or_path)
                || row.display_name.eq_ignore_ascii_case(file_name_or_path)
        })
    }
}

/// Backup CRC probe used by preview rows to explain why a branch was chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderBackupProbe {
    /// Resolved backup path under the same directory as the managed file.
    pub path: PathBuf,
    /// Backup file name used by the Python reference.
    pub file_name: String,
    /// Whether metadata reported a regular file.
    pub exists: bool,
    /// Uppercase CRC32 when the backup was readable.
    pub crc32: Option<String>,
    /// Safe read/metadata diagnostic when the backup existed but could not be read.
    pub read_error: Option<String>,
}

impl DowngraderBackupProbe {
    fn missing(path: PathBuf, file_name: impl Into<String>) -> Self {
        Self {
            path,
            file_name: file_name.into(),
            exists: false,
            crc32: None,
            read_error: None,
        }
    }
}

/// Detailed preview row for one managed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderPreviewPlanRow {
    /// Pure reference row with first-pass skip/worker action metadata.
    pub plan: DowngraderPlanRow,
    /// Display status after `steam_api64.dll` NG/AE translation.
    pub display_status: DowngraderInstallStatus,
    /// Uppercase CRC32 of the current file when readable.
    pub current_crc32: Option<String>,
    /// Resolved current file path under the game root.
    pub current_path: PathBuf,
    /// Desired-version backup probe, read only when the row reaches backup planning.
    pub desired_backup: Option<DowngraderBackupProbe>,
    /// Current-version backup probe, read only when the row reaches backup planning.
    pub current_backup: Option<DowngraderBackupProbe>,
    /// Ordered steps a later confirmed worker would perform for this file.
    pub steps: Vec<DowngraderPlanStep>,
    /// Safe per-row failure that disables execution when planning cannot continue safely.
    pub failure: Option<String>,
}

impl DowngraderPreviewPlanRow {
    /// Returns whether this row can be handed to a later mutation worker.
    pub fn can_execute(&self) -> bool {
        self.failure.is_none()
    }

    /// Returns true when the row contains a delta download step.
    pub fn requires_download(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.kind == DowngraderPlanStepKind::DownloadDelta)
    }

    /// Returns true when the row restores from an existing desired-version backup.
    pub fn restores_from_backup(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.kind == DowngraderPlanStepKind::RestoreDesiredBackup)
    }
}

/// Aggregate counts from a generated preview plan.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DowngraderPreviewPlanCounts {
    /// Rows that produce only reference skip messages.
    pub skipped_rows: usize,
    /// Rows that will restore from a desired-version backup.
    pub restore_from_backup_rows: usize,
    /// Rows that require downloading an xdelta patch.
    pub delta_download_rows: usize,
    /// Rows that failed safely during planning.
    pub failed_rows: usize,
    /// Total mutating execution steps that would happen only after confirmation.
    pub mutating_step_count: usize,
}

/// Complete inline plan returned before any mutation, download, or patching occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderPreviewPlan {
    /// Request id copied from the plan request.
    pub request_id: u64,
    /// Validated game root used to resolve all managed files.
    pub game_root: PathBuf,
    /// User options used for this plan.
    pub options: DowngraderOptionsSnapshot,
    /// Status snapshot used as the source of truth for the plan.
    pub status: DowngraderStatusSnapshot,
    /// Six preview rows in reference patch order.
    pub rows: Vec<DowngraderPreviewPlanRow>,
    /// Aggregate row/step counts for diagnostics and UI summary text.
    pub counts: DowngraderPreviewPlanCounts,
    /// False when any row failed during read-only planning.
    pub can_execute: bool,
}

impl DowngraderPreviewPlan {
    fn from_rows(
        request_id: u64,
        game_root: PathBuf,
        options: DowngraderOptionsSnapshot,
        status: DowngraderStatusSnapshot,
        rows: Vec<DowngraderPreviewPlanRow>,
    ) -> Self {
        let mut counts = DowngraderPreviewPlanCounts::default();
        for row in &rows {
            if row.failure.is_some() {
                counts.failed_rows += 1;
            }
            if row.steps.iter().all(|step| {
                matches!(
                    step.kind,
                    DowngraderPlanStepKind::SkipAlreadyDesired
                        | DowngraderPlanStepKind::SkipNotFound
                        | DowngraderPlanStepKind::SkipUnsupportedVersion
                )
            }) {
                counts.skipped_rows += 1;
            }
            if row.restores_from_backup() {
                counts.restore_from_backup_rows += 1;
            }
            if row.requires_download() {
                counts.delta_download_rows += 1;
            }
            counts.mutating_step_count += row
                .steps
                .iter()
                .filter(|step| step.kind.is_mutating_execution_step())
                .count();
        }
        let can_execute = counts.failed_rows == 0;
        Self {
            request_id,
            game_root,
            options,
            status,
            rows,
            counts,
            can_execute,
        }
    }

    /// Returns a stable digest for the material file, backup, option, and step state in this plan.
    ///
    /// Request ids are intentionally excluded so a fresh revalidation can be compared with
    /// the exact preview the user reviewed before any destructive work starts.
    pub fn stable_digest(&self) -> String {
        let mut digest = Sha256::new();
        update_plan_digest_value(&mut digest, "cmt-rs-downgrader-preview-plan-v1");
        update_plan_digest_value(&mut digest, self.game_root.to_string_lossy().as_ref());
        update_plan_digest_value(&mut digest, self.options.target.as_reference_str());
        update_plan_digest_value(&mut digest, bool_digest_value(self.options.keep_backups));
        update_plan_digest_value(&mut digest, bool_digest_value(self.options.delete_deltas));
        update_plan_digest_value(&mut digest, bool_digest_value(self.can_execute));

        for row in &self.rows {
            update_plan_digest_value(&mut digest, row.plan.relative_path);
            update_plan_digest_value(&mut digest, row.plan.display_name);
            update_plan_digest_value(&mut digest, row.display_status.as_reference_str());
            update_plan_digest_value(&mut digest, row.current_crc32.as_deref().unwrap_or(""));
            update_plan_digest_value(&mut digest, row.current_path.to_string_lossy().as_ref());
            update_plan_digest_value(&mut digest, row.plan.target.as_reference_str());
            update_plan_digest_value(&mut digest, format!("{:?}", row.plan.action));
            update_plan_digest_value(&mut digest, &row.plan.current_backup_name);
            update_plan_digest_value(&mut digest, &row.plan.desired_backup_name);
            update_plan_digest_value(&mut digest, &row.plan.patch_name);
            update_plan_digest_value(&mut digest, &row.plan.patch_url);
            update_plan_digest_backup(&mut digest, row.desired_backup.as_ref());
            update_plan_digest_backup(&mut digest, row.current_backup.as_ref());
            update_plan_digest_value(&mut digest, row.failure.as_deref().unwrap_or(""));
            for step in &row.steps {
                update_plan_digest_value(&mut digest, step.kind.as_str());
                update_plan_digest_value(&mut digest, &step.message);
            }
        }

        format!("{:x}", digest.finalize())
    }
}

fn bool_digest_value(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn update_plan_digest_value(digest: &mut Sha256, value: impl AsRef<str>) {
    let value = value.as_ref().as_bytes();
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value);
}

fn update_plan_digest_backup(digest: &mut Sha256, backup: Option<&DowngraderBackupProbe>) {
    match backup {
        Some(backup) => {
            update_plan_digest_value(digest, "backup:some");
            update_plan_digest_value(digest, backup.path.to_string_lossy().as_ref());
            update_plan_digest_value(digest, &backup.file_name);
            update_plan_digest_value(digest, bool_digest_value(backup.exists));
            update_plan_digest_value(digest, backup.crc32.as_deref().unwrap_or(""));
            update_plan_digest_value(digest, backup.read_error.as_deref().unwrap_or(""));
        }
        None => update_plan_digest_value(digest, "backup:none"),
    }
}

/// Read-only service that classifies Downgrader files and builds preview plans.
#[derive(Debug, Clone, Copy)]
pub struct DowngraderService<'a, F: Filesystem + ?Sized> {
    filesystem: &'a F,
    patch_integrity_manifest: &'a [DowngraderPatchIntegrity],
}

impl<'a, F: Filesystem + ?Sized> DowngraderService<'a, F> {
    /// Creates a Downgrader service over a read-only filesystem adapter.
    pub const fn new(filesystem: &'a F) -> Self {
        Self {
            filesystem,
            patch_integrity_manifest: &PATCH_INTEGRITY_MANIFEST,
        }
    }

    #[cfg(test)]
    const fn with_patch_integrity_manifest(
        filesystem: &'a F,
        patch_integrity_manifest: &'a [DowngraderPatchIntegrity],
    ) -> Self {
        Self {
            filesystem,
            patch_integrity_manifest,
        }
    }

    /// Builds a read-only status snapshot for the six reference-managed files.
    pub fn status_snapshot(
        &self,
        request: DowngraderStatusRequest<'_>,
    ) -> Result<DowngraderStatusSnapshot, DowngraderServiceError> {
        let span = info_span!(
            "downgrader.status_snapshot",
            request_id = request.request_id,
            has_installation = request.installation.is_some(),
        );
        let _guard = span.enter();
        info!(
            event = "downgrader-status-request",
            "Downgrader status requested"
        );

        let game_root = self.validated_game_root(request.installation)?;
        validate_managed_definitions(&game_root)?;

        let mut raw_rows = Vec::with_capacity(DOWNGRADER_FILE_DEFINITIONS.len());
        let mut diagnostics = Vec::new();
        for definition in DOWNGRADER_FILE_DEFINITIONS {
            let resolved_path =
                resolve_managed_relative_path(&game_root, definition.relative_path)?;
            let row = self.read_managed_file(definition, resolved_path);
            if let Some(read_error) = &row.read_error {
                diagnostics.push(DowngraderStatusDiagnostic {
                    relative_path: row.relative_path,
                    safe_message: read_error.clone(),
                });
            }
            debug!(
                event = "downgrader-status-row",
                request_id = request.request_id,
                relative_path = definition.relative_path,
                status = row.detected_status.as_reference_str(),
                has_crc = row.crc32.is_some(),
                "Downgrader status row classified"
            );
            raw_rows.push(row);
        }

        let fallout4_status = raw_rows
            .iter()
            .find(|row| row.relative_path == "Fallout4.exe")
            .map(|row| row.detected_status)
            .unwrap_or(DowngraderInstallStatus::Unknown);
        for row in &mut raw_rows {
            row.display_status =
                display_status_for(row.relative_path, row.detected_status, fallout4_status);
        }

        let default_target = default_target_from_fallout4(fallout4_status);
        let unknown_game = raw_rows.iter().any(|row| {
            row.group == DowngraderFileGroup::Game
                && matches!(
                    row.detected_status,
                    DowngraderInstallStatus::Unknown | DowngraderInstallStatus::NotFound
                )
        });
        let unknown_creation_kit = raw_rows.iter().any(|row| {
            row.group == DowngraderFileGroup::CreationKit
                && matches!(
                    row.detected_status,
                    DowngraderInstallStatus::Unknown | DowngraderInstallStatus::NotFound
                )
        });

        info!(
            event = "downgrader-status-complete",
            request_id = request.request_id,
            row_count = raw_rows.len(),
            diagnostic_count = diagnostics.len(),
            default_target = default_target.as_reference_str(),
            unknown_game,
            unknown_creation_kit,
            "Downgrader status snapshot built"
        );

        Ok(DowngraderStatusSnapshot {
            request_id: request.request_id,
            game_root,
            rows: raw_rows,
            default_target,
            unknown_game,
            unknown_creation_kit,
            diagnostics,
        })
    }

    /// Builds an inline preview plan without mutating files, downloading deltas, or applying patches.
    pub fn preview_plan(
        &self,
        request: DowngraderPlanRequest<'_>,
    ) -> Result<DowngraderPreviewPlan, DowngraderServiceError> {
        let span = info_span!(
            "downgrader.preview_plan",
            request_id = request.request_id,
            target = request.options.target.as_reference_str(),
            keep_backups = request.options.keep_backups,
            delete_deltas = request.options.delete_deltas,
            has_installation = request.installation.is_some(),
        );
        let _guard = span.enter();
        info!(
            event = "downgrader-plan-request",
            "Downgrader preview plan requested"
        );

        let status = self.status_snapshot(DowngraderStatusRequest::new(
            request.request_id,
            request.installation,
        ))?;
        let mut rows = Vec::with_capacity(status.rows.len());
        for status_row in &status.rows {
            let definition = definition_for_status_row(status_row)?;
            rows.push(self.plan_row(definition, status_row, request.options));
        }

        let plan = DowngraderPreviewPlan::from_rows(
            request.request_id,
            status.game_root.clone(),
            request.options,
            status,
            rows,
        );
        info!(
            event = "downgrader-plan-complete",
            request_id = plan.request_id,
            row_count = plan.rows.len(),
            skipped_rows = plan.counts.skipped_rows,
            restore_from_backup_rows = plan.counts.restore_from_backup_rows,
            delta_download_rows = plan.counts.delta_download_rows,
            failed_rows = plan.counts.failed_rows,
            mutating_step_count = plan.counts.mutating_step_count,
            can_execute = plan.can_execute,
            "Downgrader preview plan built"
        );
        Ok(plan)
    }

    fn validated_game_root(
        &self,
        installation: Option<&Fallout4Installation>,
    ) -> Result<PathBuf, DowngraderServiceError> {
        let root = installation
            .map(|installation| installation.game_path.as_path())
            .ok_or_else(|| DowngraderServiceError::MissingGameRoot {
                safe_message: FALLOUT4_NOT_FOUND_MESSAGE.to_owned(),
            })?;

        validate_game_root_path(root)?;
        match self.filesystem.metadata(root) {
            Ok(metadata) if metadata.is_dir() => Ok(root.to_path_buf()),
            Ok(_) => Err(DowngraderServiceError::InvalidGameRoot {
                root: root.to_path_buf(),
                safe_message: ROOT_NOT_DIRECTORY_MESSAGE.to_owned(),
            }),
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                warn!(
                    event = "downgrader-root-missing",
                    root = %root.display(),
                    "Downgrader root was not found"
                );
                Err(DowngraderServiceError::MissingGameRoot {
                    safe_message: FALLOUT4_NOT_FOUND_MESSAGE.to_owned(),
                })
            }
            Err(error) => {
                warn!(
                    event = "downgrader-root-unavailable",
                    root = %root.display(),
                    failure_kind = ?error.kind,
                    "Downgrader root could not be validated"
                );
                Err(DowngraderServiceError::InvalidGameRoot {
                    root: root.to_path_buf(),
                    safe_message: error.user_message().to_owned(),
                })
            }
        }
    }

    fn read_managed_file(
        &self,
        definition: DowngraderFileDefinition,
        resolved_path: PathBuf,
    ) -> DowngraderStatusFile {
        let probe = self.read_crc(&resolved_path);
        let (status, crc32, read_error) = match probe {
            CrcProbe::Readable { crc32 } => (
                definition
                    .status_for_crc(&crc32)
                    .unwrap_or(DowngraderInstallStatus::Unknown),
                Some(crc32),
                None,
            ),
            CrcProbe::Missing => (DowngraderInstallStatus::NotFound, None, None),
            CrcProbe::Unreadable { safe_message } => {
                (DowngraderInstallStatus::Unknown, None, Some(safe_message))
            }
        };
        DowngraderStatusFile {
            relative_path: definition.relative_path,
            display_name: definition.display_name,
            group: definition.group,
            detected_status: status,
            display_status: status,
            crc32,
            resolved_path,
            read_error,
        }
    }

    fn plan_row(
        &self,
        definition: DowngraderFileDefinition,
        status_row: &DowngraderStatusFile,
        options: DowngraderOptionsSnapshot,
    ) -> DowngraderPreviewPlanRow {
        let base_plan =
            DowngraderPlanRow::from_definition(definition, status_row.detected_status, options);
        let mut row = DowngraderPreviewPlanRow {
            plan: base_plan,
            display_status: status_row.display_status,
            current_crc32: status_row.crc32.clone(),
            current_path: status_row.resolved_path.clone(),
            desired_backup: None,
            current_backup: None,
            steps: Vec::new(),
            failure: None,
        };

        match row.plan.action {
            DowngraderPlanAction::SkipAlreadyDesired => {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::SkipAlreadyDesired,
                    skipped_already_message(
                        row.plan.display_name,
                        row.plan.target.desired_status(),
                    ),
                ));
                return row;
            }
            DowngraderPlanAction::SkipNotFound => {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::SkipNotFound,
                    skipped_not_found_message(row.plan.display_name),
                ));
                return row;
            }
            DowngraderPlanAction::SkipUnsupportedVersion => {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::SkipUnsupportedVersion,
                    skipped_unsupported_message(row.plan.display_name),
                ));
                return row;
            }
            DowngraderPlanAction::ValidateBackupOrPatch => {}
        }

        if status_row.read_error.is_some() {
            fail_row(&mut row, CURRENT_FILE_READ_FAILURE_MESSAGE);
            return row;
        }

        let Some(current_crc32) = row.current_crc32.clone() else {
            fail_row(&mut row, CURRENT_FILE_READ_FAILURE_MESSAGE);
            return row;
        };

        if !crc_is_supported_source_for_target(&current_crc32, row.plan.target) {
            row.steps.push(DowngraderPlanStep::new(
                DowngraderPlanStepKind::SkipUnsupportedVersion,
                skipped_unsupported_message(row.plan.display_name),
            ));
            debug!(
                event = "downgrader-plan-unsupported-source",
                relative_path = row.plan.relative_path,
                crc32 = current_crc32,
                target = row.plan.target.as_reference_str(),
                "Downgrader source CRC is unsupported for target"
            );
            return row;
        }

        let current_backup_path = backup_path_for(&row.current_path, &row.plan.current_backup_name);
        let desired_backup_path = backup_path_for(&row.current_path, &row.plan.desired_backup_name);
        let current_backup =
            self.probe_backup(current_backup_path, row.plan.current_backup_name.clone());
        let desired_backup =
            self.probe_backup(desired_backup_path, row.plan.desired_backup_name.clone());

        if let Some(read_error) = current_backup.read_error.clone() {
            row.current_backup = Some(current_backup);
            row.desired_backup = Some(desired_backup);
            fail_row(&mut row, read_error);
            return row;
        }
        if let Some(read_error) = desired_backup.read_error.clone() {
            row.current_backup = Some(current_backup);
            row.desired_backup = Some(desired_backup);
            fail_row(&mut row, read_error);
            return row;
        }

        let current_backup_matches = current_backup
            .crc32
            .as_deref()
            .is_some_and(|backup_crc| backup_crc.eq_ignore_ascii_case(&current_crc32));
        if current_backup.exists {
            if current_backup_matches {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::ReuseCurrentBackup,
                    format!(
                        "Use existing backup {} for {}.",
                        row.plan.current_backup_name, row.plan.display_name
                    ),
                ));
            } else {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::DeleteInvalidCurrentBackup,
                    format!("Delete invalid backup {}.", row.plan.current_backup_name),
                ));
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::CreateCurrentBackup,
                    format!(
                        "Create backup {} from {}.",
                        row.plan.current_backup_name, row.plan.display_name
                    ),
                ));
            }
        } else {
            row.steps.push(DowngraderPlanStep::new(
                DowngraderPlanStepKind::CreateCurrentBackup,
                format!(
                    "Create backup {} from {}.",
                    row.plan.current_backup_name, row.plan.display_name
                ),
            ));
        }

        let desired_backup_is_valid = desired_backup
            .crc32
            .as_deref()
            .is_some_and(|backup_crc| crc_is_desired_target_for_plan(backup_crc, row.plan.target));
        if desired_backup.exists && desired_backup_is_valid {
            row.steps.push(DowngraderPlanStep::new(
                DowngraderPlanStepKind::RestoreDesiredBackup,
                format!(
                    "Restore {} from {}.",
                    row.plan.display_name, row.plan.desired_backup_name
                ),
            ));
            if !options.keep_backups {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::DeleteCurrentBackup,
                    format!(
                        "Delete backup {} after restore.",
                        row.plan.current_backup_name
                    ),
                ));
            }
        } else {
            if desired_backup.exists {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::DeleteInvalidDesiredBackup,
                    format!("Delete invalid backup {}.", row.plan.desired_backup_name),
                ));
            }
            row.steps.push(DowngraderPlanStep::new(
                DowngraderPlanStepKind::DownloadDelta,
                format!("Download {}.", row.plan.patch_name),
            ));
            row.steps.push(DowngraderPlanStep::new(
                DowngraderPlanStepKind::ApplyDeltaPatch,
                format!(
                    "Apply {} to {}.",
                    row.plan.patch_name, row.plan.display_name
                ),
            ));
            if !options.keep_backups {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::DeleteCurrentBackup,
                    format!(
                        "Delete backup {} after patch.",
                        row.plan.current_backup_name
                    ),
                ));
            }
            if options.delete_deltas {
                row.steps.push(DowngraderPlanStep::new(
                    DowngraderPlanStepKind::DeleteDeltaPatch,
                    format!("Delete patch {} after patching.", row.plan.patch_name),
                ));
            }
        }

        row.current_backup = Some(current_backup);
        row.desired_backup = Some(desired_backup);
        debug!(
            event = "downgrader-plan-row",
            relative_path = row.plan.relative_path,
            target = row.plan.target.as_reference_str(),
            step_count = row.steps.len(),
            requires_download = row.requires_download(),
            restores_from_backup = row.restores_from_backup(),
            "Downgrader preview row planned"
        );
        row
    }

    fn probe_backup(&self, path: PathBuf, file_name: String) -> DowngraderBackupProbe {
        match self.read_crc(&path) {
            CrcProbe::Readable { crc32 } => DowngraderBackupProbe {
                path,
                file_name,
                exists: true,
                crc32: Some(crc32),
                read_error: None,
            },
            CrcProbe::Missing => DowngraderBackupProbe::missing(path, file_name),
            CrcProbe::Unreadable { safe_message } => DowngraderBackupProbe {
                path,
                file_name,
                exists: true,
                crc32: None,
                read_error: Some(format!("{BACKUP_READ_FAILURE_MESSAGE} {safe_message}")),
            },
        }
    }

    fn read_crc(&self, path: &Path) -> CrcProbe {
        match self.filesystem.metadata(path) {
            Ok(metadata) if metadata.is_file() => self.read_crc_bytes(path),
            Ok(_) => CrcProbe::Missing,
            Err(error) if error.kind == PlatformErrorKind::NotFound => CrcProbe::Missing,
            Err(error) => CrcProbe::Unreadable {
                safe_message: error.user_message().to_owned(),
            },
        }
    }

    fn read_crc_bytes(&self, path: &Path) -> CrcProbe {
        match self.filesystem.read_bytes(path) {
            Ok(bytes) => CrcProbe::Readable {
                crc32: crc32_hex(&bytes),
            },
            Err(error) if error.kind == PlatformErrorKind::NotFound => CrcProbe::Unreadable {
                safe_message: error.user_message().to_owned(),
            },
            Err(error) => CrcProbe::Unreadable {
                safe_message: error.user_message().to_owned(),
            },
        }
    }
}

impl<'a, F: Filesystem + WritableFilesystem + ?Sized> DowngraderService<'a, F> {
    fn validated_execution_game_root(
        &self,
        planned_root: &Path,
    ) -> Result<PathBuf, DowngraderServiceError> {
        validate_game_root_path(planned_root)?;
        let canonical_root = self
            .filesystem
            .canonicalize_path(planned_root)
            .map_err(|error| {
                warn!(
                    event = "downgrader-execute-root-canonicalize-failed",
                    root = %planned_root.display(),
                    failure_kind = ?error.kind,
                    "Downgrader execution root canonicalization failed"
                );
                DowngraderServiceError::InvalidGameRoot {
                    root: planned_root.to_path_buf(),
                    safe_message: error.user_message().to_owned(),
                }
            })?;
        match self.filesystem.metadata(&canonical_root) {
            Ok(metadata) if metadata.is_dir() => Ok(canonical_root),
            Ok(_) => Err(DowngraderServiceError::InvalidGameRoot {
                root: canonical_root,
                safe_message: ROOT_NOT_DIRECTORY_MESSAGE.to_owned(),
            }),
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                Err(DowngraderServiceError::MissingGameRoot {
                    safe_message: FALLOUT4_NOT_FOUND_MESSAGE.to_owned(),
                })
            }
            Err(error) => Err(DowngraderServiceError::InvalidGameRoot {
                root: canonical_root,
                safe_message: error.user_message().to_owned(),
            }),
        }
    }

    /// Executes a freshly revalidated Downgrader plan after the user confirms the inline preview.
    ///
    /// The executor processes each of the six managed files independently and
    /// records reference-style log rows while keeping diagnostics separate for
    /// tracing/tests. It assumes callers run it off the Slint UI thread.
    pub fn execute_confirmed<D, A>(
        &self,
        request: DowngraderExecutionRequest<'_>,
        downloader: &D,
        applier: &A,
    ) -> Result<DowngraderExecutionResult, DowngraderServiceError>
    where
        D: DeltaDownloader + ?Sized,
        A: DeltaApplier + ?Sized,
    {
        self.execute_confirmed_with_events(request, downloader, applier, |_| {}, |_| {})
    }

    /// Executes a confirmed plan while emitting per-row logs and download progress as they happen.
    pub fn execute_confirmed_with_events<D, A, L, P>(
        &self,
        request: DowngraderExecutionRequest<'_>,
        downloader: &D,
        applier: &A,
        mut log_callback: L,
        mut progress_callback: P,
    ) -> Result<DowngraderExecutionResult, DowngraderServiceError>
    where
        D: DeltaDownloader + ?Sized,
        A: DeltaApplier + ?Sized,
        L: FnMut(&DowngraderExecutionLogRow),
        P: FnMut(&DowngraderExecutionProgressEvent),
    {
        let span = info_span!(
            "downgrader.execute_confirmed",
            request_id = request.request_id,
            target = request.options.target.as_reference_str(),
            keep_backups = request.options.keep_backups,
            delete_deltas = request.options.delete_deltas,
            has_installation = request.installation.is_some(),
        );
        let _guard = span.enter();
        info!(
            event = "downgrader-execute-request",
            "Downgrader confirmed execution requested"
        );

        let plan = self.preview_plan(DowngraderPlanRequest::new(
            request.request_id,
            request.installation,
            request.options,
        ))?;
        if let Some(expected_digest) = request.confirmed_plan_digest.as_deref() {
            let actual_digest = plan.stable_digest();
            if actual_digest != expected_digest {
                warn!(
                    event = "downgrader-confirmed-plan-changed",
                    request_id = request.request_id,
                    expected_digest,
                    actual_digest = actual_digest.as_str(),
                    "Downgrader confirmed run aborted because the preview plan changed"
                );
                return Err(DowngraderServiceError::ConfirmedPlanChanged {
                    safe_message: CONFIRMED_PLAN_CHANGED_MESSAGE.to_owned(),
                    expected_digest: expected_digest.to_owned(),
                    actual_digest,
                });
            }
        }
        let execution_root = self.validated_execution_game_root(&plan.game_root)?;
        let mut result = DowngraderExecutionResult {
            request_id: request.request_id,
            game_root: execution_root.clone(),
            options: request.options,
            rows: Vec::with_capacity(plan.rows.len()),
            log_rows: Vec::with_capacity(plan.rows.len()),
            progress_events: Vec::new(),
            diagnostics: Vec::new(),
        };

        for row in &plan.rows {
            let file_result = self.execute_row(
                request.request_id,
                &execution_root,
                row,
                downloader,
                applier,
                &mut result.progress_events,
                &mut progress_callback,
            );
            debug!(
                event = "downgrader-execute-row-complete",
                request_id = request.request_id,
                relative_path = row.plan.relative_path,
                outcome = ?file_result.outcome,
                diagnostic_count = file_result.diagnostics.len(),
                "Downgrader execution row completed"
            );
            result.log_rows.push(file_result.log_row.clone());
            log_callback(&file_result.log_row);
            result.diagnostics.extend(file_result.diagnostics.clone());
            result.rows.push(file_result);
        }

        let patched_rows = result
            .rows
            .iter()
            .filter(|row| row.outcome == DowngraderExecutionOutcome::Patched)
            .count();
        let failed_rows = result
            .rows
            .iter()
            .filter(|row| row.outcome == DowngraderExecutionOutcome::Failed)
            .count();
        let skipped_rows = result
            .rows
            .iter()
            .filter(|row| row.outcome == DowngraderExecutionOutcome::Skipped)
            .count();
        info!(
            event = "downgrader-execute-complete",
            request_id = result.request_id,
            patched_rows,
            failed_rows,
            skipped_rows,
            progress_event_count = result.progress_events.len(),
            diagnostic_count = result.diagnostics.len(),
            "Downgrader confirmed execution completed"
        );

        Ok(result)
    }

    fn execute_row<D, A, P>(
        &self,
        request_id: u64,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
        downloader: &D,
        applier: &A,
        progress_events: &mut Vec<DowngraderExecutionProgressEvent>,
        progress_callback: &mut P,
    ) -> DowngraderExecutionFileResult
    where
        D: DeltaDownloader + ?Sized,
        A: DeltaApplier + ?Sized,
        P: FnMut(&DowngraderExecutionProgressEvent),
    {
        if let Some(log_row) = row.plan.skip_log_row() {
            return execution_file_result(
                row,
                DowngraderExecutionOutcome::Skipped,
                log_row,
                Vec::new(),
            );
        }

        if let Some(failure) = row.failure.clone() {
            return failed_execution_file_result(row, vec![failure]);
        }

        match self.execute_mutating_row(
            request_id,
            game_root,
            row,
            downloader,
            applier,
            progress_events,
            progress_callback,
        ) {
            RowExecutionStatus::Patched { diagnostics } => execution_file_result(
                row,
                DowngraderExecutionOutcome::Patched,
                patched_log_row(row.plan.display_name),
                diagnostics,
            ),
            RowExecutionStatus::SkippedUnsupported { diagnostics } => execution_file_result(
                row,
                DowngraderExecutionOutcome::Skipped,
                row.plan.skip_log_row().unwrap_or_else(|| {
                    crate::domain::downgrader::skipped_unsupported_log_row(row.plan.display_name)
                }),
                diagnostics,
            ),
            RowExecutionStatus::Failed { diagnostics } => {
                failed_execution_file_result(row, diagnostics)
            }
        }
    }

    fn execute_mutating_row<D, A, P>(
        &self,
        request_id: u64,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
        downloader: &D,
        applier: &A,
        progress_events: &mut Vec<DowngraderExecutionProgressEvent>,
        progress_callback: &mut P,
    ) -> RowExecutionStatus
    where
        D: DeltaDownloader + ?Sized,
        A: DeltaApplier + ?Sized,
        P: FnMut(&DowngraderExecutionProgressEvent),
    {
        let mut diagnostics = Vec::new();
        let paths = match self.revalidated_execution_paths(game_root, row) {
            Ok(paths) => paths,
            Err(diagnostic) => {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        };
        let current_crc = match self.read_required_crc(&paths.current_path) {
            Ok(crc32) => crc32,
            Err(diagnostic) => {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        };
        if !crc_is_supported_source_for_target(&current_crc, row.plan.target) {
            diagnostics.push(format!(
                "{} current CRC {current_crc} is unsupported for target {}.",
                row.plan.display_name,
                row.plan.target.as_reference_str()
            ));
            warn!(
                event = "downgrader-execute-unsupported-source",
                request_id,
                relative_path = row.plan.relative_path,
                crc32 = current_crc,
                target = row.plan.target.as_reference_str(),
                "Downgrader execution skipped unsupported source"
            );
            return RowExecutionStatus::SkippedUnsupported { diagnostics };
        }

        if let Err(diagnostic) = self.prepare_current_backup(
            game_root,
            row,
            &paths.current_path,
            &paths.current_backup_path,
            &current_crc,
        ) {
            return RowExecutionStatus::Failed {
                diagnostics: vec![diagnostic],
            };
        }

        match self.read_crc(&paths.desired_backup_path) {
            CrcProbe::Readable { crc32 }
                if crc_is_desired_target_for_plan(&crc32, row.plan.target) =>
            {
                match self.restore_desired_backup(
                    game_root,
                    row,
                    &paths.current_path,
                    &paths.current_backup_path,
                    &paths.desired_backup_path,
                ) {
                    Ok(restore_diagnostics) => {
                        diagnostics.extend(restore_diagnostics);
                        RowExecutionStatus::Patched { diagnostics }
                    }
                    Err(diagnostic) => RowExecutionStatus::Failed {
                        diagnostics: vec![diagnostic],
                    },
                }
            }
            CrcProbe::Readable { crc32 } => {
                diagnostics.push(format!(
                    "{} desired backup CRC {crc32} is invalid for target {}.",
                    row.plan.display_name,
                    row.plan.target.as_reference_str()
                ));
                if let Err(diagnostic) = self.remove_file_under_root(
                    game_root,
                    &paths.desired_backup_path,
                    row,
                    DowngraderPlanStepKind::DeleteInvalidDesiredBackup,
                ) {
                    return RowExecutionStatus::Failed {
                        diagnostics: vec![diagnostic],
                    };
                }
                self.download_apply_and_cleanup(
                    PatchApplyContext {
                        request_id,
                        game_root,
                        row,
                        current_path: &paths.current_path,
                        current_backup_path: &paths.current_backup_path,
                        patch_path: &paths.patch_path,
                        integrity: paths.integrity,
                        progress_events,
                        progress_callback,
                        diagnostics,
                    },
                    downloader,
                    applier,
                )
            }
            CrcProbe::Missing => self.download_apply_and_cleanup(
                PatchApplyContext {
                    request_id,
                    game_root,
                    row,
                    current_path: &paths.current_path,
                    current_backup_path: &paths.current_backup_path,
                    patch_path: &paths.patch_path,
                    integrity: paths.integrity,
                    progress_events,
                    progress_callback,
                    diagnostics,
                },
                downloader,
                applier,
            ),
            CrcProbe::Unreadable { safe_message } => RowExecutionStatus::Failed {
                diagnostics: vec![format!("{} {}", BACKUP_READ_FAILURE_MESSAGE, safe_message)],
            },
        }
    }

    fn revalidated_execution_paths(
        &self,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
    ) -> Result<ManagedExecutionPaths, String> {
        let current_path = resolve_managed_relative_path(game_root, row.plan.relative_path)
            .map_err(|error| error.user_message().to_owned())?;
        self.ensure_existing_file_within_root(game_root, &current_path, row, "managed target")?;

        let current_backup_path = backup_path_for(&current_path, &row.plan.current_backup_name);
        let desired_backup_path = backup_path_for(&current_path, &row.plan.desired_backup_name);
        let patch_path = game_root.join(&row.plan.patch_name);
        self.ensure_existing_or_parent_within_root(
            game_root,
            &current_backup_path,
            row,
            "current backup",
        )?;
        self.ensure_existing_or_parent_within_root(
            game_root,
            &desired_backup_path,
            row,
            "desired backup",
        )?;
        self.ensure_existing_or_parent_within_root(game_root, &patch_path, row, "delta patch")?;
        let integrity = self
            .patch_integrity_for(&row.plan.patch_name)
            .ok_or_else(|| {
                format!(
                    "{} has no pinned integrity metadata for {}.",
                    PATCH_INTEGRITY_FAILURE_MESSAGE, row.plan.patch_name
                )
            })?;

        Ok(ManagedExecutionPaths {
            current_path,
            current_backup_path,
            desired_backup_path,
            patch_path,
            integrity,
        })
    }

    fn ensure_existing_or_parent_within_root(
        &self,
        game_root: &Path,
        path: &Path,
        row: &DowngraderPreviewPlanRow,
        context: &str,
    ) -> Result<(), String> {
        match self.filesystem.symlink_metadata(path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(format!(
                        "{} {context} path is not a file.",
                        row.plan.display_name
                    ));
                }
                self.ensure_metadata_path_within_root(game_root, path, &metadata, row, context)
            }
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                let parent = path.parent().ok_or_else(|| {
                    format!("{} {context} path is invalid.", row.plan.display_name)
                })?;
                self.ensure_directory_within_root(game_root, parent, row, context)
            }
            Err(error) => Err(mutation_diagnostic(
                row,
                DowngraderPlanStepKind::ApplyDeltaPatch,
                error.user_message(),
            )),
        }
    }

    fn ensure_existing_file_within_root(
        &self,
        game_root: &Path,
        path: &Path,
        row: &DowngraderPreviewPlanRow,
        context: &str,
    ) -> Result<(), String> {
        let metadata = self.filesystem.symlink_metadata(path).map_err(|error| {
            mutation_diagnostic(
                row,
                DowngraderPlanStepKind::ApplyDeltaPatch,
                error.user_message(),
            )
        })?;
        if !metadata.is_file() {
            return Err(format!(
                "{} {context} path is not a file.",
                row.plan.display_name
            ));
        }
        self.ensure_metadata_path_within_root(game_root, path, &metadata, row, context)
    }

    fn ensure_directory_within_root(
        &self,
        game_root: &Path,
        path: &Path,
        row: &DowngraderPreviewPlanRow,
        context: &str,
    ) -> Result<(), String> {
        let metadata = self.filesystem.symlink_metadata(path).map_err(|error| {
            mutation_diagnostic(
                row,
                DowngraderPlanStepKind::ApplyDeltaPatch,
                error.user_message(),
            )
        })?;
        if !metadata.is_dir() {
            return Err(format!(
                "{} {context} parent path is not a folder.",
                row.plan.display_name
            ));
        }
        self.ensure_metadata_path_within_root(game_root, path, &metadata, row, context)
    }

    fn ensure_metadata_path_within_root(
        &self,
        game_root: &Path,
        path: &Path,
        metadata: &FileMetadata,
        row: &DowngraderPreviewPlanRow,
        context: &str,
    ) -> Result<(), String> {
        if metadata.is_symlink_or_reparse_point() {
            warn!(
                event = "downgrader-execute-reparse-rejected",
                relative_path = row.plan.relative_path,
                path = %path.display(),
                context,
                "Downgrader rejected a symlink/reparse managed path before mutation"
            );
            return Err(format!(
                "{} {context} path is unsafe.",
                row.plan.display_name
            ));
        }
        let canonical_path = self.filesystem.canonicalize_path(path).map_err(|error| {
            mutation_diagnostic(
                row,
                DowngraderPlanStepKind::ApplyDeltaPatch,
                error.user_message(),
            )
        })?;
        if !canonical_path.starts_with(game_root) {
            warn!(
                event = "downgrader-execute-path-escape-rejected",
                relative_path = row.plan.relative_path,
                path = %path.display(),
                canonical_path = %canonical_path.display(),
                canonical_root = %game_root.display(),
                context,
                "Downgrader rejected a managed path escaping the canonical root"
            );
            return Err(format!(
                "{} {context} path escapes the game folder.",
                row.plan.display_name
            ));
        }
        Ok(())
    }

    fn prepare_current_backup(
        &self,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
        current_path: &Path,
        current_backup_path: &Path,
        initial_current_crc: &str,
    ) -> Result<(), String> {
        match self.read_crc(current_backup_path) {
            CrcProbe::Readable { crc32 } if crc32.eq_ignore_ascii_case(initial_current_crc) => {
                self.ensure_existing_file_within_root(
                    game_root,
                    current_backup_path,
                    row,
                    "current backup",
                )?;
                let current_crc = self.read_required_crc(current_path)?;
                let backup_crc = self.read_required_crc(current_backup_path)?;
                if !current_crc.eq_ignore_ascii_case(&backup_crc) {
                    return Err(format!(
                        "{} current file changed before current backup reuse.",
                        row.plan.display_name
                    ));
                }
                Ok(())
            }
            CrcProbe::Readable { crc32 } => {
                debug!(
                    event = "downgrader-execute-delete-invalid-current-backup",
                    relative_path = row.plan.relative_path,
                    backup_crc = crc32,
                    "Downgrader deleting invalid current backup"
                );
                self.remove_file_under_root(
                    game_root,
                    current_backup_path,
                    row,
                    DowngraderPlanStepKind::DeleteInvalidCurrentBackup,
                )?;
                self.create_current_backup(game_root, row, current_path, current_backup_path)
            }
            CrcProbe::Missing => {
                self.create_current_backup(game_root, row, current_path, current_backup_path)
            }
            CrcProbe::Unreadable { safe_message } => {
                Err(format!("{} {}", BACKUP_READ_FAILURE_MESSAGE, safe_message))
            }
        }
    }

    fn create_current_backup(
        &self,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
        current_path: &Path,
        current_backup_path: &Path,
    ) -> Result<(), String> {
        self.ensure_existing_file_within_root(game_root, current_path, row, "managed target")?;
        self.ensure_existing_or_parent_within_root(
            game_root,
            current_backup_path,
            row,
            "current backup",
        )?;
        let current_crc = self.read_required_crc(current_path)?;
        if !crc_is_supported_source_for_target(&current_crc, row.plan.target) {
            return Err(format!(
                "{} current CRC {current_crc} changed to an unsupported source before backup creation.",
                row.plan.display_name
            ));
        }
        self.filesystem
            .copy_file(current_path, current_backup_path)
            .map_err(|error| {
                mutation_diagnostic(
                    row,
                    DowngraderPlanStepKind::CreateCurrentBackup,
                    error.user_message(),
                )
            })?;
        let backup_crc = self.read_required_crc(current_backup_path)?;
        if !backup_crc.eq_ignore_ascii_case(&current_crc) {
            return Err(format!(
                "{} current backup CRC did not match the active file after copy.",
                row.plan.display_name
            ));
        }
        Ok(())
    }

    fn restore_desired_backup(
        &self,
        game_root: &Path,
        row: &DowngraderPreviewPlanRow,
        current_path: &Path,
        current_backup_path: &Path,
        desired_backup_path: &Path,
    ) -> Result<Vec<String>, String> {
        self.ensure_existing_file_within_root(game_root, current_path, row, "managed target")?;
        self.ensure_existing_file_within_root(
            game_root,
            desired_backup_path,
            row,
            "desired backup",
        )?;
        self.ensure_existing_file_within_root(
            game_root,
            current_backup_path,
            row,
            "current backup",
        )?;
        let desired_bytes = self
            .filesystem
            .read_bytes(desired_backup_path)
            .map_err(|error| {
                mutation_diagnostic(
                    row,
                    DowngraderPlanStepKind::RestoreDesiredBackup,
                    error.user_message(),
                )
            })?;
        let desired_crc = crc32_hex(&desired_bytes);
        if !crc_is_desired_target_for_plan(&desired_crc, row.plan.target) {
            return Err(format!(
                "{} desired backup changed before restore.",
                row.plan.display_name
            ));
        }

        self.ensure_existing_file_within_root(game_root, current_path, row, "managed target")?;
        self.filesystem
            .replace_file_bytes(current_path, &desired_bytes)
            .map_err(|error| {
                mutation_diagnostic(
                    row,
                    DowngraderPlanStepKind::RestoreDesiredBackup,
                    error.user_message(),
                )
            })?;
        let restored_crc = self.read_required_crc(current_path)?;
        if !crc_is_desired_target_for_plan(&restored_crc, row.plan.target) {
            return Err(format!(
                "{} restored output CRC {restored_crc} did not match target {}.",
                row.plan.display_name,
                row.plan.target.as_reference_str()
            ));
        }

        let mut diagnostics = Vec::new();
        if !row.options().keep_backups {
            self.remove_file_under_root(
                game_root,
                desired_backup_path,
                row,
                DowngraderPlanStepKind::RestoreDesiredBackup,
            )?;
            self.remove_file_under_root(
                game_root,
                current_backup_path,
                row,
                DowngraderPlanStepKind::DeleteCurrentBackup,
            )?;
        }
        diagnostics.push(format!(
            "{} restored from {}.",
            row.plan.display_name, row.plan.desired_backup_name
        ));
        Ok(diagnostics)
    }

    fn download_apply_and_cleanup<D, A, P>(
        &self,
        mut context: PatchApplyContext<'_, P>,
        downloader: &D,
        applier: &A,
    ) -> RowExecutionStatus
    where
        D: DeltaDownloader + ?Sized,
        A: DeltaApplier + ?Sized,
        P: FnMut(&DowngraderExecutionProgressEvent),
    {
        let row = context.row;
        if let Err(diagnostic) = self.ensure_existing_file_within_root(
            context.game_root,
            context.current_backup_path,
            row,
            "current backup",
        ) {
            return RowExecutionStatus::Failed {
                diagnostics: vec![diagnostic],
            };
        }
        let source_bytes = match self.filesystem.read_bytes(context.current_backup_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![mutation_diagnostic(
                        row,
                        DowngraderPlanStepKind::ApplyDeltaPatch,
                        error.user_message(),
                    )],
                };
            }
        };
        let patch_bytes = match self.read_or_download_patch(
            context.request_id,
            row,
            context.patch_path,
            context.integrity,
            downloader,
            context.progress_events,
            context.progress_callback,
        ) {
            Ok(bytes) => bytes,
            Err(diagnostic) => {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        };
        let output_bytes = match applier.apply_delta(
            &source_bytes,
            &patch_bytes,
            context.integrity.expected_output_bytes,
        ) {
            Ok(bytes) => bytes,
            Err(error) => {
                warn!(
                    event = "downgrader-execute-apply-failed",
                    request_id = context.request_id,
                    relative_path = row.plan.relative_path,
                    diagnostic = error.diagnostic(),
                    "Downgrader delta apply failed"
                );
                return RowExecutionStatus::Failed {
                    diagnostics: vec![format!("{} {}", error.user_message(), error.diagnostic())],
                };
            }
        };
        if output_bytes.len() as u64 != context.integrity.expected_output_bytes {
            return RowExecutionStatus::Failed {
                diagnostics: vec![format!(
                    "{} {} output length {} did not match expected {} bytes.",
                    OUTPUT_INTEGRITY_FAILURE_MESSAGE,
                    row.plan.display_name,
                    output_bytes.len(),
                    context.integrity.expected_output_bytes
                )],
            };
        }
        let output_crc = crc32_hex(&output_bytes);
        if !crc_is_desired_target_for_plan(&output_crc, row.plan.target) {
            return RowExecutionStatus::Failed {
                diagnostics: vec![format!(
                    "{} patch output CRC {output_crc} did not match target {}.",
                    row.plan.display_name,
                    row.plan.target.as_reference_str()
                )],
            };
        }

        if let Err(diagnostic) = self.ensure_existing_file_within_root(
            context.game_root,
            context.current_path,
            row,
            "managed target",
        ) {
            return RowExecutionStatus::Failed {
                diagnostics: vec![diagnostic],
            };
        }
        if let Err(error) = self
            .filesystem
            .replace_file_bytes(context.current_path, &output_bytes)
        {
            return RowExecutionStatus::Failed {
                diagnostics: vec![mutation_diagnostic(
                    row,
                    DowngraderPlanStepKind::ApplyDeltaPatch,
                    error.user_message(),
                )],
            };
        }
        let active_crc = match self.read_required_crc(context.current_path) {
            Ok(crc32) => crc32,
            Err(diagnostic) => {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        };
        if !crc_is_desired_target_for_plan(&active_crc, row.plan.target) {
            return RowExecutionStatus::Failed {
                diagnostics: vec![format!(
                    "{} active file CRC {active_crc} did not match target after replacement.",
                    row.plan.display_name
                )],
            };
        }
        if !row.options().keep_backups {
            if let Err(diagnostic) = self.remove_file_under_root(
                context.game_root,
                context.current_backup_path,
                row,
                DowngraderPlanStepKind::DeleteCurrentBackup,
            ) {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        }
        if row.options().delete_deltas {
            if let Err(diagnostic) = self.remove_file_under_root(
                context.game_root,
                context.patch_path,
                row,
                DowngraderPlanStepKind::DeleteDeltaPatch,
            ) {
                return RowExecutionStatus::Failed {
                    diagnostics: vec![diagnostic],
                };
            }
        }

        context.diagnostics.push(format!(
            "{} patched with {}.",
            row.plan.display_name, row.plan.patch_name
        ));
        RowExecutionStatus::Patched {
            diagnostics: context.diagnostics,
        }
    }

    fn read_or_download_patch<D, P>(
        &self,
        request_id: u64,
        row: &DowngraderPreviewPlanRow,
        patch_path: &Path,
        integrity: DowngraderPatchIntegrity,
        downloader: &D,
        progress_events: &mut Vec<DowngraderExecutionProgressEvent>,
        progress_callback: &mut P,
    ) -> Result<Vec<u8>, String>
    where
        D: DeltaDownloader + ?Sized,
        P: FnMut(&DowngraderExecutionProgressEvent),
    {
        match self.filesystem.metadata(patch_path) {
            Ok(metadata) if metadata.is_file() => {
                if metadata.len > MAX_DELTA_PATCH_BYTES {
                    return Err(format!(
                        "{} {} exceeds configured size limit of {} bytes.",
                        PATCH_INTEGRITY_FAILURE_MESSAGE, row.plan.patch_name, MAX_DELTA_PATCH_BYTES
                    ));
                }
                if metadata.len != integrity.expected_patch_bytes {
                    return Err(format!(
                        "{} {} size {} did not match pinned size {} bytes.",
                        PATCH_INTEGRITY_FAILURE_MESSAGE,
                        row.plan.patch_name,
                        metadata.len,
                        integrity.expected_patch_bytes
                    ));
                }
                let bytes = self.filesystem.read_bytes(patch_path).map_err(|error| {
                    mutation_diagnostic(
                        row,
                        DowngraderPlanStepKind::DownloadDelta,
                        error.user_message(),
                    )
                })?;
                verify_patch_bytes(row, integrity, &bytes)?;
                Ok(bytes)
            }
            Ok(_) => Err(mutation_diagnostic(
                row,
                DowngraderPlanStepKind::DownloadDelta,
                "Delta patch path is not a file.",
            )),
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                let mut progress = |progress: DowngraderProgress| {
                    let event = DowngraderExecutionProgressEvent {
                        request_id,
                        relative_path: row.plan.relative_path,
                        patch_name: row.plan.patch_name.clone(),
                        progress,
                    };
                    progress_callback(&event);
                    progress_events.push(event);
                };
                let bytes = downloader
                    .download_delta(&row.plan.patch_url, &mut progress)
                    .map_err(|error| {
                        warn!(
                            event = "downgrader-execute-download-failed",
                            request_id,
                            relative_path = row.plan.relative_path,
                            patch_name = row.plan.patch_name,
                            diagnostic = error.diagnostic(),
                            "Downgrader delta download failed"
                        );
                        format!(
                            "{} {}",
                            error.user_message(),
                            error.diagnostic().unwrap_or("")
                        )
                    })?;
                verify_patch_bytes(row, integrity, &bytes)?;
                self.filesystem
                    .write_bytes(patch_path, &bytes)
                    .map_err(|error| {
                        mutation_diagnostic(
                            row,
                            DowngraderPlanStepKind::DownloadDelta,
                            error.user_message(),
                        )
                    })?;
                Ok(bytes)
            }
            Err(error) => Err(mutation_diagnostic(
                row,
                DowngraderPlanStepKind::DownloadDelta,
                error.user_message(),
            )),
        }
    }

    fn patch_integrity_for(&self, patch_name: &str) -> Option<DowngraderPatchIntegrity> {
        self.patch_integrity_manifest
            .iter()
            .copied()
            .find(|entry| entry.patch_name.eq_ignore_ascii_case(patch_name))
    }

    fn remove_file_under_root(
        &self,
        game_root: &Path,
        path: &Path,
        row: &DowngraderPreviewPlanRow,
        step: DowngraderPlanStepKind,
    ) -> Result<(), String> {
        self.ensure_existing_file_within_root(game_root, path, row, step.as_str())?;
        self.remove_file_with_context(path, row, step)
    }

    fn remove_file_with_context(
        &self,
        path: &Path,
        row: &DowngraderPreviewPlanRow,
        step: DowngraderPlanStepKind,
    ) -> Result<(), String> {
        self.filesystem
            .remove_file(path)
            .map_err(|error| mutation_diagnostic(row, step, error.user_message()))
    }

    fn read_required_crc(&self, path: &Path) -> Result<String, String> {
        match self.read_crc(path) {
            CrcProbe::Readable { crc32 } => Ok(crc32),
            CrcProbe::Missing => Err(format!("{} target was not found.", path.display())),
            CrcProbe::Unreadable { safe_message } => Err(safe_message),
        }
    }
}

struct ManagedExecutionPaths {
    current_path: PathBuf,
    current_backup_path: PathBuf,
    desired_backup_path: PathBuf,
    patch_path: PathBuf,
    integrity: DowngraderPatchIntegrity,
}

struct PatchApplyContext<'a, P> {
    request_id: u64,
    game_root: &'a Path,
    row: &'a DowngraderPreviewPlanRow,
    current_path: &'a Path,
    current_backup_path: &'a Path,
    patch_path: &'a Path,
    integrity: DowngraderPatchIntegrity,
    progress_events: &'a mut Vec<DowngraderExecutionProgressEvent>,
    progress_callback: &'a mut P,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RowExecutionStatus {
    Patched { diagnostics: Vec<String> },
    SkippedUnsupported { diagnostics: Vec<String> },
    Failed { diagnostics: Vec<String> },
}

fn execution_file_result(
    row: &DowngraderPreviewPlanRow,
    outcome: DowngraderExecutionOutcome,
    log_row: DowngraderExecutionLogRow,
    diagnostics: Vec<String>,
) -> DowngraderExecutionFileResult {
    DowngraderExecutionFileResult {
        relative_path: row.plan.relative_path,
        display_name: row.plan.display_name,
        outcome,
        log_row,
        diagnostics,
    }
}

fn failed_execution_file_result(
    row: &DowngraderPreviewPlanRow,
    diagnostics: Vec<String>,
) -> DowngraderExecutionFileResult {
    warn!(
        event = "downgrader-execute-row-failed",
        relative_path = row.plan.relative_path,
        diagnostic_count = diagnostics.len(),
        "Downgrader execution row failed safely"
    );
    execution_file_result(
        row,
        DowngraderExecutionOutcome::Failed,
        failed_patching_log_row(row.plan.display_name),
        diagnostics,
    )
}

fn verify_patch_bytes(
    row: &DowngraderPreviewPlanRow,
    integrity: DowngraderPatchIntegrity,
    bytes: &[u8],
) -> Result<(), String> {
    let actual_len = bytes.len() as u64;
    if actual_len > MAX_DELTA_PATCH_BYTES {
        return Err(format!(
            "{} {} exceeds configured size limit of {} bytes.",
            PATCH_INTEGRITY_FAILURE_MESSAGE, row.plan.patch_name, MAX_DELTA_PATCH_BYTES
        ));
    }
    if actual_len != integrity.expected_patch_bytes {
        return Err(format!(
            "{} {} size {actual_len} did not match pinned size {} bytes.",
            PATCH_INTEGRITY_FAILURE_MESSAGE, row.plan.patch_name, integrity.expected_patch_bytes
        ));
    }
    let actual_hash = sha256_hex(bytes);
    if !actual_hash.eq_ignore_ascii_case(integrity.sha256_hex) {
        return Err(format!(
            "{} {} SHA-256 {actual_hash} did not match pinned value.",
            PATCH_INTEGRITY_FAILURE_MESSAGE, row.plan.patch_name
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn mutation_diagnostic(
    row: &DowngraderPreviewPlanRow,
    step: DowngraderPlanStepKind,
    safe_message: &str,
) -> String {
    format!(
        "{} failed during {}: {safe_message}",
        row.plan.display_name,
        step.as_str()
    )
}

trait DowngraderPreviewPlanRowExt {
    fn options(&self) -> DowngraderOptionsSnapshot;
}

impl DowngraderPreviewPlanRowExt for DowngraderPreviewPlanRow {
    fn options(&self) -> DowngraderOptionsSnapshot {
        DowngraderOptionsSnapshot::new(
            self.plan.target,
            self.steps
                .iter()
                .all(|step| step.kind != DowngraderPlanStepKind::DeleteCurrentBackup),
            self.steps
                .iter()
                .any(|step| step.kind == DowngraderPlanStepKind::DeleteDeltaPatch),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CrcProbe {
    Readable { crc32: String },
    Missing,
    Unreadable { safe_message: String },
}

fn fail_row(row: &mut DowngraderPreviewPlanRow, safe_message: impl Into<String>) {
    let safe_message = safe_message.into();
    row.failure = Some(safe_message.clone());
    row.steps.push(DowngraderPlanStep::new(
        DowngraderPlanStepKind::PlanFailure,
        format!("Cannot plan {}: {safe_message}", row.plan.display_name),
    ));
    warn!(
        event = "downgrader-plan-row-failed",
        relative_path = row.plan.relative_path,
        safe_message,
        "Downgrader preview row failed safely"
    );
}

fn definition_for_status_row(
    row: &DowngraderStatusFile,
) -> Result<DowngraderFileDefinition, DowngraderServiceError> {
    DOWNGRADER_FILE_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| definition.relative_path == row.relative_path)
        .ok_or_else(|| DowngraderServiceError::UnsafeManagedPath {
            relative_path: row.relative_path.to_owned(),
            safe_message: UNSAFE_MANAGED_PATH_MESSAGE.to_owned(),
        })
}

fn default_target_from_fallout4(status: DowngraderInstallStatus) -> DowngraderTarget {
    if status == DowngraderInstallStatus::OldGen {
        DowngraderTarget::OldGen
    } else {
        DowngraderTarget::NextGen
    }
}

fn display_status_for(
    relative_path: &str,
    detected_status: DowngraderInstallStatus,
    fallout4_status: DowngraderInstallStatus,
) -> DowngraderInstallStatus {
    if !relative_path.eq_ignore_ascii_case("steam_api64.dll")
        || detected_status != DowngraderInstallStatus::NextGenAnniversary
    {
        return detected_status;
    }

    match fallout4_status {
        DowngraderInstallStatus::Anniversary => DowngraderInstallStatus::Anniversary,
        DowngraderInstallStatus::NextGen => DowngraderInstallStatus::NextGen,
        _ => DowngraderInstallStatus::NextGenAnniversary,
    }
}

fn crc_is_supported_source_for_target(crc32: &str, target: DowngraderTarget) -> bool {
    accepted_source_crcs(target)
        .iter()
        .any(|accepted| accepted.eq_ignore_ascii_case(crc32))
}

fn crc_is_desired_target_for_plan(crc32: &str, target: DowngraderTarget) -> bool {
    desired_target_crcs(target)
        .iter()
        .any(|accepted| accepted.eq_ignore_ascii_case(crc32))
}

fn accepted_source_crcs(target: DowngraderTarget) -> &'static Vec<&'static str> {
    static OLD_GEN_TARGET_SOURCE_CRCS: OnceLock<Vec<&'static str>> = OnceLock::new();
    static NEXT_GEN_TARGET_SOURCE_CRCS: OnceLock<Vec<&'static str>> = OnceLock::new();
    match target {
        DowngraderTarget::OldGen => OLD_GEN_TARGET_SOURCE_CRCS
            .get_or_init(|| accepted_source_crcs_for_target(DowngraderTarget::OldGen)),
        DowngraderTarget::NextGen => NEXT_GEN_TARGET_SOURCE_CRCS
            .get_or_init(|| accepted_source_crcs_for_target(DowngraderTarget::NextGen)),
    }
}

fn desired_target_crcs(target: DowngraderTarget) -> &'static Vec<&'static str> {
    static OLD_GEN_TARGET_CRCS: OnceLock<Vec<&'static str>> = OnceLock::new();
    static NEXT_GEN_TARGET_CRCS: OnceLock<Vec<&'static str>> = OnceLock::new();
    match target {
        DowngraderTarget::OldGen => {
            OLD_GEN_TARGET_CRCS.get_or_init(|| crcs_for_status(DowngraderInstallStatus::OldGen))
        }
        DowngraderTarget::NextGen => {
            NEXT_GEN_TARGET_CRCS.get_or_init(|| crcs_for_status(DowngraderInstallStatus::NextGen))
        }
    }
}

fn backup_path_for(current_path: &Path, backup_name: &str) -> PathBuf {
    current_path
        .parent()
        .map(|parent| parent.join(backup_name))
        .unwrap_or_else(|| PathBuf::from(backup_name))
}

fn crc32_hex(bytes: &[u8]) -> String {
    format!("{:08X}", crc32fast::hash(bytes))
}

fn validate_game_root_path(root: &Path) -> Result<(), DowngraderServiceError> {
    let has_component = root.components().next().is_some();
    let has_parent = root
        .components()
        .any(|component| matches!(component, Component::ParentDir));
    if !has_component || has_parent {
        warn!(
            event = "downgrader-root-unsafe",
            root = %root.display(),
            "Downgrader rejected unsafe game root"
        );
        return Err(DowngraderServiceError::InvalidGameRoot {
            root: root.to_path_buf(),
            safe_message: UNSAFE_ROOT_MESSAGE.to_owned(),
        });
    }
    Ok(())
}

fn validate_managed_definitions(root: &Path) -> Result<(), DowngraderServiceError> {
    for definition in DOWNGRADER_FILE_DEFINITIONS {
        resolve_managed_relative_path(root, definition.relative_path)?;
    }
    Ok(())
}

fn resolve_managed_relative_path(
    root: &Path,
    relative_path: &'static str,
) -> Result<PathBuf, DowngraderServiceError> {
    if relative_path.is_empty()
        || relative_path.contains(':')
        || Path::new(relative_path).is_absolute()
        || Path::new(relative_path).components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        return unsafe_managed_path(relative_path);
    }

    let mut resolved = root.to_path_buf();
    for segment in relative_path.split(['\\', '/']) {
        if segment.is_empty() || segment == "." || segment == ".." {
            return unsafe_managed_path(relative_path);
        }
        resolved.push(segment);
    }

    if !resolved.starts_with(root) {
        return unsafe_managed_path(relative_path);
    }
    Ok(resolved)
}

fn unsafe_managed_path(relative_path: &'static str) -> Result<PathBuf, DowngraderServiceError> {
    warn!(
        event = "downgrader-managed-path-unsafe",
        relative_path, "Downgrader rejected unsafe managed path"
    );
    Err(DowngraderServiceError::UnsafeManagedPath {
        relative_path: relative_path.to_owned(),
        safe_message: UNSAFE_MANAGED_PATH_MESSAGE.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use crate::{
        domain::{
            discovery::Fallout4Installation,
            downgrader::{
                DowngraderInstallStatus, DowngraderOptionsSnapshot, DowngraderPlanStepKind,
                DowngraderTarget,
            },
        },
        platform::{
            PlatformError, PlatformErrorKind, PlatformOperation, PlatformResult,
            filesystem::{DirectoryEntry, FileMetadata, FileType, Filesystem, WritableFilesystem},
        },
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        UnreadableFile,
        SymlinkFile(Vec<u8>),
        ReparseFile(Vec<u8>),
    }

    #[derive(Debug, Default, Clone)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
        metadata_reads: RefCell<Vec<PathBuf>>,
        read_files: RefCell<Vec<PathBuf>>,
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

        fn with_crc_file(self, path: impl Into<PathBuf>, crc32: &str) -> Self {
            self.with_file(path, bytes_with_crc(crc32))
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

    fn safe_platform_error(
        path: &Path,
        operation: PlatformOperation,
        kind: PlatformErrorKind,
    ) -> PlatformError {
        PlatformError::new(
            operation,
            path.display().to_string(),
            kind,
            format!(
                "{} target could not be accessed because permission was denied.",
                operation.label()
            ),
        )
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> Result<FileMetadata, PlatformError> {
            self.metadata_reads.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) | FakeNode::SymlinkFile(bytes) => {
                    Ok(FileMetadata::new(FileType::File, bytes.len() as u64))
                }
                FakeNode::ReparseFile(bytes) => Ok(FileMetadata::reparse_point(
                    FileType::File,
                    bytes.len() as u64,
                )),
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
                FakeNode::UnreadableFile => Ok(FileMetadata::new(FileType::File, 0)),
            }
        }

        fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, PlatformError> {
            self.read_files.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes)
                | FakeNode::SymlinkFile(bytes)
                | FakeNode::ReparseFile(bytes) => Ok(bytes.clone()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => Err(safe_platform_error(
                    path,
                    PlatformOperation::ReadFile,
                    PlatformErrorKind::PermissionDenied,
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

        fn read_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(Vec::new())
        }

        fn walk_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(Vec::new())
        }
    }

    fn installation() -> Fallout4Installation {
        Fallout4Installation::new(game_root())
    }

    fn game_root() -> PathBuf {
        PathBuf::from("Game")
    }

    fn options(
        target: DowngraderTarget,
        keep_backups: bool,
        delete_deltas: bool,
    ) -> DowngraderOptionsSnapshot {
        DowngraderOptionsSnapshot::new(target, keep_backups, delete_deltas)
    }

    fn full_status_fs(fallout4_crc: &str) -> FakeFilesystem {
        FakeFilesystem::default()
            .with_dir(game_root())
            .with_crc_file("Game/Fallout4.exe", fallout4_crc)
            .with_crc_file("Game/Fallout4Launcher.exe", "F6A06FF5")
            .with_crc_file("Game/steam_api64.dll", "E36E7B4D")
            .with_crc_file("Game/CreationKit.exe", "481CCE95")
            .with_crc_file("Game/Tools/Archive2/Archive2.exe", "71A5240B")
            .with_crc_file("Game/Tools/Archive2/Archive2Interop.dll", "EFBE3622")
    }

    fn fallout4_next_gen_others_old_gen_fs() -> FakeFilesystem {
        FakeFilesystem::default()
            .with_dir(game_root())
            .with_crc_file("Game/Fallout4.exe", "C5965A2E")
            .with_crc_file("Game/Fallout4Launcher.exe", "02445570")
            .with_crc_file("Game/steam_api64.dll", "BBD912FC")
            .with_crc_file("Game/CreationKit.exe", "0F5C065B")
            .with_crc_file("Game/Tools/Archive2/Archive2.exe", "4CDFC7B5")
            .with_crc_file("Game/Tools/Archive2/Archive2Interop.dll", "850D36A9")
    }

    fn row_steps(row: &DowngraderPreviewPlanRow) -> Vec<DowngraderPlanStepKind> {
        row.steps.iter().map(|step| step.kind).collect()
    }

    fn bytes_with_crc(hex: &str) -> Vec<u8> {
        let target = u32::from_str_radix(hex, 16).expect("test CRC hex");
        let base = crc32fast::hash(&[0, 0, 0, 0]);
        let mut columns = [0_u32; 32];
        for bit in 0..32 {
            let mut suffix = [0_u8; 4];
            suffix[bit / 8] = 1 << (bit % 8);
            columns[bit] = crc32fast::hash(&suffix) ^ base;
        }

        let mut basis = [0_u32; 32];
        let mut basis_vector = [0_u32; 32];
        for (bit, column) in columns.into_iter().enumerate() {
            let mut value = column;
            let mut vector = 1_u32 << bit;
            for pivot in (0..32).rev() {
                if value & (1_u32 << pivot) == 0 {
                    continue;
                }
                if basis[pivot] == 0 {
                    basis[pivot] = value;
                    basis_vector[pivot] = vector;
                    break;
                }
                value ^= basis[pivot];
                vector ^= basis_vector[pivot];
            }
        }

        let mut remainder = base ^ target;
        let mut solution = 0_u32;
        for pivot in (0..32).rev() {
            if remainder & (1_u32 << pivot) == 0 {
                continue;
            }
            assert_ne!(basis[pivot], 0, "CRC matrix should be full rank");
            remainder ^= basis[pivot];
            solution ^= basis_vector[pivot];
        }
        assert_eq!(remainder, 0, "target CRC should be reachable by four bytes");

        let mut suffix = [0_u8; 4];
        for bit in 0..32 {
            if solution & (1_u32 << bit) != 0 {
                suffix[bit / 8] |= 1 << (bit % 8);
            }
        }
        assert_eq!(crc32_hex(&suffix), hex.to_ascii_uppercase());
        suffix.to_vec()
    }

    #[test]
    fn downgrader_service_plan_status_classifies_rows_and_default_target() {
        let fs = full_status_fs("C5965A2E");
        let service = DowngraderService::new(&fs);

        let snapshot = service
            .status_snapshot(DowngraderStatusRequest::new(7, Some(&installation())))
            .expect("status snapshot");

        assert_eq!(snapshot.request_id, 7);
        assert_eq!(snapshot.rows.len(), 6);
        assert_eq!(snapshot.default_target, DowngraderTarget::NextGen);
        assert_eq!(snapshot.rows[0].display_name, "Fallout4.exe");
        assert_eq!(
            snapshot.rows[5].relative_path,
            "Tools\\Archive2\\Archive2Interop.dll"
        );
        assert_eq!(
            snapshot
                .status_for("Fallout4.exe")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            snapshot
                .status_for("steam_api64.dll")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::NextGenAnniversary)
        );
        assert_eq!(
            snapshot
                .status_for("steam_api64.dll")
                .map(|row| row.display_status),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert!(!snapshot.unknown_game);
        assert!(!snapshot.unknown_creation_kit);

        let old_gen = full_status_fs("C6053902");
        let snapshot = DowngraderService::new(&old_gen)
            .status_snapshot(DowngraderStatusRequest::new(8, Some(&installation())))
            .expect("old-gen status snapshot");
        assert_eq!(snapshot.default_target, DowngraderTarget::OldGen);
        assert_eq!(
            snapshot
                .status_for("steam_api64.dll")
                .map(|row| row.display_status),
            Some(DowngraderInstallStatus::NextGenAnniversary)
        );

        let anniversary = full_status_fs("CF47788D");
        let snapshot = DowngraderService::new(&anniversary)
            .status_snapshot(DowngraderStatusRequest::new(9, Some(&installation())))
            .expect("anniversary status snapshot");
        assert_eq!(
            snapshot
                .status_for("steam_api64.dll")
                .map(|row| row.display_status),
            Some(DowngraderInstallStatus::Anniversary)
        );
        assert_eq!(snapshot.default_target, DowngraderTarget::NextGen);
    }

    #[test]
    fn downgrader_service_plan_marks_missing_unknown_obsolete_and_anniversary_rows_safely() {
        let fs = FakeFilesystem::default()
            .with_dir(game_root())
            .with_crc_file("Game/Fallout4.exe", "97DA3E03")
            .with_file(
                "Game/Fallout4Launcher.exe",
                b"not a known launcher crc".to_vec(),
            )
            .with_unreadable_file("Game/steam_api64.dll")
            .with_crc_file("Game/CreationKit.exe", "49E45284");
        let service = DowngraderService::new(&fs);

        let snapshot = service
            .status_snapshot(DowngraderStatusRequest::new(10, Some(&installation())))
            .expect("status snapshot");

        assert_eq!(snapshot.rows.len(), 6);
        assert_eq!(
            snapshot
                .status_for("Fallout4.exe")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::Obsolete)
        );
        assert_eq!(
            snapshot
                .status_for("Fallout4Launcher.exe")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::Unknown)
        );
        assert_eq!(
            snapshot
                .status_for("steam_api64.dll")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::Unknown)
        );
        assert!(
            snapshot
                .status_for("steam_api64.dll")
                .and_then(|row| row.read_error.as_deref())
                .is_some_and(|message| message.contains("permission was denied"))
        );
        assert_eq!(
            snapshot
                .status_for("Archive2.exe")
                .map(|row| row.detected_status),
            Some(DowngraderInstallStatus::NotFound)
        );
        assert!(snapshot.unknown_game);
        assert!(snapshot.unknown_creation_kit);
    }

    #[test]
    fn downgrader_service_plan_builds_restore_from_valid_desired_backup() {
        let fs = fallout4_next_gen_others_old_gen_fs()
            .with_crc_file("Game/Fallout4_upgradeBackup.exe", "C6053902")
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E");
        let before_nodes = fs.nodes.clone();
        let service = DowngraderService::new(&fs);

        let plan = service
            .preview_plan(DowngraderPlanRequest::new(
                11,
                Some(&installation()),
                options(DowngraderTarget::OldGen, false, false),
            ))
            .expect("preview plan");

        let fallout = plan.rows.first().expect("Fallout4 plan row");
        assert_eq!(
            row_steps(fallout),
            vec![
                DowngraderPlanStepKind::ReuseCurrentBackup,
                DowngraderPlanStepKind::RestoreDesiredBackup,
                DowngraderPlanStepKind::DeleteCurrentBackup,
            ]
        );
        assert!(fallout.restores_from_backup());
        assert!(!fallout.requires_download());
        assert_eq!(
            fallout
                .desired_backup
                .as_ref()
                .and_then(|probe| probe.crc32.as_deref()),
            Some("C6053902")
        );
        assert!(plan.can_execute);
        assert_eq!(plan.counts.restore_from_backup_rows, 1);
        assert_eq!(
            fs.nodes, before_nodes,
            "preview planning must not mutate fake files"
        );
    }

    #[test]
    fn downgrader_service_plan_builds_invalid_backup_cleanup_download_patch_and_optional_cleanup() {
        let fs = fallout4_next_gen_others_old_gen_fs()
            .with_crc_file("Game/Fallout4_upgradeBackup.exe", "F6A06FF5")
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C6053902");
        let service = DowngraderService::new(&fs);

        let plan = service
            .preview_plan(DowngraderPlanRequest::new(
                12,
                Some(&installation()),
                options(DowngraderTarget::OldGen, false, true),
            ))
            .expect("preview plan");

        let fallout = &plan.rows[0];
        assert_eq!(
            row_steps(fallout),
            vec![
                DowngraderPlanStepKind::DeleteInvalidCurrentBackup,
                DowngraderPlanStepKind::CreateCurrentBackup,
                DowngraderPlanStepKind::DeleteInvalidDesiredBackup,
                DowngraderPlanStepKind::DownloadDelta,
                DowngraderPlanStepKind::ApplyDeltaPatch,
                DowngraderPlanStepKind::DeleteCurrentBackup,
                DowngraderPlanStepKind::DeleteDeltaPatch,
            ]
        );
        assert!(fallout.requires_download());
        assert_eq!(plan.counts.delta_download_rows, 1);
        assert_eq!(plan.counts.mutating_step_count, 7);
        assert!(
            fs.read_files
                .borrow()
                .iter()
                .any(|path| path == &PathBuf::from("Game/Fallout4_upgradeBackup.exe")),
            "plan should read desired backup CRC for accuracy"
        );
        assert!(
            fs.read_files
                .borrow()
                .iter()
                .all(|path| path.extension().and_then(|ext| ext.to_str()) != Some("xdelta")),
            "preview must not read or download delta files"
        );
    }

    #[test]
    fn downgrader_service_plan_skips_reference_rows_in_order() {
        let fs = FakeFilesystem::default()
            .with_dir(game_root())
            .with_crc_file("Game/Fallout4.exe", "C6053902")
            .with_crc_file("Game/Fallout4Launcher.exe", "02445570")
            .with_crc_file("Game/steam_api64.dll", "BBD912FC")
            .with_crc_file("Game/CreationKit.exe", "49E45284")
            .with_file(
                "Game/Tools/Archive2/Archive2.exe",
                b"unknown archive2".to_vec(),
            );
        let service = DowngraderService::new(&fs);

        let plan = service
            .preview_plan(DowngraderPlanRequest::new(
                13,
                Some(&installation()),
                options(DowngraderTarget::OldGen, true, false),
            ))
            .expect("preview plan");

        assert_eq!(
            plan.rows
                .iter()
                .map(|row| row.plan.display_name)
                .collect::<Vec<_>>(),
            vec![
                "Fallout4.exe",
                "Fallout4Launcher.exe",
                "steam_api64.dll",
                "CreationKit.exe",
                "Archive2.exe",
                "Archive2Interop.dll",
            ]
        );
        assert_eq!(
            row_steps(&plan.rows[0]),
            vec![DowngraderPlanStepKind::SkipAlreadyDesired]
        );
        assert_eq!(
            row_steps(&plan.rows[3]),
            vec![DowngraderPlanStepKind::SkipUnsupportedVersion]
        );
        assert_eq!(
            row_steps(&plan.rows[4]),
            vec![DowngraderPlanStepKind::SkipUnsupportedVersion]
        );
        assert_eq!(
            row_steps(&plan.rows[5]),
            vec![DowngraderPlanStepKind::SkipNotFound]
        );
        assert!(plan.can_execute);
    }

    #[test]
    fn downgrader_service_plan_fails_row_when_current_or_backup_cannot_be_read() {
        let fs = full_status_fs("C5965A2E")
            .with_unreadable_file("Game/Fallout4_upgradeBackup.exe")
            .with_unreadable_file("Game/Fallout4_downgradeBackup.exe");
        let service = DowngraderService::new(&fs);

        let plan = service
            .preview_plan(DowngraderPlanRequest::new(
                14,
                Some(&installation()),
                options(DowngraderTarget::OldGen, true, false),
            ))
            .expect("preview plan");
        let fallout = &plan.rows[0];
        assert_eq!(
            row_steps(fallout),
            vec![DowngraderPlanStepKind::PlanFailure]
        );
        assert!(
            fallout
                .failure
                .as_deref()
                .is_some_and(|failure| failure.contains(BACKUP_READ_FAILURE_MESSAGE))
        );
        assert!(!plan.can_execute);

        let fs = FakeFilesystem::default()
            .with_dir(game_root())
            .with_unreadable_file("Game/Fallout4.exe");
        let service = DowngraderService::new(&fs);
        let plan = service
            .preview_plan(DowngraderPlanRequest::new(
                15,
                Some(&installation()),
                options(DowngraderTarget::OldGen, true, false),
            ))
            .expect("preview plan with unreadable current");
        assert_eq!(
            row_steps(&plan.rows[0]),
            vec![DowngraderPlanStepKind::PlanFailure]
        );
        assert!(!plan.can_execute);
    }

    #[test]
    fn downgrader_service_plan_rejects_missing_and_unsafe_roots() {
        let fs = FakeFilesystem::default();
        let service = DowngraderService::new(&fs);

        let error = service
            .status_snapshot(DowngraderStatusRequest::new(16, None))
            .expect_err("missing installation should fail safely");
        assert_eq!(error.user_message(), FALLOUT4_NOT_FOUND_MESSAGE);

        let missing_installation = Fallout4Installation::new("MissingGame");
        let error = service
            .status_snapshot(DowngraderStatusRequest::new(
                17,
                Some(&missing_installation),
            ))
            .expect_err("missing root should fail safely");
        assert_eq!(error.user_message(), FALLOUT4_NOT_FOUND_MESSAGE);

        let unsafe_installation = Fallout4Installation::new("../Game");
        let error = service
            .status_snapshot(DowngraderStatusRequest::new(18, Some(&unsafe_installation)))
            .expect_err("unsafe root should fail safely");
        assert_eq!(error.user_message(), UNSAFE_ROOT_MESSAGE);
    }

    #[test]
    fn downgrader_service_plan_rejects_escaping_managed_paths_before_output() {
        let root = Path::new("Game");

        assert!(resolve_managed_relative_path(root, "Tools\\Archive2\\Archive2.exe").is_ok());
        let error = resolve_managed_relative_path(root, "..\\Fallout4.exe")
            .expect_err("parent escape should be rejected");
        assert_eq!(error.user_message(), UNSAFE_MANAGED_PATH_MESSAGE);
        let error = resolve_managed_relative_path(root, "/Fallout4.exe")
            .expect_err("absolute managed path should be rejected");
        assert_eq!(error.user_message(), UNSAFE_MANAGED_PATH_MESSAGE);
        let error = resolve_managed_relative_path(root, "C:\\Fallout4.exe")
            .expect_err("drive-qualified managed path should be rejected");
        assert_eq!(error.user_message(), UNSAFE_MANAGED_PATH_MESSAGE);
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RecordedWriteOp {
        Write(PathBuf),
        Copy { from: PathBuf, to: PathBuf },
        Rename { from: PathBuf, to: PathBuf },
        Remove(PathBuf),
    }

    #[derive(Debug, Default)]
    struct RecordingWritableFilesystem {
        nodes: RefCell<BTreeMap<PathBuf, FakeNode>>,
        operations: RefCell<Vec<RecordedWriteOp>>,
        failures: RefCell<BTreeMap<(PlatformOperation, PathBuf), PlatformErrorKind>>,
    }

    impl RecordingWritableFilesystem {
        fn with_dir(self, path: impl Into<PathBuf>) -> Self {
            self.nodes
                .borrow_mut()
                .insert(path.into(), FakeNode::Directory);
            self
        }

        fn with_file(self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes
                .borrow_mut()
                .insert(path, FakeNode::File(bytes.into()));
            self
        }

        fn with_crc_file(self, path: impl Into<PathBuf>, crc32: &str) -> Self {
            self.with_file(path, bytes_with_crc(crc32))
        }

        fn with_failure(
            self,
            operation: PlatformOperation,
            path: impl Into<PathBuf>,
            kind: PlatformErrorKind,
        ) -> Self {
            self.failures
                .borrow_mut()
                .insert((operation, path.into()), kind);
            self
        }

        fn ensure_parent_dirs(&self, path: &Path) {
            let mut parents = Vec::new();
            let mut current = path.parent();
            while let Some(parent) = current {
                if parent.as_os_str().is_empty() {
                    break;
                }
                parents.push(parent.to_path_buf());
                current = parent.parent();
            }
            let mut nodes = self.nodes.borrow_mut();
            for parent in parents.into_iter().rev() {
                nodes.entry(parent).or_insert(FakeNode::Directory);
            }
        }

        fn maybe_fail(
            &self,
            operation: PlatformOperation,
            path: &Path,
        ) -> Result<(), PlatformError> {
            if let Some(kind) = self
                .failures
                .borrow()
                .get(&(operation, path.to_path_buf()))
                .copied()
            {
                return Err(PlatformError::new(
                    operation,
                    path.display().to_string(),
                    kind,
                    format!(
                        "{} target could not be accessed because permission was denied.",
                        operation.label()
                    ),
                ));
            }
            Ok(())
        }

        fn node(
            &self,
            path: &Path,
            operation: PlatformOperation,
        ) -> Result<FakeNode, PlatformError> {
            self.nodes.borrow().get(path).cloned().ok_or_else(|| {
                PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )
            })
        }

        fn has_file(&self, path: impl AsRef<Path>) -> bool {
            matches!(
                self.nodes.borrow().get(path.as_ref()),
                Some(FakeNode::File(_) | FakeNode::SymlinkFile(_) | FakeNode::ReparseFile(_))
            )
        }

        fn file_crc(&self, path: impl AsRef<Path>) -> Option<String> {
            match self.nodes.borrow().get(path.as_ref()) {
                Some(
                    FakeNode::File(bytes)
                    | FakeNode::SymlinkFile(bytes)
                    | FakeNode::ReparseFile(bytes),
                ) => Some(crc32_hex(bytes)),
                _ => None,
            }
        }

        fn operations(&self) -> Vec<RecordedWriteOp> {
            self.operations.borrow().clone()
        }
    }

    impl Filesystem for RecordingWritableFilesystem {
        fn metadata(&self, path: &Path) -> Result<FileMetadata, PlatformError> {
            self.maybe_fail(PlatformOperation::ReadMetadata, path)?;
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) | FakeNode::SymlinkFile(bytes) => {
                    Ok(FileMetadata::new(FileType::File, bytes.len() as u64))
                }
                FakeNode::ReparseFile(bytes) => Ok(FileMetadata::reparse_point(
                    FileType::File,
                    bytes.len() as u64,
                )),
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
                FakeNode::UnreadableFile => Ok(FileMetadata::new(FileType::File, 0)),
            }
        }

        fn symlink_metadata(&self, path: &Path) -> Result<FileMetadata, PlatformError> {
            self.maybe_fail(PlatformOperation::ReadMetadata, path)?;
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata::new(FileType::File, bytes.len() as u64)),
                FakeNode::SymlinkFile(bytes) => {
                    Ok(FileMetadata::new(FileType::Symlink, bytes.len() as u64))
                }
                FakeNode::ReparseFile(bytes) => Ok(FileMetadata::reparse_point(
                    FileType::File,
                    bytes.len() as u64,
                )),
                FakeNode::Directory => Ok(FileMetadata::new(FileType::Directory, 0)),
                FakeNode::UnreadableFile => Ok(FileMetadata::new(FileType::File, 0)),
            }
        }

        fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, PlatformError> {
            self.maybe_fail(PlatformOperation::ReadFile, path)?;
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes)
                | FakeNode::SymlinkFile(bytes)
                | FakeNode::ReparseFile(bytes) => Ok(bytes),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => Err(safe_platform_error(
                    path,
                    PlatformOperation::ReadFile,
                    PlatformErrorKind::PermissionDenied,
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

        fn read_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(Vec::new())
        }

        fn walk_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PlatformError> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(Vec::new())
        }
    }

    impl WritableFilesystem for RecordingWritableFilesystem {
        fn write_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
            self.maybe_fail(PlatformOperation::WriteFile, path)?;
            self.ensure_parent_dirs(path);
            self.nodes
                .borrow_mut()
                .insert(path.to_path_buf(), FakeNode::File(bytes.to_vec()));
            self.operations
                .borrow_mut()
                .push(RecordedWriteOp::Write(path.to_path_buf()));
            Ok(())
        }

        fn replace_file_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
            self.maybe_fail(PlatformOperation::WriteFile, path)?;
            if !self.has_file(path) {
                return Err(PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File write target was not found.",
                ));
            }
            self.ensure_parent_dirs(path);
            self.nodes
                .borrow_mut()
                .insert(path.to_path_buf(), FakeNode::File(bytes.to_vec()));
            self.operations
                .borrow_mut()
                .push(RecordedWriteOp::Write(path.to_path_buf()));
            Ok(())
        }

        fn copy_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
            self.maybe_fail(PlatformOperation::CopyFile, to)?;
            let bytes = match self.node(from, PlatformOperation::CopyFile)? {
                FakeNode::File(bytes) => bytes,
                FakeNode::Directory
                | FakeNode::UnreadableFile
                | FakeNode::SymlinkFile(_)
                | FakeNode::ReparseFile(_) => {
                    return Err(PlatformError::new(
                        PlatformOperation::CopyFile,
                        from.display().to_string(),
                        PlatformErrorKind::InvalidInput,
                        "File copy target is invalid.",
                    ));
                }
            };
            self.ensure_parent_dirs(to);
            self.nodes
                .borrow_mut()
                .insert(to.to_path_buf(), FakeNode::File(bytes));
            self.operations.borrow_mut().push(RecordedWriteOp::Copy {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
            });
            Ok(())
        }

        fn rename_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
            self.maybe_fail(PlatformOperation::RenameFile, to)?;
            let node = self.node(from, PlatformOperation::RenameFile)?;
            if !matches!(node, FakeNode::File(_)) {
                return Err(PlatformError::new(
                    PlatformOperation::RenameFile,
                    from.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File rename target is invalid.",
                ));
            }
            self.ensure_parent_dirs(to);
            let mut nodes = self.nodes.borrow_mut();
            nodes.remove(from);
            nodes.insert(to.to_path_buf(), node);
            self.operations.borrow_mut().push(RecordedWriteOp::Rename {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
            });
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> PlatformResult<()> {
            self.maybe_fail(PlatformOperation::RemoveFile, path)?;
            match self.nodes.borrow_mut().remove(path) {
                Some(FakeNode::File(_))
                | Some(FakeNode::UnreadableFile)
                | Some(FakeNode::SymlinkFile(_))
                | Some(FakeNode::ReparseFile(_)) => {
                    self.operations
                        .borrow_mut()
                        .push(RecordedWriteOp::Remove(path.to_path_buf()));
                    Ok(())
                }
                Some(FakeNode::Directory) => Err(PlatformError::new(
                    PlatformOperation::RemoveFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File removal target is invalid.",
                )),
                None => Err(PlatformError::new(
                    PlatformOperation::RemoveFile,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File removal target was not found.",
                )),
            }
        }
    }

    #[derive(Debug)]
    struct RecordingDownloader {
        calls: RefCell<Vec<String>>,
        result: Result<Vec<u8>, DowngraderDownloadError>,
        progress: Vec<f32>,
    }

    impl RecordingDownloader {
        fn ok(bytes: impl Into<Vec<u8>>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                result: Ok(bytes.into()),
                progress: vec![0.0, 50.0, 100.0],
            }
        }

        fn failing() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                result: Err(DowngraderDownloadError::request("network unavailable")),
                progress: vec![0.0],
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl DeltaDownloader for RecordingDownloader {
        fn download_delta(
            &self,
            url: &str,
            progress: &mut dyn FnMut(DowngraderProgress),
        ) -> Result<Vec<u8>, DowngraderDownloadError> {
            self.calls.borrow_mut().push(url.to_owned());
            for percent in &self.progress {
                progress(DowngraderProgress::new(*percent));
            }
            self.result.clone()
        }
    }

    #[derive(Debug)]
    struct RecordingApplier {
        calls: RefCell<Vec<(Vec<u8>, Vec<u8>)>>,
        result: Result<Vec<u8>, DowngraderDeltaApplyError>,
    }

    impl RecordingApplier {
        fn ok(bytes: impl Into<Vec<u8>>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                result: Ok(bytes.into()),
            }
        }

        fn failing() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                result: Err(DowngraderDeltaApplyError::failed("malformed patch")),
            }
        }

        fn calls(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
            self.calls.borrow().clone()
        }
    }

    impl DeltaApplier for RecordingApplier {
        fn apply_delta(
            &self,
            source_bytes: &[u8],
            patch_bytes: &[u8],
            _expected_output_bytes: u64,
        ) -> Result<Vec<u8>, DowngraderDeltaApplyError> {
            self.calls
                .borrow_mut()
                .push((source_bytes.to_vec(), patch_bytes.to_vec()));
            self.result.clone()
        }
    }

    fn executor_request(
        request_id: u64,
        keep_backups: bool,
        delete_deltas: bool,
    ) -> DowngraderExecutionRequest<'static> {
        let installation = Box::leak(Box::new(installation()));
        DowngraderExecutionRequest::new(
            request_id,
            Some(installation),
            options(DowngraderTarget::OldGen, keep_backups, delete_deltas),
        )
    }

    fn next_gen_fallout4_executor_fs() -> RecordingWritableFilesystem {
        RecordingWritableFilesystem::default()
            .with_dir(game_root())
            .with_crc_file("Game/Fallout4.exe", "C5965A2E")
    }

    fn test_patch_integrity(
        patch_name: &'static str,
        patch_bytes: &[u8],
        expected_output_bytes: u64,
    ) -> [DowngraderPatchIntegrity; 1] {
        [DowngraderPatchIntegrity {
            patch_name,
            expected_patch_bytes: patch_bytes.len() as u64,
            sha256_hex: Box::leak(sha256_hex(patch_bytes).into_boxed_str()),
            expected_output_bytes,
        }]
    }

    #[test]
    fn downgrader_executor_restores_valid_desired_backup_and_respects_keep_backups() {
        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_upgradeBackup.exe", "C6053902")
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E");
        let downloader = RecordingDownloader::ok(b"unused".to_vec());
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));

        let result = DowngraderService::new(&fs)
            .execute_confirmed(executor_request(30, true, false), &downloader, &applier)
            .expect("restore with kept backups");

        assert_eq!(result.rows.len(), 6);
        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Patched);
        assert_eq!(result.log_rows[0], patched_log_row("Fallout4.exe"));
        assert_eq!(downloader.calls(), Vec::<String>::new());
        assert!(applier.calls().is_empty());
        assert_eq!(
            fs.file_crc("Game/Fallout4.exe").as_deref(),
            Some("C6053902")
        );
        assert!(fs.has_file("Game/Fallout4_upgradeBackup.exe"));
        assert!(fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert_eq!(
            fs.operations(),
            vec![RecordedWriteOp::Write(PathBuf::from("Game/Fallout4.exe"))]
        );

        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_upgradeBackup.exe", "C6053902")
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E");
        let result = DowngraderService::new(&fs)
            .execute_confirmed(executor_request(31, false, false), &downloader, &applier)
            .expect("restore without kept backups");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Patched);
        assert_eq!(
            fs.file_crc("Game/Fallout4.exe").as_deref(),
            Some("C6053902")
        );
        assert!(!fs.has_file("Game/Fallout4_upgradeBackup.exe"));
        assert!(!fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert_eq!(
            fs.operations(),
            vec![
                RecordedWriteOp::Write(PathBuf::from("Game/Fallout4.exe")),
                RecordedWriteOp::Remove(PathBuf::from("Game/Fallout4_upgradeBackup.exe")),
                RecordedWriteOp::Remove(PathBuf::from("Game/Fallout4_downgradeBackup.exe")),
            ]
        );
    }

    #[test]
    fn downgrader_executor_deletes_invalid_backups_downloads_applies_and_cleans_deltas() {
        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_upgradeBackup.exe", "C5965A2E")
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C6053902");
        let patch_bytes = b"delta bytes".to_vec();
        let downloader = RecordingDownloader::ok(patch_bytes.clone());
        let manifest = test_patch_integrity("NG-to-OG-Fallout4.exe.xdelta", &patch_bytes, 4);
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));

        let result = DowngraderService::with_patch_integrity_manifest(&fs, &manifest)
            .execute_confirmed(executor_request(32, false, true), &downloader, &applier)
            .expect("download/apply path");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Patched);
        assert_eq!(downloader.calls().len(), 1);
        assert!(downloader.calls()[0].ends_with("NG-to-OG-Fallout4.exe.xdelta"));
        assert_eq!(applier.calls().len(), 1);
        assert_eq!(result.progress_events.len(), 3);
        assert_eq!(
            fs.file_crc("Game/Fallout4.exe").as_deref(),
            Some("C6053902")
        );
        assert!(!fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert!(!fs.has_file("Game/NG-to-OG-Fallout4.exe.xdelta"));
        assert_eq!(
            fs.operations(),
            vec![
                RecordedWriteOp::Remove(PathBuf::from("Game/Fallout4_downgradeBackup.exe")),
                RecordedWriteOp::Copy {
                    from: PathBuf::from("Game/Fallout4.exe"),
                    to: PathBuf::from("Game/Fallout4_downgradeBackup.exe"),
                },
                RecordedWriteOp::Remove(PathBuf::from("Game/Fallout4_upgradeBackup.exe")),
                RecordedWriteOp::Write(PathBuf::from("Game/NG-to-OG-Fallout4.exe.xdelta")),
                RecordedWriteOp::Write(PathBuf::from("Game/Fallout4.exe")),
                RecordedWriteOp::Remove(PathBuf::from("Game/Fallout4_downgradeBackup.exe")),
                RecordedWriteOp::Remove(PathBuf::from("Game/NG-to-OG-Fallout4.exe.xdelta")),
            ]
        );
    }

    #[test]
    fn downgrader_executor_reuses_existing_patch_and_does_not_download_for_skipped_rows() {
        let patch_bytes = b"existing patch".to_vec();
        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E")
            .with_file("Game/NG-to-OG-Fallout4.exe.xdelta", patch_bytes.clone());
        let downloader = RecordingDownloader::ok(b"download should not be used".to_vec());
        let manifest = test_patch_integrity("NG-to-OG-Fallout4.exe.xdelta", &patch_bytes, 4);
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));

        let result = DowngraderService::with_patch_integrity_manifest(&fs, &manifest)
            .execute_confirmed(executor_request(33, true, false), &downloader, &applier)
            .expect("existing patch path");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Patched);
        assert!(downloader.calls().is_empty());
        assert_eq!(applier.calls().len(), 1);
        assert!(fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert!(fs.has_file("Game/NG-to-OG-Fallout4.exe.xdelta"));

        let fs = RecordingWritableFilesystem::default()
            .with_dir(game_root())
            .with_file("Game/Fallout4.exe", b"unsupported source".to_vec());
        let downloader = RecordingDownloader::ok(b"unused".to_vec());
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));
        let result = DowngraderService::new(&fs)
            .execute_confirmed(executor_request(34, true, true), &downloader, &applier)
            .expect("unsupported source skip");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Skipped);
        assert_eq!(
            result.log_rows[0],
            crate::domain::downgrader::skipped_unsupported_log_row("Fallout4.exe")
        );
        assert!(downloader.calls().is_empty());
        assert!(applier.calls().is_empty());
        assert!(fs.operations().is_empty());
    }

    #[test]
    fn downgrader_executor_preserves_source_backup_after_download_or_apply_failures() {
        let fs = next_gen_fallout4_executor_fs();
        let downloader = RecordingDownloader::failing();
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));

        let result = DowngraderService::new(&fs)
            .execute_confirmed(executor_request(35, false, true), &downloader, &applier)
            .expect("failed download still returns per-row result");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Failed);
        assert_eq!(result.log_rows[0], failed_patching_log_row("Fallout4.exe"));
        assert!(fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert!(fs.has_file("Game/Fallout4.exe"));
        assert!(applier.calls().is_empty());

        let patch_bytes = b"bad patch".to_vec();
        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E")
            .with_file("Game/NG-to-OG-Fallout4.exe.xdelta", patch_bytes.clone());
        let downloader = RecordingDownloader::ok(b"unused".to_vec());
        let manifest = test_patch_integrity("NG-to-OG-Fallout4.exe.xdelta", &patch_bytes, 4);
        let applier = RecordingApplier::failing();
        let result = DowngraderService::with_patch_integrity_manifest(&fs, &manifest)
            .execute_confirmed(executor_request(36, false, true), &downloader, &applier)
            .expect("failed apply still returns per-row result");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Failed);
        assert!(fs.has_file("Game/Fallout4_downgradeBackup.exe"));
        assert!(fs.has_file("Game/NG-to-OG-Fallout4.exe.xdelta"));
        assert!(fs.has_file("Game/Fallout4.exe"));
        assert_eq!(applier.calls().len(), 1);
        assert!(downloader.calls().is_empty());
    }

    #[test]
    fn downgrader_executor_logs_permission_failures_and_continues_safely() {
        let patch_bytes = b"permission patch".to_vec();
        let fs = next_gen_fallout4_executor_fs()
            .with_crc_file("Game/Fallout4_downgradeBackup.exe", "C5965A2E")
            .with_file("Game/NG-to-OG-Fallout4.exe.xdelta", patch_bytes.clone())
            .with_crc_file("Game/Fallout4Launcher.exe", "02445570")
            .with_failure(
                PlatformOperation::WriteFile,
                "Game/Fallout4.exe",
                PlatformErrorKind::PermissionDenied,
            );
        let downloader = RecordingDownloader::ok(b"unused".to_vec());
        let manifest = test_patch_integrity("NG-to-OG-Fallout4.exe.xdelta", &patch_bytes, 4);
        let applier = RecordingApplier::ok(bytes_with_crc("C6053902"));

        let result = DowngraderService::with_patch_integrity_manifest(&fs, &manifest)
            .execute_confirmed(executor_request(37, false, false), &downloader, &applier)
            .expect("permission failure should not abort full executor");

        assert_eq!(result.rows[0].outcome, DowngraderExecutionOutcome::Failed);
        assert_eq!(result.log_rows[0], failed_patching_log_row("Fallout4.exe"));
        assert_eq!(result.rows[1].outcome, DowngraderExecutionOutcome::Skipped);
        assert!(
            result.rows[0]
                .diagnostics
                .iter()
                .any(|message| message.contains("permission was denied"))
        );
        assert!(downloader.calls().is_empty());
        assert_eq!(applier.calls().len(), 1);
        assert!(fs.has_file("Game/Fallout4.exe"));
        assert!(fs.has_file("Game/Fallout4_downgradeBackup.exe"));
    }

    #[test]
    fn s09_downgrader_runtime_wiring_confirmed_plan_mismatch_fails_closed_before_mutation() {
        let fs = next_gen_fallout4_executor_fs();
        let service = DowngraderService::new(&fs);
        let preview = service
            .preview_plan(DowngraderPlanRequest::new(
                40,
                Some(&installation()),
                options(DowngraderTarget::OldGen, false, true),
            ))
            .expect("preview should build before file drift");
        let reviewed_digest = preview.stable_digest();
        fs.nodes.borrow_mut().insert(
            PathBuf::from("Game/Fallout4.exe"),
            FakeNode::File(bytes_with_crc("C6053902")),
        );

        let error = service
            .execute_confirmed(
                DowngraderExecutionRequest::new(
                    41,
                    Some(&installation()),
                    options(DowngraderTarget::OldGen, false, true),
                )
                .with_confirmed_plan_digest(reviewed_digest),
                &RecordingDownloader::ok(b"unused".to_vec()),
                &RecordingApplier::ok(bytes_with_crc("C6053902")),
            )
            .expect_err("changed file state must fail closed before execution");

        match error {
            DowngraderServiceError::ConfirmedPlanChanged { safe_message, .. } => {
                assert_eq!(safe_message, CONFIRMED_PLAN_CHANGED_MESSAGE);
            }
            other => panic!("expected confirmed plan changed error, got {other:?}"),
        }
        assert!(fs.operations().is_empty());
    }

    #[test]
    fn downgrader_executor_vcdiff_applier_fixture_decodes_source_patch() {
        let patch = vec![
            214, 195, 196, 0,  // magic
            0,  // header indicator
            1,  // VCD_SOURCE
            4,  // source segment size
            1,  // source segment position
            12, // delta window size
            13, // target window size
            0,  // delta indicator
            3,  // data length
            2,  // instruction length
            2,  // address length
            72, 33, 32,  // "H! "
            235, // ADD1 COPY4_mode6
            183, // ADD2 COPY6_mode0
            0, 4,
        ];

        let output = VcdiffDeltaApplier
            .apply_delta(b"hello", &patch, 13)
            .expect("VCDIFF fixture should decode");

        assert_eq!(output, b"Hello! Hello!".to_vec());
    }
}
