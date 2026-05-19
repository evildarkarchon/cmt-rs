//! Typed worker event contracts for long-running background operations.
//!
//! The types in this module are owned, `Send`-friendly values that can cross a
//! background-worker boundary before a UI-specific handoff layer decides how to
//! display them. They intentionally avoid Slint handles, models, and borrowed
//! filesystem state.

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    domain::{
        archive_patcher::{
            ArchivePatcherCandidateSnapshot, ArchivePatcherExecutionResult, ArchivePatcherLogRow,
            ArchivePatcherPreviewPlan, ArchivePatcherProgress,
        },
        autofix::AutoFixCompletion,
        downgrader::{DowngraderExecutionLogRow, DowngraderProgress},
        f4se::F4seScanSnapshot,
        overview::{
            OverviewActionError, OverviewDeferredActionKind, OverviewSnapshot, UpdateBannerState,
        },
        scanner::{ScannerActionFeedback, ScannerScanSnapshot},
    },
    platform::{
        PlatformError, PlatformErrorKind, PlatformOperation,
        desktop::{DesktopActionOutcome, DesktopActionResult},
    },
    services::{
        downgrader::{DowngraderExecutionResult, DowngraderPreviewPlan, DowngraderStatusSnapshot},
        tools::{AboutActionFeedback, ToolsActionFeedback},
    },
};

/// Stable identity for one background task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerTaskId(String);

impl WorkerTaskId {
    /// Creates a task id from a caller-supplied stable identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the task id as a borrowed string for logs and diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WorkerTaskId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkerTaskId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for WorkerTaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// High-level category for a background task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkerTaskKind {
    /// Complete Overview refresh, update-check, or Overview-specific orchestration work.
    Overview,
    /// Fallout 4 installation, manager, registry, process, or system discovery.
    Discovery,
    /// Scanner traversal, parsing, or classification work.
    Scan,
    /// Archive or file patching work.
    Patch,
    /// Network download or update retrieval work.
    Download,
    /// External process execution or monitoring work.
    ExternalProcess,
    /// Desktop handoff such as opening a URL or path.
    DesktopAction,
    /// Generic typed work whose concrete category is not important to the UI.
    Generic,
    /// Unknown work kind retained for forward-compatible handoff.
    Unknown,
}

impl WorkerTaskKind {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Discovery => "discovery",
            Self::Scan => "scan",
            Self::Patch => "patch",
            Self::Download => "download",
            Self::ExternalProcess => "external-process",
            Self::DesktopAction => "desktop-action",
            Self::Generic => "generic",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for WorkerTaskKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Shared task metadata attached to every worker event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerTask {
    /// Stable task identity chosen by the service that scheduled the work.
    pub id: WorkerTaskId,
    /// High-level work category used by status surfaces.
    pub kind: WorkerTaskKind,
    /// Optional short human-readable label for display surfaces.
    pub label: Option<String>,
}

impl WorkerTask {
    /// Creates a worker task from an id and work kind.
    pub fn new(id: impl Into<WorkerTaskId>, kind: WorkerTaskKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: None,
        }
    }

    /// Adds a short display label while preserving the stable id.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Lifecycle/status value for a worker event envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkerTaskStatus {
    /// Task is queued but not running yet.
    Queued,
    /// Task is executing on a worker thread.
    Running,
    /// Task emitted partial progress.
    Progress,
    /// Cancellation was requested by a caller or UI control.
    CancellationRequested,
    /// Worker observed the cancellation request and acknowledged it.
    CancellationAcknowledged,
    /// Task completed successfully.
    Completed,
    /// Task completed with cancellation as its final state.
    Cancelled,
    /// Task completed with a typed failure.
    Failed,
}

impl WorkerTaskStatus {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Progress => "progress",
            Self::CancellationRequested => "cancellation-requested",
            Self::CancellationAcknowledged => "cancellation-acknowledged",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    /// Returns true when no further events are expected for the task.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

impl fmt::Display for WorkerTaskStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Shared envelope emitted by every background worker.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerEvent {
    /// Stable task identity and high-level kind metadata.
    pub task: WorkerTask,
    /// Current lifecycle/status for this event.
    pub status: WorkerTaskStatus,
    /// Typed event-specific payload.
    pub payload: WorkerPayload,
}

impl WorkerEvent {
    /// Creates a typed worker event envelope.
    pub fn new(task: WorkerTask, status: WorkerTaskStatus, payload: WorkerPayload) -> Self {
        Self {
            task,
            status,
            payload,
        }
    }

    /// Creates a running lifecycle event.
    pub fn running(task: WorkerTask) -> Self {
        Self::new(task, WorkerTaskStatus::Running, WorkerPayload::None)
    }

    /// Creates a progress event with optional text and counts.
    pub fn progress(task: WorkerTask, progress: WorkerProgress) -> Self {
        Self::new(
            task,
            WorkerTaskStatus::Progress,
            WorkerPayload::Progress(progress),
        )
    }

    /// Creates a successful completion event.
    pub fn completed(task: WorkerTask, payload: WorkerPayload) -> Self {
        Self::new(task, WorkerTaskStatus::Completed, payload)
    }

    /// Creates a typed failure event.
    pub fn failed(task: WorkerTask, failure: WorkerFailure) -> Self {
        Self::new(
            task,
            WorkerTaskStatus::Failed,
            WorkerPayload::Error(failure),
        )
    }

    /// Creates a cancellation-request event.
    pub fn cancellation_requested(task: WorkerTask, cancellation: WorkerCancellation) -> Self {
        Self::new(
            task,
            WorkerTaskStatus::CancellationRequested,
            WorkerPayload::Cancellation(cancellation),
        )
    }

    /// Creates a cancellation-acknowledgement event.
    pub fn cancellation_acknowledged(task: WorkerTask, cancellation: WorkerCancellation) -> Self {
        Self::new(
            task,
            WorkerTaskStatus::CancellationAcknowledged,
            WorkerPayload::Cancellation(cancellation),
        )
    }

    /// Creates the final cancelled completion event.
    pub fn cancelled(task: WorkerTask, cancellation: WorkerCancellation) -> Self {
        Self::new(
            task,
            WorkerTaskStatus::Cancelled,
            WorkerPayload::Cancellation(cancellation),
        )
    }
}

/// Optional progress text and counts for a worker task.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct WorkerProgress {
    /// Optional safe text for status labels or logs.
    pub message: Option<String>,
    /// Optional completed item count.
    pub current: Option<u64>,
    /// Optional total item count when it is cheap and meaningful to know.
    pub total: Option<u64>,
}

impl WorkerProgress {
    /// Creates an empty progress value with no text or counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds safe human-readable progress text.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds optional current and total counts without implying percentages or ETA.
    pub const fn with_counts(mut self, current: Option<u64>, total: Option<u64>) -> Self {
        self.current = current;
        self.total = total;
        self
    }

