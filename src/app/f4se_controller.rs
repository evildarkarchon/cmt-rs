//! Slint-free F4SE diagnostics controller and worker-payload reducer.
//!
//! The controller owns no Slint handles, performs no filesystem work, and stores
//! only the current render snapshot plus one active scan id. UI code can request
//! a lazy initial scan when the F4SE tab is first activated, schedule the returned
//! worker task off the event loop, and feed owned worker events back here.

use crate::{
    domain::f4se::{F4SE_LOADING_TEXT, F4seScanSnapshot, F4seScanStatus},
    workers::{
        F4seWorkerPayload, WorkerEvent, WorkerFailure, WorkerPayload, WorkerSpawnError, WorkerTask,
        WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for S06 F4SE scan worker task identifiers.
pub const F4SE_SCAN_TASK_PREFIX: &str = "s06-f4se-scan:";
/// Safe loading-error text shown when the worker cannot be scheduled.
pub const F4SE_SCAN_START_FAILED_MESSAGE: &str = "F4SE scan could not be started.";

/// Monotonic identity assigned to each F4SE DLL scan request.
pub type F4seScanId = u64;

/// Render-relevant lifecycle state for the F4SE diagnostics tab.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seControllerPhase {
    /// No scan has been requested or started yet.
    #[default]
    NotStarted,
    /// A worker is currently scanning DLLs.
    Scanning,
    /// A scan completed with inspectable rows or an empty successful result.
    Ready,
    /// A scan could not start or completed with a safe loading error.
    LoadingError,
}

/// Result of applying a controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seTransitionResult {
    /// The event or method matched the active F4SE scan and changed state.
    Applied,
    /// The event belonged to an older F4SE scan and was intentionally ignored.
    StaleIgnored,
    /// The event was not an F4SE scan event for this controller.
    Ignored,
}

impl F4seTransitionResult {
    /// Returns true when the transition changed controller state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// Work request returned by F4SE scan intents and consumed by worker wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F4seScanWorkerRequest {
    /// Monotonic scan request id used to reject stale worker results.
    pub scan_id: F4seScanId,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl F4seScanWorkerRequest {
    /// Creates a worker request for the supplied scan id.
    pub fn new(scan_id: F4seScanId) -> Self {
        Self {
            scan_id,
            task: f4se_scan_task(scan_id),
        }
    }
}

/// Pure reducer for F4SE diagnostics UI state and owned worker events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F4seController {
    snapshot: F4seScanSnapshot,
    phase: F4seControllerPhase,
    next_scan_id: F4seScanId,
    active_scan_id: Option<F4seScanId>,
    initial_scan_requested: bool,
}

impl Default for F4seController {
    fn default() -> Self {
        Self::new()
    }
}

impl F4seController {
    /// Creates an idle controller with no rows and no queued work.
    pub fn new() -> Self {
        Self {
            snapshot: F4seScanSnapshot::idle(),
            phase: F4seControllerPhase::NotStarted,
            next_scan_id: 1,
            active_scan_id: None,
            initial_scan_requested: false,
        }
    }

    /// Returns the current render-ready F4SE scan snapshot.
    pub fn snapshot(&self) -> &F4seScanSnapshot {
        &self.snapshot
    }

    /// Returns the current controller lifecycle phase.
    pub const fn phase(&self) -> F4seControllerPhase {
        self.phase
    }

    /// Returns the one active scan id, if a scan is in flight.
    pub const fn active_scan_id(&self) -> Option<F4seScanId> {
        self.active_scan_id
    }

    /// Returns the next scan id that will be assigned.
    pub const fn next_scan_id(&self) -> F4seScanId {
        self.next_scan_id
    }

    /// Requests the lazy initial F4SE scan exactly once.
    ///
    /// Repeated tab activation must not enqueue unbounded work. Future explicit
    /// refresh buttons can call [`F4seController::request_scan`] instead.
    pub fn request_initial_scan(&mut self) -> Option<F4seScanWorkerRequest> {
        if self.initial_scan_requested {
            tracing::debug!(
                event = "s06-f4se-initial-scan-ignored",
                active_scan_id = ?self.active_scan_id,
                "F4SE initial scan request ignored because it was already requested"
            );
            return None;
        }

        self.initial_scan_requested = true;
        Some(self.request_scan())
    }

    /// Requests a new F4SE scan and makes it the only active scan id.
    pub fn request_scan(&mut self) -> F4seScanWorkerRequest {
        let scan_id = self.next_scan_id;
        self.next_scan_id = self.next_scan_id.saturating_add(1);
        self.active_scan_id = Some(scan_id);

        let request = F4seScanWorkerRequest::new(scan_id);
        tracing::info!(
            event = "s06-f4se-scan-requested",
            scan_id,
            task_id = %request.task.id,
            "F4SE DLL scan requested"
        );
        request
    }

