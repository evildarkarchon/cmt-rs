//! Pure Scanner Auto-Fix domain contract.
//!
//! The reference implementation keeps Auto-Fix handlers in `CMT/src/autofixes.py`
//! and keys them by `SolutionType` enum values, not by display strings. This
//! module models that lifecycle as inert Rust data so later service/controller
//! slices can fail closed, keep the production registry empty, and reject stale
//! or tampered selections before mutating user files.

use std::{fmt, path::PathBuf};

/// Reference Auto-Fix button label before an action starts.
pub const AUTO_FIX_BUTTON_LABEL: &str = "Auto-Fix";
/// Reference Auto-Fix button label while the handler is running.
pub const AUTO_FIXING_BUTTON_LABEL: &str = "Fixing...";
/// Reference Auto-Fix button label after a successful handler completion.
pub const AUTO_FIX_FIXED_BUTTON_LABEL: &str = "Fixed!";
/// Reference Auto-Fix button label after a failed handler completion.
pub const AUTO_FIX_FAILED_BUTTON_LABEL: &str = "Fix Failed";
/// Reference Auto-Fix result dialog title.
pub const AUTO_FIX_RESULTS_TITLE: &str = "Auto-Fix Results";

const SELECTION_IDENTITY_PREFIX: &str = "scanner-result:v1";
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable, owned fingerprint for the selected scanner result facts.
///
/// The value is intentionally derived from already-displayed strings and paths;
/// generating it never performs filesystem I/O. Later Auto-Fix requests can
/// carry the identity captured at preview time and compare it with a fresh
/// pre-mutation identity to reject stale or tampered rows.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AutoFixSelectionIdentity {
    fingerprint: String,
}

impl AutoFixSelectionIdentity {
    /// Creates an identity from an already-computed fingerprint string.
    pub fn from_fingerprint(fingerprint: impl Into<String>) -> Self {
        Self {
            fingerprint: fingerprint.into(),
        }
    }

    /// Creates a deterministic fingerprint from structural result facts.
    pub fn from_parts<I, P>(parts: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<str>,
    {
        let mut hash = FNV_OFFSET_BASIS;
        for part in parts {
            let part = part.as_ref();
            hash = feed_hash(hash, &(part.len() as u64).to_le_bytes());
            hash = feed_hash(hash, b"\0");
            hash = feed_hash(hash, part.as_bytes());
        }
        Self {
            fingerprint: format!("{SELECTION_IDENTITY_PREFIX}:{hash:016x}"),
        }
    }

    /// Returns the stable fingerprint string.
    pub fn as_str(&self) -> &str {
        self.fingerprint.as_str()
    }
}

impl fmt::Display for AutoFixSelectionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Typed Auto-Fix registry key corresponding to reference `SolutionType` values.
///
/// This is deliberately not constructible from arbitrary solution text; callers
/// must retain typed scanner solution identity when creating results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AutoFixOperationKey {
    /// Reference `SolutionType.ArchiveOrDeleteFile` registry key.
    ArchiveOrDeleteFile,
    /// Reference `SolutionType.ArchiveFolder` registry key.
    ArchiveFolder,
    /// Reference `SolutionType.DeleteFile` registry key.
    DeleteFile,
    /// Reference `SolutionType.ConvertDeleteOrIgnoreFile` registry key.
    ConvertDeleteOrIgnoreFile,
    /// Reference `SolutionType.DeleteOrIgnoreFile` registry key.
    DeleteOrIgnoreFile,
    /// Reference `SolutionType.DeleteOrIgnoreFolder` registry key.
    DeleteOrIgnoreFolder,
    /// Reference `SolutionType.RenameArchive` registry key.
    RenameArchive,
    /// Reference `SolutionType.DownloadMod` registry key.
    DownloadMod,
    /// Reference `SolutionType.VerifyFiles` registry key.
    VerifyFiles,
    /// Reference `SolutionType.UnknownFormat` registry key.
    UnknownFormat,
}