    /// Returns true when this progress value has no text or counts.
    pub const fn is_empty(&self) -> bool {
        self.message.is_none() && self.current.is_none() && self.total.is_none()
    }
}

/// Safe text payload used by task-specific message variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerMessage {
    /// User-safe message text.
    pub safe_message: String,
}

impl WorkerMessage {
    /// Creates a safe worker message.
    pub fn new(safe_message: impl Into<String>) -> Self {
        Self {
            safe_message: safe_message.into(),
        }
    }
}

/// Typed worker failure with user-safe text separated from diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerFailure {
    /// User-safe failure text suitable for UI status surfaces.
    pub safe_message: String,
    /// Optional diagnostic detail for logs or tests, never modal text.
    pub diagnostic: Option<String>,
}

impl WorkerFailure {
    /// Creates a worker failure from safe user-facing text.
    pub fn new(safe_message: impl Into<String>) -> Self {
        Self {
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail while preserving the safe message.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }

    /// Returns the safe user-facing failure message.
    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    /// Returns optional diagnostic detail for structured logs and tests.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

/// Cancellation payload for requested, acknowledged, and final-cancelled events.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct WorkerCancellation {
    /// Optional safe cancellation text.
    pub message: Option<String>,
}

impl WorkerCancellation {
    /// Creates a cancellation payload.
    pub fn new(message: Option<String>) -> Self {
        Self { message }
    }

    /// Creates an empty cancellation payload.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a cancellation payload with safe text.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}

/// Typed payload variants carried by worker events.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerPayload {
    /// No additional payload is needed for this lifecycle event.
    None,
    /// Progress text and optional counts.
    Progress(WorkerProgress),
    /// Discovery-specific status or result text.
    Discovery(WorkerMessage),
    /// Scanner-specific status or result text.
    Scan(WorkerMessage),
    /// Scanner-specific snapshot or read-only action payload.
    Scanner(ScannerWorkerPayload),
    /// F4SE diagnostics-specific scan completion payload.
    F4se(F4seWorkerPayload),
    /// Patcher-specific status or result text.
    Patch(WorkerMessage),
    /// Download-specific status or result text.
    Download(WorkerMessage),
    /// Overview-specific snapshot, update, or desktop-action result.
    Overview(OverviewWorkerPayload),
    /// Tools-tab action completion result.
    ToolsAction(ToolsActionWorkerPayload),
    /// About-tab action completion result.
    AboutAction(AboutActionWorkerPayload),
    /// Downgrader modal status, plan, progress, log, or run result.
    Downgrader(DowngraderWorkerPayload),
    /// Archive Patcher modal candidates, plan, progress, logs, patch, or restore result.
    ArchivePatcher(ArchivePatcherWorkerPayload),
    /// External process, tool, or desktop-action result.
    ExternalAction(ExternalActionPayload),
    /// Cancellation details.
    Cancellation(WorkerCancellation),
    /// User-safe failure plus optional diagnostics.
    Error(WorkerFailure),
    /// Generic status or result text.
    Generic(WorkerMessage),
}

/// Scanner-specific worker payloads that must cross the UI handoff boundary intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannerWorkerPayload {
    /// A full Scanner scan produced a render-ready snapshot.
    ScanCompleted {
        /// Scan request id used to reject stale worker results.
        scan_id: u64,
        /// Owned snapshot produced off the UI thread.
        snapshot: Box<ScannerScanSnapshot>,
    },
    /// A read-only Scanner action completed with safe visible feedback.
    ActionCompleted {
        /// Owned action feedback produced by a copy/open/file-list worker.
        feedback: ScannerActionFeedback,
    },
    /// A Scanner Auto-Fix worker completed with safe result feedback.
    AutoFixCompleted {
        /// Owned Auto-Fix completion produced off the UI thread.
        completion: Box<AutoFixCompletion>,
    },
}

impl ScannerWorkerPayload {
    /// Creates a successful Scanner scan payload.
    pub fn scan_completed(scan_id: u64, snapshot: ScannerScanSnapshot) -> Self {
        Self::ScanCompleted {
            scan_id,
            snapshot: Box::new(snapshot),
        }
    }

    /// Creates a Scanner read-only action completion payload.
    pub fn action_completed(feedback: ScannerActionFeedback) -> Self {
        Self::ActionCompleted { feedback }
    }

    /// Creates a Scanner Auto-Fix completion payload.
    pub fn auto_fix_completed(completion: AutoFixCompletion) -> Self {
        Self::AutoFixCompleted {
            completion: Box::new(completion),
        }
    }

    /// Returns the scan id attached to this payload when one is present.
    pub fn scan_id(&self) -> Option<u64> {
        match self {
            Self::ScanCompleted { scan_id, .. } => Some(*scan_id),
            Self::ActionCompleted { feedback } => feedback.scan_id,
            Self::AutoFixCompleted { completion } => completion.scan_id,
        }
    }

    /// Returns the owned scan snapshot by shared reference when this is a scan payload.
    pub fn snapshot(&self) -> Option<&ScannerScanSnapshot> {
        match self {
            Self::ScanCompleted { snapshot, .. } => Some(snapshot),
            Self::ActionCompleted { .. } | Self::AutoFixCompleted { .. } => None,
        }
    }

    /// Returns the owned Auto-Fix completion by shared reference when present.
    pub fn auto_fix_completion(&self) -> Option<&AutoFixCompletion> {
        match self {
            Self::AutoFixCompleted { completion } => Some(completion),
            Self::ScanCompleted { .. } | Self::ActionCompleted { .. } => None,
        }
    }
}

/// F4SE-specific worker payloads that must cross the UI handoff boundary intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum F4seWorkerPayload {
    /// A full F4SE scan produced a render-ready snapshot.
    ScanCompleted {
        /// Scan request id used to reject stale worker results.
        scan_id: u64,
        /// Owned snapshot produced off the UI thread.
        snapshot: Box<F4seScanSnapshot>,
    },
}

impl F4seWorkerPayload {
    /// Creates a successful F4SE scan payload.
    pub fn scan_completed(scan_id: u64, snapshot: F4seScanSnapshot) -> Self {
        Self::ScanCompleted {
            scan_id,
            snapshot: Box::new(snapshot),
        }
    }

    /// Returns the scan request id attached to this payload.
    pub const fn scan_id(&self) -> u64 {
        match self {
            Self::ScanCompleted { scan_id, .. } => *scan_id,
        }
    }

    /// Returns the owned scan snapshot by shared reference.
    pub fn snapshot(&self) -> &F4seScanSnapshot {
        match self {
            Self::ScanCompleted { snapshot, .. } => snapshot,
        }
    }
}

