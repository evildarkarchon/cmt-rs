//! Slint-free Downgrader modal controller and worker-payload reducer.
//!
//! The controller owns modal lifecycle state, target/options, inline plan
//! confirmation, visible log/progress rows, and stale-event rejection. It does
//! no filesystem, network, xdelta, settings persistence, or Slint work. UI code
//! should schedule the returned owned worker requests off the event loop and feed
//! typed [`DowngraderWorkerPayload`](crate::workers::DowngraderWorkerPayload)
//! events back through this reducer.

use crate::{
    domain::{
        discovery::Fallout4Installation,
        downgrader::{
            DowngraderExecutionLogRow, DowngraderLogLevel, DowngraderOptionsSnapshot,
            DowngraderProgress, DowngraderStatusRow, DowngraderTarget,
        },
        settings::DowngraderSettings,
    },
    services::downgrader::{
        DowngraderExecutionResult, DowngraderPreviewPlan, DowngraderStatusSnapshot,
    },
    workers::{
        DowngraderWorkerPayload, DowngraderWorkerStage, WorkerEvent, WorkerPayload,
        WorkerSpawnError, WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for S09 Downgrader status worker task identifiers.
pub const DOWNGRADER_STATUS_TASK_PREFIX: &str = "s09-downgrader-status:";
/// Stable prefix for S09 Downgrader plan worker task identifiers.
pub const DOWNGRADER_PLAN_TASK_PREFIX: &str = "s09-downgrader-plan:";
/// Stable prefix for S09 Downgrader confirmed-run worker task identifiers.
pub const DOWNGRADER_RUN_TASK_PREFIX: &str = "s09-downgrader-run:";

/// Safe status shown while the modal loads current file status.
pub const DOWNGRADER_STATUS_LOADING_MESSAGE: &str = "Loading Downgrader status...";
/// Safe status shown while the read-only inline plan is being prepared.
pub const DOWNGRADER_PLANNING_MESSAGE: &str = "Preparing Downgrader plan...";
/// Safe status shown after the first Patch All click produces a confirmation plan.
pub const DOWNGRADER_PLAN_READY_MESSAGE: &str =
    "Review the plan, then click Patch All again to confirm.";
/// Safe status shown while the confirmed run is active.
pub const DOWNGRADER_RUNNING_MESSAGE: &str = "Patching files...";
/// Safe status shown after a confirmed run completes.
pub const DOWNGRADER_COMPLETED_MESSAGE: &str = "Patching complete.";
/// Safe status shown when a status worker cannot be scheduled.
pub const DOWNGRADER_STATUS_START_FAILED_MESSAGE: &str = "Downgrader status could not be started.";
/// Safe status shown when a plan worker cannot be scheduled.
pub const DOWNGRADER_PLAN_START_FAILED_MESSAGE: &str = "Downgrader plan could not be started.";
/// Safe status shown when a run worker cannot be scheduled.
pub const DOWNGRADER_RUN_START_FAILED_MESSAGE: &str = "Downgrader patching could not be started.";
/// Safe status shown when a run is requested without an executable plan.
pub const DOWNGRADER_PLAN_NOT_EXECUTABLE_MESSAGE: &str =
    "Downgrader plan cannot be executed. Refresh status and try again.";

/// Monotonic identity assigned to each Downgrader status, plan, or run request.
pub type DowngraderRequestId = u64;

/// Render-relevant lifecycle state for the Downgrader modal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderControllerPhase {
    /// Modal is not visible and no pending events should be applied.
    #[default]
    Closed,
    /// Modal has opened and current-file status is loading.
    LoadingStatus,
    /// Current-file status is loaded and the first Patch All click can plan.
    Ready,
    /// A read-only inline plan worker is active.
    Planning,
    /// A plan is visible; a second explicit Patch All click can confirm execution.
    PlanReady,
    /// A confirmed run is active; close/Escape must be blocked.
    Running,
    /// A run completed and patch action is re-enabled while status refreshes.
    Completed,
    /// A safe error is visible and the modal can be closed.
    SafeError,
}

/// Result of applying a Downgrader controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderTransitionResult {
    /// The event or intent matched current state and changed renderable data.
    Applied,
    /// The event belonged to an older request and was intentionally ignored.
    StaleIgnored,
    /// The event was not relevant to this reducer.
    Ignored,
    /// The intent was recognized but unavailable or malformed in the current state.
    Rejected,
    /// A close/Escape intent was blocked because a confirmed run is active.
    CloseBlocked,
}

impl DowngraderTransitionResult {
    /// Returns true when the transition changed controller state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// Worker request stage used by spawn-failure routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DowngraderWorkerRequestKind {
    /// Current file status request.
    Status,
    /// Read-only inline plan request.
    Plan,
    /// Explicitly confirmed mutation/download/patch request.
    Run,
}

impl DowngraderWorkerRequestKind {
    const fn stage(self) -> DowngraderWorkerStage {
        match self {
            Self::Status => DowngraderWorkerStage::Status,
            Self::Plan => DowngraderWorkerStage::Plan,
            Self::Run => DowngraderWorkerStage::Run,
        }
    }

    const fn start_failed_message(self) -> &'static str {
        match self {
            Self::Status => DOWNGRADER_STATUS_START_FAILED_MESSAGE,
            Self::Plan => DOWNGRADER_PLAN_START_FAILED_MESSAGE,
            Self::Run => DOWNGRADER_RUN_START_FAILED_MESSAGE,
        }
    }
}

/// Work request returned when opening the modal or refreshing status after completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderStatusWorkerRequest {
    /// Monotonic status request id used to reject stale worker results.
    pub request_id: DowngraderRequestId,
    /// Persisted Downgrader settings snapshot captured when the request was made.
    pub settings_snapshot: DowngraderSettings,
    /// Owned discovered installation facts, if available.
    pub installation: Option<Fallout4Installation>,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl DowngraderStatusWorkerRequest {
    /// Creates a status worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: DowngraderRequestId,
        settings_snapshot: DowngraderSettings,
        installation: Option<Fallout4Installation>,
    ) -> Self {
        Self {
            request_id,
            settings_snapshot,
            installation,
            task: downgrader_status_task(request_id),
        }
    }
}

/// Work request returned by the first Patch All click and consumed by plan worker wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderPlanWorkerRequest {
    /// Monotonic plan request id used to reject stale worker results.
    pub request_id: DowngraderRequestId,
    /// Owned discovered installation facts, if available.
    pub installation: Option<Fallout4Installation>,
    /// User-selected target and cleanup options captured at plan start.
    pub options: DowngraderOptionsSnapshot,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl DowngraderPlanWorkerRequest {
    /// Creates a read-only plan worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: DowngraderRequestId,
        installation: Option<Fallout4Installation>,
        options: DowngraderOptionsSnapshot,
    ) -> Self {
        Self {
            request_id,
            installation,
            options,
            task: downgrader_plan_task(request_id),
        }
    }
}

/// Work request returned by the second explicit Patch All click after plan confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderRunWorkerRequest {
    /// Monotonic run request id used to reject stale worker results.
    pub request_id: DowngraderRequestId,
    /// Plan request id that was visible when the user confirmed the run.
    pub confirmed_plan_request_id: DowngraderRequestId,
    /// Owned discovered installation facts, if available.
    pub installation: Option<Fallout4Installation>,
    /// User-selected target and cleanup options captured at run start.
    pub options: DowngraderOptionsSnapshot,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl DowngraderRunWorkerRequest {
    /// Creates a confirmed run worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: DowngraderRequestId,
        confirmed_plan_request_id: DowngraderRequestId,
        installation: Option<Fallout4Installation>,
        options: DowngraderOptionsSnapshot,
    ) -> Self {
        Self {
            request_id,
            confirmed_plan_request_id,
            installation,
            options,
            task: downgrader_run_task(request_id),
        }
    }
}