impl AutoFixOperationKey {
    /// Returns a stable lowercase operation id for logs, tests, and future UI payloads.
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::ArchiveOrDeleteFile => "archive-or-delete-file",
            Self::ArchiveFolder => "archive-folder",
            Self::DeleteFile => "delete-file",
            Self::ConvertDeleteOrIgnoreFile => "convert-delete-or-ignore-file",
            Self::DeleteOrIgnoreFile => "delete-or-ignore-file",
            Self::DeleteOrIgnoreFolder => "delete-or-ignore-folder",
            Self::RenameArchive => "rename-archive",
            Self::DownloadMod => "download-mod",
            Self::VerifyFiles => "verify-files",
            Self::UnknownFormat => "unknown-format",
        }
    }

    /// Parses a stable operation id into a typed operation key.
    pub fn from_id(operation_id: &str) -> Option<Self> {
        match operation_id {
            "archive-or-delete-file" => Some(Self::ArchiveOrDeleteFile),
            "archive-folder" => Some(Self::ArchiveFolder),
            "delete-file" => Some(Self::DeleteFile),
            "convert-delete-or-ignore-file" => Some(Self::ConvertDeleteOrIgnoreFile),
            "delete-or-ignore-file" => Some(Self::DeleteOrIgnoreFile),
            "delete-or-ignore-folder" => Some(Self::DeleteOrIgnoreFolder),
            "rename-archive" => Some(Self::RenameArchive),
            "download-mod" => Some(Self::DownloadMod),
            "verify-files" => Some(Self::VerifyFiles),
            "unknown-format" => Some(Self::UnknownFormat),
            _ => None,
        }
    }
}

/// High-level Auto-Fix lifecycle state safe for UI and worker plumbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutoFixStatusKind {
    /// An operation is eligible and has not been requested.
    Ready,
    /// A request was accepted and is executing.
    Fixing,
    /// A request completed successfully.
    Fixed,
    /// A request completed unsuccessfully.
    Failed,
    /// A request was rejected before execution.
    Rejected,
}

impl AutoFixStatusKind {
    /// Returns the reference button label for this lifecycle state.
    pub const fn button_label(self) -> &'static str {
        match self {
            Self::Ready => AUTO_FIX_BUTTON_LABEL,
            Self::Fixing => AUTO_FIXING_BUTTON_LABEL,
            Self::Fixed => AUTO_FIX_FIXED_BUTTON_LABEL,
            Self::Failed | Self::Rejected => AUTO_FIX_FAILED_BUTTON_LABEL,
        }
    }

    /// Returns whether the button may be pressed in this state.
    pub const fn button_enabled(self) -> bool {
        !matches!(self, Self::Fixing)
    }
}

/// Typed Auto-Fix status text and optional diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixStatus {
    /// Machine-readable lifecycle state.
    pub kind: AutoFixStatusKind,
    /// User-safe status text for inline Scanner feedback.
    pub safe_message: String,
    /// Raw adapter or validation details for tests/logs; not primary UI text.
    pub diagnostic: Option<String>,
}

impl AutoFixStatus {
    /// Creates an Auto-Fix status with safe user-facing text.
    pub fn new(kind: AutoFixStatusKind, safe_message: impl Into<String>) -> Self {
        Self {
            kind,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail while preserving the safe message.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

/// Render-ready Auto-Fix button state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixButtonState {
    /// Machine-readable lifecycle state.
    pub kind: AutoFixStatusKind,
    /// Exact button label.
    pub label: &'static str,
    /// Whether the button should be enabled.
    pub enabled: bool,
    /// Whether the reference accent style should be used.
    pub accent: bool,
}

impl AutoFixButtonState {
    /// Creates a render-ready button state from a lifecycle kind.
    pub const fn from_status(kind: AutoFixStatusKind) -> Self {
        Self {
            kind,
            label: kind.button_label(),
            enabled: kind.button_enabled(),
            accent: matches!(kind, AutoFixStatusKind::Ready),
        }
    }
}

/// Details shown in the reference `Auto-Fix Results` dialog/inline feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixResultDetail {
    /// Exact result surface title.
    pub title: &'static str,
    /// Safe one-line result summary.
    pub safe_summary: String,
    /// Detailed result body shown after completion or rejection.
    pub details: String,
    /// Raw adapter details for logs/tests; not primary UI text.
    pub diagnostic: Option<String>,
}

impl AutoFixResultDetail {
    /// Creates Auto-Fix result details with the reference title.
    pub fn new(safe_summary: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            title: AUTO_FIX_RESULTS_TITLE,
            safe_summary: safe_summary.into(),
            details: details.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail while preserving the safe summary and details.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

/// Explicit pre-mutation revalidation policy carried by previews and requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixRevalidationPlan {
    /// Identity captured when the result was selected or previewed.
    pub expected_identity: AutoFixSelectionIdentity,
    /// Whether the worker must recompute and compare the selected result before mutation.
    pub required_before_mutation: bool,
    /// Identity observed immediately before mutation, populated by later workers.
    pub observed_identity: Option<AutoFixSelectionIdentity>,
}

impl AutoFixRevalidationPlan {
    /// Creates a revalidation plan that requires a pre-mutation identity check.
    pub fn required(expected_identity: AutoFixSelectionIdentity) -> Self {
        Self {
            expected_identity,
            required_before_mutation: true,
            observed_identity: None,
        }
    }