/// Overview-specific worker payloads that must cross the UI handoff boundary intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewWorkerPayload {
    /// A full Overview refresh produced a render-ready snapshot.
    RefreshCompleted {
        /// Refresh request id used to reject stale worker results.
        refresh_id: u64,
        /// Owned snapshot produced off the UI thread.
        snapshot: Box<OverviewSnapshot>,
    },
    /// The update-check worker produced a final banner state.
    UpdateCheckCompleted {
        /// Refresh request id the update check was tied to.
        refresh_id: u64,
        /// Banner state safe for the Overview UI.
        update_banner: UpdateBannerState,
    },
    /// A URL/path action completed and may have produced a safe visible error.
    DesktopActionCompleted {
        /// Deferred action that was attempted.
        action: OverviewDeferredActionKind,
        /// Safe action error when the desktop adapter rejected the request.
        error: Option<OverviewActionError>,
    },
}

impl OverviewWorkerPayload {
    /// Creates a successful refresh payload.
    pub fn refresh_completed(refresh_id: u64, snapshot: OverviewSnapshot) -> Self {
        Self::RefreshCompleted {
            refresh_id,
            snapshot: Box::new(snapshot),
        }
    }

    /// Creates an update-check completion payload.
    pub fn update_check_completed(refresh_id: u64, update_banner: UpdateBannerState) -> Self {
        Self::UpdateCheckCompleted {
            refresh_id,
            update_banner,
        }
    }

    /// Creates a desktop-action completion payload.
    pub fn desktop_action_completed(
        action: OverviewDeferredActionKind,
        error: Option<OverviewActionError>,
    ) -> Self {
        Self::DesktopActionCompleted { action, error }
    }
}

/// Worker payload for one Tools-tab action completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsActionWorkerPayload {
    /// Owned feedback produced by the action service.
    pub feedback: ToolsActionFeedback,
}

impl ToolsActionWorkerPayload {
    /// Creates a Tools action-completion payload.
    pub fn action_completed(feedback: ToolsActionFeedback) -> Self {
        Self { feedback }
    }
}

/// Worker payload for one About-tab action completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutActionWorkerPayload {
    /// Owned feedback produced by the action service.
    pub feedback: AboutActionFeedback,
}

impl AboutActionWorkerPayload {
    /// Creates an About action-completion payload.
    pub fn action_completed(feedback: AboutActionFeedback) -> Self {
        Self { feedback }
    }
}

/// Archive Patcher worker stage used for safe failure routing and stale-event rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchivePatcherWorkerStage {
    /// The worker was selecting candidate archive rows from Overview records.
    Candidates,
    /// The worker was preparing a read-only preview plan.
    Plan,
    /// The worker was executing an explicitly confirmed patch run.
    Patch,
    /// The worker was restoring the latest manifest-backed patch run.
    Restore,
}

impl ArchivePatcherWorkerStage {
    /// Returns a stable label suitable for structured tracing and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Candidates => "candidates",
            Self::Plan => "plan",
            Self::Patch => "patch",
            Self::Restore => "restore",
        }
    }

    /// Returns true when this stage performs file mutation and must block close/Escape.
    pub const fn is_mutation(self) -> bool {
        matches!(self, Self::Patch | Self::Restore)
    }
}

/// Archive Patcher-specific worker payloads that must cross the UI handoff boundary intact.
#[derive(Debug, Clone, PartialEq)]
pub enum ArchivePatcherWorkerPayload {
    /// A candidate-loading worker produced render-ready candidate rows.
    CandidatesLoaded {
        /// Candidate request id used to reject stale worker results.
        request_id: u64,
        /// Owned candidate snapshot produced off the UI thread.
        snapshot: Box<ArchivePatcherCandidateSnapshot>,
    },
    /// A read-only planning worker produced an inline confirmation plan.
    PlanReady {
        /// Plan request id used to reject stale worker results.
        request_id: u64,
        /// Owned preview plan produced off the UI thread.
        plan: Box<ArchivePatcherPreviewPlan>,
    },
    /// A confirmed patch or restore worker produced a user-visible log row.
    LogRow {
        /// Patch or restore request id used to reject stale worker results.
        request_id: u64,
        /// Mutation stage that emitted this row.
        stage: ArchivePatcherWorkerStage,
        /// Reference-style user-visible row.
        row: ArchivePatcherLogRow,
    },
    /// A confirmed patch or restore worker produced bounded progress.
    Progress {
        /// Patch or restore request id used to reject stale worker results.
        request_id: u64,
        /// Mutation stage that emitted this progress value.
        stage: ArchivePatcherWorkerStage,
        /// Clamped progress value.
        progress: ArchivePatcherProgress,
    },
    /// A confirmed patch worker completed and returned final execution facts.
    PatchCompleted {
        /// Patch request id used to reject stale worker results.
        request_id: u64,
        /// Owned execution result produced off the UI thread.
        result: Box<ArchivePatcherExecutionResult>,
    },
    /// A restore-last-run worker completed and returned final execution facts.
    RestoreCompleted {
        /// Restore request id used to reject stale worker results.
        request_id: u64,
        /// Owned execution result produced off the UI thread.
        result: Box<ArchivePatcherExecutionResult>,
    },
    /// A worker failed safely before producing its normal payload.
    SafeFailure {
        /// Request id used to reject stale worker failures.
        request_id: u64,
        /// Worker stage that failed.
        stage: ArchivePatcherWorkerStage,
        /// User-safe failure text suitable for modal logs.
        safe_message: String,
        /// Optional diagnostic detail for logs/tests, never modal text.
        diagnostic: Option<String>,
    },
}

impl ArchivePatcherWorkerPayload {
    /// Creates an Archive Patcher candidates-loaded payload.
    pub fn candidates_loaded(request_id: u64, snapshot: ArchivePatcherCandidateSnapshot) -> Self {
        Self::CandidatesLoaded {
            request_id,
            snapshot: Box::new(snapshot),
        }
    }

    /// Creates an Archive Patcher plan-ready payload.
    pub fn plan_ready(request_id: u64, plan: ArchivePatcherPreviewPlan) -> Self {
        Self::PlanReady {
            request_id,
            plan: Box::new(plan),
        }
    }

    /// Creates an Archive Patcher log-row payload.
    pub fn log_row(
        request_id: u64,
        stage: ArchivePatcherWorkerStage,
        row: ArchivePatcherLogRow,
    ) -> Self {
        Self::LogRow {
            request_id,
            stage,
            row,
        }
    }

    /// Creates an Archive Patcher progress payload.
    pub fn progress(
        request_id: u64,
        stage: ArchivePatcherWorkerStage,
        progress: ArchivePatcherProgress,
    ) -> Self {
        Self::Progress {
            request_id,
            stage,
            progress,
        }
    }

    /// Creates an Archive Patcher patch-completed payload.
    pub fn patch_completed(request_id: u64, result: ArchivePatcherExecutionResult) -> Self {
        Self::PatchCompleted {
            request_id,
            result: Box::new(result),
        }
    }