    /// Applies the loading transition once worker scheduling succeeds.
    pub fn scan_started(&mut self, scan_id: F4seScanId) -> F4seTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s06-f4se-scan-start-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale F4SE scan start"
            );
            return F4seTransitionResult::StaleIgnored;
        }

        self.phase = F4seControllerPhase::Scanning;
        self.snapshot = F4seScanSnapshot::loading();
        tracing::info!(
            event = "s06-f4se-scan-started",
            scan_id,
            status = F4seScanStatus::Loading.label(),
            status_message = F4SE_LOADING_TEXT,
            "F4SE DLL scan started"
        );
        F4seTransitionResult::Applied
    }

    /// Applies a worker snapshot if it belongs to the active F4SE scan.
    pub fn scan_completed(
        &mut self,
        scan_id: F4seScanId,
        snapshot: F4seScanSnapshot,
    ) -> F4seTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s06-f4se-scan-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale F4SE scan completion"
            );
            return F4seTransitionResult::StaleIgnored;
        }

        let status = snapshot.status;
        let row_count = snapshot.rows.len();
        let status_message = snapshot.status_message.clone();
        self.phase = phase_from_snapshot(&snapshot);
        self.snapshot = snapshot;
        self.active_scan_id = None;

        tracing::info!(
            event = "s06-f4se-scan-completed",
            scan_id,
            status = status.label(),
            rows = row_count,
            status_message = status_message.as_str(),
            "F4SE DLL scan completed"
        );
        F4seTransitionResult::Applied
    }

    /// Maps a worker failure into the safe loading-error state for the active scan.
    pub fn scan_failed(
        &mut self,
        scan_id: F4seScanId,
        failure: WorkerFailure,
    ) -> F4seTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s06-f4se-scan-failure-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale F4SE scan failure"
            );
            return F4seTransitionResult::StaleIgnored;
        }

        tracing::error!(
            event = "s06-f4se-scan-failed",
            scan_id,
            safe_message = failure.safe_message(),
            diagnostic = failure.diagnostic().unwrap_or(""),
            "F4SE DLL scan failed"
        );
        self.phase = F4seControllerPhase::LoadingError;
        self.snapshot = F4seScanSnapshot::error(failure.safe_message().to_owned());
        self.active_scan_id = None;
        F4seTransitionResult::Applied
    }

    /// Maps a worker spawn failure into the safe loading-error state.
    pub fn spawn_failed(
        &mut self,
        scan_id: F4seScanId,
        error: WorkerSpawnError,
    ) -> F4seTransitionResult {
        tracing::error!(
            event = "s06-f4se-scan-spawn-failed",
            scan_id,
            diagnostic = %error,
            "F4SE DLL scan worker could not be scheduled"
        );
        self.scan_failed(
            scan_id,
            WorkerFailure::new(F4SE_SCAN_START_FAILED_MESSAGE).with_diagnostic(error.to_string()),
        )
    }

    /// Applies an owned worker event if it carries a matching F4SE payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> F4seTransitionResult {
        let task = event.task;
        let status = event.status;
        match event.payload {
            WorkerPayload::F4se(payload)
                if task.kind == WorkerTaskKind::Scan
                    && status == WorkerTaskStatus::Completed
                    && f4se_scan_id_from_task_id(&task.id) == Some(payload.scan_id()) =>
            {
                self.handle_f4se_payload(payload)
            }
            WorkerPayload::Error(failure)
                if task.kind == WorkerTaskKind::Scan && status == WorkerTaskStatus::Failed =>
            {
                match f4se_scan_id_from_task_id(&task.id) {
                    Some(scan_id) => self.scan_failed(scan_id, failure),
                    None => F4seTransitionResult::Ignored,
                }
            }
            WorkerPayload::None
                if task.kind == WorkerTaskKind::Scan && status == WorkerTaskStatus::Running =>
            {
                match f4se_scan_id_from_task_id(&task.id) {
                    Some(scan_id) => self.scan_started(scan_id),
                    None => F4seTransitionResult::Ignored,
                }
            }
            _ => F4seTransitionResult::Ignored,
        }
    }

    fn handle_f4se_payload(&mut self, payload: F4seWorkerPayload) -> F4seTransitionResult {
        match payload {
            F4seWorkerPayload::ScanCompleted { scan_id, snapshot } => {
                self.scan_completed(scan_id, *snapshot)
            }
        }
    }

    fn is_active_scan(&self, scan_id: F4seScanId) -> bool {
        self.active_scan_id == Some(scan_id)
    }
}