/// Patch All intent result: first click prepares a plan, second click confirms a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DowngraderPatchWorkerRequest {
    /// Read-only plan request produced by the first Patch All click.
    PreviewPlan(DowngraderPlanWorkerRequest),
    /// Confirmed run request produced only by the second Patch All click.
    ConfirmedRun(DowngraderRunWorkerRequest),
}

/// Pure reducer for Downgrader modal state and owned worker events.
#[derive(Debug, Clone, PartialEq)]
pub struct DowngraderController {
    phase: DowngraderControllerPhase,
    next_request_id: DowngraderRequestId,
    active_status_request_id: Option<DowngraderRequestId>,
    active_plan_request_id: Option<DowngraderRequestId>,
    active_run_request_id: Option<DowngraderRequestId>,
    latest_status_request_id: Option<DowngraderRequestId>,
    latest_plan_request_id: Option<DowngraderRequestId>,
    latest_run_request_id: Option<DowngraderRequestId>,
    installation: Option<Fallout4Installation>,
    options: DowngraderOptionsSnapshot,
    status: Option<DowngraderStatusSnapshot>,
    plan: Option<DowngraderPreviewPlan>,
    log_rows: Vec<DowngraderExecutionLogRow>,
    progress: DowngraderProgress,
    safe_error: Option<String>,
    pending_status_refresh: Option<DowngraderStatusWorkerRequest>,
    run_log_row_count: usize,
}

impl Default for DowngraderController {
    fn default() -> Self {
        Self::new()
    }
}

impl DowngraderController {
    /// Creates a closed Downgrader controller with default persisted options.
    pub fn new() -> Self {
        Self {
            phase: DowngraderControllerPhase::Closed,
            next_request_id: 1,
            active_status_request_id: None,
            active_plan_request_id: None,
            active_run_request_id: None,
            latest_status_request_id: None,
            latest_plan_request_id: None,
            latest_run_request_id: None,
            installation: None,
            options: options_from_settings(
                &DowngraderSettings::default(),
                DowngraderTarget::OldGen,
            ),
            status: None,
            plan: None,
            log_rows: Vec::new(),
            progress: DowngraderProgress::idle(),
            safe_error: None,
            pending_status_refresh: None,
            run_log_row_count: 0,
        }
    }

    /// Returns the current modal lifecycle phase.
    pub const fn phase(&self) -> DowngraderControllerPhase {
        self.phase
    }

    /// Returns true when the modal is considered visible/open.
    pub const fn is_open(&self) -> bool {
        !matches!(self.phase, DowngraderControllerPhase::Closed)
    }

    /// Returns the next monotonic request id that will be assigned.
    pub const fn next_request_id(&self) -> DowngraderRequestId {
        self.next_request_id
    }

    /// Returns the active status request id, if any.
    pub const fn active_status_request_id(&self) -> Option<DowngraderRequestId> {
        self.active_status_request_id
    }

    /// Returns the active plan request id, if any.
    pub const fn active_plan_request_id(&self) -> Option<DowngraderRequestId> {
        self.active_plan_request_id
    }

    /// Returns the active run request id, if any.
    pub const fn active_run_request_id(&self) -> Option<DowngraderRequestId> {
        self.active_run_request_id
    }

    /// Returns the latest status request id assigned by this controller.
    pub const fn latest_status_request_id(&self) -> Option<DowngraderRequestId> {
        self.latest_status_request_id
    }

    /// Returns the latest plan request id assigned by this controller.
    pub const fn latest_plan_request_id(&self) -> Option<DowngraderRequestId> {
        self.latest_plan_request_id
    }

    /// Returns the latest run request id assigned by this controller.
    pub const fn latest_run_request_id(&self) -> Option<DowngraderRequestId> {
        self.latest_run_request_id
    }

    /// Returns the current target and cleanup options snapshot.
    pub const fn options(&self) -> DowngraderOptionsSnapshot {
        self.options
    }

    /// Returns the current status snapshot, if one has loaded.
    pub fn status(&self) -> Option<&DowngraderStatusSnapshot> {
        self.status.as_ref()
    }

    /// Returns the current inline plan, if one is visible.
    pub fn plan(&self) -> Option<&DowngraderPreviewPlan> {
        self.plan.as_ref()
    }

    /// Returns display rows derived from the current status snapshot.
    pub fn status_rows(&self) -> Vec<DowngraderStatusRow> {
        self.status
            .as_ref()
            .map(|status| status.rows.iter().map(|row| row.display_row()).collect())
            .unwrap_or_default()
    }

    /// Returns user-visible log rows in modal order.
    pub fn log_rows(&self) -> &[DowngraderExecutionLogRow] {
        &self.log_rows
    }

    /// Returns the current progress value.
    pub const fn progress(&self) -> DowngraderProgress {
        self.progress
    }

    /// Returns the current safe error text, if any.
    pub fn safe_error(&self) -> Option<&str> {
        self.safe_error.as_deref()
    }

    /// Returns the current safe status text for the modal status/progress surface.
    pub fn status_text(&self) -> &str {
        if let Some(error) = self.safe_error.as_deref() {
            return error;
        }

        match self.phase {
            DowngraderControllerPhase::Closed | DowngraderControllerPhase::Ready => "",
            DowngraderControllerPhase::LoadingStatus => DOWNGRADER_STATUS_LOADING_MESSAGE,
            DowngraderControllerPhase::Planning => DOWNGRADER_PLANNING_MESSAGE,
            DowngraderControllerPhase::PlanReady => DOWNGRADER_PLAN_READY_MESSAGE,
            DowngraderControllerPhase::Running => DOWNGRADER_RUNNING_MESSAGE,
            DowngraderControllerPhase::Completed => DOWNGRADER_COMPLETED_MESSAGE,
            DowngraderControllerPhase::SafeError => "",
        }
    }

    /// Returns whether the Patch All action should be enabled.
    pub fn patch_button_enabled(&self) -> bool {
        match self.phase {
            DowngraderControllerPhase::Ready | DowngraderControllerPhase::Completed => {
                self.status.is_some()
            }
            DowngraderControllerPhase::PlanReady => {
                self.plan.as_ref().is_some_and(|plan| plan.can_execute)
            }
            DowngraderControllerPhase::Closed
            | DowngraderControllerPhase::LoadingStatus
            | DowngraderControllerPhase::Planning
            | DowngraderControllerPhase::Running
            | DowngraderControllerPhase::SafeError => false,
        }
    }

    /// Returns whether the modal close/Escape action should be enabled.
    pub const fn close_enabled(&self) -> bool {
        !matches!(self.phase, DowngraderControllerPhase::Running)
    }

