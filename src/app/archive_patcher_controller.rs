//! Slint-free Archive Patcher modal controller and worker-payload reducer.
//!
//! The controller owns modal lifecycle state, desired BA2 target, name filtering,
//! candidate/preview/log/progress rows, manifest availability, confirmation
//! gating, stale-event rejection, and close blocking during mutation. It does no
//! filesystem mutation and never stores Slint component handles. UI code should
//! schedule the returned owned worker requests off the event loop and feed typed
//! [`ArchivePatcherWorkerPayload`](crate::workers::ArchivePatcherWorkerPayload)
//! events back through this reducer.

use std::path::PathBuf;

use crate::{
    domain::{
        archive_patcher::{
            ArchivePatcherCandidateRow, ArchivePatcherCandidateSnapshot,
            ArchivePatcherExecutionResult, ArchivePatcherLogLevel, ArchivePatcherLogRow,
            ArchivePatcherPreviewPlan, ArchivePatcherPreviewPlanRow, ArchivePatcherProgress,
            ArchivePatcherTarget, DEFAULT_ARCHIVE_PATCHER_TARGET,
        },
        discovery::ArchiveRecord,
    },
    workers::{
        ArchivePatcherWorkerPayload, ArchivePatcherWorkerStage, WorkerEvent, WorkerPayload,
        WorkerSpawnError, WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for S10 Archive Patcher candidate-loading worker task identifiers.
pub const ARCHIVE_PATCHER_CANDIDATES_TASK_PREFIX: &str = "s10-archive-patcher-candidates:";
/// Stable prefix for S10 Archive Patcher read-only plan worker task identifiers.
pub const ARCHIVE_PATCHER_PLAN_TASK_PREFIX: &str = "s10-archive-patcher-plan:";
/// Stable prefix for S10 Archive Patcher confirmed patch worker task identifiers.
pub const ARCHIVE_PATCHER_PATCH_TASK_PREFIX: &str = "s10-archive-patcher-patch:";
/// Stable prefix for S10 Archive Patcher restore-last-run worker task identifiers.
pub const ARCHIVE_PATCHER_RESTORE_TASK_PREFIX: &str = "s10-archive-patcher-restore:";

/// Safe status shown while the modal loads candidate BA2 archives.
pub const ARCHIVE_PATCHER_LOADING_MESSAGE: &str = "Loading Archive Patcher candidates...";
/// Safe status shown while the read-only preview plan is being prepared.
pub const ARCHIVE_PATCHER_PLANNING_MESSAGE: &str = "Preparing Archive Patcher plan...";
/// Safe status shown after the first Patch All click produces a confirmation plan.
pub const ARCHIVE_PATCHER_PLAN_READY_MESSAGE: &str =
    "Review the plan, then click Patch All again to confirm.";
/// Safe status shown while a confirmed patch run is active.
pub const ARCHIVE_PATCHER_PATCH_RUNNING_MESSAGE: &str = "Patching archives...";
/// Safe status shown while a restore-last-run operation is active.
pub const ARCHIVE_PATCHER_RESTORE_RUNNING_MESSAGE: &str = "Restoring archives...";
/// Safe status shown after a patch or restore operation completes.
pub const ARCHIVE_PATCHER_COMPLETED_MESSAGE: &str = "Archive Patcher operation complete.";
/// Safe status shown when candidate loading cannot be scheduled.
pub const ARCHIVE_PATCHER_CANDIDATES_START_FAILED_MESSAGE: &str =
    "Archive Patcher candidates could not be loaded.";
/// Safe status shown when preview planning cannot be scheduled.
pub const ARCHIVE_PATCHER_PLAN_START_FAILED_MESSAGE: &str =
    "Archive Patcher plan could not be started.";
/// Safe status shown when confirmed patching cannot be scheduled.
pub const ARCHIVE_PATCHER_PATCH_START_FAILED_MESSAGE: &str =
    "Archive Patcher patching could not be started.";
/// Safe status shown when restore-last-run cannot be scheduled.
pub const ARCHIVE_PATCHER_RESTORE_START_FAILED_MESSAGE: &str =
    "Archive Patcher restore could not be started.";
/// Safe status shown when a confirmed run is requested without an executable plan.
pub const ARCHIVE_PATCHER_PLAN_NOT_EXECUTABLE_MESSAGE: &str =
    "Archive Patcher plan cannot be executed. Refresh the plan and try again.";
/// Safe status shown when restore is requested without a latest manifest.
pub const ARCHIVE_PATCHER_RESTORE_UNAVAILABLE_MESSAGE: &str =
    "No Archive Patcher restore manifest is available.";
/// Safe status shown when Overview has not supplied archive records for the modal.
pub const ARCHIVE_PATCHER_OVERVIEW_UNAVAILABLE_MESSAGE: &str =
    "Archive Patcher needs a refreshed Overview with discovered BA2 archives. Refresh Overview or fix game discovery, then try again.";

/// Monotonic identity assigned to each Archive Patcher request.
pub type ArchivePatcherRequestId = u64;

/// Render-relevant lifecycle state for the Archive Patcher modal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchivePatcherControllerPhase {
    /// Modal is not visible and no pending events should be applied.
    #[default]
    Closed,
    /// Modal has opened and candidate rows are loading.
    LoadingCandidates,
    /// Candidate rows are loaded and Patch All can prepare a read-only plan.
    Ready,
    /// A read-only preview-plan worker is active.
    Planning,
    /// A plan is visible; an explicit confirmation can start mutation.
    PlanReady,
    /// A confirmed patch worker is active; close/Escape must be blocked.
    PatchRunning,
    /// A restore-last-run worker is active; close/Escape must be blocked.
    RestoreRunning,
    /// A mutation workflow completed and controls are safe to use again.
    Completed,
    /// A safe error is visible and the modal can be closed or retried.
    SafeError,
}

/// Result of applying an Archive Patcher controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchivePatcherTransitionResult {
    /// The event or intent matched current state and changed renderable data.
    Applied,
    /// The event belonged to an older request and was intentionally ignored.
    StaleIgnored,
    /// The event was not relevant to this reducer.
    Ignored,
    /// The intent was recognized but unavailable or malformed in the current state.
    Rejected,
    /// A close/Escape intent was blocked because mutation is active.
    CloseBlocked,
}

impl ArchivePatcherTransitionResult {
    /// Returns true when the transition changed controller state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// Worker request stage used by spawn-failure routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchivePatcherWorkerRequestKind {
    /// Candidate-row loading request.
    Candidates,
    /// Read-only preview-plan request.
    Plan,
    /// Explicitly confirmed patch request.
    Patch,
    /// Restore-last-run request.
    Restore,
}

impl ArchivePatcherWorkerRequestKind {
    const fn stage(self) -> ArchivePatcherWorkerStage {
        match self {
            Self::Candidates => ArchivePatcherWorkerStage::Candidates,
            Self::Plan => ArchivePatcherWorkerStage::Plan,
            Self::Patch => ArchivePatcherWorkerStage::Patch,
            Self::Restore => ArchivePatcherWorkerStage::Restore,
        }
    }

    const fn start_failed_message(self) -> &'static str {
        match self {
            Self::Candidates => ARCHIVE_PATCHER_CANDIDATES_START_FAILED_MESSAGE,
            Self::Plan => ARCHIVE_PATCHER_PLAN_START_FAILED_MESSAGE,
            Self::Patch => ARCHIVE_PATCHER_PATCH_START_FAILED_MESSAGE,
            Self::Restore => ARCHIVE_PATCHER_RESTORE_START_FAILED_MESSAGE,
        }
    }
}

/// Work request returned when opening the modal or refreshing candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherCandidateWorkerRequest {
    /// Monotonic request id used to reject stale worker results.
    pub request_id: ArchivePatcherRequestId,
    /// Current Overview archive records captured when the request was made.
    pub archives: Vec<ArchiveRecord>,
    /// Desired target captured at request start.
    pub target: ArchivePatcherTarget,
    /// Optional name filter captured at request start.
    pub name_filter: Option<String>,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ArchivePatcherCandidateWorkerRequest {
    /// Creates a candidate-loading worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: ArchivePatcherRequestId,
        archives: Vec<ArchiveRecord>,
        target: ArchivePatcherTarget,
        name_filter: Option<String>,
    ) -> Self {
        Self {
            request_id,
            archives,
            target,
            name_filter,
            task: archive_patcher_candidates_task(request_id),
        }
    }
}

/// Work request returned by the first Patch All click and consumed by plan worker wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherPlanWorkerRequest {
    /// Monotonic request id used to reject stale worker results.
    pub request_id: ArchivePatcherRequestId,
    /// Optional validated Data folder captured for path-containment checks.
    pub data_root: Option<PathBuf>,
    /// Current Overview archive records captured when the request was made.
    pub archives: Vec<ArchiveRecord>,
    /// Desired target captured at plan start.
    pub target: ArchivePatcherTarget,
    /// Optional name filter captured at plan start.
    pub name_filter: Option<String>,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ArchivePatcherPlanWorkerRequest {
    /// Creates a read-only plan worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: ArchivePatcherRequestId,
        data_root: Option<PathBuf>,
        archives: Vec<ArchiveRecord>,
        target: ArchivePatcherTarget,
        name_filter: Option<String>,
    ) -> Self {
        Self {
            request_id,
            data_root,
            archives,
            target,
            name_filter,
            task: archive_patcher_plan_task(request_id),
        }
    }
}