/// Builds worker metadata for a blocking F4SE DLL scan.
pub fn f4se_scan_task(scan_id: F4seScanId) -> WorkerTask {
    WorkerTask::new(
        format!("{F4SE_SCAN_TASK_PREFIX}{scan_id}"),
        WorkerTaskKind::Scan,
    )
    .with_label("Scan F4SE DLLs")
}

/// Converts a completed scan snapshot into the F4SE worker payload shape.
pub fn f4se_scan_completed_payload(
    scan_id: F4seScanId,
    snapshot: F4seScanSnapshot,
) -> WorkerPayload {
    WorkerPayload::F4se(F4seWorkerPayload::scan_completed(scan_id, snapshot))
}

/// Parses an S06 F4SE scan id from a worker task id.
pub fn f4se_scan_id_from_task_id(task_id: &WorkerTaskId) -> Option<F4seScanId> {
    task_id
        .as_str()
        .strip_prefix(F4SE_SCAN_TASK_PREFIX)
        .and_then(|value| value.parse::<F4seScanId>().ok())
}

fn phase_from_snapshot(snapshot: &F4seScanSnapshot) -> F4seControllerPhase {
    match snapshot.status {
        F4seScanStatus::Idle => F4seControllerPhase::NotStarted,
        F4seScanStatus::Loading => F4seControllerPhase::Scanning,
        F4seScanStatus::Ready => F4seControllerPhase::Ready,
        F4seScanStatus::Error => F4seControllerPhase::LoadingError,
    }
}

trait F4seScanStatusLabel {
    fn label(self) -> &'static str;
}