    /// Creates an Archive Patcher restore-completed payload.
    pub fn restore_completed(request_id: u64, result: ArchivePatcherExecutionResult) -> Self {
        Self::RestoreCompleted {
            request_id,
            result: Box::new(result),
        }
    }

    /// Creates an Archive Patcher safe-failure payload.
    pub fn safe_failure(
        request_id: u64,
        stage: ArchivePatcherWorkerStage,
        safe_message: impl Into<String>,
        diagnostic: Option<String>,
    ) -> Self {
        Self::SafeFailure {
            request_id,
            stage,
            safe_message: safe_message.into(),
            diagnostic,
        }
    }

    /// Returns the request id attached to this payload.
    pub const fn request_id(&self) -> u64 {
        match self {
            Self::CandidatesLoaded { request_id, .. }
            | Self::PlanReady { request_id, .. }
            | Self::LogRow { request_id, .. }
            | Self::Progress { request_id, .. }
            | Self::PatchCompleted { request_id, .. }
            | Self::RestoreCompleted { request_id, .. }
            | Self::SafeFailure { request_id, .. } => *request_id,
        }
    }

    /// Returns the stage associated with this payload.
    pub const fn stage(&self) -> ArchivePatcherWorkerStage {
        match self {
            Self::CandidatesLoaded { .. } => ArchivePatcherWorkerStage::Candidates,
            Self::PlanReady { .. } => ArchivePatcherWorkerStage::Plan,
            Self::LogRow { stage, .. }
            | Self::Progress { stage, .. }
            | Self::SafeFailure { stage, .. } => *stage,
            Self::PatchCompleted { .. } => ArchivePatcherWorkerStage::Patch,
            Self::RestoreCompleted { .. } => ArchivePatcherWorkerStage::Restore,
        }
    }
}

/// Downgrader worker stage used for safe failure routing and stale-event rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DowngraderWorkerStage {
    /// The worker was loading current file status.
    Status,
    /// The worker was preparing a read-only inline plan.
    Plan,
    /// The worker was executing the explicitly confirmed patch run.
    Run,
}

impl DowngraderWorkerStage {
    /// Returns a stable label suitable for structured tracing and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Plan => "plan",
            Self::Run => "run",
        }
    }
}

/// Downgrader-specific worker payloads that must cross the UI handoff boundary intact.
#[derive(Debug, Clone, PartialEq)]
pub enum DowngraderWorkerPayload {
    /// A current-file status worker produced a render-ready snapshot.
    StatusLoaded {
        /// Status request id used to reject stale worker results.
        request_id: u64,
        /// Owned status snapshot produced off the UI thread.
        snapshot: Box<DowngraderStatusSnapshot>,
    },
    /// A read-only planning worker produced an inline confirmation plan.
    PlanReady {
        /// Plan request id used to reject stale worker results.
        request_id: u64,
        /// Owned preview plan produced off the UI thread.
        plan: Box<DowngraderPreviewPlan>,
    },
    /// A confirmed run worker produced a user-visible log row.
    LogRow {
        /// Run request id used to reject stale worker results.
        request_id: u64,
        /// Reference-style user-visible row.
        row: DowngraderExecutionLogRow,
    },
    /// A confirmed run worker produced bounded progress.
    Progress {
        /// Run request id used to reject stale worker results.
        request_id: u64,
        /// Clamped progress value.
        progress: DowngraderProgress,
    },
    /// A confirmed run worker completed and returned final execution facts.
    RunCompleted {
        /// Run request id used to reject stale worker results.
        request_id: u64,
        /// Owned execution result produced off the UI thread.
        result: Box<DowngraderExecutionResult>,
    },
    /// A worker failed safely before producing its normal payload.
    SafeFailure {
        /// Request id used to reject stale worker failures.
        request_id: u64,
        /// Worker stage that failed.
        stage: DowngraderWorkerStage,
        /// User-safe failure text suitable for modal logs.
        safe_message: String,
        /// Optional diagnostic detail for logs/tests, never modal text.
        diagnostic: Option<String>,
    },
}

impl DowngraderWorkerPayload {
    /// Creates a Downgrader status-loaded payload.
    pub fn status_loaded(request_id: u64, snapshot: DowngraderStatusSnapshot) -> Self {
        Self::StatusLoaded {
            request_id,
            snapshot: Box::new(snapshot),
        }
    }

    /// Creates a Downgrader plan-ready payload.
    pub fn plan_ready(request_id: u64, plan: DowngraderPreviewPlan) -> Self {
        Self::PlanReady {
            request_id,
            plan: Box::new(plan),
        }
    }

    /// Creates a Downgrader log-row payload.
    pub fn log_row(request_id: u64, row: DowngraderExecutionLogRow) -> Self {
        Self::LogRow { request_id, row }
    }

    /// Creates a Downgrader progress payload.
    pub fn progress(request_id: u64, progress: DowngraderProgress) -> Self {
        Self::Progress {
            request_id,
            progress,
        }
    }

    /// Creates a Downgrader run-completed payload.
    pub fn run_completed(request_id: u64, result: DowngraderExecutionResult) -> Self {
        Self::RunCompleted {
            request_id,
            result: Box::new(result),
        }
    }

    /// Creates a Downgrader safe-failure payload.
    pub fn safe_failure(
        request_id: u64,
        stage: DowngraderWorkerStage,
        safe_message: impl Into<String>,
        diagnostic: Option<String>,
    ) -> Self {
        Self::SafeFailure {
            request_id,
            stage,
            safe_message: safe_message.into(),
            diagnostic,
        }
    }

    /// Returns the request id attached to this payload.
    pub const fn request_id(&self) -> u64 {
        match self {
            Self::StatusLoaded { request_id, .. }
            | Self::PlanReady { request_id, .. }
            | Self::LogRow { request_id, .. }
            | Self::Progress { request_id, .. }
            | Self::RunCompleted { request_id, .. }
            | Self::SafeFailure { request_id, .. } => *request_id,
        }
    }

    /// Returns the stage associated with this payload.
    pub const fn stage(&self) -> DowngraderWorkerStage {
        match self {
            Self::StatusLoaded { .. } => DowngraderWorkerStage::Status,
            Self::PlanReady { .. } => DowngraderWorkerStage::Plan,
            Self::LogRow { .. } | Self::Progress { .. } | Self::RunCompleted { .. } => {
                DowngraderWorkerStage::Run
            }
            Self::SafeFailure { stage, .. } => *stage,
        }
    }
}

/// High-level external operation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExternalActionKind {
    /// Desktop shell action such as opening a URL or path.
    Desktop,
    /// Child process or process-table action.
    Process,
    /// External tool launch action.
    Tool,
    /// Unknown external action retained for forward compatibility.
    Unknown,
}

impl ExternalActionKind {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Process => "process",
            Self::Tool => "tool",
            Self::Unknown => "unknown",
        }
    }
}