/// Work request returned after explicit confirmation of a preview plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherPatchWorkerRequest {
    /// Monotonic request id used to reject stale worker results.
    pub request_id: ArchivePatcherRequestId,
    /// Plan request id that was visible when the user confirmed the run.
    pub confirmed_plan_request_id: ArchivePatcherRequestId,
    /// Stable digest of the reviewed plan, used to fail closed if files change.
    pub confirmed_plan_digest: String,
    /// Optional validated Data folder captured for path-containment checks.
    pub data_root: Option<PathBuf>,
    /// Current Overview archive records captured when the request was made.
    pub archives: Vec<ArchiveRecord>,
    /// Desired target captured at run start.
    pub target: ArchivePatcherTarget,
    /// Optional name filter captured at run start.
    pub name_filter: Option<String>,
    /// App-owned latest restore manifest path.
    pub manifest_path: PathBuf,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ArchivePatcherPatchWorkerRequest {
    /// Creates a confirmed patch worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: ArchivePatcherRequestId,
        confirmed_plan_request_id: ArchivePatcherRequestId,
        confirmed_plan_digest: String,
        data_root: Option<PathBuf>,
        archives: Vec<ArchiveRecord>,
        target: ArchivePatcherTarget,
        name_filter: Option<String>,
        manifest_path: PathBuf,
    ) -> Self {
        Self {
            request_id,
            confirmed_plan_request_id,
            confirmed_plan_digest,
            data_root,
            archives,
            target,
            name_filter,
            manifest_path,
            task: archive_patcher_patch_task(request_id),
        }
    }
}

/// Work request returned by the Restore Last Run intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePatcherRestoreWorkerRequest {
    /// Monotonic request id used to reject stale worker results.
    pub request_id: ArchivePatcherRequestId,
    /// Optional validated Data folder captured for path-containment checks.
    pub data_root: Option<PathBuf>,
    /// App-owned latest restore manifest path.
    pub manifest_path: PathBuf,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ArchivePatcherRestoreWorkerRequest {
    /// Creates a restore-last-run worker request with owned inputs safe to move off-thread.
    pub fn new(
        request_id: ArchivePatcherRequestId,
        data_root: Option<PathBuf>,
        manifest_path: PathBuf,
    ) -> Self {
        Self {
            request_id,
            data_root,
            manifest_path,
            task: archive_patcher_restore_task(request_id),
        }
    }
}

/// Patch All intent result: first click prepares a plan, second click confirms a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePatcherPatchAllRequest {
    /// Read-only plan request produced by the first Patch All click.
    PreviewPlan(ArchivePatcherPlanWorkerRequest),
    /// Confirmed patch request produced only after explicit confirmation.
    ConfirmedPatch(ArchivePatcherPatchWorkerRequest),
}

/// Pure reducer for Archive Patcher modal state and owned worker events.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchivePatcherController {
    phase: ArchivePatcherControllerPhase,
    next_request_id: ArchivePatcherRequestId,
    active_request_id: Option<ArchivePatcherRequestId>,
    active_stage: Option<ArchivePatcherWorkerStage>,
    latest_candidate_request_id: Option<ArchivePatcherRequestId>,
    latest_plan_request_id: Option<ArchivePatcherRequestId>,
    latest_patch_request_id: Option<ArchivePatcherRequestId>,
    latest_restore_request_id: Option<ArchivePatcherRequestId>,
    archives: Vec<ArchiveRecord>,
    data_root: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    target: ArchivePatcherTarget,
    name_filter: String,
    candidate_rows: Vec<ArchivePatcherCandidateRow>,
    plan: Option<ArchivePatcherPreviewPlan>,
    log_rows: Vec<ArchivePatcherLogRow>,
    progress: ArchivePatcherProgress,
    safe_error: Option<String>,
    manifest_available: bool,
    about_open: bool,
    run_log_row_count: usize,
}

impl Default for ArchivePatcherController {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchivePatcherController {
    /// Creates a closed Archive Patcher controller with the reference default target.
    pub fn new() -> Self {
        Self {
            phase: ArchivePatcherControllerPhase::Closed,
            next_request_id: 1,
            active_request_id: None,
            active_stage: None,
            latest_candidate_request_id: None,
            latest_plan_request_id: None,
            latest_patch_request_id: None,
            latest_restore_request_id: None,
            archives: Vec::new(),
            data_root: None,
            manifest_path: None,
            target: DEFAULT_ARCHIVE_PATCHER_TARGET,
            name_filter: String::new(),
            candidate_rows: Vec::new(),
            plan: None,
            log_rows: Vec::new(),
            progress: ArchivePatcherProgress::idle(),
            safe_error: None,
            manifest_available: false,
            about_open: false,
            run_log_row_count: 0,
        }
    }

    /// Returns the current modal lifecycle phase.
    pub const fn phase(&self) -> ArchivePatcherControllerPhase {
        self.phase
    }

    /// Returns true when the modal is considered visible/open.
    pub const fn is_open(&self) -> bool {
        !matches!(self.phase, ArchivePatcherControllerPhase::Closed)
    }

    /// Returns the next monotonic request id that will be assigned.
    pub const fn next_request_id(&self) -> ArchivePatcherRequestId {
        self.next_request_id
    }

    /// Returns the currently active worker request id, if any.
    pub const fn active_request_id(&self) -> Option<ArchivePatcherRequestId> {
        self.active_request_id
    }

    /// Returns the currently active worker stage, if any.
    pub const fn active_stage(&self) -> Option<ArchivePatcherWorkerStage> {
        self.active_stage
    }

    /// Returns the latest candidate-loading request id assigned by this controller.
    pub const fn latest_candidate_request_id(&self) -> Option<ArchivePatcherRequestId> {
        self.latest_candidate_request_id
    }

    /// Returns the latest preview-plan request id assigned by this controller.
    pub const fn latest_plan_request_id(&self) -> Option<ArchivePatcherRequestId> {
        self.latest_plan_request_id
    }

    /// Returns the latest confirmed patch request id assigned by this controller.
    pub const fn latest_patch_request_id(&self) -> Option<ArchivePatcherRequestId> {
        self.latest_patch_request_id
    }

    /// Returns the latest restore request id assigned by this controller.
    pub const fn latest_restore_request_id(&self) -> Option<ArchivePatcherRequestId> {
        self.latest_restore_request_id
    }

    /// Returns the current desired BA2 target.
    pub const fn target(&self) -> ArchivePatcherTarget {
        self.target
    }

    /// Returns the current raw name filter text.
    pub fn name_filter(&self) -> &str {
        &self.name_filter
    }

    /// Returns the effective filter supplied to worker requests.
    pub fn effective_name_filter(&self) -> Option<String> {
        effective_filter(&self.name_filter)
    }

    /// Returns the current candidate rows in modal order.
    pub fn candidate_rows(&self) -> &[ArchivePatcherCandidateRow] {
        &self.candidate_rows
    }

    /// Returns the current preview plan, if one is visible.
    pub fn plan(&self) -> Option<&ArchivePatcherPreviewPlan> {
        self.plan.as_ref()
    }

    /// Returns the current preview plan rows in modal order.
    pub fn preview_plan_rows(&self) -> &[ArchivePatcherPreviewPlanRow] {
        self.plan
            .as_ref()
            .map(|plan| plan.rows.as_slice())
            .unwrap_or(&[])
    }

    /// Returns user-visible log rows in modal order.
    pub fn log_rows(&self) -> &[ArchivePatcherLogRow] {
        &self.log_rows
    }

    /// Returns the current progress value.
    pub fn progress(&self) -> &ArchivePatcherProgress {
        &self.progress
    }

    /// Returns whether a latest restore manifest is known to be available.
    pub const fn manifest_available(&self) -> bool {
        self.manifest_available
    }

    /// Returns whether the About dialog is open.
    pub const fn about_open(&self) -> bool {
        self.about_open
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
            ArchivePatcherControllerPhase::Closed | ArchivePatcherControllerPhase::Ready => "",
            ArchivePatcherControllerPhase::LoadingCandidates => ARCHIVE_PATCHER_LOADING_MESSAGE,
            ArchivePatcherControllerPhase::Planning => ARCHIVE_PATCHER_PLANNING_MESSAGE,
            ArchivePatcherControllerPhase::PlanReady => ARCHIVE_PATCHER_PLAN_READY_MESSAGE,
            ArchivePatcherControllerPhase::PatchRunning => ARCHIVE_PATCHER_PATCH_RUNNING_MESSAGE,
            ArchivePatcherControllerPhase::RestoreRunning => {
                ARCHIVE_PATCHER_RESTORE_RUNNING_MESSAGE
            }
            ArchivePatcherControllerPhase::Completed => ARCHIVE_PATCHER_COMPLETED_MESSAGE,
            ArchivePatcherControllerPhase::SafeError => "",
        }
    }