    /// Opens the modal from persisted settings and requests a status worker.
    pub fn open(
        &mut self,
        settings_snapshot: DowngraderSettings,
        installation: Option<Fallout4Installation>,
    ) -> Option<DowngraderStatusWorkerRequest> {
        if matches!(self.phase, DowngraderControllerPhase::Running) {
            tracing::warn!(
                event = "s09-downgrader-open-blocked",
                active_run_request_id = ?self.active_run_request_id,
                "Downgrader open request ignored because a run is active"
            );
            return None;
        }

        let request_id = self.assign_request_id();
        self.phase = DowngraderControllerPhase::LoadingStatus;
        self.active_status_request_id = Some(request_id);
        self.active_plan_request_id = None;
        self.active_run_request_id = None;
        self.latest_status_request_id = Some(request_id);
        self.installation = installation.clone();
        self.options = options_from_settings(&settings_snapshot, DowngraderTarget::OldGen);
        self.status = None;
        self.plan = None;
        self.log_rows.clear();
        self.log_rows.push(DowngraderExecutionLogRow::initial());
        self.progress = DowngraderProgress::idle();
        self.safe_error = None;
        self.pending_status_refresh = None;
        self.run_log_row_count = 0;

        let request =
            DowngraderStatusWorkerRequest::new(request_id, settings_snapshot, installation);
        tracing::info!(
            event = "s09-downgrader-opened",
            request_id,
            task_id = %request.task.id,
            "Downgrader modal opened and status requested"
        );
        Some(request)
    }