impl F4seScanStatusLabel for F4seScanStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::f4se::{
            F4seCompatibilityIcon, F4seDllFacts, F4seGameTarget, F4seRowSeverity,
            render_f4se_dll_row,
        },
        workers::{WorkerMessage, WorkerPayload},
    };

    fn ready_snapshot(dll_name: &str) -> F4seScanSnapshot {
        let facts = F4seDllFacts::f4se(dll_name, true, true, Some(true), Some(false));
        F4seScanSnapshot::ready(vec![render_f4se_dll_row(&facts, F4seGameTarget::NextGen)])
    }

    fn unknown_game_warning_snapshot(dll_name: &str) -> F4seScanSnapshot {
        let facts = F4seDllFacts::f4se(dll_name, false, true, Some(true), Some(true));
        F4seScanSnapshot::ready(vec![render_f4se_dll_row(&facts, F4seGameTarget::Unknown)])
    }

    #[test]
    fn f4se_controller_initial_scan_is_lazy_and_requested_once() {
        let mut controller = F4seController::new();
        assert_eq!(controller.phase(), F4seControllerPhase::NotStarted);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Idle);

        let request = controller
            .request_initial_scan()
            .expect("first activation should request work");
        let duplicate = controller.request_initial_scan();

        assert_eq!(request.scan_id, 1);
        assert_eq!(request.task.kind, WorkerTaskKind::Scan);
        assert_eq!(request.task.id.as_str(), "s06-f4se-scan:1");
        assert_eq!(controller.active_scan_id(), Some(1));
        assert!(duplicate.is_none());
        assert_eq!(controller.next_scan_id(), 2);
    }

    #[test]
    fn f4se_controller_scan_started_sets_loading_state_for_active_scan() {
        let mut controller = F4seController::new();
        let request = controller
            .request_initial_scan()
            .expect("scan should be requested");

        let result = controller.scan_started(request.scan_id);

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(controller.phase(), F4seControllerPhase::Scanning);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Loading);
        assert_eq!(controller.snapshot().status_message, F4SE_LOADING_TEXT);
    }

    #[test]
    fn f4se_controller_scan_ids_increment_monotonically() {
        let mut controller = F4seController::new();

        let first = controller.request_scan();
        let second = controller.request_scan();
        let third = controller.request_scan();

        assert_eq!(first.scan_id, 1);
        assert_eq!(second.scan_id, 2);
        assert_eq!(third.scan_id, 3);
        assert_eq!(controller.active_scan_id(), Some(3));
        assert_eq!(controller.next_scan_id(), 4);
    }

    #[test]
    fn f4se_controller_worker_payload_applies_latest_ready_rows() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        let event = WorkerEvent::completed(
            request.task.clone(),
            f4se_scan_completed_payload(request.scan_id, ready_snapshot("modern.dll")),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(controller.phase(), F4seControllerPhase::Ready);
        assert_eq!(controller.active_scan_id(), None);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Ready);
        assert_eq!(controller.snapshot().rows[0].dll_name, "modern.dll");
    }

    #[test]
    fn f4se_controller_stale_completion_is_ignored_and_preserves_ready_rows() {
        let mut controller = F4seController::new();
        let first = controller.request_scan();
        controller.scan_started(first.scan_id);
        let second = controller.request_scan();
        controller.scan_started(second.scan_id);

        let latest = WorkerEvent::completed(
            second.task.clone(),
            f4se_scan_completed_payload(second.scan_id, ready_snapshot("new.dll")),
        );
        assert_eq!(
            controller.handle_worker_event(latest),
            F4seTransitionResult::Applied
        );

        let stale = WorkerEvent::completed(
            first.task.clone(),
            f4se_scan_completed_payload(first.scan_id, ready_snapshot("old.dll")),
        );
        let result = controller.handle_worker_event(stale);

        assert_eq!(result, F4seTransitionResult::StaleIgnored);
        assert_eq!(controller.phase(), F4seControllerPhase::Ready);
        assert_eq!(controller.snapshot().rows.len(), 1);
        assert_eq!(controller.snapshot().rows[0].dll_name, "new.dll");
    }

    #[test]
    fn f4se_controller_worker_failure_event_surfaces_safe_message_without_diagnostic() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        let event = WorkerEvent::failed(
            request.task.clone(),
            WorkerFailure::new("F4SE scan failed safely.")
                .with_diagnostic(r"raw C:\Users\example diagnostic"),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(controller.phase(), F4seControllerPhase::LoadingError);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Error);
        assert_eq!(
            controller.snapshot().status_message,
            "F4SE scan failed safely."
        );
        assert!(!controller.snapshot().status_message.contains("raw"));
        assert_eq!(controller.active_scan_id(), None);
    }

    #[test]
    fn f4se_controller_spawn_failure_surfaces_safe_loading_error() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();

        let result = controller.spawn_failed(
            request.scan_id,
            WorkerSpawnError::NoActiveRuntime {
                task_id: request.task.id.clone(),
            },
        );

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(controller.phase(), F4seControllerPhase::LoadingError);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Error);
        assert_eq!(
            controller.snapshot().status_message,
            F4SE_SCAN_START_FAILED_MESSAGE
        );
        assert!(!controller.snapshot().status_message.contains("Tokio"));
        assert_eq!(controller.active_scan_id(), None);
    }

    #[test]
    fn f4se_controller_unrelated_payload_is_ignored_and_preserves_ready_rows() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        controller.scan_completed(request.scan_id, ready_snapshot("kept.dll"));
        let before = controller.snapshot().clone();
        let event = WorkerEvent::completed(
            WorkerTask::new("generic-worker", WorkerTaskKind::Generic),
            WorkerPayload::Generic(WorkerMessage::new("Generic complete.")),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Ignored);
        assert_eq!(controller.snapshot(), &before);
    }

    #[test]
    fn f4se_controller_non_f4se_scan_payload_is_ignored() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        let event = WorkerEvent::completed(
            WorkerTask::new("other-scan", WorkerTaskKind::Scan),
            WorkerPayload::Scan(WorkerMessage::new("Other scan complete.")),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Ignored);
        assert_eq!(controller.phase(), F4seControllerPhase::Scanning);
    }

    #[test]
    fn f4se_controller_failed_worker_event_with_unmatched_task_id_is_ignored() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        let event = WorkerEvent::failed(
            WorkerTask::new("other-scan", WorkerTaskKind::Scan),
            WorkerFailure::new("Other scan failed.").with_diagnostic("raw"),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Ignored);
        assert_eq!(controller.phase(), F4seControllerPhase::Scanning);
        assert_eq!(controller.snapshot().status, F4seScanStatus::Loading);
    }

    #[test]
    fn f4se_controller_unknown_game_warning_rows_remain_visible_after_completion() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        controller.scan_started(request.scan_id);
        let event = WorkerEvent::completed(
            request.task.clone(),
            f4se_scan_completed_payload(
                request.scan_id,
                unknown_game_warning_snapshot("unknown-game.dll"),
            ),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(controller.phase(), F4seControllerPhase::Ready);
        assert_eq!(
            controller.snapshot().rows[0].severity,
            F4seRowSeverity::Warning
        );
        assert_eq!(
            controller.snapshot().rows[0].your_game.icon,
            F4seCompatibilityIcon::Warning
        );
        assert_eq!(controller.snapshot().rows[0].dll_name, "unknown-game.dll");
    }
}