    /// Returns whether either write action may be used in the current state.
    pub fn write_controls_enabled(&self) -> bool {
        self.patch_button_enabled() || self.restore_button_enabled()
    }

    /// Returns whether the Patch All action should be enabled.
    pub fn patch_button_enabled(&self) -> bool {
        match self.phase {
            ArchivePatcherControllerPhase::Ready
            | ArchivePatcherControllerPhase::Completed
            | ArchivePatcherControllerPhase::SafeError => self.manifest_path.is_some(),
            ArchivePatcherControllerPhase::PlanReady => self
                .plan
                .as_ref()
                .is_some_and(|plan| plan.can_execute && self.manifest_path.is_some()),
            ArchivePatcherControllerPhase::Closed
            | ArchivePatcherControllerPhase::LoadingCandidates
            | ArchivePatcherControllerPhase::Planning
            | ArchivePatcherControllerPhase::PatchRunning
            | ArchivePatcherControllerPhase::RestoreRunning => false,
        }
    }

    /// Returns whether Restore Last Run should be enabled.
    pub fn restore_button_enabled(&self) -> bool {
        self.manifest_available
            && self.manifest_path.is_some()
            && matches!(
                self.phase,
                ArchivePatcherControllerPhase::Ready
                    | ArchivePatcherControllerPhase::PlanReady
                    | ArchivePatcherControllerPhase::Completed
                    | ArchivePatcherControllerPhase::SafeError
            )
    }

    /// Returns whether the modal close/Escape action should be enabled.
    pub const fn close_enabled(&self) -> bool {
        !matches!(
            self.phase,
            ArchivePatcherControllerPhase::PatchRunning
                | ArchivePatcherControllerPhase::RestoreRunning
        )
    }

    /// Opens the modal from current Overview archive records and requests candidates.
    pub fn open(
        &mut self,
        archives: Vec<ArchiveRecord>,
        data_root: Option<PathBuf>,
        manifest_path: PathBuf,
        manifest_available: bool,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        if self.is_mutating() {
            tracing::warn!(
                event = "s10-archive-patcher-open-blocked",
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Archive Patcher open request ignored because mutation is active"
            );
            return None;
        }

        self.archives = archives;
        self.data_root = data_root;
        self.manifest_path = Some(manifest_path);
        self.manifest_available = manifest_available;
        self.target = DEFAULT_ARCHIVE_PATCHER_TARGET;
        self.name_filter.clear();
        self.about_open = false;
        self.log_rows.clear();
        self.start_candidate_request("opened")
    }