/// Success or typed failure state for an external action payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExternalActionOutcome {
    /// The external action was handed off or completed successfully.
    Succeeded,
    /// The external action failed with a typed platform failure kind.
    Failed(PlatformErrorKind),
}

/// Worker payload for process/tool/desktop action results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalActionPayload {
    /// High-level source of the external operation.
    pub action_kind: ExternalActionKind,
    /// Platform operation that was attempted.
    pub operation: PlatformOperation,
    /// User-selected or adapter-supplied target.
    pub target: String,
    /// Success or typed failure state.
    pub outcome: ExternalActionOutcome,
    /// Safe user-facing status text.
    pub safe_message: String,
}

impl ExternalActionPayload {
    /// Creates a successful external action payload.
    pub fn succeeded(
        action_kind: ExternalActionKind,
        operation: PlatformOperation,
        target: impl Into<String>,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            action_kind,
            operation,
            target: target.into(),
            outcome: ExternalActionOutcome::Succeeded,
            safe_message: safe_message.into(),
        }
    }

    /// Creates a failed external action payload.
    pub fn failed(
        action_kind: ExternalActionKind,
        operation: PlatformOperation,
        target: impl Into<String>,
        kind: PlatformErrorKind,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            action_kind,
            operation,
            target: target.into(),
            outcome: ExternalActionOutcome::Failed(kind),
            safe_message: safe_message.into(),
        }
    }

    /// Creates a failed external action payload from a typed platform error.
    pub fn from_platform_error(action_kind: ExternalActionKind, error: PlatformError) -> Self {
        let safe_message = error.user_message().to_owned();
        Self::failed(
            action_kind,
            error.operation,
            error.target,
            error.kind,
            safe_message,
        )
    }

    /// Returns true when the external action succeeded.
    pub const fn is_success(&self) -> bool {
        matches!(self.outcome, ExternalActionOutcome::Succeeded)
    }

    /// Returns the typed failure kind when the external action failed.
    pub const fn failure_kind(&self) -> Option<PlatformErrorKind> {
        match self.outcome {
            ExternalActionOutcome::Succeeded => None,
            ExternalActionOutcome::Failed(kind) => Some(kind),
        }
    }
}

impl From<DesktopActionResult> for ExternalActionPayload {
    fn from(result: DesktopActionResult) -> Self {
        let action_kind = match result.operation {
            PlatformOperation::OpenUrl | PlatformOperation::OpenPath => ExternalActionKind::Desktop,
            PlatformOperation::LaunchTool => ExternalActionKind::Tool,
            _ => ExternalActionKind::Unknown,
        };
        let outcome = match result.outcome {
            DesktopActionOutcome::Succeeded => ExternalActionOutcome::Succeeded,
            DesktopActionOutcome::Failed(kind) => ExternalActionOutcome::Failed(kind),
        };

        let safe_message = result.safe_message().to_owned();
        let operation = result.operation;
        let target = result.target;

        Self {
            action_kind,
            operation,
            target,
            outcome,
            safe_message,
        }
    }
}

