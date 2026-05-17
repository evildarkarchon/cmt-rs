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

use crate::platform::{
    PlatformError, PlatformErrorKind, PlatformOperation,
    desktop::{DesktopActionOutcome, DesktopActionResult},
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerPayload {
    /// No additional payload is needed for this lifecycle event.
    None,
    /// Progress text and optional counts.
    Progress(WorkerProgress),
    /// Discovery-specific status or result text.
    Discovery(WorkerMessage),
    /// Scanner-specific status or result text.
    Scan(WorkerMessage),
    /// Patcher-specific status or result text.
    Patch(WorkerMessage),
    /// Download-specific status or result text.
    Download(WorkerMessage),
    /// External process, tool, or desktop-action result.
    ExternalAction(ExternalActionPayload),
    /// Cancellation details.
    Cancellation(WorkerCancellation),
    /// User-safe failure plus optional diagnostics.
    Error(WorkerFailure),
    /// Generic status or result text.
    Generic(WorkerMessage),
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