    /// Opens a fail-closed modal state when Overview lacks archive records or game data.
    pub fn open_unavailable(
        &mut self,
        safe_message: impl Into<String>,
    ) -> ArchivePatcherTransitionResult {
        if self.is_mutating() {
            tracing::warn!(
                event = "s10-archive-patcher-open-unavailable-blocked",
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Archive Patcher unavailable open request ignored because mutation is active"
            );
            return ArchivePatcherTransitionResult::CloseBlocked;
        }

        let safe_message = safe_message.into();
        self.phase = ArchivePatcherControllerPhase::SafeError;
        self.active_request_id = None;
        self.active_stage = None;
        self.archives.clear();
        self.data_root = None;
        self.manifest_path = None;
        self.target = DEFAULT_ARCHIVE_PATCHER_TARGET;
        self.name_filter.clear();
        self.candidate_rows.clear();
        self.plan = None;
        self.log_rows.clear();
        self.log_rows.push(ArchivePatcherLogRow::new(
            ArchivePatcherLogLevel::Bad,
            safe_message.clone(),
        ));
        self.progress = ArchivePatcherProgress::idle();
        self.safe_error = Some(safe_message);
        self.manifest_available = false;
        self.about_open = false;
        self.run_log_row_count = 0;
        tracing::warn!(
            event = "s10-archive-patcher-open-unavailable",
            "Archive Patcher opened in fail-closed unavailable state"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Replaces the Overview archive snapshot and refreshes candidate rows.
    pub fn reload_candidates(
        &mut self,
        archives: Vec<ArchiveRecord>,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        if !self.is_open() || self.is_mutating() {
            return None;
        }
        self.archives = archives;
        self.start_candidate_request("archives-reloaded")
    }

    /// Updates whether the latest restore manifest is available.
    pub fn set_manifest_available(&mut self, available: bool) -> ArchivePatcherTransitionResult {
        if !self.is_open() {
            return ArchivePatcherTransitionResult::Rejected;
        }
        self.manifest_available = available;
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies a desired-version change and requests a fresh candidate set.
    pub fn set_target(
        &mut self,
        target: ArchivePatcherTarget,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        if !self.can_refresh_candidates() {
            tracing::debug!(
                event = "s10-archive-patcher-target-change-rejected",
                phase = ?self.phase,
                "Archive Patcher target change rejected in current phase"
            );
            return None;
        }
        if self.target == target && self.active_stage != Some(ArchivePatcherWorkerStage::Candidates)
        {
            return None;
        }
        self.target = target;
        self.start_candidate_request("target-changed")
    }

    /// Applies a desired-version UI value and requests a fresh candidate set when valid.
    pub fn set_target_from_ui_value(
        &mut self,
        value: &str,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        parse_target_value(value).and_then(|target| self.set_target(target))
    }

    /// Applies a name-filter change and requests a fresh candidate set.
    pub fn set_name_filter(
        &mut self,
        value: impl Into<String>,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        if !self.can_refresh_candidates() {
            tracing::debug!(
                event = "s10-archive-patcher-filter-change-rejected",
                phase = ?self.phase,
                "Archive Patcher name-filter change rejected in current phase"
            );
            return None;
        }
        let value = value.into();
        if self.name_filter == value
            && self.active_stage != Some(ArchivePatcherWorkerStage::Candidates)
        {
            return None;
        }
        self.name_filter = value;
        self.start_candidate_request("filter-changed")
    }

    /// Handles a Patch All click: plan first, then confirm when a plan is visible.
    pub fn request_patch_all(&mut self) -> Option<ArchivePatcherPatchAllRequest> {
        match self.phase {
            ArchivePatcherControllerPhase::Ready
            | ArchivePatcherControllerPhase::Completed
            | ArchivePatcherControllerPhase::SafeError => self
                .request_plan()
                .map(ArchivePatcherPatchAllRequest::PreviewPlan),
            ArchivePatcherControllerPhase::PlanReady => self
                .confirm_plan()
                .map(ArchivePatcherPatchAllRequest::ConfirmedPatch),
            ArchivePatcherControllerPhase::Closed
            | ArchivePatcherControllerPhase::LoadingCandidates
            | ArchivePatcherControllerPhase::Planning
            | ArchivePatcherControllerPhase::PatchRunning
            | ArchivePatcherControllerPhase::RestoreRunning => None,
        }
    }

    /// Requests a read-only preview plan without mutating archives.
    pub fn request_plan(&mut self) -> Option<ArchivePatcherPlanWorkerRequest> {
        if !matches!(
            self.phase,
            ArchivePatcherControllerPhase::Ready
                | ArchivePatcherControllerPhase::Completed
                | ArchivePatcherControllerPhase::SafeError
        ) || self.manifest_path.is_none()
        {
            return None;
        }

        let request_id = self.assign_request_id();
        self.phase = ArchivePatcherControllerPhase::Planning;
        self.active_request_id = Some(request_id);
        self.active_stage = Some(ArchivePatcherWorkerStage::Plan);
        self.latest_plan_request_id = Some(request_id);
        self.plan = None;
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        let request = ArchivePatcherPlanWorkerRequest::new(
            request_id,
            self.data_root.clone(),
            self.archives.clone(),
            self.target,
            self.effective_name_filter(),
        );
        tracing::info!(
            event = "s10-archive-patcher-plan-requested",
            request_id,
            task_id = %request.task.id,
            target = self.target.as_reference_str(),
            has_filter = request.name_filter.is_some(),
            "Archive Patcher preview plan requested"
        );
        Some(request)
    }

    /// Confirms the visible plan and requests fail-closed mutation.
    pub fn confirm_plan(&mut self) -> Option<ArchivePatcherPatchWorkerRequest> {
        if self.phase != ArchivePatcherControllerPhase::PlanReady {
            tracing::debug!(
                event = "s10-archive-patcher-confirm-rejected",
                phase = ?self.phase,
                reason = "no-visible-plan",
                "Archive Patcher confirmation rejected without a visible plan"
            );
            return None;
        }
        let Some(plan) = self.plan.as_ref() else {
            return None;
        };
        let plan_request_id = plan.request_id;
        let plan_can_execute = plan.can_execute;
        if !plan_can_execute
            || plan.target != self.target
            || plan.name_filter != self.effective_name_filter()
        {
            self.fail_visible(ARCHIVE_PATCHER_PLAN_NOT_EXECUTABLE_MESSAGE.to_owned());
            tracing::warn!(
                event = "s10-archive-patcher-confirm-rejected",
                plan_request_id,
                can_execute = plan_can_execute,
                "Archive Patcher confirmed patch rejected because the plan is not executable"
            );
            return None;
        }
        let Some(manifest_path) = self.manifest_path.clone() else {
            self.fail_visible(ARCHIVE_PATCHER_PLAN_NOT_EXECUTABLE_MESSAGE.to_owned());
            return None;
        };

        let confirmed_plan_request_id = plan.request_id;
        let confirmed_plan_digest = plan.stable_digest();
        let request_id = self.assign_request_id();
        self.phase = ArchivePatcherControllerPhase::PatchRunning;
        self.active_request_id = Some(request_id);
        self.active_stage = Some(ArchivePatcherWorkerStage::Patch);
        self.latest_patch_request_id = Some(request_id);
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        self.run_log_row_count = 0;
        let request = ArchivePatcherPatchWorkerRequest::new(
            request_id,
            confirmed_plan_request_id,
            confirmed_plan_digest,
            self.data_root.clone(),
            self.archives.clone(),
            self.target,
            self.effective_name_filter(),
            manifest_path,
        );
        tracing::info!(
            event = "s10-archive-patcher-patch-confirmed",
            request_id,
            confirmed_plan_request_id,
            task_id = %request.task.id,
            target = self.target.as_reference_str(),
            has_filter = request.name_filter.is_some(),
            "Archive Patcher confirmed patch requested"
        );
        Some(request)
    }

    /// Requests restore of the latest Archive Patcher manifest.
    pub fn request_restore_last_run(&mut self) -> Option<ArchivePatcherRestoreWorkerRequest> {
        if !matches!(
            self.phase,
            ArchivePatcherControllerPhase::Ready
                | ArchivePatcherControllerPhase::PlanReady
                | ArchivePatcherControllerPhase::Completed
                | ArchivePatcherControllerPhase::SafeError
        ) {
            return None;
        }
        if !self.manifest_available {
            self.fail_visible(ARCHIVE_PATCHER_RESTORE_UNAVAILABLE_MESSAGE.to_owned());
            tracing::warn!(
                event = "s10-archive-patcher-restore-rejected",
                reason = "manifest-unavailable",
                "Archive Patcher restore rejected because no latest manifest is available"
            );
            return None;
        }
        let Some(manifest_path) = self.manifest_path.clone() else {
            self.fail_visible(ARCHIVE_PATCHER_RESTORE_UNAVAILABLE_MESSAGE.to_owned());
            return None;
        };

        let request_id = self.assign_request_id();
        self.phase = ArchivePatcherControllerPhase::RestoreRunning;
        self.active_request_id = Some(request_id);
        self.active_stage = Some(ArchivePatcherWorkerStage::Restore);
        self.latest_restore_request_id = Some(request_id);
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        self.run_log_row_count = 0;
        let request = ArchivePatcherRestoreWorkerRequest::new(
            request_id,
            self.data_root.clone(),
            manifest_path,
        );
        tracing::info!(
            event = "s10-archive-patcher-restore-requested",
            request_id,
            task_id = %request.task.id,
            "Archive Patcher restore-last-run requested"
        );
        Some(request)
    }

    /// Opens the Archive Patcher About dialog state.
    pub fn open_about(&mut self) -> ArchivePatcherTransitionResult {
        if !self.is_open() {
            return ArchivePatcherTransitionResult::Rejected;
        }
        self.about_open = true;
        ArchivePatcherTransitionResult::Applied
    }

    /// Closes the Archive Patcher About dialog state.
    pub fn close_about(&mut self) -> ArchivePatcherTransitionResult {
        if !self.is_open() {
            return ArchivePatcherTransitionResult::Rejected;
        }
        self.about_open = false;
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies a loaded candidate snapshot if it belongs to the active request.
    pub fn candidates_loaded(
        &mut self,
        request_id: ArchivePatcherRequestId,
        snapshot: ArchivePatcherCandidateSnapshot,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(ArchivePatcherWorkerStage::Candidates, request_id)
            || snapshot.request_id != request_id
        {
            tracing::debug!(
                event = "s10-archive-patcher-candidates-stale-ignored",
                request_id,
                snapshot_request_id = snapshot.request_id,
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher candidate payload"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }
        if snapshot.target != self.target || snapshot.name_filter != self.effective_name_filter() {
            tracing::debug!(
                event = "s10-archive-patcher-candidates-filter-stale-ignored",
                request_id,
                snapshot_target = snapshot.target.as_reference_str(),
                current_target = self.target.as_reference_str(),
                "Ignoring Archive Patcher candidate payload whose target/filter no longer match"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }

        let row_count = snapshot.rows.len();
        self.active_request_id = None;
        self.active_stage = None;
        self.candidate_rows = snapshot.rows;
        self.plan = None;
        self.log_rows.clear();
        self.log_rows.push(snapshot.log_row);
        self.phase = ArchivePatcherControllerPhase::Ready;
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        tracing::info!(
            event = "s10-archive-patcher-candidates-loaded",
            request_id,
            row_count,
            target = self.target.as_reference_str(),
            "Archive Patcher candidates applied"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies a read-only inline plan if it belongs to the active plan request.
    pub fn plan_ready(
        &mut self,
        request_id: ArchivePatcherRequestId,
        plan: ArchivePatcherPreviewPlan,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(ArchivePatcherWorkerStage::Plan, request_id)
            || plan.request_id != request_id
        {
            tracing::debug!(
                event = "s10-archive-patcher-plan-stale-ignored",
                request_id,
                plan_request_id = plan.request_id,
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher plan payload"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }
        if plan.target != self.target || plan.name_filter != self.effective_name_filter() {
            tracing::debug!(
                event = "s10-archive-patcher-plan-filter-stale-ignored",
                request_id,
                plan_target = plan.target.as_reference_str(),
                current_target = self.target.as_reference_str(),
                "Ignoring Archive Patcher plan payload whose target/filter no longer match"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }

        let row_count = plan.rows.len();
        let patchable_rows = plan.counts.patchable_rows;
        let failed_rows = plan.counts.failed_rows;
        self.active_request_id = None;
        self.active_stage = None;
        self.candidate_rows = plan.candidates.rows.clone();
        self.log_rows.clear();
        self.log_rows.push(plan.summary_log_row.clone());
        self.log_rows.extend(
            plan.rows
                .iter()
                .filter_map(ArchivePatcherPreviewPlanRow::failure_log_row),
        );
        self.plan = Some(plan);
        self.phase = ArchivePatcherControllerPhase::PlanReady;
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        tracing::info!(
            event = "s10-archive-patcher-plan-ready",
            request_id,
            row_count,
            patchable_rows,
            failed_rows,
            "Archive Patcher preview plan applied"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies a log row emitted by the active patch or restore worker.
    pub fn run_log_row(
        &mut self,
        request_id: ArchivePatcherRequestId,
        stage: ArchivePatcherWorkerStage,
        row: ArchivePatcherLogRow,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(stage, request_id) || !stage.is_mutation() {
            tracing::debug!(
                event = "s10-archive-patcher-log-stale-ignored",
                request_id,
                stage = stage.label(),
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher log row"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }
        let level = row.level.as_reference_str();
        let message = row.message.clone();
        self.log_rows.push(row);
        self.run_log_row_count += 1;
        tracing::debug!(
            event = "s10-archive-patcher-log-applied",
            request_id,
            stage = stage.label(),
            level,
            message = message.as_str(),
            "Archive Patcher log row applied"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies progress emitted by the active patch or restore worker.
    pub fn run_progress(
        &mut self,
        request_id: ArchivePatcherRequestId,
        stage: ArchivePatcherWorkerStage,
        progress: ArchivePatcherProgress,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(stage, request_id) || !stage.is_mutation() {
            tracing::debug!(
                event = "s10-archive-patcher-progress-stale-ignored",
                request_id,
                stage = stage.label(),
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher progress"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }
        self.progress = progress;
        tracing::debug!(
            event = "s10-archive-patcher-progress-applied",
            request_id,
            stage = stage.label(),
            percent = self.progress.percent,
            "Archive Patcher progress applied"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies confirmed patch completion if it belongs to the active patch request.
    pub fn patch_completed(
        &mut self,
        request_id: ArchivePatcherRequestId,
        result: ArchivePatcherExecutionResult,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(ArchivePatcherWorkerStage::Patch, request_id)
            || result.request_id != request_id
        {
            tracing::debug!(
                event = "s10-archive-patcher-patch-stale-ignored",
                request_id,
                result_request_id = result.request_id,
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher patch completion"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }

        let patched = result.counts.patched;
        let failed = result.counts.failed;
        let summary = result
            .log_rows
            .last()
            .map(|row| row.message.clone())
            .unwrap_or_else(|| result.counts.patching_complete_message());
        if self.run_log_row_count == 0 {
            self.log_rows.extend(result.log_rows.iter().cloned());
        }
        self.phase = ArchivePatcherControllerPhase::Completed;
        self.active_request_id = None;
        self.active_stage = None;
        self.plan = None;
        self.progress = ArchivePatcherProgress::complete(summary);
        self.safe_error = None;
        self.run_log_row_count = 0;
        self.manifest_available = self.manifest_available || !result.rows.is_empty();
        tracing::info!(
            event = "s10-archive-patcher-patch-completed",
            request_id,
            patched,
            failed,
            manifest_available = self.manifest_available,
            "Archive Patcher confirmed patch completed"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Applies restore-last-run completion if it belongs to the active restore request.
    pub fn restore_completed(
        &mut self,
        request_id: ArchivePatcherRequestId,
        result: ArchivePatcherExecutionResult,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(ArchivePatcherWorkerStage::Restore, request_id)
            || result.request_id != request_id
        {
            tracing::debug!(
                event = "s10-archive-patcher-restore-stale-ignored",
                request_id,
                result_request_id = result.request_id,
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher restore completion"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }

        let restored = result.counts.restored;
        let skipped = result.counts.skipped;
        let failed = result.counts.failed;
        let summary = result
            .log_rows
            .last()
            .map(|row| row.message.clone())
            .unwrap_or_else(|| result.counts.restore_complete_message());
        if self.run_log_row_count == 0 {
            self.log_rows.extend(result.log_rows.iter().cloned());
        }
        self.phase = ArchivePatcherControllerPhase::Completed;
        self.active_request_id = None;
        self.active_stage = None;
        self.plan = None;
        self.progress = ArchivePatcherProgress::complete(summary);
        self.safe_error = None;
        self.run_log_row_count = 0;
        tracing::info!(
            event = "s10-archive-patcher-restore-completed",
            request_id,
            restored,
            skipped,
            failed,
            "Archive Patcher restore-last-run completed"
        );
        ArchivePatcherTransitionResult::Applied
    }

    /// Maps a worker spawn failure into a safe visible error when the request is still active.
    pub fn spawn_failed(
        &mut self,
        kind: ArchivePatcherWorkerRequestKind,
        request_id: ArchivePatcherRequestId,
        error: WorkerSpawnError,
    ) -> ArchivePatcherTransitionResult {
        tracing::error!(
            event = "s10-archive-patcher-worker-spawn-failed",
            request_id,
            stage = kind.stage().label(),
            diagnostic = %error,
            "Archive Patcher worker could not be scheduled"
        );
        self.worker_failed(
            request_id,
            kind.stage(),
            kind.start_failed_message().to_owned(),
            Some(error.to_string()),
        )
    }

    /// Applies an owned worker event if it carries a matching Archive Patcher payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> ArchivePatcherTransitionResult {
        let task = event.task;
        let status = event.status;
        match event.payload {
            WorkerPayload::ArchivePatcher(payload) => {
                let Some((stage, task_request_id)) = archive_patcher_stage_and_request_id(&task.id)
                else {
                    return ArchivePatcherTransitionResult::Ignored;
                };
                if task.kind != WorkerTaskKind::Patch
                    || stage != payload.stage()
                    || task_request_id != payload.request_id()
                    || !archive_patcher_payload_matches_status(&payload, status)
                {
                    tracing::debug!(
                        event = "s10-archive-patcher-payload-rejected",
                        task_id = %task.id,
                        task_kind = task.kind.label(),
                        task_status = status.label(),
                        payload_stage = payload.stage().label(),
                        payload_request_id = payload.request_id(),
                        "Archive Patcher worker payload did not match its envelope"
                    );
                    return ArchivePatcherTransitionResult::Ignored;
                }
                self.handle_archive_patcher_payload(payload)
            }
            WorkerPayload::Error(failure)
                if task.kind == WorkerTaskKind::Patch && status == WorkerTaskStatus::Failed =>
            {
                let Some((stage, request_id)) = archive_patcher_stage_and_request_id(&task.id)
                else {
                    return ArchivePatcherTransitionResult::Ignored;
                };
                self.worker_failed(
                    request_id,
                    stage,
                    failure.safe_message().to_owned(),
                    failure.diagnostic().map(str::to_owned),
                )
            }
            _ => ArchivePatcherTransitionResult::Ignored,
        }
    }

    /// Handles close/Escape. Closing is blocked while patch or restore mutation is active.
    pub fn request_close(&mut self) -> ArchivePatcherTransitionResult {
        if self.is_mutating() {
            tracing::warn!(
                event = "s10-archive-patcher-close-blocked",
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Archive Patcher close/Escape blocked while mutation is active"
            );
            return ArchivePatcherTransitionResult::CloseBlocked;
        }

        self.phase = ArchivePatcherControllerPhase::Closed;
        self.active_request_id = None;
        self.active_stage = None;
        self.archives.clear();
        self.data_root = None;
        self.manifest_path = None;
        self.candidate_rows.clear();
        self.plan = None;
        self.log_rows.clear();
        self.progress = ArchivePatcherProgress::idle();
        self.safe_error = None;
        self.about_open = false;
        self.run_log_row_count = 0;
        tracing::info!(
            event = "s10-archive-patcher-closed",
            "Archive Patcher modal closed"
        );
        ArchivePatcherTransitionResult::Applied
    }

    fn handle_archive_patcher_payload(
        &mut self,
        payload: ArchivePatcherWorkerPayload,
    ) -> ArchivePatcherTransitionResult {
        match payload {
            ArchivePatcherWorkerPayload::CandidatesLoaded {
                request_id,
                snapshot,
            } => self.candidates_loaded(request_id, *snapshot),
            ArchivePatcherWorkerPayload::PlanReady { request_id, plan } => {
                self.plan_ready(request_id, *plan)
            }
            ArchivePatcherWorkerPayload::LogRow {
                request_id,
                stage,
                row,
            } => self.run_log_row(request_id, stage, row),
            ArchivePatcherWorkerPayload::Progress {
                request_id,
                stage,
                progress,
            } => self.run_progress(request_id, stage, progress),
            ArchivePatcherWorkerPayload::PatchCompleted { request_id, result } => {
                self.patch_completed(request_id, *result)
            }
            ArchivePatcherWorkerPayload::RestoreCompleted { request_id, result } => {
                self.restore_completed(request_id, *result)
            }
            ArchivePatcherWorkerPayload::SafeFailure {
                request_id,
                stage,
                safe_message,
                diagnostic,
            } => self.worker_failed(request_id, stage, safe_message, diagnostic),
        }
    }

    fn worker_failed(
        &mut self,
        request_id: ArchivePatcherRequestId,
        stage: ArchivePatcherWorkerStage,
        safe_message: String,
        diagnostic: Option<String>,
    ) -> ArchivePatcherTransitionResult {
        if !self.is_active_request(stage, request_id) {
            tracing::debug!(
                event = "s10-archive-patcher-worker-failure-stale-ignored",
                request_id,
                stage = stage.label(),
                active_request_id = ?self.active_request_id,
                active_stage = ?self.active_stage,
                "Ignoring stale Archive Patcher worker failure"
            );
            return ArchivePatcherTransitionResult::StaleIgnored;
        }

        tracing::error!(
            event = "s10-archive-patcher-worker-failed",
            request_id,
            stage = stage.label(),
            safe_message = safe_message.as_str(),
            diagnostic = diagnostic.as_deref().unwrap_or(""),
            "Archive Patcher worker failed safely"
        );
        self.active_request_id = None;
        self.active_stage = None;
        self.phase = ArchivePatcherControllerPhase::SafeError;
        self.safe_error = Some(safe_message.clone());
        self.progress = ArchivePatcherProgress::idle();
        self.run_log_row_count = 0;
        self.log_rows.push(ArchivePatcherLogRow::new(
            ArchivePatcherLogLevel::Bad,
            safe_message,
        ));
        ArchivePatcherTransitionResult::Applied
    }

    fn fail_visible(&mut self, safe_message: String) {
        self.phase = ArchivePatcherControllerPhase::SafeError;
        self.safe_error = Some(safe_message.clone());
        self.progress = ArchivePatcherProgress::idle();
        self.active_request_id = None;
        self.active_stage = None;
        self.log_rows.push(ArchivePatcherLogRow::new(
            ArchivePatcherLogLevel::Bad,
            safe_message,
        ));
    }

    fn start_candidate_request(
        &mut self,
        reason: &'static str,
    ) -> Option<ArchivePatcherCandidateWorkerRequest> {
        if self.manifest_path.is_none() {
            return None;
        }
        let request_id = self.assign_request_id();
        self.phase = ArchivePatcherControllerPhase::LoadingCandidates;
        self.active_request_id = Some(request_id);
        self.active_stage = Some(ArchivePatcherWorkerStage::Candidates);
        self.latest_candidate_request_id = Some(request_id);
        self.candidate_rows.clear();
        self.plan = None;
        self.log_rows.clear();
        self.safe_error = None;
        self.progress = ArchivePatcherProgress::idle();
        self.run_log_row_count = 0;
        let request = ArchivePatcherCandidateWorkerRequest::new(
            request_id,
            self.archives.clone(),
            self.target,
            self.effective_name_filter(),
        );
        tracing::info!(
            event = "s10-archive-patcher-candidates-requested",
            request_id,
            task_id = %request.task.id,
            reason,
            target = self.target.as_reference_str(),
            has_filter = request.name_filter.is_some(),
            archive_count = request.archives.len(),
            "Archive Patcher candidates requested"
        );
        Some(request)
    }

    fn is_active_request(
        &self,
        stage: ArchivePatcherWorkerStage,
        request_id: ArchivePatcherRequestId,
    ) -> bool {
        self.active_stage == Some(stage) && self.active_request_id == Some(request_id)
    }

    fn can_refresh_candidates(&self) -> bool {
        self.is_open() && !self.is_mutating()
    }

    fn is_mutating(&self) -> bool {
        matches!(
            self.phase,
            ArchivePatcherControllerPhase::PatchRunning
                | ArchivePatcherControllerPhase::RestoreRunning
        )
    }

    fn assign_request_id(&mut self) -> ArchivePatcherRequestId {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        request_id
    }
}

/// Builds worker metadata for loading Archive Patcher candidates.
pub fn archive_patcher_candidates_task(request_id: ArchivePatcherRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{ARCHIVE_PATCHER_CANDIDATES_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Load Archive Patcher candidates")
}

/// Builds worker metadata for preparing an Archive Patcher preview plan.
pub fn archive_patcher_plan_task(request_id: ArchivePatcherRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{ARCHIVE_PATCHER_PLAN_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Prepare Archive Patcher plan")
}

/// Builds worker metadata for executing a confirmed Archive Patcher patch run.
pub fn archive_patcher_patch_task(request_id: ArchivePatcherRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{ARCHIVE_PATCHER_PATCH_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Run Archive Patcher patches")
}

/// Builds worker metadata for restoring the latest Archive Patcher run.
pub fn archive_patcher_restore_task(request_id: ArchivePatcherRequestId) -> WorkerTask {
    WorkerTask::new(
        format!("{ARCHIVE_PATCHER_RESTORE_TASK_PREFIX}{request_id}"),
        WorkerTaskKind::Patch,
    )
    .with_label("Restore Archive Patcher latest run")
}

/// Converts a candidate snapshot into the Archive Patcher worker payload shape.
pub fn archive_patcher_candidates_loaded_payload(
    request_id: ArchivePatcherRequestId,
    snapshot: ArchivePatcherCandidateSnapshot,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::candidates_loaded(
        request_id, snapshot,
    ))
}

/// Converts a preview plan into the Archive Patcher worker payload shape.
pub fn archive_patcher_plan_ready_payload(
    request_id: ArchivePatcherRequestId,
    plan: ArchivePatcherPreviewPlan,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::plan_ready(request_id, plan))
}

/// Converts a patch or restore log row into the Archive Patcher worker payload shape.
pub fn archive_patcher_log_row_payload(
    request_id: ArchivePatcherRequestId,
    stage: ArchivePatcherWorkerStage,
    row: ArchivePatcherLogRow,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::log_row(request_id, stage, row))
}

/// Converts patch or restore progress into the Archive Patcher worker payload shape.
pub fn archive_patcher_progress_payload(
    request_id: ArchivePatcherRequestId,
    stage: ArchivePatcherWorkerStage,
    progress: ArchivePatcherProgress,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::progress(
        request_id, stage, progress,
    ))
}

/// Converts a patch result into the Archive Patcher worker payload shape.
pub fn archive_patcher_patch_completed_payload(
    request_id: ArchivePatcherRequestId,
    result: ArchivePatcherExecutionResult,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::patch_completed(
        request_id, result,
    ))
}

/// Converts a restore result into the Archive Patcher worker payload shape.
pub fn archive_patcher_restore_completed_payload(
    request_id: ArchivePatcherRequestId,
    result: ArchivePatcherExecutionResult,
) -> WorkerPayload {
    WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::restore_completed(
        request_id, result,
    ))
}

/// Parses an S10 Archive Patcher request id from any Archive Patcher worker task id.
pub fn archive_patcher_request_id_from_task_id(
    task_id: &WorkerTaskId,
) -> Option<ArchivePatcherRequestId> {
    archive_patcher_stage_and_request_id(task_id).map(|(_, request_id)| request_id)
}

/// Parses an S10 Archive Patcher stage from a worker task id.
pub fn archive_patcher_stage_from_task_id(
    task_id: &WorkerTaskId,
) -> Option<ArchivePatcherWorkerStage> {
    archive_patcher_stage_and_request_id(task_id).map(|(stage, _)| stage)
}

fn archive_patcher_stage_and_request_id(
    task_id: &WorkerTaskId,
) -> Option<(ArchivePatcherWorkerStage, ArchivePatcherRequestId)> {
    let id = task_id.as_str();
    if let Some(value) = id.strip_prefix(ARCHIVE_PATCHER_CANDIDATES_TASK_PREFIX) {
        return value
            .parse::<ArchivePatcherRequestId>()
            .ok()
            .map(|request_id| (ArchivePatcherWorkerStage::Candidates, request_id));
    }
    if let Some(value) = id.strip_prefix(ARCHIVE_PATCHER_PLAN_TASK_PREFIX) {
        return value
            .parse::<ArchivePatcherRequestId>()
            .ok()
            .map(|request_id| (ArchivePatcherWorkerStage::Plan, request_id));
    }
    if let Some(value) = id.strip_prefix(ARCHIVE_PATCHER_PATCH_TASK_PREFIX) {
        return value
            .parse::<ArchivePatcherRequestId>()
            .ok()
            .map(|request_id| (ArchivePatcherWorkerStage::Patch, request_id));
    }
    if let Some(value) = id.strip_prefix(ARCHIVE_PATCHER_RESTORE_TASK_PREFIX) {
        return value
            .parse::<ArchivePatcherRequestId>()
            .ok()
            .map(|request_id| (ArchivePatcherWorkerStage::Restore, request_id));
    }
    None
}

fn archive_patcher_payload_matches_status(
    payload: &ArchivePatcherWorkerPayload,
    status: WorkerTaskStatus,
) -> bool {
    match payload {
        ArchivePatcherWorkerPayload::CandidatesLoaded { .. }
        | ArchivePatcherWorkerPayload::PlanReady { .. }
        | ArchivePatcherWorkerPayload::PatchCompleted { .. }
        | ArchivePatcherWorkerPayload::RestoreCompleted { .. } => {
            status == WorkerTaskStatus::Completed
        }
        ArchivePatcherWorkerPayload::LogRow { .. }
        | ArchivePatcherWorkerPayload::Progress { .. } => status == WorkerTaskStatus::Progress,
        ArchivePatcherWorkerPayload::SafeFailure { .. } => status == WorkerTaskStatus::Failed,
    }
}

fn parse_target_value(value: &str) -> Option<ArchivePatcherTarget> {
    match normalized_ui_value(value).as_str() {
        "v1_(og)" | "v1_og" | "old_gen" | "oldgen" => Some(ArchivePatcherTarget::OldGen),
        "v8_(ng)" | "v8_ng" | "next_gen" | "nextgen" => Some(ArchivePatcherTarget::NextGen),
        _ => None,
    }
}

fn effective_filter(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn normalized_ui_value(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            archive_patcher::{
                ArchivePatcherArchiveFormat, ArchivePatcherExecutionFileResult,
                ArchivePatcherExecutionOutcome, ArchivePatcherHeader, ArchivePatcherPlanAction,
                ArchivePatcherRestoreManifestEntry, ArchivePatcherSummaryCounts,
                TARGET_NEXT_GEN_LABEL, TARGET_OLD_GEN_LABEL, ba2_header_prefix,
                nothing_to_do_log_row, patched_to_target_log_row, restore_complete_message,
                restored_to_original_log_row,
            },
            discovery::{ArchiveFormat, ArchiveVersion},
        },
        workers::WorkerMessage,
    };

    fn archive_record(path: &str, version: ArchiveVersion) -> ArchiveRecord {
        ArchiveRecord::new(path, ArchiveFormat::General, version, true)
    }

    fn records() -> Vec<ArchiveRecord> {
        vec![
            archive_record("Game/Data/A.ba2", ArchiveVersion::NextGen8),
            archive_record("Game/Data/B.ba2", ArchiveVersion::NextGen7),
            archive_record("Game/Data/Old.ba2", ArchiveVersion::OldGen),
        ]
    }

    fn candidate_row(name: &str, target: ArchivePatcherTarget) -> ArchivePatcherCandidateRow {
        let version = match target {
            ArchivePatcherTarget::OldGen => ArchiveVersion::NextGen8,
            ArchivePatcherTarget::NextGen => ArchiveVersion::OldGen,
        };
        ArchivePatcherCandidateRow::new(
            format!("Game/Data/{name}"),
            name,
            ArchiveFormat::General,
            version,
            target,
        )
    }

    fn candidate_snapshot(
        request_id: ArchivePatcherRequestId,
        target: ArchivePatcherTarget,
        filter: Option<&str>,
        names: &[&str],
    ) -> ArchivePatcherCandidateSnapshot {
        ArchivePatcherCandidateSnapshot::new(
            request_id,
            target,
            filter.map(str::to_owned),
            names
                .iter()
                .map(|name| candidate_row(name, target))
                .collect(),
        )
    }

    fn plan(
        request_id: ArchivePatcherRequestId,
        target: ArchivePatcherTarget,
        filter: Option<&str>,
        can_execute: bool,
    ) -> ArchivePatcherPreviewPlan {
        let candidate = candidate_row("A.ba2", target);
        let candidates = ArchivePatcherCandidateSnapshot::new(
            request_id,
            target,
            filter.map(str::to_owned),
            vec![candidate.clone()],
        );
        let current_version = match target {
            ArchivePatcherTarget::OldGen => 8,
            ArchivePatcherTarget::NextGen => 1,
        };
        let header =
            ArchivePatcherHeader::new(current_version, ArchivePatcherArchiveFormat::General);
        let manifest_entry = ArchivePatcherRestoreManifestEntry::new(
            candidate.path.clone(),
            "A.ba2",
            "A.ba2",
            ArchivePatcherArchiveFormat::General,
            current_version,
            target.target_header_value(),
        )
        .with_header_prefixes(
            ba2_header_prefix(current_version, ArchivePatcherArchiveFormat::General),
            ba2_header_prefix(
                target.target_header_value(),
                ArchivePatcherArchiveFormat::General,
            ),
        );
        let row = if can_execute {
            ArchivePatcherPreviewPlanRow::patch(candidate, header, manifest_entry)
        } else {
            ArchivePatcherPreviewPlanRow::failure(candidate, Some(header), "Cannot patch A.ba2")
        };
        ArchivePatcherPreviewPlan::from_rows(
            request_id,
            target,
            filter.map(str::to_owned),
            Some(PathBuf::from("Game/Data")),
            candidates,
            vec![row],
        )
    }

    fn patch_result(
        request_id: ArchivePatcherRequestId,
        target: ArchivePatcherTarget,
    ) -> ArchivePatcherExecutionResult {
        let log_row = patched_to_target_log_row(target, "A.ba2");
        ArchivePatcherExecutionResult {
            request_id,
            target,
            manifest_path: PathBuf::from("State/archive-patcher-latest.json"),
            plan_digest: "digest".to_owned(),
            rows: vec![ArchivePatcherExecutionFileResult {
                archive_path: PathBuf::from("Game/Data/A.ba2"),
                file_name: "A.ba2".to_owned(),
                outcome: ArchivePatcherExecutionOutcome::Patched,
                log_row: log_row.clone(),
                diagnostics: Vec::new(),
            }],
            log_rows: vec![
                log_row,
                ArchivePatcherLogRow::new(
                    ArchivePatcherLogLevel::Info,
                    ArchivePatcherSummaryCounts::patch(1, 0).patching_complete_message(),
                ),
            ],
            counts: ArchivePatcherSummaryCounts::patch(1, 0),
            diagnostics: Vec::new(),
        }
    }

    fn restore_result(request_id: ArchivePatcherRequestId) -> ArchivePatcherExecutionResult {
        let log_row = restored_to_original_log_row(8, "A.ba2");
        ArchivePatcherExecutionResult {
            request_id,
            target: ArchivePatcherTarget::OldGen,
            manifest_path: PathBuf::from("State/archive-patcher-latest.json"),
            plan_digest: "digest".to_owned(),
            rows: vec![ArchivePatcherExecutionFileResult {
                archive_path: PathBuf::from("Game/Data/A.ba2"),
                file_name: "A.ba2".to_owned(),
                outcome: ArchivePatcherExecutionOutcome::Restored,
                log_row: log_row.clone(),
                diagnostics: Vec::new(),
            }],
            log_rows: vec![
                log_row,
                ArchivePatcherLogRow::new(
                    ArchivePatcherLogLevel::Info,
                    restore_complete_message(1, 0, 0),
                ),
            ],
            counts: ArchivePatcherSummaryCounts::restore(1, 0, 0),
            diagnostics: Vec::new(),
        }
    }

    fn open_controller(
        manifest_available: bool,
    ) -> (
        ArchivePatcherController,
        ArchivePatcherCandidateWorkerRequest,
    ) {
        let mut controller = ArchivePatcherController::new();
        let request = controller
            .open(
                records(),
                Some(PathBuf::from("Game/Data")),
                PathBuf::from("State/archive-patcher-latest.json"),
                manifest_available,
            )
            .expect("open should request candidates");
        (controller, request)
    }

    fn load_default_candidates(
        controller: &mut ArchivePatcherController,
        request: &ArchivePatcherCandidateWorkerRequest,
    ) {
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                request.task.clone(),
                archive_patcher_candidates_loaded_payload(
                    request.request_id,
                    candidate_snapshot(
                        request.request_id,
                        ArchivePatcherTarget::OldGen,
                        None,
                        &["A.ba2", "B.ba2"],
                    ),
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
    }

    #[test]
    fn archive_patcher_controller_open_loads_candidates_and_ignores_stale_payloads() {
        let (mut controller, first) = open_controller(false);
        let second = controller
            .open(
                records(),
                Some(PathBuf::from("Game/Data")),
                PathBuf::from("State/archive-patcher-latest.json"),
                false,
            )
            .expect("second open should replace candidate request");

        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::LoadingCandidates
        );
        assert_eq!(controller.active_request_id(), Some(second.request_id));
        assert_eq!(
            controller.latest_candidate_request_id(),
            Some(second.request_id)
        );
        assert_eq!(controller.next_request_id(), 3);
        assert_eq!(second.task.id.as_str(), "s10-archive-patcher-candidates:2");
        assert_eq!(second.task.kind, WorkerTaskKind::Patch);
        assert!(!controller.patch_button_enabled());
        assert!(controller.close_enabled());

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                first.task,
                archive_patcher_candidates_loaded_payload(
                    first.request_id,
                    candidate_snapshot(
                        first.request_id,
                        ArchivePatcherTarget::OldGen,
                        None,
                        &["Old.ba2"]
                    ),
                ),
            )),
            ArchivePatcherTransitionResult::StaleIgnored
        );
        assert!(controller.candidate_rows().is_empty());

        load_default_candidates(&mut controller, &second);
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::Ready);
        assert_eq!(controller.active_request_id(), None);
        assert_eq!(controller.candidate_rows().len(), 2);
        assert_eq!(controller.log_rows().len(), 1);
        assert_eq!(
            controller.log_rows()[0].message,
            "Showing 2 files to be patched."
        );
        assert!(controller.patch_button_enabled());
        assert!(!controller.restore_button_enabled());
    }

    #[test]
    fn archive_patcher_controller_plan_confirmation_gates_patch_run() {
        let (mut controller, load_request) = open_controller(false);
        load_default_candidates(&mut controller, &load_request);

        assert!(controller.confirm_plan().is_none());
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::Ready);

        let plan_request = match controller.request_patch_all().expect("first click plans") {
            ArchivePatcherPatchAllRequest::PreviewPlan(request) => request,
            ArchivePatcherPatchAllRequest::ConfirmedPatch(_) => {
                panic!("first click must not mutate")
            }
        };
        assert_eq!(plan_request.request_id, 2);
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::Planning);
        assert_eq!(
            controller.active_stage(),
            Some(ArchivePatcherWorkerStage::Plan)
        );
        assert!(!controller.patch_button_enabled());
        assert!(controller.confirm_plan().is_none());

        let preview = plan(
            plan_request.request_id,
            plan_request.target,
            plan_request.name_filter.as_deref(),
            true,
        );
        let expected_digest = preview.stable_digest();
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                plan_request.task.clone(),
                archive_patcher_plan_ready_payload(plan_request.request_id, preview),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::PlanReady);
        assert_eq!(controller.preview_plan_rows().len(), 1);
        assert!(controller.patch_button_enabled());
        assert_eq!(controller.status_text(), ARCHIVE_PATCHER_PLAN_READY_MESSAGE);

        let patch_request = controller.confirm_plan().expect("visible plan confirms");
        assert_eq!(patch_request.request_id, 3);
        assert_eq!(patch_request.confirmed_plan_request_id, 2);
        assert_eq!(patch_request.confirmed_plan_digest, expected_digest);
        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::PatchRunning
        );
        assert_eq!(
            controller.active_stage(),
            Some(ArchivePatcherWorkerStage::Patch)
        );
        assert!(!controller.close_enabled());
    }

    #[test]
    fn archive_patcher_controller_disables_write_controls_blocks_close_and_completes_patch() {
        let (mut controller, load_request) = open_controller(false);
        load_default_candidates(&mut controller, &load_request);
        let plan_request = match controller.request_patch_all().expect("plan") {
            ArchivePatcherPatchAllRequest::PreviewPlan(request) => request,
            ArchivePatcherPatchAllRequest::ConfirmedPatch(_) => panic!("unexpected run"),
        };
        controller.plan_ready(
            plan_request.request_id,
            plan(plan_request.request_id, plan_request.target, None, true),
        );
        let patch_request = controller.confirm_plan().expect("patch");

        assert!(!controller.write_controls_enabled());
        assert!(!controller.patch_button_enabled());
        assert!(!controller.restore_button_enabled());
        assert_eq!(
            controller.request_close(),
            ArchivePatcherTransitionResult::CloseBlocked
        );
        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::PatchRunning
        );

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                archive_patcher_patch_task(99),
                archive_patcher_patch_completed_payload(99, patch_result(99, patch_request.target)),
            )),
            ArchivePatcherTransitionResult::StaleIgnored
        );
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::new(
                patch_request.task.clone(),
                WorkerTaskStatus::Progress,
                archive_patcher_progress_payload(
                    patch_request.request_id,
                    ArchivePatcherWorkerStage::Patch,
                    ArchivePatcherProgress::new("Half done", 50.0),
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.progress().percent, 50.0);

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                patch_request.task.clone(),
                archive_patcher_patch_completed_payload(
                    patch_request.request_id,
                    patch_result(patch_request.request_id, patch_request.target),
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::Completed);
        assert_eq!(controller.active_request_id(), None);
        assert!(controller.close_enabled());
        assert!(controller.patch_button_enabled());
        assert!(controller.restore_button_enabled());
        assert!(controller.manifest_available());
        assert_eq!(controller.progress().percent, 100.0);
        assert_eq!(
            controller.progress().text,
            "Patching complete. 1 Successful, 0 Failed."
        );
    }

    #[test]
    fn archive_patcher_controller_filter_and_target_changes_recandidate_and_invalidate_plan() {
        let (mut controller, load_request) = open_controller(false);
        load_default_candidates(&mut controller, &load_request);
        let plan_request = match controller.request_patch_all().expect("plan") {
            ArchivePatcherPatchAllRequest::PreviewPlan(request) => request,
            ArchivePatcherPatchAllRequest::ConfirmedPatch(_) => panic!("unexpected run"),
        };
        let old_plan = plan(plan_request.request_id, plan_request.target, None, true);
        controller.plan_ready(plan_request.request_id, old_plan.clone());
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::PlanReady);
        assert!(controller.plan().is_some());

        let filter_request = controller
            .set_name_filter("A")
            .expect("filter change should request candidates");
        assert_eq!(filter_request.request_id, 3);
        assert_eq!(filter_request.name_filter.as_deref(), Some("A"));
        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::LoadingCandidates
        );
        assert!(controller.plan().is_none());
        assert!(controller.candidate_rows().is_empty());
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                plan_request.task,
                archive_patcher_plan_ready_payload(plan_request.request_id, old_plan),
            )),
            ArchivePatcherTransitionResult::StaleIgnored
        );
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                filter_request.task.clone(),
                archive_patcher_candidates_loaded_payload(
                    filter_request.request_id,
                    candidate_snapshot(
                        filter_request.request_id,
                        ArchivePatcherTarget::OldGen,
                        Some("A"),
                        &["A.ba2"]
                    ),
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.candidate_rows().len(), 1);
        assert_eq!(controller.name_filter(), "A");

        let target_request = controller
            .set_target_from_ui_value(TARGET_NEXT_GEN_LABEL)
            .expect("target change should request candidates");
        assert_eq!(target_request.target, ArchivePatcherTarget::NextGen);
        assert_eq!(controller.target(), ArchivePatcherTarget::NextGen);
        assert!(controller.candidate_rows().is_empty());
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                target_request.task.clone(),
                archive_patcher_candidates_loaded_payload(
                    target_request.request_id,
                    candidate_snapshot(
                        target_request.request_id,
                        ArchivePatcherTarget::NextGen,
                        Some("A"),
                        &["Old.ba2"]
                    ),
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.candidate_rows().len(), 1);
        assert_eq!(
            parse_target_value(TARGET_OLD_GEN_LABEL),
            Some(ArchivePatcherTarget::OldGen)
        );
    }

    #[test]
    fn archive_patcher_controller_spawn_failure_visible_safe_retry_and_restore_availability() {
        let (mut controller, load_request) = open_controller(true);
        load_default_candidates(&mut controller, &load_request);
        assert!(controller.restore_button_enabled());
        let restore_request = controller
            .request_restore_last_run()
            .expect("available manifest should request restore");
        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::RestoreRunning
        );
        assert!(!controller.write_controls_enabled());

        let result = controller.spawn_failed(
            ArchivePatcherWorkerRequestKind::Restore,
            restore_request.request_id,
            WorkerSpawnError::NoActiveRuntime {
                task_id: restore_request.task.id.clone(),
            },
        );

        assert_eq!(result, ArchivePatcherTransitionResult::Applied);
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::SafeError);
        assert_eq!(
            controller.safe_error(),
            Some(ARCHIVE_PATCHER_RESTORE_START_FAILED_MESSAGE)
        );
        assert!(controller.close_enabled());
        assert!(controller.restore_button_enabled());
        assert!(controller.log_rows().iter().any(|row| {
            row.level == ArchivePatcherLogLevel::Bad
                && row.message == ARCHIVE_PATCHER_RESTORE_START_FAILED_MESSAGE
        }));

        let retry = controller
            .request_restore_last_run()
            .expect("safe error state should allow restore retry");
        assert_eq!(retry.request_id, restore_request.request_id + 1);
        assert_eq!(
            controller.phase(),
            ArchivePatcherControllerPhase::RestoreRunning
        );
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                retry.task.clone(),
                archive_patcher_restore_completed_payload(
                    retry.request_id,
                    restore_result(retry.request_id)
                ),
            )),
            ArchivePatcherTransitionResult::Applied
        );
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::Completed);
        assert_eq!(
            controller.progress().text,
            "Restore complete. 1 Successful, 0 Skipped, 0 Failed."
        );
    }

    #[test]
    fn archive_patcher_controller_restore_without_manifest_and_non_archive_events_are_safe() {
        let (mut controller, load_request) = open_controller(false);
        load_default_candidates(&mut controller, &load_request);

        assert!(controller.request_restore_last_run().is_none());
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::SafeError);
        assert_eq!(
            controller.safe_error(),
            Some(ARCHIVE_PATCHER_RESTORE_UNAVAILABLE_MESSAGE)
        );
        assert!(!controller.restore_button_enabled());

        let ignored = controller.handle_worker_event(WorkerEvent::completed(
            WorkerTask::new("not-archive-patcher", WorkerTaskKind::Generic),
            WorkerPayload::Generic(WorkerMessage::new("not for this controller")),
        ));
        assert_eq!(ignored, ArchivePatcherTransitionResult::Ignored);
        assert_eq!(controller.phase(), ArchivePatcherControllerPhase::SafeError);

        let bad_plan = plan(100, ArchivePatcherTarget::OldGen, None, false);
        assert_eq!(
            bad_plan.rows[0].action,
            ArchivePatcherPlanAction::PlanFailure
        );
        assert_eq!(
            bad_plan.summary_log_row.message,
            "Showing 1 files to be patched."
        );
        assert_eq!(nothing_to_do_log_row().message, "Nothing to do!");
    }