    /// Applies a loaded status snapshot and selects the reference default target.
    pub fn status_loaded(
        &mut self,
        request_id: DowngraderRequestId,
        snapshot: DowngraderStatusSnapshot,
    ) -> DowngraderTransitionResult {
        if self.active_status_request_id != Some(request_id) || snapshot.request_id != request_id {
            tracing::debug!(
                event = "s09-downgrader-status-stale-ignored",
                request_id,
                snapshot_request_id = snapshot.request_id,
                active_status_request_id = ?self.active_status_request_id,
                "Ignoring stale Downgrader status payload"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }

        let is_initial_load = matches!(self.phase, DowngraderControllerPhase::LoadingStatus);
        let row_count = snapshot.rows.len();
        let default_target = snapshot.default_target;
        let unknown_game = snapshot.unknown_game;
        let unknown_creation_kit = snapshot.unknown_creation_kit;
        self.active_status_request_id = None;
        self.status = Some(snapshot);
        self.safe_error = None;
        if is_initial_load {
            self.options.target = default_target;
            self.phase = DowngraderControllerPhase::Ready;
        } else if matches!(self.phase, DowngraderControllerPhase::SafeError)
            && self.status.is_some()
        {
            self.phase = DowngraderControllerPhase::Ready;
        }

        tracing::info!(
            event = "s09-downgrader-status-loaded",
            request_id,
            row_count,
            default_target = default_target.as_reference_str(),
            unknown_game,
            unknown_creation_kit,
            phase = ?self.phase,
            "Downgrader status applied"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies a UI target change; malformed values are rejected without mutating state.
    pub fn set_target_from_ui_value(&mut self, value: &str) -> DowngraderTransitionResult {
        let Some(target) = parse_target_value(value) else {
            tracing::warn!(
                event = "s09-downgrader-target-rejected",
                value,
                "Downgrader target UI value was malformed"
            );
            return DowngraderTransitionResult::Rejected;
        };
        self.set_target(target)
    }

    /// Applies a typed target change and invalidates any visible confirmation plan.
    pub fn set_target(&mut self, target: DowngraderTarget) -> DowngraderTransitionResult {
        if !self.controls_can_change() {
            tracing::debug!(
                event = "s09-downgrader-target-change-rejected",
                phase = ?self.phase,
                "Downgrader target change rejected in current phase"
            );
            return DowngraderTransitionResult::Rejected;
        }
        if self.options.target == target {
            return DowngraderTransitionResult::Ignored;
        }

        self.options.target = target;
        self.invalidate_plan_after_option_change("target");
        tracing::info!(
            event = "s09-downgrader-target-updated",
            target = target.as_reference_str(),
            "Downgrader target updated"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies a UI option checkbox change; malformed option ids are rejected without mutation.
    pub fn set_option_from_ui_value(
        &mut self,
        option_id: &str,
        enabled: bool,
    ) -> DowngraderTransitionResult {
        let Some(option) = parse_option_value(option_id) else {
            tracing::warn!(
                event = "s09-downgrader-option-rejected",
                option_id,
                enabled,
                "Downgrader option UI value was malformed"
            );
            return DowngraderTransitionResult::Rejected;
        };
        match option {
            DowngraderOptionField::KeepBackups => self.set_keep_backups(enabled),
            DowngraderOptionField::DeleteDeltas => self.set_delete_deltas(enabled),
        }
    }

    /// Applies a Keep Backups checkbox change and invalidates any visible confirmation plan.
    pub fn set_keep_backups(&mut self, keep_backups: bool) -> DowngraderTransitionResult {
        if !self.controls_can_change() {
            return DowngraderTransitionResult::Rejected;
        }
        if self.options.keep_backups == keep_backups {
            return DowngraderTransitionResult::Ignored;
        }
        self.options.keep_backups = keep_backups;
        self.invalidate_plan_after_option_change("keep_backups");
        tracing::info!(
            event = "s09-downgrader-option-updated",
            option = "keep_backups",
            enabled = keep_backups,
            "Downgrader option updated"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies a Delete Patches checkbox change and invalidates any visible confirmation plan.
    pub fn set_delete_deltas(&mut self, delete_deltas: bool) -> DowngraderTransitionResult {
        if !self.controls_can_change() {
            return DowngraderTransitionResult::Rejected;
        }
        if self.options.delete_deltas == delete_deltas {
            return DowngraderTransitionResult::Ignored;
        }
        self.options.delete_deltas = delete_deltas;
        self.invalidate_plan_after_option_change("delete_deltas");
        tracing::info!(
            event = "s09-downgrader-option-updated",
            option = "delete_deltas",
            enabled = delete_deltas,
            "Downgrader option updated"
        );
        DowngraderTransitionResult::Applied
    }

    /// Handles Patch All: first click plans, second explicit click confirms execution.
    pub fn request_patch_all(&mut self) -> Option<DowngraderPatchWorkerRequest> {
        match self.phase {
            DowngraderControllerPhase::Ready | DowngraderControllerPhase::Completed => {
                self.status.as_ref()?;
                let request_id = self.assign_request_id();
                self.phase = DowngraderControllerPhase::Planning;
                self.active_plan_request_id = Some(request_id);
                self.latest_plan_request_id = Some(request_id);
                self.plan = None;
                self.safe_error = None;
                self.progress = DowngraderProgress::idle();
                let request = DowngraderPlanWorkerRequest::new(
                    request_id,
                    self.installation.clone(),
                    self.options,
                );
                tracing::info!(
                    event = "s09-downgrader-plan-requested",
                    request_id,
                    task_id = %request.task.id,
                    target = self.options.target.as_reference_str(),
                    keep_backups = self.options.keep_backups,
                    delete_deltas = self.options.delete_deltas,
                    "Downgrader inline plan requested"
                );
                Some(DowngraderPatchWorkerRequest::PreviewPlan(request))
            }
            DowngraderControllerPhase::PlanReady => {
                let plan = self.plan.as_ref()?;
                if !plan.can_execute {
                    self.safe_error = Some(DOWNGRADER_PLAN_NOT_EXECUTABLE_MESSAGE.to_owned());
                    tracing::warn!(
                        event = "s09-downgrader-run-rejected",
                        reason = "plan-not-executable",
                        plan_request_id = plan.request_id,
                        "Downgrader confirmed run rejected because plan cannot execute"
                    );
                    return None;
                }
                let confirmed_plan_request_id = plan.request_id;
                let request_id = self.assign_request_id();
                self.phase = DowngraderControllerPhase::Running;
                self.active_run_request_id = Some(request_id);
                self.latest_run_request_id = Some(request_id);
                self.active_plan_request_id = None;
                self.safe_error = None;
                self.progress = DowngraderProgress::idle();
                self.run_log_row_count = 0;
                let request = DowngraderRunWorkerRequest::new(
                    request_id,
                    confirmed_plan_request_id,
                    self.installation.clone(),
                    self.options,
                );
                tracing::info!(
                    event = "s09-downgrader-run-confirmed",
                    request_id,
                    confirmed_plan_request_id,
                    task_id = %request.task.id,
                    target = self.options.target.as_reference_str(),
                    keep_backups = self.options.keep_backups,
                    delete_deltas = self.options.delete_deltas,
                    "Downgrader confirmed run requested"
                );
                Some(DowngraderPatchWorkerRequest::ConfirmedRun(request))
            }
            DowngraderControllerPhase::Closed
            | DowngraderControllerPhase::LoadingStatus
            | DowngraderControllerPhase::Planning
            | DowngraderControllerPhase::Running
            | DowngraderControllerPhase::SafeError => None,
        }
    }

    /// Applies a read-only inline plan if it belongs to the active plan request.
    pub fn plan_ready(
        &mut self,
        request_id: DowngraderRequestId,
        plan: DowngraderPreviewPlan,
    ) -> DowngraderTransitionResult {
        if self.active_plan_request_id != Some(request_id) || plan.request_id != request_id {
            tracing::debug!(
                event = "s09-downgrader-plan-stale-ignored",
                request_id,
                plan_request_id = plan.request_id,
                active_plan_request_id = ?self.active_plan_request_id,
                "Ignoring stale Downgrader plan payload"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }
        if plan.options != self.options {
            tracing::debug!(
                event = "s09-downgrader-plan-options-stale-ignored",
                request_id,
                plan_target = plan.options.target.as_reference_str(),
                current_target = self.options.target.as_reference_str(),
                "Ignoring Downgrader plan payload whose options no longer match"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }

        let row_count = plan.rows.len();
        let can_execute = plan.can_execute;
        let mutating_step_count = plan.counts.mutating_step_count;
        let failed_rows = plan.counts.failed_rows;
        self.active_plan_request_id = None;
        self.status = Some(plan.status.clone());
        self.plan = Some(plan);
        self.phase = DowngraderControllerPhase::PlanReady;
        self.safe_error = None;
        self.progress = DowngraderProgress::idle();
        tracing::info!(
            event = "s09-downgrader-plan-ready",
            request_id,
            row_count,
            can_execute,
            failed_rows,
            mutating_step_count,
            "Downgrader inline plan applied"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies a log row emitted by the active confirmed run.
    pub fn run_log_row(
        &mut self,
        request_id: DowngraderRequestId,
        row: DowngraderExecutionLogRow,
    ) -> DowngraderTransitionResult {
        if self.active_run_request_id != Some(request_id) {
            tracing::debug!(
                event = "s09-downgrader-log-stale-ignored",
                request_id,
                active_run_request_id = ?self.active_run_request_id,
                "Ignoring stale Downgrader log row"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }
        let level = row.level.as_reference_str();
        let message = row.message.clone();
        self.log_rows.push(row);
        self.run_log_row_count += 1;
        tracing::debug!(
            event = "s09-downgrader-log-applied",
            request_id,
            level,
            message = message.as_str(),
            "Downgrader log row applied"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies progress emitted by the active confirmed run.
    pub fn run_progress(
        &mut self,
        request_id: DowngraderRequestId,
        progress: DowngraderProgress,
    ) -> DowngraderTransitionResult {
        if self.active_run_request_id != Some(request_id) {
            tracing::debug!(
                event = "s09-downgrader-progress-stale-ignored",
                request_id,
                active_run_request_id = ?self.active_run_request_id,
                "Ignoring stale Downgrader progress"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }
        self.progress = progress;
        tracing::debug!(
            event = "s09-downgrader-progress-applied",
            request_id,
            percent = self.progress.percent,
            "Downgrader progress applied"
        );
        DowngraderTransitionResult::Applied
    }

    /// Applies confirmed-run completion and queues a non-blocking status refresh request.
    pub fn run_completed(
        &mut self,
        request_id: DowngraderRequestId,
        result: DowngraderExecutionResult,
    ) -> DowngraderTransitionResult {
        if self.active_run_request_id != Some(request_id) || result.request_id != request_id {
            tracing::debug!(
                event = "s09-downgrader-run-stale-ignored",
                request_id,
                result_request_id = result.request_id,
                active_run_request_id = ?self.active_run_request_id,
                "Ignoring stale Downgrader run completion"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }

        let patched_rows = result
            .rows
            .iter()
            .filter(|row| {
                matches!(
                    row.outcome,
                    crate::services::downgrader::DowngraderExecutionOutcome::Patched
                )
            })
            .count();
        let failed_rows = result
            .rows
            .iter()
            .filter(|row| {
                matches!(
                    row.outcome,
                    crate::services::downgrader::DowngraderExecutionOutcome::Failed
                )
            })
            .count();
        if self.run_log_row_count == 0 {
            self.log_rows.extend(result.log_rows.iter().cloned());
        }
        self.phase = DowngraderControllerPhase::Completed;
        self.active_run_request_id = None;
        self.progress = DowngraderProgress::complete();
        self.safe_error = None;
        self.run_log_row_count = 0;
        self.queue_completion_status_refresh();
        tracing::info!(
            event = "s09-downgrader-run-completed",
            request_id,
            patched_rows,
            failed_rows,
            refresh_request_id = ?self.active_status_request_id,
            "Downgrader run completed and status refresh queued"
        );
        DowngraderTransitionResult::Applied
    }

    /// Maps a worker spawn failure into a safe visible error when the request is still active.
    pub fn spawn_failed(
        &mut self,
        kind: DowngraderWorkerRequestKind,
        request_id: DowngraderRequestId,
        error: WorkerSpawnError,
    ) -> DowngraderTransitionResult {
        tracing::error!(
            event = "s09-downgrader-worker-spawn-failed",
            request_id,
            stage = kind.stage().label(),
            diagnostic = %error,
            "Downgrader worker could not be scheduled"
        );
        self.worker_failed(
            request_id,
            kind.stage(),
            kind.start_failed_message().to_owned(),
            Some(error.to_string()),
        )
    }

    /// Applies an owned worker event if it carries a matching Downgrader payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> DowngraderTransitionResult {
        let task = event.task;
        let status = event.status;
        match event.payload {
            WorkerPayload::Downgrader(payload) => {
                let Some((stage, task_request_id)) = downgrader_stage_and_request_id(&task.id)
                else {
                    return DowngraderTransitionResult::Ignored;
                };
                if task.kind != WorkerTaskKind::Patch
                    || stage != payload.stage()
                    || task_request_id != payload.request_id()
                    || !downgrader_payload_matches_status(&payload, status)
                {
                    tracing::debug!(
                        event = "s09-downgrader-payload-rejected",
                        task_id = %task.id,
                        task_kind = task.kind.label(),
                        task_status = status.label(),
                        payload_stage = payload.stage().label(),
                        payload_request_id = payload.request_id(),
                        "Downgrader worker payload did not match its envelope"
                    );
                    return DowngraderTransitionResult::Ignored;
                }
                self.handle_downgrader_payload(payload)
            }
            WorkerPayload::Error(failure)
                if task.kind == WorkerTaskKind::Patch && status == WorkerTaskStatus::Failed =>
            {
                let Some((stage, request_id)) = downgrader_stage_and_request_id(&task.id) else {
                    return DowngraderTransitionResult::Ignored;
                };
                self.worker_failed(
                    request_id,
                    stage,
                    failure.safe_message().to_owned(),
                    failure.diagnostic().map(str::to_owned),
                )
            }
            _ => DowngraderTransitionResult::Ignored,
        }
    }

    /// Returns a queued post-completion status refresh request, if one exists.
    pub fn take_pending_status_refresh(&mut self) -> Option<DowngraderStatusWorkerRequest> {
        self.pending_status_refresh.take()
    }

    /// Handles close/Escape. Closing is blocked while a confirmed run is active.
    pub fn request_close(&mut self) -> DowngraderTransitionResult {
        if matches!(self.phase, DowngraderControllerPhase::Running) {
            tracing::warn!(
                event = "s09-downgrader-close-blocked",
                active_run_request_id = ?self.active_run_request_id,
                "Downgrader close/Escape blocked while run is active"
            );
            return DowngraderTransitionResult::CloseBlocked;
        }

        self.phase = DowngraderControllerPhase::Closed;
        self.active_status_request_id = None;
        self.active_plan_request_id = None;
        self.active_run_request_id = None;
        self.installation = None;
        self.status = None;
        self.plan = None;
        self.pending_status_refresh = None;
        self.safe_error = None;
        self.progress = DowngraderProgress::idle();
        self.run_log_row_count = 0;
        tracing::info!(event = "s09-downgrader-closed", "Downgrader modal closed");
        DowngraderTransitionResult::Applied
    }

    fn handle_downgrader_payload(
        &mut self,
        payload: DowngraderWorkerPayload,
    ) -> DowngraderTransitionResult {
        match payload {
            DowngraderWorkerPayload::StatusLoaded {
                request_id,
                snapshot,
            } => self.status_loaded(request_id, *snapshot),
            DowngraderWorkerPayload::PlanReady { request_id, plan } => {
                self.plan_ready(request_id, *plan)
            }
            DowngraderWorkerPayload::LogRow { request_id, row } => {
                self.run_log_row(request_id, row)
            }
            DowngraderWorkerPayload::Progress {
                request_id,
                progress,
            } => self.run_progress(request_id, progress),
            DowngraderWorkerPayload::RunCompleted { request_id, result } => {
                self.run_completed(request_id, *result)
            }
            DowngraderWorkerPayload::SafeFailure {
                request_id,
                stage,
                safe_message,
                diagnostic,
            } => self.worker_failed(request_id, stage, safe_message, diagnostic),
        }
    }

    fn worker_failed(
        &mut self,
        request_id: DowngraderRequestId,
        stage: DowngraderWorkerStage,
        safe_message: String,
        diagnostic: Option<String>,
    ) -> DowngraderTransitionResult {
        if !self.is_active_request(stage, request_id) {
            tracing::debug!(
                event = "s09-downgrader-worker-failure-stale-ignored",
                request_id,
                stage = stage.label(),
                active_status_request_id = ?self.active_status_request_id,
                active_plan_request_id = ?self.active_plan_request_id,
                active_run_request_id = ?self.active_run_request_id,
                "Ignoring stale Downgrader worker failure"
            );
            return DowngraderTransitionResult::StaleIgnored;
        }

        tracing::error!(
            event = "s09-downgrader-worker-failed",
            request_id,
            stage = stage.label(),
            safe_message = safe_message.as_str(),
            diagnostic = diagnostic.as_deref().unwrap_or(""),
            "Downgrader worker failed safely"
        );
        match stage {
            DowngraderWorkerStage::Status => self.active_status_request_id = None,
            DowngraderWorkerStage::Plan => self.active_plan_request_id = None,
            DowngraderWorkerStage::Run => {
                self.active_run_request_id = None;
                self.run_log_row_count = 0;
            }
        }
        self.phase = DowngraderControllerPhase::SafeError;
        self.safe_error = Some(safe_message.clone());
        self.progress = DowngraderProgress::idle();
        self.pending_status_refresh = None;
        self.log_rows.push(DowngraderExecutionLogRow::new(
            DowngraderLogLevel::Bad,
            safe_message,
        ));
        DowngraderTransitionResult::Applied
    }

    fn controls_can_change(&self) -> bool {
        self.status.is_some()
            && matches!(
                self.phase,
                DowngraderControllerPhase::Ready
                    | DowngraderControllerPhase::PlanReady
                    | DowngraderControllerPhase::Completed
                    | DowngraderControllerPhase::SafeError
            )
    }

    fn invalidate_plan_after_option_change(&mut self, field: &'static str) {
        self.plan = None;
        self.active_plan_request_id = None;
        self.safe_error = None;
        if !matches!(
            self.phase,
            DowngraderControllerPhase::Closed | DowngraderControllerPhase::Running
        ) {
            self.phase = DowngraderControllerPhase::Ready;
        }
        tracing::debug!(
            event = "s09-downgrader-plan-invalidated",
            field,
            "Downgrader inline plan invalidated after option change"
        );
    }

    fn is_active_request(
        &self,
        stage: DowngraderWorkerStage,
        request_id: DowngraderRequestId,
    ) -> bool {
        match stage {
            DowngraderWorkerStage::Status => self.active_status_request_id == Some(request_id),
            DowngraderWorkerStage::Plan => self.active_plan_request_id == Some(request_id),
            DowngraderWorkerStage::Run => self.active_run_request_id == Some(request_id),
        }
    }

    fn assign_request_id(&mut self) -> DowngraderRequestId {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        request_id
    }

    fn queue_completion_status_refresh(&mut self) {
        let request_id = self.assign_request_id();
        self.active_status_request_id = Some(request_id);
        self.latest_status_request_id = Some(request_id);
        let settings_snapshot = DowngraderSettings {
            keep_backups: self.options.keep_backups,
            delete_deltas: self.options.delete_deltas,
        };
        self.pending_status_refresh = Some(DowngraderStatusWorkerRequest::new(
            request_id,
            settings_snapshot,
            self.installation.clone(),
        ));
    }
}

/// Builds worker metadata for loading Downgrader file status.
pub fn downgrader_status_task(request_id: DowngraderRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{DOWNGRADER_STATUS_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Load Downgrader status")
}

/// Builds worker metadata for preparing a Downgrader inline plan.
pub fn downgrader_plan_task(request_id: DowngraderRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{DOWNGRADER_PLAN_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Prepare Downgrader plan")
}

/// Builds worker metadata for executing a confirmed Downgrader run.
pub fn downgrader_run_task(request_id: DowngraderRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{DOWNGRADER_RUN_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Run Downgrader patches")
}

/// Converts a loaded status snapshot into the Downgrader worker payload shape.
pub fn downgrader_status_loaded_payload(
    request_id: DowngraderRequestId,
    snapshot: DowngraderStatusSnapshot,
) -> WorkerPayload {
    WorkerPayload::Downgrader(DowngraderWorkerPayload::status_loaded(request_id, snapshot))
}

/// Converts a preview plan into the Downgrader worker payload shape.
pub fn downgrader_plan_ready_payload(
    request_id: DowngraderRequestId,
    plan: DowngraderPreviewPlan,
) -> WorkerPayload {
    WorkerPayload::Downgrader(DowngraderWorkerPayload::plan_ready(request_id, plan))
}

/// Converts a run log row into the Downgrader worker payload shape.
pub fn downgrader_log_row_payload(
    request_id: DowngraderRequestId,
    row: DowngraderExecutionLogRow,
) -> WorkerPayload {
    WorkerPayload::Downgrader(DowngraderWorkerPayload::log_row(request_id, row))
}

/// Converts run progress into the Downgrader worker payload shape.
pub fn downgrader_progress_payload(
    request_id: DowngraderRequestId,
    progress: DowngraderProgress,
) -> WorkerPayload {
    WorkerPayload::Downgrader(DowngraderWorkerPayload::progress(request_id, progress))
}

/// Converts a run result into the Downgrader worker payload shape.
pub fn downgrader_run_completed_payload(
    request_id: DowngraderRequestId,
    result: DowngraderExecutionResult,
) -> WorkerPayload {
    WorkerPayload::Downgrader(DowngraderWorkerPayload::run_completed(request_id, result))
}

/// Parses an S09 Downgrader request id from any Downgrader worker task id.
pub fn downgrader_request_id_from_task_id(task_id: &WorkerTaskId) -> Option<DowngraderRequestId> {
    downgrader_stage_and_request_id(task_id).map(|(_, request_id)| request_id)
}

/// Parses an S09 Downgrader stage from a worker task id.
pub fn downgrader_stage_from_task_id(task_id: &WorkerTaskId) -> Option<DowngraderWorkerStage> {
    downgrader_stage_and_request_id(task_id).map(|(stage, _)| stage)
}

fn downgrader_stage_and_request_id(
    task_id: &WorkerTaskId,
) -> Option<(DowngraderWorkerStage, DowngraderRequestId)> {
    let id = task_id.as_str();
    if let Some(value) = id.strip_prefix(DOWNGRADER_STATUS_TASK_PREFIX) {
        return value
            .parse::<DowngraderRequestId>()
            .ok()
            .map(|request_id| (DowngraderWorkerStage::Status, request_id));
    }
    if let Some(value) = id.strip_prefix(DOWNGRADER_PLAN_TASK_PREFIX) {
        return value
            .parse::<DowngraderRequestId>()
            .ok()
            .map(|request_id| (DowngraderWorkerStage::Plan, request_id));
    }
    if let Some(value) = id.strip_prefix(DOWNGRADER_RUN_TASK_PREFIX) {
        return value
            .parse::<DowngraderRequestId>()
            .ok()
            .map(|request_id| (DowngraderWorkerStage::Run, request_id));
    }
    None
}

fn downgrader_payload_matches_status(
    payload: &DowngraderWorkerPayload,
    status: WorkerTaskStatus,
) -> bool {
    match payload {
        DowngraderWorkerPayload::StatusLoaded { .. }
        | DowngraderWorkerPayload::PlanReady { .. }
        | DowngraderWorkerPayload::RunCompleted { .. } => status == WorkerTaskStatus::Completed,
        DowngraderWorkerPayload::LogRow { .. } | DowngraderWorkerPayload::Progress { .. } => {
            status == WorkerTaskStatus::Progress
        }
        DowngraderWorkerPayload::SafeFailure { .. } => status == WorkerTaskStatus::Failed,
    }
}

fn options_from_settings(
    settings: &DowngraderSettings,
    target: DowngraderTarget,
) -> DowngraderOptionsSnapshot {
    DowngraderOptionsSnapshot::new(target, settings.keep_backups, settings.delete_deltas)
}

fn parse_target_value(value: &str) -> Option<DowngraderTarget> {
    match normalized_ui_value(value).as_str() {
        "old_gen" | "oldgen" => Some(DowngraderTarget::OldGen),
        "next_gen" | "nextgen" => Some(DowngraderTarget::NextGen),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DowngraderOptionField {
    KeepBackups,
    DeleteDeltas,
}

fn parse_option_value(value: &str) -> Option<DowngraderOptionField> {
    match normalized_ui_value(value).as_str() {
        "keep_backups" | "downgrader_keep_backups" => Some(DowngraderOptionField::KeepBackups),
        "delete_patches" | "delete_deltas" | "downgrader_delete_deltas" => {
            Some(DowngraderOptionField::DeleteDeltas)
        }
        _ => None,
    }
}

fn normalized_ui_value(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        domain::downgrader::{
            DELETE_PATCHES_CHECKBOX_LABEL, DowngraderFileGroup, INITIAL_LOG_LINE,
            KEEP_BACKUPS_CHECKBOX_LABEL, TARGET_NEXT_GEN_LABEL, TARGET_OLD_GEN_LABEL,
        },
        services::downgrader::{
            DowngraderExecutionFileResult, DowngraderExecutionOutcome, DowngraderPreviewPlanCounts,
            DowngraderStatusDiagnostic, DowngraderStatusFile,
        },
        workers::{RecordingEventSink, WorkerEventSink},
    };

    fn settings(keep_backups: bool, delete_deltas: bool) -> DowngraderSettings {
        DowngraderSettings {
            keep_backups,
            delete_deltas,
        }
    }

    fn installation() -> Fallout4Installation {
        Fallout4Installation::new("Game")
    }

    fn status_snapshot(
        request_id: DowngraderRequestId,
        default_target: DowngraderTarget,
    ) -> DowngraderStatusSnapshot {
        DowngraderStatusSnapshot {
            request_id,
            game_root: PathBuf::from("Game"),
            rows: vec![DowngraderStatusFile {
                relative_path: "Fallout4.exe",
                display_name: "Fallout4.exe",
                group: DowngraderFileGroup::Game,
                detected_status: default_target.desired_status(),
                display_status: default_target.desired_status(),
                crc32: Some("C6053902".to_owned()),
                resolved_path: PathBuf::from("Game/Fallout4.exe"),
                read_error: None,
            }],
            default_target,
            unknown_game: false,
            unknown_creation_kit: false,
            diagnostics: Vec::<DowngraderStatusDiagnostic>::new(),
        }
    }

    fn plan(
        request_id: DowngraderRequestId,
        options: DowngraderOptionsSnapshot,
        can_execute: bool,
    ) -> DowngraderPreviewPlan {
        let counts = DowngraderPreviewPlanCounts {
            mutating_step_count: usize::from(can_execute),
            failed_rows: usize::from(!can_execute),
            ..DowngraderPreviewPlanCounts::default()
        };
        DowngraderPreviewPlan {
            request_id,
            game_root: PathBuf::from("Game"),
            options,
            status: status_snapshot(request_id, options.target),
            rows: Vec::new(),
            counts,
            can_execute,
        }
    }

    fn execution_result(
        request_id: DowngraderRequestId,
        options: DowngraderOptionsSnapshot,
    ) -> DowngraderExecutionResult {
        let log_row =
            DowngraderExecutionLogRow::new(DowngraderLogLevel::Good, "Patched Fallout4.exe");
        DowngraderExecutionResult {
            request_id,
            game_root: PathBuf::from("Game"),
            options,
            rows: vec![DowngraderExecutionFileResult {
                relative_path: "Fallout4.exe",
                display_name: "Fallout4.exe",
                outcome: DowngraderExecutionOutcome::Patched,
                log_row: log_row.clone(),
                diagnostics: Vec::new(),
            }],
            log_rows: vec![log_row],
            progress_events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn recorded_event(event: WorkerEvent) -> WorkerEvent {
        let sink = RecordingEventSink::new();
        sink.emit(event).expect("event should record");
        sink.events()
            .expect("recorded events should read")
            .remove(0)
    }

    #[test]
    fn downgrader_controller_open_requests_status_and_loading_state() {
        let mut controller = DowngraderController::new();
        let request = controller
            .open(settings(false, true), Some(installation()))
            .expect("open should request status");

        assert_eq!(controller.phase(), DowngraderControllerPhase::LoadingStatus);
        assert!(controller.is_open());
        assert_eq!(controller.active_status_request_id(), Some(1));
        assert_eq!(controller.latest_status_request_id(), Some(1));
        assert_eq!(controller.next_request_id(), 2);
        assert_eq!(request.request_id, 1);
        assert_eq!(request.task.id.as_str(), "s09-downgrader-status:1");
        assert_eq!(request.task.kind, WorkerTaskKind::Patch);
        assert!(!request.settings_snapshot.keep_backups);
        assert!(request.settings_snapshot.delete_deltas);
        assert_eq!(
            request.installation.as_ref().map(|i| i.game_path.clone()),
            Some(PathBuf::from("Game"))
        );
        assert!(!controller.options().keep_backups);
        assert!(controller.options().delete_deltas);
        assert_eq!(controller.log_rows().len(), 1);
        assert_eq!(controller.log_rows()[0].message, INITIAL_LOG_LINE);
        assert_eq!(controller.status_text(), DOWNGRADER_STATUS_LOADING_MESSAGE);
        assert!(!controller.patch_button_enabled());
        assert!(controller.close_enabled());
    }

    #[test]
    fn downgrader_controller_status_loaded_default_target_through_recording_sink() {
        let mut controller = DowngraderController::new();
        let request = controller
            .open(settings(true, true), Some(installation()))
            .expect("open should request status");
        let snapshot = status_snapshot(request.request_id, DowngraderTarget::NextGen);
        let event = recorded_event(WorkerEvent::completed(
            request.task.clone(),
            downgrader_status_loaded_payload(request.request_id, snapshot.clone()),
        ));

        let result = controller.handle_worker_event(event);

        assert_eq!(result, DowngraderTransitionResult::Applied);
        assert_eq!(controller.phase(), DowngraderControllerPhase::Ready);
        assert_eq!(controller.active_status_request_id(), None);
        assert_eq!(controller.options().target, DowngraderTarget::NextGen);
        assert_eq!(controller.status(), Some(&snapshot));
        assert_eq!(controller.status_rows().len(), 1);
        assert!(controller.patch_button_enabled());
    }

    #[test]
    fn downgrader_controller_option_changes_invalidate_plan_and_reject_malformed_values() {
        let mut controller = DowngraderController::new();
        let request = controller
            .open(settings(true, true), Some(installation()))
            .expect("open should request status");
        assert_eq!(
            controller.status_loaded(
                request.request_id,
                status_snapshot(1, DowngraderTarget::OldGen)
            ),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(
            controller.set_target_from_ui_value(TARGET_NEXT_GEN_LABEL),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.options().target, DowngraderTarget::NextGen);
        let before_invalid_target = controller.options();
        assert_eq!(
            controller.set_target_from_ui_value("definitely-not-a-target"),
            DowngraderTransitionResult::Rejected
        );
        assert_eq!(controller.options(), before_invalid_target);

        assert_eq!(
            controller.set_option_from_ui_value(KEEP_BACKUPS_CHECKBOX_LABEL, false),
            DowngraderTransitionResult::Applied
        );
        assert!(!controller.options().keep_backups);
        assert_eq!(
            controller.set_option_from_ui_value(DELETE_PATCHES_CHECKBOX_LABEL, false),
            DowngraderTransitionResult::Applied
        );
        assert!(!controller.options().delete_deltas);
        let before_invalid_option = controller.options();
        assert_eq!(
            controller.set_option_from_ui_value("unknown option", true),
            DowngraderTransitionResult::Rejected
        );
        assert_eq!(controller.options(), before_invalid_option);
        assert_eq!(controller.phase(), DowngraderControllerPhase::Ready);
    }

    #[test]
    fn downgrader_controller_first_patch_click_plans_and_second_click_runs() {
        let mut controller = DowngraderController::new();
        let status_request = controller
            .open(settings(true, true), Some(installation()))
            .expect("open should request status");
        controller.status_loaded(
            status_request.request_id,
            status_snapshot(status_request.request_id, DowngraderTarget::OldGen),
        );

        let first = controller
            .request_patch_all()
            .expect("first click should plan");
        let DowngraderPatchWorkerRequest::PreviewPlan(plan_request) = first else {
            panic!("first click must not request execution");
        };
        assert_eq!(plan_request.request_id, 2);
        assert_eq!(controller.phase(), DowngraderControllerPhase::Planning);
        assert_eq!(controller.active_plan_request_id(), Some(2));
        assert_eq!(controller.active_run_request_id(), None);
        assert!(!controller.patch_button_enabled());

        let plan_event = recorded_event(WorkerEvent::completed(
            plan_request.task.clone(),
            downgrader_plan_ready_payload(
                plan_request.request_id,
                plan(plan_request.request_id, plan_request.options, true),
            ),
        ));
        assert_eq!(
            controller.handle_worker_event(plan_event),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::PlanReady);
        assert!(controller.patch_button_enabled());

        let second = controller
            .request_patch_all()
            .expect("second click should confirm run");
        let DowngraderPatchWorkerRequest::ConfirmedRun(run_request) = second else {
            panic!("second click should request execution");
        };
        assert_eq!(run_request.request_id, 3);
        assert_eq!(run_request.confirmed_plan_request_id, 2);
        assert_eq!(controller.phase(), DowngraderControllerPhase::Running);
        assert_eq!(controller.active_run_request_id(), Some(3));
        assert!(!controller.close_enabled());
    }

    #[test]
    fn downgrader_controller_close_is_blocked_while_running() {
        let mut controller = DowngraderController::new();
        let status_request = controller
            .open(settings(true, true), Some(installation()))
            .expect("open should request status");
        controller.status_loaded(
            status_request.request_id,
            status_snapshot(status_request.request_id, DowngraderTarget::OldGen),
        );
        let plan_request = match controller.request_patch_all().expect("plan request") {
            DowngraderPatchWorkerRequest::PreviewPlan(request) => request,
            DowngraderPatchWorkerRequest::ConfirmedRun(_) => panic!("unexpected run"),
        };
        controller.plan_ready(
            plan_request.request_id,
            plan(plan_request.request_id, plan_request.options, true),
        );
        let run_request = match controller.request_patch_all().expect("run request") {
            DowngraderPatchWorkerRequest::ConfirmedRun(request) => request,
            DowngraderPatchWorkerRequest::PreviewPlan(_) => panic!("unexpected plan"),
        };

        assert_eq!(
            controller.request_close(),
            DowngraderTransitionResult::CloseBlocked
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::Running);
        assert_eq!(
            controller.active_run_request_id(),
            Some(run_request.request_id)
        );
    }

    #[test]
    fn downgrader_controller_stale_status_plan_and_run_events_are_ignored() {
        let mut controller = DowngraderController::new();
        let first = controller
            .open(settings(true, true), Some(installation()))
            .expect("first status");
        let second = controller
            .open(settings(false, false), Some(installation()))
            .expect("second status");

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                first.task,
                downgrader_status_loaded_payload(
                    first.request_id,
                    status_snapshot(first.request_id, DowngraderTarget::OldGen),
                ),
            )),
            DowngraderTransitionResult::StaleIgnored
        );
        assert_eq!(
            controller.active_status_request_id(),
            Some(second.request_id)
        );
        assert_eq!(
            controller.status_loaded(
                second.request_id,
                status_snapshot(second.request_id, DowngraderTarget::NextGen)
            ),
            DowngraderTransitionResult::Applied
        );

        let active_plan = match controller.request_patch_all().expect("active plan") {
            DowngraderPatchWorkerRequest::PreviewPlan(request) => request,
            DowngraderPatchWorkerRequest::ConfirmedRun(_) => panic!("unexpected run"),
        };
        let stale_plan_event = WorkerEvent::completed(
            downgrader_plan_task(99),
            downgrader_plan_ready_payload(99, plan(99, active_plan.options, true)),
        );
        assert_eq!(
            controller.handle_worker_event(stale_plan_event),
            DowngraderTransitionResult::StaleIgnored
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::Planning);
        controller.plan_ready(
            active_plan.request_id,
            plan(active_plan.request_id, active_plan.options, true),
        );
        let run = match controller.request_patch_all().expect("run") {
            DowngraderPatchWorkerRequest::ConfirmedRun(request) => request,
            DowngraderPatchWorkerRequest::PreviewPlan(_) => panic!("unexpected plan"),
        };
        let stale_result = execution_result(42, run.options);
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                downgrader_run_task(42),
                downgrader_run_completed_payload(42, stale_result),
            )),
            DowngraderTransitionResult::StaleIgnored
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::Running);
        assert_eq!(controller.active_run_request_id(), Some(run.request_id));
    }

    #[test]
    fn downgrader_controller_worker_failure_recovers_with_safe_error_and_allows_close() {
        let mut controller = DowngraderController::new();
        let status_request = controller
            .open(settings(true, true), Some(installation()))
            .expect("status");
        controller.status_loaded(
            status_request.request_id,
            status_snapshot(status_request.request_id, DowngraderTarget::OldGen),
        );
        let plan_request = match controller.request_patch_all().expect("plan") {
            DowngraderPatchWorkerRequest::PreviewPlan(request) => request,
            DowngraderPatchWorkerRequest::ConfirmedRun(_) => panic!("unexpected run"),
        };

        let result = controller.spawn_failed(
            DowngraderWorkerRequestKind::Plan,
            plan_request.request_id,
            WorkerSpawnError::NoActiveRuntime {
                task_id: plan_request.task.id.clone(),
            },
        );

        assert_eq!(result, DowngraderTransitionResult::Applied);
        assert_eq!(controller.phase(), DowngraderControllerPhase::SafeError);
        assert_eq!(
            controller.safe_error(),
            Some(DOWNGRADER_PLAN_START_FAILED_MESSAGE)
        );
        assert!(controller.close_enabled());
        assert_eq!(controller.active_plan_request_id(), None);
        assert!(!controller.patch_button_enabled());
        assert!(
            controller
                .log_rows()
                .iter()
                .any(|row| row.level == DowngraderLogLevel::Bad
                    && row.message == DOWNGRADER_PLAN_START_FAILED_MESSAGE)
        );
        assert_eq!(
            controller.request_close(),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::Closed);
    }

    #[test]
    fn downgrader_controller_completion_reenables_patch_and_queues_status_refresh() {
        let mut controller = DowngraderController::new();
        let status_request = controller
            .open(settings(true, true), Some(installation()))
            .expect("status");
        controller.status_loaded(
            status_request.request_id,
            status_snapshot(status_request.request_id, DowngraderTarget::OldGen),
        );
        let plan_request = match controller.request_patch_all().expect("plan") {
            DowngraderPatchWorkerRequest::PreviewPlan(request) => request,
            DowngraderPatchWorkerRequest::ConfirmedRun(_) => panic!("unexpected run"),
        };
        controller.plan_ready(
            plan_request.request_id,
            plan(plan_request.request_id, plan_request.options, true),
        );
        let run_request = match controller.request_patch_all().expect("run") {
            DowngraderPatchWorkerRequest::ConfirmedRun(request) => request,
            DowngraderPatchWorkerRequest::PreviewPlan(_) => panic!("unexpected plan"),
        };
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::new(
                run_request.task.clone(),
                WorkerTaskStatus::Progress,
                downgrader_progress_payload(run_request.request_id, DowngraderProgress::new(55.0)),
            )),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.progress().percent, 55.0);

        let event = recorded_event(WorkerEvent::completed(
            run_request.task.clone(),
            downgrader_run_completed_payload(
                run_request.request_id,
                execution_result(run_request.request_id, run_request.options),
            ),
        ));
        assert_eq!(
            controller.handle_worker_event(event),
            DowngraderTransitionResult::Applied
        );

        assert_eq!(controller.phase(), DowngraderControllerPhase::Completed);
        assert_eq!(controller.active_run_request_id(), None);
        assert!(controller.close_enabled());
        assert!(controller.patch_button_enabled());
        assert_eq!(controller.progress().percent, 100.0);
        assert_eq!(controller.latest_status_request_id(), Some(4));
        assert_eq!(controller.active_status_request_id(), Some(4));
        let refresh = controller
            .take_pending_status_refresh()
            .expect("completion should queue status refresh");
        assert_eq!(refresh.request_id, 4);
        assert_eq!(refresh.task.id.as_str(), "s09-downgrader-status:4");
        assert_eq!(
            refresh.settings_snapshot.keep_backups,
            run_request.options.keep_backups
        );
        assert_eq!(
            refresh.settings_snapshot.delete_deltas,
            run_request.options.delete_deltas
        );
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                refresh.task,
                downgrader_status_loaded_payload(
                    refresh.request_id,
                    status_snapshot(refresh.request_id, DowngraderTarget::NextGen),
                ),
            )),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::Completed);
        assert_eq!(controller.active_status_request_id(), None);
    }

    #[test]
    fn downgrader_controller_safe_failure_payload_uses_safe_text_without_diagnostic() {
        let mut controller = DowngraderController::new();
        let status_request = controller
            .open(settings(true, true), Some(installation()))
            .expect("status");
        let event = WorkerEvent::new(
            status_request.task,
            WorkerTaskStatus::Failed,
            WorkerPayload::Downgrader(DowngraderWorkerPayload::safe_failure(
                status_request.request_id,
                DowngraderWorkerStage::Status,
                "Could not load Downgrader status.",
                Some(r"raw C:\Users\example diagnostic".to_owned()),
            )),
        );

        assert_eq!(
            controller.handle_worker_event(event),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(controller.phase(), DowngraderControllerPhase::SafeError);
        assert_eq!(
            controller.safe_error(),
            Some("Could not load Downgrader status.")
        );
        assert!(!controller.status_text().contains("raw"));
        assert!(controller.close_enabled());
    }

    #[test]
    fn downgrader_task_id_parsing_and_payload_helpers_are_stable() {
        let status = downgrader_status_task(7);
        let plan_task = downgrader_plan_task(8);
        let run = downgrader_run_task(9);

        assert_eq!(downgrader_request_id_from_task_id(&status.id), Some(7));
        assert_eq!(
            downgrader_stage_from_task_id(&status.id),
            Some(DowngraderWorkerStage::Status)
        );
        assert_eq!(downgrader_request_id_from_task_id(&plan_task.id), Some(8));
        assert_eq!(
            downgrader_stage_from_task_id(&plan_task.id),
            Some(DowngraderWorkerStage::Plan)
        );
        assert_eq!(downgrader_request_id_from_task_id(&run.id), Some(9));
        assert_eq!(
            downgrader_stage_from_task_id(&run.id),
            Some(DowngraderWorkerStage::Run)
        );

        assert!(matches!(
            downgrader_log_row_payload(
                9,
                DowngraderExecutionLogRow::new(DowngraderLogLevel::Info, "hello"),
            ),
            WorkerPayload::Downgrader(DowngraderWorkerPayload::LogRow { request_id: 9, .. })
        ));
        assert_eq!(
            parse_target_value(TARGET_OLD_GEN_LABEL),
            Some(DowngraderTarget::OldGen)
        );
        assert_eq!(
            parse_target_value(TARGET_NEXT_GEN_LABEL),
            Some(DowngraderTarget::NextGen)
        );
    }
}