/// Shared cancellation token passed to blocking worker closures.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    requested: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a cancellation token in the non-requested state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks cancellation as requested.
    pub fn request_cancellation(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    /// Returns whether cancellation has been requested.
    pub fn is_cancellation_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_kinds_cover_required_background_categories() {
        let kinds = [
            WorkerTaskKind::Overview,
            WorkerTaskKind::Discovery,
            WorkerTaskKind::Scan,
            WorkerTaskKind::Patch,
            WorkerTaskKind::Download,
            WorkerTaskKind::ExternalProcess,
            WorkerTaskKind::Generic,
            WorkerTaskKind::Unknown,
        ];

        assert_eq!(
            kinds.map(WorkerTaskKind::label),
            [
                "overview",
                "discovery",
                "scan",
                "patch",
                "download",
                "external-process",
                "generic",
                "unknown",
            ]
        );
    }

    #[test]
    fn event_envelope_carries_task_status_and_typed_payload() {
        let task =
            WorkerTask::new("discover-game", WorkerTaskKind::Discovery).with_label("Discover game");
        let event = WorkerEvent::completed(
            task.clone(),
            WorkerPayload::Discovery(WorkerMessage::new("Discovery complete.")),
        );

        assert_eq!(event.task, task);
        assert_eq!(event.status, WorkerTaskStatus::Completed);
        assert!(event.status.is_terminal());
        assert!(matches!(event.payload, WorkerPayload::Discovery(_)));
    }

    #[test]
    fn overview_payload_carries_owned_snapshot_and_update_state() {
        let task = WorkerTask::new("overview-refresh-1", WorkerTaskKind::Overview);
        let snapshot = OverviewSnapshot::loading("Refreshing Overview...");
        let refresh = WorkerEvent::completed(
            task.clone(),
            WorkerPayload::Overview(OverviewWorkerPayload::refresh_completed(
                1,
                snapshot.clone(),
            )),
        );
        let update = WorkerEvent::completed(
            task,
            WorkerPayload::Overview(OverviewWorkerPayload::update_check_completed(
                1,
                UpdateBannerState::Disabled,
            )),
        );

        assert!(matches!(
            refresh.payload,
            WorkerPayload::Overview(OverviewWorkerPayload::RefreshCompleted {
                refresh_id: 1,
                snapshot: ref actual,
            }) if actual.as_ref() == &snapshot
        ));
        assert!(matches!(
            update.payload,
            WorkerPayload::Overview(OverviewWorkerPayload::UpdateCheckCompleted {
                refresh_id: 1,
                update_banner: UpdateBannerState::Disabled,
            })
        ));
    }

    #[test]
    fn f4se_worker_payload_round_trips_owned_scan_snapshot() {
        let task = WorkerTask::new("s06-f4se-scan:7", WorkerTaskKind::Scan);
        let snapshot = F4seScanSnapshot::ready(Vec::new());
        let event = WorkerEvent::completed(
            task,
            WorkerPayload::F4se(F4seWorkerPayload::scan_completed(7, snapshot.clone())),
        );

        assert!(matches!(
            event.payload,
            WorkerPayload::F4se(F4seWorkerPayload::ScanCompleted {
                scan_id: 7,
                snapshot: ref actual,
            }) if actual.as_ref() == &snapshot
        ));
    }

    #[test]
    fn scanner_worker_payload_round_trips_owned_scan_snapshot_and_action_feedback() {
        use crate::domain::scanner::{
            ScannerActionFeedback, ScannerActionKind, ScannerScanSnapshot,
        };

        let task = WorkerTask::new("s07-scanner-scan:11", WorkerTaskKind::Scan);
        let snapshot = ScannerScanSnapshot::empty(11, "No scanner issues found.");
        let event = WorkerEvent::completed(
            task,
            WorkerPayload::Scanner(ScannerWorkerPayload::scan_completed(11, snapshot.clone())),
        );

        assert!(matches!(
            event.payload,
            WorkerPayload::Scanner(ScannerWorkerPayload::ScanCompleted {
                scan_id: 11,
                snapshot: ref actual,
            }) if actual.as_ref() == &snapshot
        ));

        let feedback = ScannerActionFeedback::failed(
            Some(11),
            ScannerActionKind::OpenLocation,
            "Location could not be opened.",
        )
        .with_diagnostic("raw platform detail kept out of safe text");
        let action_event = WorkerEvent::completed(
            WorkerTask::new(
                "s07-scanner-action:open-location",
                WorkerTaskKind::DesktopAction,
            ),
            WorkerPayload::Scanner(ScannerWorkerPayload::action_completed(feedback.clone())),
        );

        assert!(matches!(
            action_event.payload,
            WorkerPayload::Scanner(ScannerWorkerPayload::ActionCompleted { feedback: ref actual })
                if actual == &feedback
        ));
        assert!(!feedback.safe_message().contains("raw platform"));
    }

    #[test]
    fn scanner_worker_payload_autofix_round_trips_owned_completion() {
        use crate::domain::autofix::{
            AutoFixCompletion, AutoFixOperationKey, AutoFixResultDetail, AutoFixRevalidationPlan,
            AutoFixSelectionIdentity, AutoFixStatus, AutoFixStatusKind,
        };

        let identity = AutoFixSelectionIdentity::from_fingerprint("scanner-result:v1:test");
        let completion = AutoFixCompletion {
            scan_id: Some(42),
            result_index: Some(3),
            operation_key: AutoFixOperationKey::DeleteFile,
            selection_identity: identity.clone(),
            revalidation: AutoFixRevalidationPlan::required(identity.clone())
                .with_observed_identity(identity),
            status: AutoFixStatus::new(AutoFixStatusKind::Fixed, "Fixed fake result."),
            detail: AutoFixResultDetail::new("Fixed fake result.", "Fake details."),
        };
        let task = WorkerTask::new(
            "s08-scanner-autofix:42:3:delete-file",
            WorkerTaskKind::Patch,
        );
        let event = WorkerEvent::completed(
            task,
            WorkerPayload::Scanner(ScannerWorkerPayload::auto_fix_completed(completion.clone())),
        );

        assert!(matches!(
            event.payload,
            WorkerPayload::Scanner(ScannerWorkerPayload::AutoFixCompleted { completion: ref actual })
                if actual.as_ref() == &completion
        ));
        let payload = ScannerWorkerPayload::auto_fix_completed(completion.clone());
        assert_eq!(payload.scan_id(), Some(42));
        assert!(payload.snapshot().is_none());
        assert_eq!(payload.auto_fix_completion(), Some(&completion));
    }

    #[test]
    fn downgrader_worker_payload_round_trips_owned_status_plan_log_progress_run_and_failure() {
        use std::path::PathBuf;

        use crate::{
            domain::downgrader::{
                DowngraderExecutionLogRow, DowngraderLogLevel, DowngraderOptionsSnapshot,
                DowngraderProgress, DowngraderTarget,
            },
            services::downgrader::{
                DowngraderExecutionResult, DowngraderPreviewPlan, DowngraderPreviewPlanCounts,
                DowngraderStatusSnapshot,
            },
            workers::{RecordingEventSink, WorkerEventSink},
        };

        let status = DowngraderStatusSnapshot {
            request_id: 10,
            game_root: PathBuf::from("Game"),
            rows: Vec::new(),
            default_target: DowngraderTarget::OldGen,
            unknown_game: false,
            unknown_creation_kit: false,
            diagnostics: Vec::new(),
        };
        let options = DowngraderOptionsSnapshot::new(DowngraderTarget::OldGen, true, true);
        let plan = DowngraderPreviewPlan {
            request_id: 11,
            game_root: PathBuf::from("Game"),
            options,
            status: status.clone(),
            rows: Vec::new(),
            counts: DowngraderPreviewPlanCounts::default(),
            can_execute: true,
        };
        let log_row =
            DowngraderExecutionLogRow::new(DowngraderLogLevel::Good, "Patched Fallout4.exe");
        let result = DowngraderExecutionResult {
            request_id: 12,
            game_root: PathBuf::from("Game"),
            options,
            rows: Vec::new(),
            log_rows: vec![log_row.clone()],
            progress_events: Vec::new(),
            diagnostics: Vec::new(),
        };
        let sink = RecordingEventSink::new();
        let task = WorkerTask::new("s09-downgrader-run:12", WorkerTaskKind::Patch);

        for event in [
            WorkerEvent::completed(
                WorkerTask::new("s09-downgrader-status:10", WorkerTaskKind::Patch),
                WorkerPayload::Downgrader(DowngraderWorkerPayload::status_loaded(
                    10,
                    status.clone(),
                )),
            ),
            WorkerEvent::completed(
                WorkerTask::new("s09-downgrader-plan:11", WorkerTaskKind::Patch),
                WorkerPayload::Downgrader(DowngraderWorkerPayload::plan_ready(11, plan.clone())),
            ),
            WorkerEvent::new(
                task.clone(),
                WorkerTaskStatus::Progress,
                WorkerPayload::Downgrader(DowngraderWorkerPayload::log_row(12, log_row.clone())),
            ),
            WorkerEvent::new(
                task.clone(),
                WorkerTaskStatus::Progress,
                WorkerPayload::Downgrader(DowngraderWorkerPayload::progress(
                    12,
                    DowngraderProgress::new(42.5),
                )),
            ),
            WorkerEvent::completed(
                task.clone(),
                WorkerPayload::Downgrader(DowngraderWorkerPayload::run_completed(
                    12,
                    result.clone(),
                )),
            ),
            WorkerEvent::failed(
                WorkerTask::new("s09-downgrader-run:13", WorkerTaskKind::Patch),
                WorkerFailure::new("Downgrader failed safely."),
            ),
            WorkerEvent::new(
                WorkerTask::new("s09-downgrader-plan:14", WorkerTaskKind::Patch),
                WorkerTaskStatus::Failed,
                WorkerPayload::Downgrader(DowngraderWorkerPayload::safe_failure(
                    14,
                    DowngraderWorkerStage::Plan,
                    "Downgrader plan failed safely.",
                    Some("raw diagnostic".to_owned()),
                )),
            ),
        ] {
            sink.emit(event).expect("recording sink should store event");
        }

        let events = sink.events().expect("recorded events should be readable");
        assert_eq!(events.len(), 7);
        assert!(matches!(
            &events[0].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::StatusLoaded { request_id: 10, snapshot })
                if snapshot.as_ref() == &status
        ));
        assert!(matches!(
            &events[1].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::PlanReady { request_id: 11, plan: actual })
                if actual.as_ref() == &plan
        ));
        assert!(matches!(
            &events[2].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::LogRow { request_id: 12, row })
                if row == &log_row
        ));
        assert!(matches!(
            &events[3].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::Progress { request_id: 12, progress })
                if progress.percent == 42.5
        ));
        assert!(matches!(
            &events[4].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::RunCompleted { request_id: 12, result: actual })
                if actual.as_ref() == &result
        ));
        assert!(matches!(&events[5].payload, WorkerPayload::Error(_)));
        assert!(matches!(
            &events[6].payload,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::SafeFailure {
                request_id: 14,
                stage: DowngraderWorkerStage::Plan,
                safe_message,
                diagnostic: Some(diagnostic),
            }) if safe_message == "Downgrader plan failed safely." && diagnostic == "raw diagnostic"
        ));
    }

    #[test]
    fn archive_patcher_worker_payload_round_trips_owned_candidates_plan_log_progress_patch_restore_and_failure()
     {
        use std::path::PathBuf;

        use crate::{
            domain::{
                archive_patcher::{
                    ArchivePatcherArchiveFormat, ArchivePatcherCandidateRow,
                    ArchivePatcherCandidateSnapshot, ArchivePatcherExecutionFileResult,
                    ArchivePatcherExecutionOutcome, ArchivePatcherExecutionResult,
                    ArchivePatcherHeader, ArchivePatcherLogLevel, ArchivePatcherLogRow,
                    ArchivePatcherPreviewPlan, ArchivePatcherPreviewPlanRow,
                    ArchivePatcherProgress, ArchivePatcherRestoreManifestEntry,
                    ArchivePatcherSummaryCounts, ArchivePatcherTarget, ba2_header_prefix,
                    patched_to_target_log_row, restored_to_original_log_row,
                },
                discovery::{ArchiveFormat, ArchiveVersion},
            },
            workers::{RecordingEventSink, WorkerEventSink},
        };

        let candidate = ArchivePatcherCandidateRow::new(
            "Game/Data/A.ba2",
            "A.ba2",
            ArchiveFormat::General,
            ArchiveVersion::NextGen8,
            ArchivePatcherTarget::OldGen,
        );
        let candidates = ArchivePatcherCandidateSnapshot::new(
            20,
            ArchivePatcherTarget::OldGen,
            Some("A".to_owned()),
            vec![candidate.clone()],
        );
        let header = ArchivePatcherHeader::new(8, ArchivePatcherArchiveFormat::General);
        let manifest_entry = ArchivePatcherRestoreManifestEntry::new(
            "Game/Data/A.ba2",
            "A.ba2",
            "A.ba2",
            ArchivePatcherArchiveFormat::General,
            8,
            1,
        )
        .with_header_prefixes(
            ba2_header_prefix(8, ArchivePatcherArchiveFormat::General),
            ba2_header_prefix(1, ArchivePatcherArchiveFormat::General),
        );
        let plan = ArchivePatcherPreviewPlan::from_rows(
            21,
            ArchivePatcherTarget::OldGen,
            Some("A".to_owned()),
            Some(PathBuf::from("Game/Data")),
            ArchivePatcherCandidateSnapshot::new(
                21,
                ArchivePatcherTarget::OldGen,
                Some("A".to_owned()),
                vec![candidate.clone()],
            ),
            vec![ArchivePatcherPreviewPlanRow::patch(
                candidate.clone(),
                header,
                manifest_entry,
            )],
        );
        let patch_log = patched_to_target_log_row(ArchivePatcherTarget::OldGen, "A.ba2");
        let patch_result = ArchivePatcherExecutionResult {
            request_id: 22,
            target: ArchivePatcherTarget::OldGen,
            manifest_path: PathBuf::from("State/archive-patcher-latest.json"),
            plan_digest: plan.stable_digest(),
            rows: vec![ArchivePatcherExecutionFileResult {
                archive_path: PathBuf::from("Game/Data/A.ba2"),
                file_name: "A.ba2".to_owned(),
                outcome: ArchivePatcherExecutionOutcome::Patched,
                log_row: patch_log.clone(),
                diagnostics: Vec::new(),
            }],
            log_rows: vec![patch_log.clone()],
            counts: ArchivePatcherSummaryCounts::patch(1, 0),
            diagnostics: Vec::new(),
        };
        let restore_log = restored_to_original_log_row(8, "A.ba2");
        let restore_result = ArchivePatcherExecutionResult {
            request_id: 23,
            target: ArchivePatcherTarget::OldGen,
            manifest_path: PathBuf::from("State/archive-patcher-latest.json"),
            plan_digest: "digest".to_owned(),
            rows: vec![ArchivePatcherExecutionFileResult {
                archive_path: PathBuf::from("Game/Data/A.ba2"),
                file_name: "A.ba2".to_owned(),
                outcome: ArchivePatcherExecutionOutcome::Restored,
                log_row: restore_log.clone(),
                diagnostics: Vec::new(),
            }],
            log_rows: vec![restore_log],
            counts: ArchivePatcherSummaryCounts::restore(1, 0, 0),
            diagnostics: Vec::new(),
        };
        let sink = RecordingEventSink::new();
        let patch_task = WorkerTask::new("s10-archive-patcher-patch:22", WorkerTaskKind::Patch);
        let restore_task = WorkerTask::new("s10-archive-patcher-restore:23", WorkerTaskKind::Patch);

        for event in [
            WorkerEvent::completed(
                WorkerTask::new("s10-archive-patcher-candidates:20", WorkerTaskKind::Patch),
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::candidates_loaded(
                    20,
                    candidates.clone(),
                )),
            ),
            WorkerEvent::completed(
                WorkerTask::new("s10-archive-patcher-plan:21", WorkerTaskKind::Patch),
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::plan_ready(
                    21,
                    plan.clone(),
                )),
            ),
            WorkerEvent::new(
                patch_task.clone(),
                WorkerTaskStatus::Progress,
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::log_row(
                    22,
                    ArchivePatcherWorkerStage::Patch,
                    ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Info, "Patching A.ba2"),
                )),
            ),
            WorkerEvent::new(
                patch_task.clone(),
                WorkerTaskStatus::Progress,
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::progress(
                    22,
                    ArchivePatcherWorkerStage::Patch,
                    ArchivePatcherProgress::new("Half", 50.0),
                )),
            ),
            WorkerEvent::completed(
                patch_task,
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::patch_completed(
                    22,
                    patch_result.clone(),
                )),
            ),
            WorkerEvent::completed(
                restore_task.clone(),
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::restore_completed(
                    23,
                    restore_result.clone(),
                )),
            ),
            WorkerEvent::failed(
                WorkerTask::new("s10-archive-patcher-restore:24", WorkerTaskKind::Patch),
                WorkerFailure::new("Archive Patcher failed safely."),
            ),
            WorkerEvent::new(
                WorkerTask::new("s10-archive-patcher-plan:25", WorkerTaskKind::Patch),
                WorkerTaskStatus::Failed,
                WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::safe_failure(
                    25,
                    ArchivePatcherWorkerStage::Plan,
                    "Archive Patcher plan failed safely.",
                    Some("raw diagnostic".to_owned()),
                )),
            ),
        ] {
            sink.emit(event).expect("recording sink should store event");
        }

        let events = sink.events().expect("recorded events should be readable");
        assert_eq!(events.len(), 8);
        assert!(matches!(
            &events[0].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::CandidatesLoaded { request_id: 20, snapshot })
                if snapshot.as_ref() == &candidates
        ));
        assert!(matches!(
            &events[1].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::PlanReady { request_id: 21, plan: actual })
                if actual.as_ref() == &plan
        ));
        assert!(matches!(
            &events[2].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::LogRow {
                request_id: 22,
                stage: ArchivePatcherWorkerStage::Patch,
                ..
            })
        ));
        assert!(matches!(
            &events[3].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::Progress {
                request_id: 22,
                stage: ArchivePatcherWorkerStage::Patch,
                progress,
            }) if progress.percent == 50.0
        ));
        assert!(matches!(
            &events[4].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::PatchCompleted { request_id: 22, result })
                if result.as_ref() == &patch_result
        ));
        assert!(matches!(
            &events[5].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::RestoreCompleted { request_id: 23, result })
                if result.as_ref() == &restore_result
        ));
        assert!(matches!(&events[6].payload, WorkerPayload::Error(_)));
        assert!(matches!(
            &events[7].payload,
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::SafeFailure {
                request_id: 25,
                stage: ArchivePatcherWorkerStage::Plan,
                safe_message,
                diagnostic: Some(diagnostic),
            }) if safe_message == "Archive Patcher plan failed safely." && diagnostic == "raw diagnostic"
        ));

        let payload = ArchivePatcherWorkerPayload::progress(
            30,
            ArchivePatcherWorkerStage::Restore,
            ArchivePatcherProgress::new("Restoring", 25.0),
        );
        assert_eq!(payload.request_id(), 30);
        assert_eq!(payload.stage(), ArchivePatcherWorkerStage::Restore);
        assert!(ArchivePatcherWorkerStage::Restore.is_mutation());
        assert!(!ArchivePatcherWorkerStage::Plan.is_mutation());
    }

    #[test]
    fn progress_supports_optional_text_and_counts_without_percentage_requirements() {
        let text_only = WorkerProgress::new().with_message("Scanning Data folder");
        let counts_only = WorkerProgress::new().with_counts(Some(3), Some(10));
        let open_ended = WorkerProgress::new().with_counts(Some(7), None);

        assert_eq!(text_only.message.as_deref(), Some("Scanning Data folder"));
        assert_eq!(text_only.current, None);
        assert_eq!(counts_only.current, Some(3));
        assert_eq!(counts_only.total, Some(10));
        assert_eq!(open_ended.current, Some(7));
        assert_eq!(open_ended.total, None);
    }

    #[test]
    fn cancellation_request_acknowledgement_and_final_cancelled_are_distinct_events() {
        let task = WorkerTask::new("scan-cancel", WorkerTaskKind::Scan);
        let requested = WorkerEvent::cancellation_requested(
            task.clone(),
            WorkerCancellation::with_message("Cancel requested."),
        );
        let acknowledged = WorkerEvent::cancellation_acknowledged(
            task.clone(),
            WorkerCancellation::with_message("Worker stopping."),
        );
        let cancelled = WorkerEvent::cancelled(task, WorkerCancellation::with_message("Stopped."));

        assert_eq!(requested.status, WorkerTaskStatus::CancellationRequested);
        assert_eq!(
            acknowledged.status,
            WorkerTaskStatus::CancellationAcknowledged
        );
        assert_eq!(cancelled.status, WorkerTaskStatus::Cancelled);
        assert!(!requested.status.is_terminal());
        assert!(!acknowledged.status.is_terminal());
        assert!(cancelled.status.is_terminal());
    }

    #[test]
    fn s05_actions_worker_payload_round_trips_tools_and_about_feedback() {
        use crate::{
            domain::tools::{AboutActionId, AboutLinkId, ToolActionId},
            services::tools::{
                AboutActionFeedback, AboutActionKind, ActionRejectionKind, ToolsActionFeedback,
            },
        };

        let tools_feedback = ToolsActionFeedback::rejected(
            ToolActionId::ArchivePatcher.as_str(),
            None,
            ActionRejectionKind::DisabledUtility,
            "Archive Patcher is not available in this Rust port yet.",
            Some("deferred utility".to_owned()),
        );
        let about_feedback = AboutActionFeedback::succeeded(
            AboutActionId::CopyGithub.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Github,
                action_id: AboutActionId::CopyGithub,
            },
            "Copied to clipboard.",
        );

        let tools_event = WorkerEvent::completed(
            WorkerTask::new("tools-action", WorkerTaskKind::DesktopAction),
            WorkerPayload::ToolsAction(ToolsActionWorkerPayload::action_completed(
                tools_feedback.clone(),
            )),
        );
        let about_event = WorkerEvent::completed(
            WorkerTask::new("about-action", WorkerTaskKind::DesktopAction),
            WorkerPayload::AboutAction(AboutActionWorkerPayload::action_completed(
                about_feedback.clone(),
            )),
        );

        assert!(matches!(
            tools_event.payload,
            WorkerPayload::ToolsAction(ToolsActionWorkerPayload { ref feedback })
                if feedback == &tools_feedback
        ));
        assert!(matches!(
            about_event.payload,
            WorkerPayload::AboutAction(AboutActionWorkerPayload { ref feedback })
                if feedback == &about_feedback
        ));
    }

    #[test]
    fn desktop_action_results_flow_through_external_action_payloads() {
        let success =
            DesktopActionResult::success(PlatformOperation::OpenUrl, "https://example.invalid/cmt");
        let success_payload = ExternalActionPayload::from(success);

        assert_eq!(success_payload.action_kind, ExternalActionKind::Desktop);
        assert_eq!(success_payload.operation, PlatformOperation::OpenUrl);
        assert_eq!(success_payload.target, "https://example.invalid/cmt");
        assert!(success_payload.is_success());
        assert_eq!(success_payload.safe_message, "Opened URL.");

        let failure = DesktopActionResult::failure(PlatformError::command_failed(
            PlatformOperation::LaunchTool,
            r"C:\Tools\BSArch.exe",
            "raw OS diagnostic detail",
        ));
        let failure_payload = ExternalActionPayload::from(failure);

        assert_eq!(failure_payload.action_kind, ExternalActionKind::Tool);
        assert_eq!(failure_payload.operation, PlatformOperation::LaunchTool);
        assert_eq!(failure_payload.target, r"C:\Tools\BSArch.exe");
        assert_eq!(
            failure_payload.failure_kind(),
            Some(PlatformErrorKind::CommandFailed)
        );
        assert_eq!(failure_payload.safe_message, "Tool launch failed.");
        assert!(!failure_payload.safe_message.contains("raw OS"));
    }
}