    #[test]
    fn archive_patcher_task_id_parsing_and_payload_helpers_are_stable() {
        let candidates = archive_patcher_candidates_task(7);
        let plan_task = archive_patcher_plan_task(8);
        let patch = archive_patcher_patch_task(9);
        let restore = archive_patcher_restore_task(10);

        assert_eq!(
            archive_patcher_request_id_from_task_id(&candidates.id),
            Some(7)
        );
        assert_eq!(
            archive_patcher_stage_from_task_id(&candidates.id),
            Some(ArchivePatcherWorkerStage::Candidates)
        );
        assert_eq!(
            archive_patcher_request_id_from_task_id(&plan_task.id),
            Some(8)
        );
        assert_eq!(
            archive_patcher_stage_from_task_id(&plan_task.id),
            Some(ArchivePatcherWorkerStage::Plan)
        );
        assert_eq!(archive_patcher_request_id_from_task_id(&patch.id), Some(9));
        assert_eq!(
            archive_patcher_stage_from_task_id(&patch.id),
            Some(ArchivePatcherWorkerStage::Patch)
        );
        assert_eq!(
            archive_patcher_request_id_from_task_id(&restore.id),
            Some(10)
        );
        assert_eq!(
            archive_patcher_stage_from_task_id(&restore.id),
            Some(ArchivePatcherWorkerStage::Restore)
        );

        assert!(matches!(
            archive_patcher_log_row_payload(
                9,
                ArchivePatcherWorkerStage::Patch,
                ArchivePatcherLogRow::new(ArchivePatcherLogLevel::Info, "hello"),
            ),
            WorkerPayload::ArchivePatcher(ArchivePatcherWorkerPayload::LogRow {
                request_id: 9,
                ..
            })
        ));
        assert_eq!(
            parse_target_value(TARGET_NEXT_GEN_LABEL),
            Some(ArchivePatcherTarget::NextGen)
        );
    }
}