    /// Attaches the identity observed by a later pre-mutation validation step.
    pub fn with_observed_identity(mut self, observed_identity: AutoFixSelectionIdentity) -> Self {
        self.observed_identity = Some(observed_identity);
        self
    }
}

/// Auto-Fix plan preview shown before scheduling a write-capable operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixPlanPreview {
    /// Typed operation key selected from a retained scanner solution identity.
    pub operation_key: AutoFixOperationKey,
    /// Identity of the selected scanner result at preview time.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Optional target path expected to be mutated by the operation.
    pub target_path: Option<PathBuf>,
    /// Safe human-readable preview of what the operation would do.
    pub safe_preview: String,
    /// Whether explicit confirmation is required before scheduling.
    pub confirmation_required: bool,
    /// Optional confirmation prompt preserved for future UI flows.
    pub confirmation_prompt: Option<String>,
    /// Pre-mutation validation policy used to reject stale/tampered requests.
    pub revalidation: AutoFixRevalidationPlan,
}

/// User confirmation captured before scheduling an Auto-Fix request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixConfirmation {
    /// Operation the user confirmed.
    pub operation_key: AutoFixOperationKey,
    /// Selection identity the user confirmed.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Whether the user accepted the confirmation prompt.
    pub accepted: bool,
    /// Prompt text the user saw, if any.
    pub prompt: Option<String>,
    /// Optional opaque token for future double-submit/stale-form guards.
    pub confirmation_token: Option<String>,
    /// Whether the confirmed request still requires pre-mutation revalidation.
    pub requires_pre_mutation_revalidation: bool,
}

/// Scheduled Auto-Fix request safe to hand to a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixRequest {
    /// Optional scan id the selected result belonged to.
    pub scan_id: Option<u64>,
    /// Typed operation key resolved without display-string matching.
    pub operation_key: AutoFixOperationKey,
    /// Selected scanner result identity captured when requested.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Optional target path for path-based operations.
    pub target_path: Option<PathBuf>,
    /// Confirmation payload, when this operation requires one.
    pub confirmation: Option<AutoFixConfirmation>,
    /// Pre-mutation validation policy used by the worker before writing files.
    pub revalidation: AutoFixRevalidationPlan,
}

/// Completed Auto-Fix worker result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixCompletion {
    /// Optional scan id the request belonged to.
    pub scan_id: Option<u64>,
    /// Operation that completed.
    pub operation_key: AutoFixOperationKey,
    /// Selected result identity from the request.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Pre-mutation validation policy and observed identity, if populated.
    pub revalidation: AutoFixRevalidationPlan,
    /// Final safe status.
    pub status: AutoFixStatus,
    /// Details for inline feedback or the result dialog.
    pub detail: AutoFixResultDetail,
}

/// Reason an Auto-Fix request was rejected before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutoFixRejectionReason {
    /// No registry handler exists for the typed operation key.
    NoRegisteredHandler,
    /// The selected result had no typed solution key.
    UnsupportedSolution,
    /// The operation requires a path and the selected result had none.
    MissingTargetPath,
    /// The pre-mutation identity did not match the selected identity.
    StaleSelection,
    /// Confirmation was required but not supplied.
    ConfirmationRequired,
    /// Confirmation was supplied but declined.
    ConfirmationDeclined,
    /// The worker could not be scheduled.
    WorkerUnavailable,
    /// Validation failed before the operation could safely run.
    ValidationFailed,
}

/// Rejected Auto-Fix request with safe text and optional diagnostic detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixRejection {
    /// Optional scan id the request belonged to.
    pub scan_id: Option<u64>,
    /// Operation that was rejected, when it was known.
    pub operation_key: Option<AutoFixOperationKey>,
    /// Selected result identity from the request, when available.
    pub selection_identity: Option<AutoFixSelectionIdentity>,
    /// Identity observed during pre-mutation validation, when available.
    pub observed_identity: Option<AutoFixSelectionIdentity>,
    /// Machine-readable rejection reason.
    pub reason: AutoFixRejectionReason,
    /// User-safe rejection text.
    pub safe_message: String,
    /// Raw diagnostic for tests/logs; not primary UI text.
    pub diagnostic: Option<String>,
    /// Button state the UI should show after the rejection.
    pub button_state: AutoFixButtonState,
}

impl AutoFixRejection {
    /// Creates a rejected request with safe user-facing text.
    pub fn new(reason: AutoFixRejectionReason, safe_message: impl Into<String>) -> Self {
        Self {
            scan_id: None,
            operation_key: None,
            selection_identity: None,
            observed_identity: None,
            reason,
            safe_message: safe_message.into(),
            diagnostic: None,
            button_state: AutoFixButtonState::from_status(AutoFixStatusKind::Rejected),
        }
    }

    /// Adds diagnostic detail while preserving the safe message.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

fn feed_hash(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod scanner_autofix_domain {
    use super::*;

    #[test]
    fn scanner_autofix_domain_labels_match_reference_lifecycle() {
        assert_eq!(AUTO_FIX_BUTTON_LABEL, "Auto-Fix");
        assert_eq!(AUTO_FIXING_BUTTON_LABEL, "Fixing...");
        assert_eq!(AUTO_FIX_FIXED_BUTTON_LABEL, "Fixed!");
        assert_eq!(AUTO_FIX_FAILED_BUTTON_LABEL, "Fix Failed");
        assert_eq!(AUTO_FIX_RESULTS_TITLE, "Auto-Fix Results");

        let ready = AutoFixButtonState::from_status(AutoFixStatusKind::Ready);
        assert_eq!(ready.label, AUTO_FIX_BUTTON_LABEL);
        assert!(ready.enabled);
        assert!(ready.accent);

        let fixing = AutoFixButtonState::from_status(AutoFixStatusKind::Fixing);
        assert_eq!(fixing.label, AUTO_FIXING_BUTTON_LABEL);
        assert!(!fixing.enabled);
        assert!(!fixing.accent);

        let fixed = AutoFixButtonState::from_status(AutoFixStatusKind::Fixed);
        assert_eq!(fixed.label, AUTO_FIX_FIXED_BUTTON_LABEL);
        assert!(fixed.enabled);

        let failed = AutoFixButtonState::from_status(AutoFixStatusKind::Failed);
        assert_eq!(failed.label, AUTO_FIX_FAILED_BUTTON_LABEL);
        assert!(failed.enabled);
    }

    #[test]
    fn scanner_autofix_domain_operation_ids_are_typed_and_round_trip() {
        let key = AutoFixOperationKey::DeleteOrIgnoreFile;
        assert_eq!(key.as_id(), "delete-or-ignore-file");
        assert_eq!(AutoFixOperationKey::from_id(key.as_id()), Some(key));
        assert_eq!(
            AutoFixOperationKey::from_id("It can either be deleted or ignored."),
            None
        );
    }

    #[test]
    fn scanner_autofix_domain_selection_identity_is_owned_and_deterministic() {
        let first = AutoFixSelectionIdentity::from_parts([
            "Junk File",
            "Data/desktop.ini",
            "This is a junk file not used by the game or mod managers.",
        ]);
        let same = AutoFixSelectionIdentity::from_parts([
            "Junk File",
            "Data/desktop.ini",
            "This is a junk file not used by the game or mod managers.",
        ]);
        let changed = AutoFixSelectionIdentity::from_parts([
            "Junk File",
            "Data/thumbs.db",
            "This is a junk file not used by the game or mod managers.",
        ]);

        assert_eq!(first, same);
        assert_ne!(first, changed);
        assert!(first.as_str().starts_with("scanner-result:v1:"));
    }
}
